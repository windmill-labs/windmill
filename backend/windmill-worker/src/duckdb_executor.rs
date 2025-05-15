use std::sync::Arc;

use duckdb::Row;
use futures::StreamExt;
use serde_json::json;
use serde_json::value::RawValue;
use tokio::sync::Mutex;
use windmill_common::error::Error;
use windmill_common::worker::{to_raw_value, Connection};
use windmill_parser_sql::{parse_duckdb_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::common::{build_args_map, OccupancyMetrics};
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
) -> windmill_common::error::Result<Box<RawValue>> {
    let job_args = build_args_map(job, client, conn).await?;
    let query_block_list = parse_sql_blocks(query);

    let conn = duckdb::Connection::open_in_memory()
        .map_err(|e| Error::ConnectingToDatabase(e.to_string()))?;

    for (query_block_index, query_block) in query_block_list.iter().enumerate() {
        let mut rows_vec = vec![];

        let mut stmt = conn
            .prepare(&query_block)
            .map_err(|e| Error::ExecutionErr(e.to_string()))?;

        let mut rows = stmt
            .query([])
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

fn row_to_value(
    row: &Row<'_>,
    column_names: &[String],
) -> windmill_common::error::Result<Box<RawValue>> {
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
