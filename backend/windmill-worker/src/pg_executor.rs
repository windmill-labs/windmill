use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use base64::{engine, Engine as _};
use chrono::Utc;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use itertools::Itertools;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde_json::value::RawValue;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tokio_postgres::Client;
use tokio_postgres::{types::ToSql, Row};
use tokio_postgres::{
    types::{FromSql, Type},
    Column,
};
use uuid::Uuid;
use windmill_common::error::to_anyhow;
use windmill_common::error::{self, Error};
use windmill_object_store::convert_json_line_stream;
use windmill_common::worker::{
    to_raw_value, Connection, SqlResultCollectionStrategy, CLOUD_HOSTED,
};
use windmill_common::workspaces::get_datatable_resource_from_db_unchecked;
use windmill_common::{PgDatabase, PrepareQueryColumnInfo, PrepareQueryResult};
use windmill_parser::{Arg, Typ};
use windmill_parser_sql::{
    parse_db_resource, parse_pg_statement_arg_indices, parse_pgsql_sig, parse_s3_mode,
    parse_sql_blocks,
};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::agent_workers::get_datatable_resource_from_agent_http;
use crate::common::{
    build_args_values, get_reserved_variables, s3_mode_args_to_worker_data, sizeof_val,
    OccupancyMetrics, S3ModeWorkerData,
};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::sql_utils::remove_comments;
use crate::MAX_RESULT_SIZE;
use bytes::Buf;
use lazy_static::lazy_static;
use windmill_common::client::AuthedClient;

lazy_static! {
    pub static ref CONNECTION_CACHE: Arc<Mutex<Option<(String, tokio_postgres::Client)>>> =
        Arc::new(Mutex::new(None));
    pub static ref CONNECTION_COUNTER: Arc<RwLock<HashMap<String, u64>>> =
        Arc::new(RwLock::new(HashMap::new()));
    pub static ref LAST_QUERY: AtomicU64 = AtomicU64::new(0);
}

fn do_postgresql_inner<'a>(
    mut query: String,
    param_idx_to_arg_and_value: &HashMap<i32, (&Arg, Option<&Value>)>,
    client: &'a Client,
    column_order: Option<&'a mut Option<Vec<String>>>,
    siz: &'a AtomicUsize,
    skip_collect: bool,
    first_row_only: bool,
    s3: Option<S3ModeWorkerData>,
) -> error::Result<BoxFuture<'a, error::Result<Vec<Box<RawValue>>>>> {
    let mut query_params = vec![];

    let arg_indices = parse_pg_statement_arg_indices(&query);

    let mut i = 1;
    for oidx in arg_indices.iter().sorted() {
        if let Some((arg, value)) = param_idx_to_arg_and_value.get(&oidx) {
            if *oidx as usize != i {
                query = query.replace(&format!("${}", oidx), &format!("${}", i));
            }
            let value = value.unwrap_or_else(|| &serde_json::Value::Null);
            let arg_t = arg
                .otyp
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing otyp for pg arg"))?;
            let typ = &arg.typ;
            let param = convert_val(value, arg_t, typ)?;
            query_params.push(param);
            i += 1;
        }
    }

    let result_f = async move {
        // Now we can execute a simple statement that just returns its parameter.

        let mut res: Vec<Box<serde_json::value::RawValue>> = vec![];

        let query_params = query_params
            .iter()
            .map(|p| &**p as &(dyn ToSql + Sync))
            .collect_vec();

        if skip_collect {
            client
                .execute_raw(&query, query_params)
                .await
                .map_err(to_anyhow)?;
        } else if let Some(ref s3) = s3 {
            let rows_stream = client
                .query_raw(&query, query_params)
                .map_err(to_anyhow)
                .await?
                .map_err(to_anyhow)
                .map(|row_result| {
                    row_result.and_then(|row| postgres_row_to_json_value(row).map_err(to_anyhow))
                });

            let stream = convert_json_line_stream(rows_stream.boxed(), s3.format).await?;
            s3.upload(stream.boxed()).await?;

            return Ok(vec![to_raw_value(&s3.to_return_s3_obj())]);
        } else {
            let rows = client
                .query_raw(&query, query_params)
                .await
                .map_err(to_anyhow)?;

            let rows = if first_row_only {
                rows.take(1).boxed()
            } else {
                rows.boxed()
            };

            let rows = rows.try_collect::<Vec<Row>>().await.map_err(to_anyhow)?;

            if let Some(column_order) = column_order {
                *column_order = Some(
                    rows.first()
                        .map(|x| {
                            x.columns()
                                .iter()
                                .map(|x| x.name().to_string())
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default(),
                );
            }

            for row in rows.into_iter() {
                let r = postgres_row_to_json_value(row);
                if let Ok(v) = r.as_ref() {
                    let size = sizeof_val(v);
                    siz.fetch_add(size, Ordering::Relaxed);
                }
                if *CLOUD_HOSTED {
                    let siz = siz.load(Ordering::Relaxed);
                    if siz > MAX_RESULT_SIZE * 4 {
                        return Err(Error::ExecutionErr(format!(
                            "Query result too large for cloud (size = {} > {})",
                            siz,
                            MAX_RESULT_SIZE & 4,
                        )));
                    }
                }
                if let Ok(v) = r {
                    res.push(to_raw_value(&v));
                } else {
                    return Err(to_anyhow(r.err().unwrap()).into());
                }
            }
        }

        Ok(res)
    };

    Ok(result_f.boxed())
}

pub async fn do_postgresql(
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
    run_inline: bool,
) -> error::Result<Box<RawValue>> {
    let pg_args = build_args_values(job, client, conn).await?;

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
        match pg_args.get("database").cloned() {
            Some(Value::String(db_str)) if db_str.starts_with("datatable://") => {
                let db_str = db_str.trim_start_matches("datatable://");
                Some(match conn {
                    Connection::Http(client) => {
                        get_datatable_resource_from_agent_http(client, &db_str, &job.workspace_id)
                            .await?
                    }
                    Connection::Sql(db) => {
                        get_datatable_resource_from_db_unchecked(db, &job.workspace_id, &db_str)
                            .await?
                    }
                })
            }
            database => database,
        }
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_value::<PgDatabase>(db.clone())
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

    let database_string = database.to_uri();
    let database_string_clone = database_string.clone();

    let mtex;
    if !*CLOUD_HOSTED {
        mtex = CONNECTION_CACHE.try_lock().ok();
        increment_connection_counter(&database_string).await;
    } else {
        mtex = None;
    }

    let has_cached_con = mtex
        .as_ref()
        .is_some_and(|x| x.as_ref().is_some_and(|y| y.0 == database_string));

    // tracing::error!("HAS CACHED CON: {}", has_cached_con);
    let (new_client, mtex) = if has_cached_con {
        tracing::info!("Using cached connection");
        LAST_QUERY.store(
            chrono::Utc::now().timestamp().try_into().unwrap_or(0),
            std::sync::atomic::Ordering::Relaxed,
        );
        (None, mtex)
    } else {
        let (client, connection) = database.connect().await?;
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                let mut mtex = CONNECTION_CACHE.lock().await;
                *mtex = None;
                tracing::error!("connection error: {}", e);
            }
        });
        (Some((client, handle)), None)
    };

    let sig = parse_pgsql_sig(&query).map_err(|x| Error::ExecutionErr(x.to_string()))?;

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let (query, _) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig.args, &pg_args, &reserved_variables)?;

    let queries = parse_sql_blocks(query);

    let (client, handle) = if let Some((client, handle)) = new_client.as_ref() {
        (client, Some(handle))
    } else {
        let (_, client) = mtex.as_ref().unwrap().as_ref().unwrap();
        (client, None)
    };

    let param_idx_to_arg_and_value = sig
        .args
        .iter()
        .filter_map(|x| x.oidx.map(|oidx| (oidx, (x, pg_args.get(&x.name)))))
        .collect::<HashMap<_, _>>();

    let size = AtomicUsize::new(0);
    let size_ref = &size;
    let result_f = async move {
        let mut results = vec![];
        for (i, query) in queries.iter().enumerate() {
            if annotations.prepare {
                let query = remove_comments(query);
                // Used by the data table typechecker to set default schemas
                if query.starts_with("SET search_path") || query.starts_with("RESET search_path") {
                    let _ = client.execute(&query.to_string(), &[]).await;
                    continue;
                }
                let prepared = client.prepare(&query).await;
                let prepared = match prepared {
                    Ok(prepared) => {
                        let columns: Option<Vec<PrepareQueryColumnInfo>> = Some(
                            prepared
                                .columns()
                                .iter()
                                .map(|col| PrepareQueryColumnInfo {
                                    name: col.name().to_string(),
                                    type_name: col.type_().name().to_string(),
                                })
                                .collect(),
                        );
                        PrepareQueryResult { columns, error: None }
                    }
                    Err(e) => PrepareQueryResult { columns: None, error: Some(e.to_string()) },
                };
                results.push(vec![to_raw_value(&prepared)]);
                continue;
            }

            let result = do_postgresql_inner(
                query.to_string(),
                &param_idx_to_arg_and_value,
                client,
                if i == queries.len() - 1
                    && s3.is_none()
                    && collection_strategy.collect_last_statement_only(queries.len())
                    && !collection_strategy.collect_scalar()
                {
                    Some(column_order)
                } else {
                    None
                },
                size_ref,
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

    let result = if run_inline {
        result_f.await?
    } else {
        run_future_with_polling_update_job_poller(
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
        .await?
    };

    // drop the mtex to avoid holding the lock for too long, result has been returned
    drop(mtex);

    *mem_peak = size.load(Ordering::Relaxed) as i32;

    if let Some(handle) = handle {
        if !*CLOUD_HOSTED {
            // tracing::error!("Found handle");
            if let Ok(mut mtex) = CONNECTION_CACHE.try_lock() {
                if mtex.as_ref().is_none_or(|x| x.0 != database_string) {
                    // tracing::error!("Locked conn cached");
                    let abort_handler = handle.abort_handle();

                    let mut cache_new_con = false;
                    if let Some(new_client) = new_client {
                        cache_new_con = is_most_used_conn(&database_string).await;
                        if cache_new_con {
                            *mtex = Some((database_string, new_client.0));
                        } else {
                            new_client.1.abort();
                        }
                    } else {
                        handle.abort();
                    }

                    if cache_new_con {
                        LAST_QUERY.store(
                            chrono::Utc::now().timestamp().try_into().unwrap_or(0),
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        tokio::spawn(async move {
                            loop {
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                let last_query =
                                    LAST_QUERY.load(std::sync::atomic::Ordering::Relaxed);
                                let now = chrono::Utc::now().timestamp().try_into().unwrap_or(0);

                                //we cache connection for 5 minutes at most
                                if last_query + 60 * 1 < now {
                                    // tracing::error!("Closing cache connection due to inactivity");
                                    tracing::info!(
                                        "Closing cache pg executor connection due to inactivity"
                                    );
                                    break;
                                }
                                let mtex = CONNECTION_CACHE.lock().await;
                                if mtex.is_none() {
                                    // connection is not in the mutex anymore
                                    break;
                                } else if let Some(mtex) = mtex.as_ref() {
                                    if mtex.0.as_str() != &database_string_clone {
                                        // connection is not the latest one
                                        break;
                                    }
                                }

                                tracing::debug!(
                                    "Keeping cached pg executor connection alive due to activity"
                                )
                            }
                            let mut mtex = CONNECTION_CACHE.lock().await;
                            *mtex = None;
                            abort_handler.abort();
                        });
                    }
                } else {
                    handle.abort();
                }
            } else {
                handle.abort();
            }
        } else {
            handle.abort();
        }
    }
    *mem_peak = (result.get().len() / 1000) as i32;
    // And then check that we got back the same string we sent over.
    return Ok(result);
}

async fn is_most_used_conn(database_string: &str) -> bool {
    let counter_map = CONNECTION_COUNTER.read().await;
    let current_count = counter_map.get(database_string).copied().unwrap_or(0);
    let max_count = counter_map.values().copied().max().unwrap_or(0);
    current_count >= max_count
}

async fn increment_connection_counter(database_string: &str) {
    let mut counter_map = CONNECTION_COUNTER.write().await;
    *counter_map.entry(database_string.to_string()).or_insert(0) += 1;
}

fn map_as_single_type<T>(
    vec: Option<&Vec<Value>>,
    f: impl Fn(&Value) -> Option<T>,
) -> anyhow::Result<Option<Vec<Option<T>>>> {
    if let Some(vec) = vec {
        Ok(Some(
            vec.into_iter()
                .map(|v| {
                    // first option is if the value is of the right type (if none, will stop the collection and throw error)
                    // second option is if the value is null
                    // allow nulls in arrays
                    if matches!(v, Value::Null) {
                        Some(None)
                    } else {
                        f(v).map(Some)
                    }
                })
                .collect::<Option<Vec<Option<T>>>>()
                .ok_or_else(|| anyhow::anyhow!("Mixed types in array"))?,
        ))
    } else {
        Ok(None)
    }
}

fn convert_vec_val(
    vec: Option<&Vec<Value>>,
    arg_t: &String,
) -> windmill_common::error::Result<Box<dyn ToSql + Sync + Send>> {
    match arg_t.as_str() {
        "bool" | "boolean" => Ok(Box::new(map_as_single_type(vec, |v| v.as_bool())?)),
        "char" | "character" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_i64().map(|x| x as i8)
        })?)),
        "smallint" | "smallserial" | "int2" | "serial2" => {
            Ok(Box::new(map_as_single_type(vec, |v| {
                v.as_i64().map(|x| x as i16)
            })?))
        }
        "int" | "integer" | "int4" | "serial" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_i64().map(|x| x as i32)
        })?)),
        "numeric" | "decimal" => Ok(Box::new(map_as_single_type(vec, |v| {
            if v.is_i64() {
                Decimal::from_i64(v.as_i64().unwrap())
            } else if v.is_f64() {
                Decimal::from_f64(v.as_f64().unwrap())
            } else {
                None
            }
        })?)),
        "oid" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_u64().map(|x| x as u32)
        })?)),
        "bigint" | "bigserial" | "int8" | "serial8" => {
            Ok(Box::new(map_as_single_type(vec, |v| {
                v.as_u64().map(|x| x as i64)
            })?))
        }
        "real" | "float4" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_f64().map(|x| x as f32)
        })?)),
        "double" | "float8" => Ok(Box::new(map_as_single_type(vec, |v| v.as_f64())?)),
        "uuid" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str().map(|x| Uuid::parse_str(x).ok()).flatten()
        })?)),
        "date" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str().map(|x| {
                chrono::NaiveDate::parse_from_str(x, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default()
            })
        })?)),
        "time" | "timetz" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str().map(|x| {
                chrono::NaiveTime::parse_from_str(x, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default()
            })
        })?)),
        "timestamp" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str().map(|x| {
                chrono::NaiveDateTime::parse_from_str(x, "%Y-%m-%dT%H:%M:%S.%3fZ")
                    .unwrap_or_default()
            })
        })?)),
        "timestamptz" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str()
                .map(|x| x.parse::<chrono::DateTime<Utc>>().unwrap_or_default())
        })?)),
        "jsonb" | "json" => Ok(Box::new(
            vec.map(|v| v.clone().into_iter().map(Some).collect_vec()),
        )),
        "bytea" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str().map(|x| {
                engine::general_purpose::STANDARD
                    .decode(x)
                    .unwrap_or(vec![])
            })
        })?)),
        "text" | "varchar" => Ok(Box::new(map_as_single_type(vec, |v| {
            v.as_str().map(|x| x.to_string())
        })?)),
        _ => Err(anyhow::anyhow!("Unsupported JSON array type"))?,
    }
}

fn convert_val(
    value: &Value,
    arg_t: &String,
    typ: &Typ,
) -> windmill_common::error::Result<Box<dyn ToSql + Sync + Send>> {
    match value {
        Value::Array(vec) if arg_t.ends_with("[]") => {
            let arg_t = arg_t.trim_end_matches("[]").to_string();
            convert_vec_val(Some(vec), &arg_t)
        }
        Value::Null if arg_t.ends_with("[]") => {
            let arg_t = arg_t.trim_end_matches("[]").to_string();
            convert_vec_val(None, &arg_t)
        }
        Value::Null => match arg_t.as_str() {
            "bool" | "boolean" => Ok(Box::new(None::<bool>)),
            "char" | "character" => Ok(Box::new(None::<i8>)),
            "smallint" | "smallserial" | "int2" | "serial2" => Ok(Box::new(None::<i16>)),
            "int" | "integer" | "int4" | "serial" => Ok(Box::new(None::<i32>)),
            "numeric" | "decimal" => Ok(Box::new(None::<Decimal>)),
            "oid" => Ok(Box::new(None::<u32>)),
            "bigint" | "bigserial" | "int8" | "serial8" => Ok(Box::new(None::<i64>)),
            "real" | "float4" => Ok(Box::new(None::<f32>)),
            "double" | "float8" => Ok(Box::new(None::<f64>)),
            "uuid" => Ok(Box::new(None::<Uuid>)),
            "date" => Ok(Box::new(None::<chrono::NaiveDate>)),
            "time" | "timetz" => Ok(Box::new(None::<chrono::NaiveTime>)),
            "timestamp" => Ok(Box::new(None::<chrono::NaiveDateTime>)),
            "timestamptz" => Ok(Box::new(None::<chrono::DateTime<Utc>>)),
            "jsonb" | "json" => Ok(Box::new(None::<Option<Value>>)),
            "bytea" => Ok(Box::new(None::<Vec<u8>>)),
            "text" | "varchar" => Ok(Box::new(None::<String>)),
            _ => Err(anyhow::anyhow!("Unsupported JSON null type"))?,
        },
        Value::Bool(b) => Ok(Box::new(b.clone())),
        Value::Number(n) if matches!(typ, Typ::Str(_)) => Ok(Box::new(n.to_string())),
        Value::Number(n) if arg_t == "char" && n.is_i64() => {
            Ok(Box::new(n.as_i64().unwrap() as i8))
        }
        Value::Number(n)
            if (arg_t == "smallint"
                || arg_t == "smallserial"
                || arg_t == "int2"
                || arg_t == "serial2")
                && n.is_i64() =>
        {
            Ok(Box::new(n.as_i64().unwrap() as i16))
        }
        Value::Number(n)
            if (arg_t == "int" || arg_t == "integer" || arg_t == "int4" || arg_t == "serial")
                && n.is_i64() =>
        {
            Ok(Box::new(n.as_i64().unwrap() as i32))
        }
        Value::Number(n) if (arg_t == "real" || arg_t == "float4") && n.as_f64().is_some() => {
            Ok(Box::new(n.as_f64().unwrap() as f32))
        }
        Value::Number(n) if (arg_t == "double" || arg_t == "float8") && n.as_f64().is_some() => {
            Ok(Box::new(n.as_f64().unwrap()))
        }
        Value::Number(n) if (arg_t == "numeric" || arg_t == "decimal") && n.is_i64() => Ok(
            Box::new(Decimal::from_i64(n.as_i64().unwrap()).unwrap_or_default()),
        ),
        Value::Number(n) if (arg_t == "numeric" || arg_t == "decimal") && n.is_f64() => Ok(
            Box::new(Decimal::from_f64(n.as_f64().unwrap()).unwrap_or_default()),
        ),
        Value::Number(n) if arg_t == "oid" && n.is_u64() => {
            Ok(Box::new(n.as_u64().unwrap() as u32))
        }
        Value::Number(n)
            if (arg_t == "bigint"
                || arg_t == "bigserial"
                || arg_t == "int8"
                || arg_t == "serial8")
                && n.is_u64() =>
        {
            Ok(Box::new(n.as_u64().unwrap() as i64))
        }
        Value::Number(n) if n.is_i64() => Ok(Box::new(n.as_i64().unwrap())),
        Value::Number(n) => Ok(Box::new(n.as_f64().unwrap())),
        Value::String(s) if arg_t == "uuid" => Ok(Box::new(Uuid::parse_str(s)?)),
        Value::String(s) if arg_t == "date" => {
            let date =
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            Ok(Box::new(date))
        }
        Value::String(s) if arg_t == "time" || arg_t == "timetz" => {
            let time =
                chrono::NaiveTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            Ok(Box::new(time))
        }
        Value::String(s) if arg_t == "timestamp" => {
            let datetime = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ")
                .unwrap_or_default();
            Ok(Box::new(datetime))
        }
        Value::String(s) if arg_t == "timestamptz" => {
            let datetime = s.parse::<chrono::DateTime<Utc>>().unwrap_or_default();
            Ok(Box::new(datetime))
        }
        Value::String(s) if arg_t == "bytea" => {
            let bytes = engine::general_purpose::STANDARD
                .decode(s)
                .unwrap_or(vec![]);
            Ok(Box::new(bytes))
        }
        Value::Array(_) if arg_t == "jsonb" || arg_t == "json" => Ok(Box::new(value.clone())),
        Value::Object(_) if arg_t == "text" || arg_t == "varchar" => {
            Ok(Box::new(serde_json::to_string(value).map_err(|err| {
                Error::ExecutionErr(format!("Failed to convert JSON to text: {}", err))
            })?))
        }
        Value::Object(_) => Ok(Box::new(value.clone())),
        Value::String(s) => Ok(Box::new(s.clone())),
        _ => Err(Error::ExecutionErr(format!(
            "Unsupported type in query: {:?} and signature {arg_t:?}",
            value
        ))),
    }
}

pub fn pg_cell_to_json_value(
    row: &Row,
    column: &Column,
    column_i: usize,
) -> Result<JSONValue, Error> {
    let f64_to_json_number = |raw_val: f64| -> Result<JSONValue, Error> {
        let temp = serde_json::Number::from_f64(raw_val.into())
            .ok_or(anyhow::anyhow!("invalid json-float"))?;
        Ok(JSONValue::Number(temp))
    };
    Ok(match *column.type_() {
        // for rust-postgres <> postgres type-mappings: https://docs.rs/postgres/latest/postgres/types/trait.FromSql.html#types
        // for postgres types: https://www.postgresql.org/docs/7.4/datatype.html#DATATYPE-TABLE

        // single types
        Type::BOOL => get_basic(row, column, column_i, |a: bool| Ok(JSONValue::Bool(a)))?,
        Type::BIT => get_basic(row, column, column_i, |a: bit_vec::BitVec| match a.len() {
            1 => Ok(JSONValue::Bool(a.get(0).unwrap())),
            _ => Ok(JSONValue::String(
                a.iter()
                    .map(|x| if x { "1" } else { "0" })
                    .collect::<String>(),
            )),
        })?,
        Type::INT2 => get_basic(row, column, column_i, |a: i16| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT4 => get_basic(row, column, column_i, |a: i32| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT8 => get_basic(row, column, column_i, |a: i64| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::TEXT | Type::VARCHAR => {
            get_basic(row, column, column_i, |a: String| Ok(JSONValue::String(a)))?
        }
        Type::TIMESTAMP => get_basic(row, column, column_i, |a: chrono::NaiveDateTime| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::DATE => get_basic(row, column, column_i, |a: chrono::NaiveDate| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIME => get_basic(row, column, column_i, |a: chrono::NaiveTime| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIMETZ => get_basic(row, column, column_i, |a: TimeTZStr| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::TIMESTAMPTZ => get_basic(row, column, column_i, |a: chrono::DateTime<Utc>| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::UUID => get_basic(row, column, column_i, |a: uuid::Uuid| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::INET => get_basic(row, column, column_i, |a: IpAddr| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::INTERVAL => get_basic(row, column, column_i, |a: IntervalStr| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::JSON | Type::JSONB => get_basic(row, column, column_i, |a: JSONValue| Ok(a))?,
        Type::FLOAT4 => get_basic(row, column, column_i, |a: f32| {
            Ok(f64_to_json_number(a.into())?)
        })?,
        Type::NUMERIC => get_basic(row, column, column_i, |a: Decimal| {
            Ok(serde_json::to_value(a)
                .map_err(|_| anyhow::anyhow!("Cannot convert decimal to json"))?)
        })?,
        Type::FLOAT8 => get_basic(row, column, column_i, |a: f64| f64_to_json_number(a))?,
        Type::BYTEA => get_basic(row, column, column_i, |a: Vec<u8>| {
            Ok(JSONValue::String(format!("\\x{}", hex::encode(a))))
        })?,
        // these types require a custom StringCollector struct as an intermediary (see struct at bottom)
        Type::TS_VECTOR => get_basic(row, column, column_i, |a: StringCollector| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::OID => get_basic(row, column, column_i, |a: u32| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        // array types
        Type::BOOL_ARRAY => get_array(row, column, column_i, |a: bool| Ok(JSONValue::Bool(a)))?,
        Type::BIT_ARRAY => get_array(row, column, column_i, |a: bit_vec::BitVec| match a.len() {
            1 => Ok(JSONValue::Bool(a.get(0).unwrap())),
            _ => Ok(JSONValue::String(
                a.iter()
                    .map(|x| if x { "1" } else { "0" })
                    .collect::<String>(),
            )),
        })?,
        Type::INT2_ARRAY => get_array(row, column, column_i, |a: i16| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT4_ARRAY => get_array(row, column, column_i, |a: i32| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT8_ARRAY => get_array(row, column, column_i, |a: i64| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::TEXT_ARRAY | Type::VARCHAR_ARRAY => {
            get_array(row, column, column_i, |a: String| Ok(JSONValue::String(a)))?
        }
        Type::JSON_ARRAY | Type::JSONB_ARRAY => {
            get_array(row, column, column_i, |a: JSONValue| Ok(a))?
        }
        Type::FLOAT4_ARRAY => get_array(row, column, column_i, |a: f32| {
            Ok(f64_to_json_number(a.into())?)
        })?,
        Type::FLOAT8_ARRAY => {
            get_array(row, column, column_i, |a: f64| Ok(f64_to_json_number(a)?))?
        }
        Type::NUMERIC_ARRAY => get_array(row, column, column_i, |a: Decimal| {
            Ok(serde_json::to_value(a)
                .map_err(|_| anyhow::anyhow!("Cannot convert decimal to json"))?)
        })?,
        // these types require a custom StringCollector struct as an intermediary (see struct at bottom)
        Type::TS_VECTOR_ARRAY => get_array(row, column, column_i, |a: StringCollector| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::TIMESTAMP_ARRAY => get_array(row, column, column_i, |a: chrono::NaiveDateTime| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::DATE_ARRAY => get_array(row, column, column_i, |a: chrono::NaiveDate| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIME_ARRAY => get_array(row, column, column_i, |a: chrono::NaiveTime| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIMETZ_ARRAY => get_array(row, column, column_i, |a: TimeTZStr| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::TIMESTAMPTZ_ARRAY => get_array(row, column, column_i, |a: chrono::DateTime<Utc>| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::BYTEA_ARRAY => get_array(row, column, column_i, |a: Vec<u8>| {
            Ok(JSONValue::String(format!("\\x{}", hex::encode(a))))
        })?,
        Type::VOID => JSONValue::Null,
        _ => get_basic(row, column, column_i, |a: String| Ok(JSONValue::String(a)))?,
    })
}

pub fn postgres_row_to_json_value(row: Row) -> Result<JSONValue, Error> {
    let row_data = postgres_row_to_row_data(row)?;
    Ok(JSONValue::Object(row_data))
}

// some type-aliases I use in my project
pub type JSONValue = serde_json::Value;
pub type RowData = Map<String, JSONValue>;

pub fn postgres_row_to_row_data(row: Row) -> Result<RowData, Error> {
    let mut result: Map<String, JSONValue> = Map::new();
    for (i, column) in row.columns().iter().enumerate() {
        let name = column.name();
        let json_value = pg_cell_to_json_value(&row, column, i)?;
        result.insert(name.to_string(), json_value);
    }
    Ok(result)
}

fn get_basic<'a, T: FromSql<'a>>(
    row: &'a Row,
    column: &Column,
    column_i: usize,
    val_to_json_val: impl Fn(T) -> Result<JSONValue, Error>,
) -> Result<JSONValue, Error> {
    let raw_val = row.try_get::<_, Option<T>>(column_i).with_context(|| {
        format!(
            "conversion issue for value at column_name `{}` with type {:?}",
            column.name(),
            column.type_()
        )
    })?;
    raw_val.map_or(Ok(JSONValue::Null), val_to_json_val)
}

struct IntervalStr(String);

impl<'a> FromSql<'a> for IntervalStr {
    fn from_sql(
        _: &Type,
        mut raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let microseconds = raw.get_i64();
        let days = raw.get_i32();
        let months = raw.get_i32();
        Ok(IntervalStr(format!(
            "{:?} months {:?} days {:?} ms",
            months, days, microseconds
        )))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty, &Type::INTERVAL)
    }
}

struct TimeTZStr(String);
impl<'a> FromSql<'a> for TimeTZStr {
    fn from_sql(
        _: &Type,
        mut raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let microsecond = raw.get_i64();
        let offset = raw.get_i32();
        let utc_sec = (microsecond / 1_000_000) + offset as i64;
        let utc = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
            ((utc_sec + 3600 * 24) % (3600 * 24)) as u32,
            ((microsecond % 1_000_000) * 1_000) as u32,
        )
        .ok_or_else(|| anyhow::anyhow!("Invalid time value"))?;
        Ok(TimeTZStr(format!("{:?} UTC", utc)))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty, &Type::TIMETZ)
    }
}

fn get_array<'a, T: FromSql<'a>>(
    row: &'a Row,
    column: &Column,
    column_i: usize,
    val_to_json_val: impl Fn(T) -> Result<JSONValue, Error>,
) -> Result<JSONValue, Error> {
    let raw_val_array = row
        .try_get::<_, Option<Vec<Option<T>>>>(column_i)
        .with_context(|| {
            format!(
                "conversion issue for array at column_name `{}`",
                column.name()
            )
        })?;
    Ok(match raw_val_array {
        Some(val_array) => {
            let mut result = vec![];
            for val in val_array {
                result.push(
                    val.map(|v| val_to_json_val(v))
                        .transpose()?
                        .unwrap_or(Value::Null),
                );
            }
            JSONValue::Array(result)
        }
        None => JSONValue::Null,
    })
}

// you can remove this section if not using TS_VECTOR (or other types requiring an intermediary `FromSQL` struct)
struct StringCollector(String);
impl FromSql<'_> for StringCollector {
    fn from_sql(
        _: &Type,
        raw: &[u8],
    ) -> Result<StringCollector, Box<dyn std::error::Error + Sync + Send>> {
        let result = std::str::from_utf8(raw)?;
        Ok(StringCollector(result.to_owned()))
    }
    fn accepts(_ty: &Type) -> bool {
        true
    }
}
