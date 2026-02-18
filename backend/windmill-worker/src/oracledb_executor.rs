use anyhow::anyhow;
use chrono::Utc;
use std::{collections::HashMap, str::FromStr, sync::Arc, vec};
use windmill_parser::Arg;

use futures::{future::BoxFuture, FutureExt, StreamExt};
use itertools::Itertools;
use oracle::sql_type::{InnerValue, OracleType, ToSql};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue, Value};
use windmill_common::{
    error::{to_anyhow, Error},
    worker::{to_raw_value, Connection, SqlResultCollectionStrategy},
};
use windmill_object_store::convert_json_line_stream;
use windmill_queue::MiniPulledJob;

use windmill_parser_sql::{
    parse_db_resource, parse_oracledb_sig, parse_s3_mode, parse_sql_blocks,
    parse_sql_statement_named_params,
};
use windmill_queue::CanceledBy;

use crate::{
    common::{
        build_args_values, check_executor_binary_exists, get_reserved_variables,
        s3_mode_args_to_worker_data, OccupancyMetrics, S3ModeWorkerData,
    },
    handle_child::run_future_with_polling_update_job_poller,
    sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args,
};
use windmill_common::client::AuthedClient;

#[derive(Deserialize)]
struct OracleDatabase {
    user: String,
    password: String,
    database: String,
}

lazy_static::lazy_static! {
    static ref ORACLE_LIB_DIR: String = std::env::var("ORACLE_LIB_DIR").unwrap_or_else(|_| "/opt/oracle/23/lib".to_string());
}

pub fn do_oracledb_inner<'a>(
    query: &str,
    params: Vec<(String, Box<dyn ToSql + Send + Sync>)>,
    conn: Arc<std::sync::Mutex<oracle::Connection>>,
    column_order: Option<&'a mut Option<Vec<String>>>,
    skip_collect: bool,
    first_row_only: bool,
    s3: Option<S3ModeWorkerData>,
) -> windmill_common::error::Result<BoxFuture<'a, windmill_common::error::Result<Vec<Box<RawValue>>>>>
{
    let qw = query.trim_end_matches(';').to_string();

    let result_f = async move {
        let param_names = parse_sql_statement_named_params(&qw, ':')
            .into_iter()
            .map(|x| x.into_bytes())
            .collect_vec();

        if skip_collect {
            tokio::task::spawn_blocking(move || {
                let c = conn.lock()?;

                let params2: Vec<(&str, &dyn ToSql)> = params
                    .iter()
                    .filter(|(k, _)| param_names.contains(&k.clone().into_bytes()))
                    .map(|(key, val)| (key.as_str(), &**val as &dyn ToSql))
                    .collect();

                let mut stmt = c.statement(&qw).build()?;

                match stmt.statement_type() {
                    oracle::StatementType::Select => {
                        stmt.query_named(&params2)?;
                    }
                    _ => {
                        stmt.execute_named(&params2)?;
                        c.commit()?;
                    }
                }
                oracle::Result::Ok(())
            })
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            Ok(vec![])
        } else {
            // We use an mpsc because we need an async stream for s3 mode. However since everything is sync
            // in rust-oracle, I assumed that calling ResultSet::next() is blocking when it has to refetch.
            let (tx, rx) = tokio::sync::mpsc::channel::<oracle::Result<Value>>(1000);
            let (column_order_oneshot_tx, column_order_oneshot_rx) =
                tokio::sync::oneshot::channel::<Option<Vec<String>>>();
            let mut column_order_oneshot_tx = Some(column_order_oneshot_tx);
            let rows_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
            tokio::task::spawn_blocking(move || {
                let result = (|| {
                    let tx = tx.clone();
                    let params2: Vec<(&str, &dyn ToSql)> = params
                        .iter()
                        .filter(|(k, _)| param_names.contains(&k.clone().into_bytes()))
                        .map(|(key, val)| (key.as_str(), &**val as &dyn ToSql))
                        .collect();

                    let c = conn.lock()?;
                    let mut stmt = c.statement(&qw).build()?;

                    match stmt.statement_type() {
                        oracle::StatementType::Select => {
                            let mut result_rows = stmt.query_named(&params2)?.enumerate();
                            while let Some((i, row)) = result_rows.next() {
                                match row {
                                    Ok(row) => {
                                        // If first row, infer column order and send it to the channel
                                        if i == 0 {
                                            let col_order: Vec<String> = row
                                                .column_info()
                                                .iter()
                                                .map(|x| x.name().to_string())
                                                .collect::<Vec<String>>();
                                            let _ = column_order_oneshot_tx
                                                .take()
                                                .unwrap()
                                                .send(Some(col_order));
                                        }

                                        // called in a spawn_blocking synchronous context, unwrap won't panic
                                        tx.blocking_send(Ok(convert_row_to_value(row))).unwrap()
                                    }
                                    Err(e) => {
                                        tx.blocking_send(Err(e)).unwrap();
                                        break;
                                    }
                                }
                                if first_row_only {
                                    break;
                                }
                            }
                        }
                        _ => {
                            stmt.execute_named(&params2)?;
                            c.commit()?;
                        }
                    };
                    drop(column_order_oneshot_tx);
                    Ok::<_, oracle::Error>(())
                })();
                match result {
                    Ok(_) => {}
                    Err(e) => tx.blocking_send(Err(e)).unwrap(),
                }
                // all instances of tx should be dropped here
            });

            if let Ok(Some(col_order)) = column_order_oneshot_rx.await {
                if let Some(column_order) = column_order {
                    *column_order = Some(col_order);
                }
            }

            if let Some(s3) = s3 {
                let stream = convert_json_line_stream(rows_stream.boxed(), s3.format).await?;
                s3.upload(stream.boxed()).await?;
                return Ok(vec![to_raw_value(&s3.to_return_s3_obj())]);
            } else {
                let rows: Vec<_> = rows_stream.collect().await;
                Ok(rows
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(to_anyhow)?
                    .iter()
                    .map(to_raw_value)
                    .collect::<Vec<_>>())
            }
        }
    };

    Ok(result_f.boxed())
}

fn convert_row_to_value(row: oracle::Row) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for (key, value) in row.column_info().iter().zip(row.sql_values()) {
        map.insert(
            key.name().to_string(),
            convert_oracledb_value_to_json(value, key.oracle_type()),
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

fn convert_oracledb_value_to_json(v: &oracle::SqlValue, c: &OracleType) -> serde_json::Value {
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

fn get_statement_values(
    sig: Vec<Arg>,
    job_args: &HashMap<String, Value>,
    args_to_skip: &Vec<String>,
) -> (Vec<(String, Box<dyn ToSql + Send + Sync>)>, Vec<String>) {
    let mut statement_values = vec![];
    let mut errors = vec![];

    for arg in &sig {
        if args_to_skip.contains(&arg.name) {
            continue;
        }
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "text".to_string());
        let arg_n = arg.name.clone();
        let oracle_v: Box<dyn ToSql + Send + Sync> = match job_args
            .get(arg.name.as_str())
            .unwrap_or_else(|| &json!(null))
        {
            // Value::Null => todo!(),
            Value::Bool(b) => Box::new(*b),
            Value::String(s)
                if arg_t == "timestamp"
                    || arg_t == "datetime"
                    || arg_t == "date"
                    || arg_t == "time" =>
            {
                if let Ok(d) = chrono::DateTime::<Utc>::from_str(s.as_str()) {
                    Box::new(d)
                } else {
                    Box::new(s.clone())
                }
            }
            Value::String(s) => Box::new(s.clone()),
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
            Value::Number(n) if n.is_u64() && arg_t == "uint" => Box::new(n.as_u64().unwrap()),
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
                errors.push(format!(
                    "Unsupported type in query: {value:?} and signature {arg_t:?} for {arg_n}"
                ));
                continue;
            }
        };

        statement_values.push((arg_n, oracle_v));
    }

    (statement_values, errors)
}

pub async fn do_oracledb(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
    parent_runnable_path: Option<String>,
) -> windmill_common::error::Result<Box<RawValue>> {
    check_executor_binary_exists(
        "the Oracle client lib",
        ORACLE_LIB_DIR.as_str(),
        "Oracle Database",
    )?;

    let job_args = build_args_values(job, client, conn).await?;

    let inline_db_res_path = parse_db_resource(&query);
    let s3 = parse_s3_mode(&query)?.map(|s3| s3_mode_args_to_worker_data(s3, client.clone(), job));

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        Some(
            client
                .get_resource_value_interpolated::<serde_json::Value>(
                    &inline_db_res_path,
                    Some(job.id.to_string()),
                )
                .await?,
        )
    } else {
        job_args.get("database").cloned()
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_value::<OracleDatabase>(db)
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let annotations = windmill_common::worker::SqlAnnotations::parse(query);
    let collection_strategy = if annotations.return_last_result {
        SqlResultCollectionStrategy::LastStatementAllRows
    } else {
        annotations.result_collection
    };

    let sig = parse_oracledb_sig(query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let (query, args_to_skip) =
        sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args, &reserved_variables)?;

    let (_, errors) = get_statement_values(sig.clone(), &job_args, &args_to_skip);

    if !errors.is_empty() {
        return Err(Error::ExecutionErr(errors.join("\n")));
    }

    if !oracle::InitParams::is_initialized() {
        let _ = oracle::InitParams::new()
            .oracle_client_lib_dir(ORACLE_LIB_DIR.as_str())
            .map_err(|e| anyhow!("Failed to initialize oracle client: {e}"))?
            .init();
    }

    let oracle_conn = tokio::task::spawn_blocking(|| {
        oracle::Connection::connect(database.user, database.password, database.database)
            .map_err(|e| Error::ExecutionErr(e.to_string()))
    })
    .await
    .map_err(to_anyhow)??;

    let conn_a = Arc::new(std::sync::Mutex::new(oracle_conn));

    let queries = parse_sql_blocks(&query);

    let result_f = async move {
        let mut results = vec![];
        for (i, q) in queries.iter().enumerate() {
            let (vals, _) = get_statement_values(sig.clone(), &job_args, &args_to_skip);

            let result = do_oracledb_inner(
                q,
                vals,
                conn_a.clone(),
                if i == queries.len() - 1
                    && s3.is_none()
                    && collection_strategy.collect_last_statement_only(queries.len())
                    && !collection_strategy.collect_scalar()
                {
                    Some(column_order)
                } else {
                    None
                },
                collection_strategy.collect_last_statement_only(queries.len())
                    && i < queries.len() - 1,
                collection_strategy.collect_first_row_only(),
                s3.clone(),
            )?
            .await?;
            results.push(result);
        }

        collection_strategy.collect(results)
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

    let raw_result = windmill_common::worker::to_raw_value(&json!(result));
    *mem_peak = (raw_result.get().len() / 1000) as i32;

    return Ok(raw_result);
}
