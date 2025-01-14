use anyhow::anyhow;
use chrono::Utc;
use std::{str::FromStr, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;
use oracle::{
    sql_type::{InnerValue, OracleType, ToSql},
    ResultSet,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue, Value};
use sqlx::types::Json;
use tokio::sync::Mutex;
use windmill_common::{
    error::{to_anyhow, Error},
    jobs::QueuedJob,
    worker::to_raw_value,
};
use windmill_parser_sql::{parse_db_resource, parse_mysql_sig};
use windmill_queue::CanceledBy;

use crate::{
    common::{build_args_map, OccupancyMetrics},
    handle_child::run_future_with_polling_update_job_poller,
    AuthedClientBackgroundTask,
};

#[derive(Deserialize)]
struct OracleDatabase {
    user: String,
    password: String,
    database: String,
}

pub fn do_oracledb_inner<'a>(
    query: &'a str,
    params: Vec<(String, Box<dyn ToSql + Send + Sync>)>,
    conn2: oracle::Connection,
    column_order: Option<&'a mut Option<Vec<String>>>,
    skip_collect: bool,
) -> windmill_common::error::Result<BoxFuture<'a, anyhow::Result<Box<RawValue>>>> {
    let qw = query.to_string();
    let result_f = async move {
        if skip_collect {
            tokio::task::spawn_blocking(move || {
                let params2: Vec<(&str, &dyn ToSql)> = params
                    .iter()
                    .map(|(key, val)| (key.as_str(), &**val as &dyn ToSql))
                    .collect();
                conn2.execute_named(&qw, &params2)?;
                conn2.commit()
            })
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            Ok(to_raw_value(&Value::Array(vec![])))
        } else {
            let rows = tokio::task::spawn_blocking(move || {
                let params2: Vec<(&str, &dyn ToSql)> = params
                    .iter()
                    .map(|(key, val)| (key.as_str(), &**val as &dyn ToSql))
                    .collect();

                let mut stmt = conn2.statement(&qw).build()?;

                let rows = match stmt.statement_type() {
                    oracle::StatementType::Select => {
                        let result_rows = stmt.query_named(&params2)?;
                        let rows: Vec<oracle::Row> =
                            result_rows.into_iter().filter_map(Result::ok).collect_vec();
                        rows
                    }
                    _ => {
                        stmt.execute_named(&params2)?;
                        conn2.commit()?;
                        vec![]
                    }
                };

                oracle::Result::Ok(rows)
            })
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            if let Some(column_order) = column_order {
                *column_order = Some(
                    rows.first()
                        .map(|x| {
                            x.column_info()
                                .iter()
                                .map(|x| x.name().to_string())
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default(),
                );
            }

            Ok(to_raw_value(
                &rows
                    .into_iter()
                    .map(|x| convert_row_to_value(x))
                    .collect::<Vec<serde_json::Value>>(),
            ))
        }
    };

    Ok(result_f.boxed())
}

fn convert_row_to_value(row: oracle::Row) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for (key, value) in row.column_info().iter().zip(row.sql_values()) {
        map.insert(
            key.name().to_string(),
            convert_mysql_value_to_json(value, key.oracle_type()),
        );
    }
    serde_json::Value::Object(map)
}

fn conversion_error<T: Serialize>(r: Result<T, oracle::Error>) -> serde_json::Value {
    match r {
        Ok(v) => json!(v),
        Err(e) => json!(format!("Error converting value: {:?}", e)),
    }
}

fn convert_mysql_value_to_json(v: &oracle::SqlValue, c: &OracleType) -> serde_json::Value {
    tracing::error!(
        "converting to json:: {:?} - {:?} - {c:?}",
        v,
        v.as_inner_value()
    );
    match v.as_inner_value() {
        Err(_) => serde_json::Value::Null,
        Ok(iv) => match iv {
            InnerValue::Int64(n) => json!(n),
            InnerValue::UInt64(n) => json!(n),
            InnerValue::Float(n) => json!(n),
            InnerValue::Double(n) => json!(n),
            InnerValue::Char(n) => json!(String::from_utf8_lossy(n)),
            InnerValue::Number(n) => json!(n),
            InnerValue::Boolean(n) => json!(n),
            InnerValue::Timestamp(_) => conversion_error(v.get::<String>()),
            InnerValue::IntervalDS(_) => conversion_error(v.get::<String>()),
            InnerValue::IntervalYM(_) => conversion_error(v.get::<String>()),
            InnerValue::Clob(_) => json!("Unsupported type Clob"),
            InnerValue::Blob(_) => json!("Unsupported type Blob"),
            InnerValue::Rowid(_) => json!("Unsupported type Rowid"),
            InnerValue::Object(_) => json!("Unsuppported type Object"),
            InnerValue::Stmt(_) => json!("Unsupported type Stmt"),
            InnerValue::Raw(b) => {
                match c {
                    OracleType::Varchar2(_)
                    | OracleType::NVarchar2(_)
                    | OracleType::Char(_)
                    | OracleType::NChar(_)
                    | OracleType::Json
                    | OracleType::Xml => {
                        let s = String::from_utf8_lossy(b);
                        json!(s)
                    }

                    // OracleType::Rowid => todo!(),
                    // OracleType::Raw(_) => todo!(),
                    //
                    // OracleType::BinaryFloat => todo!(),
                    // OracleType::BinaryDouble => todo!(),
                    // OracleType::Number(_, _) => todo!(),
                    // OracleType::Float(_) => todo!(),
                    // OracleType::Int64 => todo!(),
                    // OracleType::UInt64 => todo!(),
                    // OracleType::Long => todo!(),
                    // OracleType::LongRaw => todo!(),
                    OracleType::Date
                    | OracleType::Timestamp(_)
                    | OracleType::TimestampTZ(_)
                    | OracleType::TimestampLTZ(_)
                    | OracleType::IntervalDS(_, _)
                    | OracleType::IntervalYM(_) => conversion_error(v.get::<String>()),

                    // OracleType::CLOB => todo!(),
                    // OracleType::NCLOB => todo!(),
                    // OracleType::BLOB => todo!(),
                    // OracleType::BFILE => todo!(),
                    // OracleType::RefCursor => todo!(),
                    // OracleType::Boolean => todo!(),
                    // OracleType::Object(_) => todo!(),
                    _ => json!(format!("Unsupported type: {c:?}")),
                }
            }
            _ => json!(format!("Unsupported type: {c:?}")),
        },
    }
}

pub async fn do_oracledb(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let inline_db_res_path = parse_db_resource(&query);

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        let val = client
            .get_authed()
            .await
            .get_resource_value_interpolated::<serde_json::Value>(
                &inline_db_res_path,
                Some(job.id.to_string()),
            )
            .await?;

        let as_raw = serde_json::from_value(val).map_err(|e| {
            Error::InternalErr(format!("Error while parsing inline resource: {e:#}"))
        })?;

        Some(as_raw)
    } else {
        job_args.and_then(|x| x.get("database").cloned())
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_str::<OracleDatabase>(db.get())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let sig = parse_mysql_sig(query)
    .map_err(|x| Error::ExecutionErr(x.to_string()))?
    .args;

    let mut statement_values = vec![];

    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "text".to_string());
        let arg_n = arg.name.clone();
        let oracle_v: Box<dyn ToSql + Send + Sync> = match job_args
            .and_then(|x| {
                x.get(arg.name.as_str())
                    .map(|x| serde_json::from_str::<serde_json::Value>(x.get()).ok())
            })
            .flatten()
            .unwrap_or_else(|| json!(null))
        {
            // Value::Null => todo!(),
            Value::Bool(b) => Box::new(b),
            Value::String(s)
                if arg_t == "timestamp"
                    || arg_t == "datetime"
                    || arg_t == "date"
                    || arg_t == "time" =>
            {
                if let Ok(d) = chrono::DateTime::<Utc>::from_str(s.as_str()) {
                    Box::new(d)
                } else {
                    Box::new(s)
                }
            }
            Value::String(s) => Box::new(s),
            Value::Number(n)
                if n.is_i64()
                    && (arg_t == "int"
                        || arg_t == "integer"
                        || arg_t == "smallint"
                        || arg_t == "bigint") =>
            {
                Box::new(n.as_i64().unwrap())
            }
            Value::Number(n) if n.is_f64() && arg_t == "float" => {
                Box::new(n.as_f64().unwrap() as f32)
            }
            Value::Number(n) if n.is_i64() && arg_t == "float" => {
                Box::new(n.as_i64().unwrap() as f32)
            }
            Value::Number(n) if n.is_u64() && arg_t == "uint" => {
                Box::new(n.as_u64().unwrap())
            }
            Value::Number(n)
                if n.is_f64() && (arg_t == "real" || arg_t == "dec" || arg_t == "fixed") =>
            {
                Box::new(n.as_f64().unwrap())
            }
            Value::Number(n)
                if n.is_i64() && (arg_t == "real" || arg_t == "dec" || arg_t == "fixed") =>
            {
                Box::new(n.as_i64().unwrap() as f64)
            }
            value @ _ => {
                return Err(Error::ExecutionErr(format!(
                    "Unsupported type in query: {:?} and signature {arg_t:?}",
                    value
                )))
            }
        };

        statement_values.push((arg_n, oracle_v));
    }


    let conn = tokio::task::spawn_blocking(|| {
        oracle::Connection::connect(database.user, database.password, database.database)
            .map_err(|e| Error::ExecutionErr(e.to_string()))
    })
    .await
    .map_err(to_anyhow)??;

    let result_f = do_oracledb_inner(query, statement_values, conn, Some(column_order), false)?;

    let result = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        db,
        mem_peak,
        canceled_by,
        result_f,
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;

    let raw_result = windmill_common::worker::to_raw_value(&json!(result));
    *mem_peak = (raw_result.get().len() / 1000) as i32;

    return Ok(raw_result);
}
