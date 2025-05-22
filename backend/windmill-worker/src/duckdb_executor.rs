use std::collections::HashMap;
use std::env;

use duckdb::types::TimeUnit;
use duckdb::{params_from_iter, Row};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use tokio::fs::remove_file;
use tokio::task;
use uuid::Uuid;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::s3_helpers::{DuckdbConnectionSettingsQueryV2, S3Object, S3ResourceInfoQuery};
use windmill_common::worker::{to_raw_value, Connection};
use windmill_parser_sql::{parse_duckdb_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::common::{build_args_values, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
#[cfg(feature = "mysql")]
use crate::mysql_executor::MysqlDatabase;
use crate::pg_executor::PgDatabase;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::AuthedClient;

fn do_duckdb_inner(
    conn: &duckdb::Connection,
    query: &str,
    job_args: &HashMap<String, duckdb::types::Value>,
    skip_collect: bool,
    column_order: &mut Option<Vec<String>>,
) -> Result<Box<RawValue>> {
    let mut rows_vec = vec![];

    let (query, job_args) = interpolate_named_args(query, &job_args);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let mut rows = stmt
        .query(params_from_iter(job_args))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    if skip_collect {
        return Ok(to_raw_value(&json!([])));
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

pub async fn do_duckdb(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order_ref: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>> {
    let result_f = async {
        let mut s3_conn_strings: Vec<String> = vec![];

        let sig = parse_duckdb_sig(query)?.args;
        let mut job_args = build_args_values(job, client, conn).await?;

        let (query, _) = &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args)?;
        // Prevent interpolate_named_args from detecting argument identifiers in the signature for
        // the first query block
        let query = trunc_sig(query);

        let job_args = {
            let mut m: HashMap<String, duckdb::types::Value> = HashMap::new();
            for sig_arg in sig.into_iter() {
                let json_value = job_args
                    .remove(&sig_arg.name)
                    .or_else(|| sig_arg.default)
                    .unwrap_or_else(|| json!(null));

                if matches!(&sig_arg.otyp.as_ref().map(String::as_str), Some("s3object")) {
                    let s3_obj = serde_json::from_value::<S3Object>(json_value).map_err(|e| {
                        Error::ExecutionErr(format!("Failed to deserialize S3Object: {}", e))
                    })?;
                    let s3_info = client
                        .get_s3_resource_info(&S3ResourceInfoQuery {
                            s3_resource_path: None,
                            storage: s3_obj.storage.clone(),
                        })
                        .await?;
                    let s3_conn_str = client
                        .get_duckdb_connection_settings(DuckdbConnectionSettingsQueryV2 {
                            s3_resource_path: None,
                            storage: s3_obj.storage,
                        })
                        .await?;

                    let s3_obj_string = format!("s3://{}/{}", &s3_info.bucket, &s3_obj.s3);
                    m.insert(sig_arg.name, duckdb::types::Value::Text(s3_obj_string));
                    s3_conn_strings.push(s3_conn_str.connection_settings_str);
                } else {
                    let duckdb_value = json_value_to_duckdb_value(
                        &json_value,
                        sig_arg
                            .otyp
                            .clone()
                            .unwrap_or_else(|| "text".to_string())
                            .as_str(),
                        client,
                    )?;
                    m.insert(sig_arg.name, duckdb_value);
                }
            }
            m
        };

        let query_block_list = parse_sql_blocks(query);

        // Replace windmill resource ATTACH statements with the real instructions
        let query_block_list = {
            let mut v = vec![];
            for query_block in query_block_list.iter() {
                match parse_attach_db_resource(query_block) {
                    Some(parsed) => v.extend(
                        transform_attach_db_resource_query(&parsed, &job.id, client).await?,
                    ),
                    None => v.push(query_block.to_string()),
                };
            }
            v
        };

        let bq_credentials_path = make_bq_credentials_path(&job.id);
        env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &bq_credentials_path);

        // duckdb::Connection is not send so we do it in a single blocking task
        let (result, column_order) = task::spawn_blocking(move || {
            let conn = duckdb::Connection::open_in_memory()
                .map_err(|e| Error::ConnectingToDatabase(e.to_string()))?;

            for s3_conn_string in s3_conn_strings {
                conn.execute_batch(&s3_conn_string)
                    .map_err(|e| Error::ExecutionErr(e.to_string()))?;
            }

            let mut result: Option<Box<RawValue>> = None;
            let mut column_order = None;
            for (query_block_index, query_block) in query_block_list.iter().enumerate() {
                result = Some(
                    do_duckdb_inner(
                        &conn,
                        query_block.as_str(),
                        &job_args,
                        query_block_index != query_block_list.len() - 1,
                        &mut column_order,
                    )
                    .map_err(|e| Error::ExecutionErr(e.to_string()))?,
                );
            }
            let result = result.unwrap_or_else(|| to_raw_value(&json!([])));
            Ok::<_, Error>((result, column_order))
        })
        .await
        .map_err(to_anyhow)??;

        *column_order_ref = column_order;

        if matches!(tokio::fs::try_exists(&bq_credentials_path).await, Ok(true)) {
            remove_file(&bq_credentials_path).await.map_err(to_anyhow)?;
        }
        Ok(result)
    };

    let result = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        result_f,
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;

    Ok(result)
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
    client: &AuthedClient,
) -> Result<duckdb::types::Value> {
    let arg_type = arg_type.to_lowercase();
    let duckdb_value = match json_value {
        serde_json::Value::Null => duckdb::types::Value::Null,
        serde_json::Value::Bool(b) => duckdb::types::Value::Boolean(*b),

        serde_json::Value::String(s)
            if matches!(
                arg_type.as_str(),
                "timestamp" | "timestamptz" | "timestamp with time zone" | "datetime"
            ) =>
        {
            string_to_duckdb_timestamp(&s)?
        }
        serde_json::Value::String(s) if arg_type.as_str() == "date" => string_to_duckdb_date(&s)?,
        serde_json::Value::String(s) if arg_type.as_str() == "time" => string_to_duckdb_time(&s)?,
        serde_json::Value::String(s) => duckdb::types::Value::Text(s.clone()),

        serde_json::Value::Number(n) if n.is_i64() => {
            let v = n.as_i64().unwrap();
            match arg_type.as_str() {
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
            match arg_type.as_str() {
                "utinyint" => duckdb::types::Value::UTinyInt(v as u8),
                "usmallint" => duckdb::types::Value::USmallInt(v as u16),
                "uinteger" => duckdb::types::Value::UInt(v as u32),
                "ubigint" | "uhugeint" => duckdb::types::Value::UBigInt(v),
                _ => duckdb::types::Value::UBigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_f64() => {
            let v = n.as_f64().unwrap();
            match arg_type.as_str() {
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
                .map(|val| json_value_to_duckdb_value(val, arg_type.as_str(), client))
                .collect::<Result<Vec<_>>>()?,
        ),
        serde_json::Value::Object(map) => duckdb::types::Value::Struct(
            map.iter()
                .map(|(k, v)| {
                    Ok::<_, Error>((
                        k.clone(),
                        json_value_to_duckdb_value(v, arg_type.as_str(), client)?,
                    ))
                })
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

struct ParsedAttachDbResource<'a> {
    resource_path: &'a str,
    name: &'a str,
    db_type: &'a str,
    extra_args: Option<&'a str>,
}
fn parse_attach_db_resource<'a>(query: &'a str) -> Option<ParsedAttachDbResource<'a>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"ATTACH '\$res:([^']+)' AS (\S+) \(TYPE (\w+)(.*)\)").unwrap();
    }

    for cap in RE.captures_iter(query) {
        if let (Some(resource_path), Some(name), Some(db_type)) =
            (cap.get(1), cap.get(2), cap.get(3))
        {
            let extra_args = cap.get(4).map(|m| query[m.start()..m.end()].trim());
            return Some(ParsedAttachDbResource {
                resource_path: query[resource_path.start()..resource_path.end()].trim(),
                name: query[name.start()..name.end()].trim(),
                db_type: query[db_type.start()..db_type.end()].trim(),
                extra_args,
            });
        }
    }
    None
}

async fn transform_attach_db_resource_query(
    parsed: &ParsedAttachDbResource<'_>,
    job_id: &Uuid,
    client: &AuthedClient,
) -> Result<Vec<String>> {
    match parsed.db_type.to_lowercase().as_str() {
        "postgres" => {
            let resource: PgDatabase = client
                .get_resource_value_interpolated(parsed.resource_path, Some(job_id.to_string()))
                .await?;

            let attach_str = format!(
                "ATTACH 'dbname={} {} host={} {} {}' AS {} (TYPE postgres{});",
                resource.dbname,
                resource
                    .user
                    .map(|u| format!("user={}", u))
                    .unwrap_or_default(),
                resource.host,
                resource
                    .password
                    .map(|p| format!("password={}", p))
                    .unwrap_or_default(),
                resource
                    .port
                    .map(|p| format!("port={}", p))
                    .unwrap_or_default(),
                parsed.name,
                parsed.extra_args.unwrap_or("")
            );

            Ok(vec![
                "INSTALL postgres;".to_string(),
                "LOAD postgres;".to_string(),
                attach_str,
            ])
        }
        "mysql" => {
            #[cfg(not(feature = "mysql"))]
            return Err(Error::ExecutionErr(
                "MySQL feature is not enabled".to_string(),
            ));

            #[cfg(feature = "mysql")]
            {
                let resource: MysqlDatabase = client
                    .get_resource_value_interpolated(parsed.resource_path, Some(job_id.to_string()))
                    .await?;

                let attach_str = format!(
                    "ATTACH 'database={} host={} ssl_mode={} {} {} {}' AS {} (TYPE mysql{});",
                    resource.database,
                    resource.host,
                    resource
                        .ssl
                        .map(|ssl| if ssl { "required" } else { "disabled" })
                        .unwrap_or("preferred"),
                    resource
                        .password
                        .map(|p| format!("password={}", p))
                        .unwrap_or_default(),
                    resource
                        .port
                        .map(|p| format!("port={}", p))
                        .unwrap_or_default(),
                    resource
                        .user
                        .map(|u| format!("user={}", u))
                        .unwrap_or_default(),
                    parsed.name,
                    parsed.extra_args.unwrap_or("")
                );

                Ok(vec![
                    "INSTALL mysql;".to_string(),
                    "LOAD mysql;".to_string(),
                    attach_str,
                ])
            }
        }
        "bigquery" => {
            let resource: Value = client
                .get_resource_value_interpolated(parsed.resource_path, Some(job_id.to_string()))
                .await?;
            // duckdb's bigquery extension requires a json file as credentials
            let bq_credentials_path = make_bq_credentials_path(job_id);
            tokio::fs::write(&bq_credentials_path, resource.to_string())
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!(
                        "Failed to write BigQuery credentials to {}: {}",
                        &bq_credentials_path, e
                    ))
                })?;
            let project_id: String = serde_json::from_value(
                resource
                    .get("project_id")
                    .ok_or_else(|| {
                        Error::ExecutionErr("BigQuery resource must contain project_id".to_string())
                    })?
                    .to_owned(),
            )
            .map_err(|_e| Error::ExecutionErr("failed project_id deserialize".to_string()))?;
            let attach_str = format!(
                "ATTACH 'project={}' as {} (TYPE bigquery{});",
                project_id,
                parsed.name,
                parsed.extra_args.unwrap_or("")
            )
            .to_string();
            Ok(vec![
                "INSTALL bigquery FROM community;".to_string(),
                "LOAD bigquery;".to_string(),
                attach_str,
            ])
        }
        _ => Err(Error::ExecutionErr(format!(
            "Unsupported db type in DuckDB ATTACH: {}",
            parsed.db_type
        ))),
    }
}

// BigQuery extension requires a json file as credentials
// The file path is set as an env var by do_duckdb
// It is created by transform_attach_db_resource_query (when bigquery is detected)
// and deleted by do_duckdb after the query is executed
fn make_bq_credentials_path(job_id: &Uuid) -> String {
    format!("/tmp/service-account-credentials-{}.json", job_id)
}

// duckdb-rs does not support named parameters,
// and it raises an error when passing unused arguments. We cannot prepare batch statements
// but only single SQL statements so it doesn't work when all arguments are not used by
// every single statement.
fn interpolate_named_args<'a>(
    query: &str,
    args: &'a HashMap<String, duckdb::types::Value>,
) -> (String, Vec<&'a duckdb::types::Value>) {
    let mut query = query.to_string();

    let mut values = vec![];
    for (arg_name, arg_value) in args {
        let pat = format!("${}", arg_name);
        if !query.contains(&pat) {
            continue;
        }
        values.push(arg_value);
        query = query.replace(&pat, &format!("${}", values.len()));
    }
    (query, values)
}

fn trunc_sig(query: &str) -> &str {
    let idx = query.rfind("-- $").unwrap_or(query.len());
    // find next \n starting from idx and return everything after it
    let idx = query[idx..].find('\n').map(|i| i + idx).unwrap_or(0);
    &query[idx..]
}
