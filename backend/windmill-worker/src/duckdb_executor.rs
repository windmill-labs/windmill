use duckdb::types::TimeUnit;
use duckdb::{params_from_iter, Row};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::json;
use serde_json::value::RawValue;
use windmill_common::error::{Error, Result};
use windmill_common::s3_helpers::DuckdbConnectionSettingsQueryV2;
use windmill_common::worker::{to_raw_value, Connection};
use windmill_parser_sql::{parse_duckdb_sig, parse_s3_mode, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::common::{build_args_values, OccupancyMetrics};
use crate::AuthedClient;

pub async fn do_duckdb(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupation_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>> {
    let sig = parse_duckdb_sig(query)?.args;
    let job_args = build_args_values(job, client, conn).await?;
    let job_args = sig
        .iter()
        .map(|sig_arg| {
            let json_value = job_args
                .get(&sig_arg.name)
                .or_else(|| sig_arg.default.as_ref())
                .unwrap_or_else(|| &json!(null));
            let duckdb_value = json_value_to_duckdb_value(
                json_value,
                sig_arg
                    .otyp
                    .clone()
                    .unwrap_or_else(|| "text".to_string())
                    .as_str(),
            );
            duckdb_value
        })
        .collect::<Result<Vec<_>>>()?;

    let query_block_list = parse_sql_blocks(query);

    let conn = duckdb::Connection::open_in_memory()
        .map_err(|e| Error::ConnectingToDatabase(e.to_string()))?;

    if let Some(s3) = parse_s3_mode(query)? {
        let s3_conn_str = client
            .get_duckdb_connection_settings(
                DuckdbConnectionSettingsQueryV2 { s3_resource_path: None, storage: s3.storage },
                job.workspace_id.as_str(),
            )
            .await?;
        conn.execute_batch(s3_conn_str.connection_settings_str.as_str())
            .map_err(|e| Error::ConnectingToDatabase(e.to_string()))?;
    }

    for (query_block_index, query_block) in query_block_list.iter().enumerate() {
        let mut rows_vec = vec![];

        let mut stmt = conn
            .prepare(&query_block)
            .map_err(|e| Error::ExecutionErr(e.to_string()))?;

        let mut rows = stmt
            .query(params_from_iter(job_args.iter()))
            .map_err(|e| Error::ExecutionErr(e.to_string()))?;

        let is_last = query_block_index == query_block_list.len() - 1;
        if !is_last {
            continue;
        }
        // Statement needs to be stepped at least once or stmt.column_names() will panic
        let mut column_names = None;
        loop {
            let row = rows.next();
            match row {
                Ok(Some(row)) => {
                    // Set column names if not already set
                    let stmt = row.as_ref();
                    let column_names = match column_names.as_ref() {
                        Some(column_names) => column_names,
                        None => {
                            column_names = Some(stmt.column_names());
                            column_names.as_ref().unwrap()
                        }
                    };

                    let row = row_to_value(row, &column_names.as_slice())
                        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
                    rows_vec.push(row);
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(Error::ExecutionErr(e.to_string()));
                }
            }
        }

        if let (Some(column_order), Some(column_names)) = (column_order.as_mut(), column_names) {
            *column_order = column_names.clone();
        }

        return Ok(to_raw_value(&rows_vec));
    }
    Ok(to_raw_value(&json!([])))
}

fn row_to_value(row: &Row<'_>, column_names: &[String]) -> Result<Box<RawValue>> {
    let mut obj = serde_json::Map::new();
    for (i, key) in column_names.iter().enumerate() {
        let value: duckdb::types::Value =
            row.get(i).map_err(|e| Error::ExecutionErr(e.to_string()))?;
        let json_value = match value {
            duckdb::types::Value::Null => serde_json::Value::Null,
            duckdb::types::Value::Boolean(b) => serde_json::Value::Bool(b),
            duckdb::types::Value::TinyInt(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::SmallInt(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::Int(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::BigInt(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::HugeInt(i) => serde_json::Value::String(i.to_string()),
            duckdb::types::Value::UTinyInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::USmallInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::UInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::UBigInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::Float(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(f as f64)
                    .ok_or_else(|| Error::ExecutionErr("Could not convert to f64".to_string()))?,
            ),
            duckdb::types::Value::Double(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(f)
                    .ok_or_else(|| Error::ExecutionErr("Could not convert to f64".to_string()))?,
            ),
            duckdb::types::Value::Decimal(d) => serde_json::Value::String(d.to_string()),
            duckdb::types::Value::Timestamp(_, ts) => serde_json::Value::String(ts.to_string()),
            duckdb::types::Value::Text(s) => serde_json::Value::String(s),
            duckdb::types::Value::Blob(b) => serde_json::Value::Array(
                b.into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            ),
            duckdb::types::Value::Date32(d) => serde_json::Value::Number(d.into()),
            duckdb::types::Value::Time64(_, t) => serde_json::Value::String(t.to_string()),
            duckdb::types::Value::Interval { months, days, nanos } => serde_json::json!({
                "months": months,
                "days": days,
                "nanos": nanos
            }),
            duckdb::types::Value::List(values) => serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|v| serde_json::Value::String(format!("{:?}", v)))
                    .collect(),
            ),
            duckdb::types::Value::Enum(e) => serde_json::Value::String(e),
            duckdb::types::Value::Struct(fields) => serde_json::Value::Object(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(format!("{:?}", v))))
                    .collect(),
            ),
            duckdb::types::Value::Array(values) => serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|v| serde_json::Value::String(format!("{:?}", v)))
                    .collect(),
            ),
            duckdb::types::Value::Map(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| {
                        (
                            format!("{:?}", k),
                            serde_json::Value::String(format!("{:?}", v)),
                        )
                    })
                    .collect(),
            ),
            duckdb::types::Value::Union(value) => {
                serde_json::Value::String(format!("{:?}", *value))
            }
        };
        obj.insert(key.clone(), json_value);
    }
    serde_json::value::to_raw_value(&obj).map_err(|e| e.into())
}

fn json_value_to_duckdb_value(
    json_value: &serde_json::Value,
    arg_type: &str,
) -> Result<duckdb::types::Value> {
    let duckdb_value = match json_value {
        serde_json::Value::Null => duckdb::types::Value::Null,
        serde_json::Value::Bool(b) => duckdb::types::Value::Boolean(*b),

        serde_json::Value::String(s)
            if matches!(
                arg_type,
                "timestamp" | "timestamptz" | "timestamp with time zone" | "datetime"
            ) =>
        {
            string_to_duckdb_timestamp(&s)?
        }
        serde_json::Value::String(s) if arg_type == "date" => string_to_duckdb_date(&s)?,
        serde_json::Value::String(s) if arg_type == "time" => string_to_duckdb_time(&s)?,
        serde_json::Value::String(s) => duckdb::types::Value::Text(s.clone()),

        serde_json::Value::Number(n) if n.is_i64() => {
            let v = n.as_i64().unwrap();
            match arg_type {
                "tinyint" | "int1" => duckdb::types::Value::TinyInt(v as i8),
                "smallint" | "int2" | "short" => duckdb::types::Value::SmallInt(v as i16),
                "integer" | "int4" | "int" | "signed" => duckdb::types::Value::Int(v as i32),
                "bigint" | "int8" | "long" => duckdb::types::Value::BigInt(v),
                "hugeint" => duckdb::types::Value::HugeInt(v as i128),
                "float" | "float4" | "real" => duckdb::types::Value::Float(v as f32),
                "double" | "float8" => duckdb::types::Value::Double(v as f64),
                _ => duckdb::types::Value::BigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_u64() => {
            let v = n.as_u64().unwrap();
            match arg_type {
                "utinyint" => duckdb::types::Value::UTinyInt(v as u8),
                "usmallint" => duckdb::types::Value::USmallInt(v as u16),
                "uinteger" => duckdb::types::Value::UInt(v as u32),
                "ubigint" | "uhugeint" => duckdb::types::Value::UBigInt(v),
                _ => duckdb::types::Value::UBigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_f64() => {
            let v = n.as_f64().unwrap();
            match arg_type {
                "float" | "float4" | "real" => duckdb::types::Value::Float(v as f32),
                "double" | "float8" => duckdb::types::Value::Double(v),
                "decimal" | "numeric" => {
                    duckdb::types::Value::Decimal(Decimal::from_f64(v).unwrap())
                }
                _ => duckdb::types::Value::Double(v), // default fallback
            }
        }

        serde_json::Value::Array(arr) => duckdb::types::Value::Array(
            arr.iter()
                .map(|val| json_value_to_duckdb_value(val, arg_type))
                .collect::<Result<Vec<_>>>()?,
        ),

        serde_json::Value::Object(map) => duckdb::types::Value::Struct(
            map.iter()
                .map(|(k, v)| Ok((k.clone(), json_value_to_duckdb_value(v, arg_type)?)))
                .collect::<Result<Vec<_>>>()?
                .into(),
        ),

        value @ _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported type in query: {:?} and signature {arg_type:?}",
                value
            )))
        }
    };
    Ok(duckdb_value)
}

fn string_to_duckdb_timestamp(s: &str) -> Result<duckdb::types::Value> {
    let ts = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e: chrono::ParseError| Error::ExecutionErr(e.to_string()))?;
    Ok(duckdb::types::Value::Timestamp(
        TimeUnit::Millisecond,
        ts.timestamp_millis(),
    ))
}

fn string_to_duckdb_date(s: &str) -> Result<duckdb::types::Value> {
    use chrono::Datelike;
    let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
    Ok(duckdb::types::Value::Date32(date.num_days_from_ce()))
}

fn string_to_duckdb_time(s: &str) -> Result<duckdb::types::Value> {
    use chrono::Timelike;
    let time = chrono::NaiveTime::parse_from_str(s, "%H:%M:%S").unwrap();
    Ok(duckdb::types::Value::Time64(
        TimeUnit::Microsecond,
        time.num_seconds_from_midnight() as i64,
    ))
}
