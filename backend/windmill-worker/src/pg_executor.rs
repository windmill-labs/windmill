use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use base64::{engine, Engine as _};
use chrono::Utc;
use futures::future::BoxFuture;
use futures::{FutureExt, TryStreamExt};
use itertools::Itertools;
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio_postgres::types::IsNull;
use tokio_postgres::Client;
use tokio_postgres::{
    types::{to_sql_checked, ToSql},
    NoTls, Row,
};
use tokio_postgres::{
    types::{FromSql, Type},
    Column,
};
use uuid::Uuid;
use windmill_common::error::{self, Error};
use windmill_common::worker::{to_raw_value, CLOUD_HOSTED};
use windmill_common::{error::to_anyhow, jobs::QueuedJob};
use windmill_parser::{Arg, Typ};
use windmill_parser_sql::{
    parse_db_resource, parse_pg_statement_arg_indices, parse_pgsql_sig, parse_sql_blocks,
};
use windmill_queue::CanceledBy;

use crate::common::{build_args_values, sizeof_val, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::{AuthedClientBackgroundTask, MAX_RESULT_SIZE};
use bytes::{Buf, BytesMut};
use lazy_static::lazy_static;
use urlencoding::encode;

#[derive(Deserialize)]
struct PgDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    sslmode: Option<String>,
    dbname: String,
    root_certificate_pem: Option<String>,
}

lazy_static! {
    pub static ref CONNECTION_CACHE: Arc<Mutex<Option<(String, tokio_postgres::Client)>>> =
        Arc::new(Mutex::new(None));
    pub static ref LAST_QUERY: AtomicU64 = AtomicU64::new(0);
    pub static ref RUNNING: AtomicBool = AtomicBool::new(false);
}

fn do_postgresql_inner<'a>(
    mut query: String,
    param_idx_to_arg_and_value: &HashMap<i32, (&Arg, Option<&Value>)>,
    client: &'a Client,
    column_order: Option<&'a mut Option<Vec<String>>>,
    siz: &'a AtomicUsize,
    skip_collect: bool,
) -> error::Result<BoxFuture<'a, anyhow::Result<Box<RawValue>>>> {
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

        let mut res: Vec<serde_json::Value> = vec![];

        if skip_collect {
            client
                .execute_raw(&query, query_params)
                .await
                .map_err(to_anyhow)?;
        } else {
            let rows = client
                .query_raw(&query, query_params)
                .await
                .map_err(to_anyhow)?;

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
                        return Err(anyhow::anyhow!(
                            "Query result too large for cloud (size = {} > {})",
                            siz,
                            MAX_RESULT_SIZE & 4
                        ));
                    }
                }
                if let Ok(v) = r {
                    res.push(v);
                } else {
                    return Err(to_anyhow(r.err().unwrap()));
                }
            }
        }

        Ok(to_raw_value(&res))
    };

    Ok(result_f.boxed())
}

pub async fn do_postgresql(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<RawValue>> {
    let pg_args = build_args_values(job, client, db).await?;

    let inline_db_res_path = parse_db_resource(&query);

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        Some(
            client
                .get_authed()
                .await
                .get_resource_value_interpolated::<serde_json::Value>(
                    &inline_db_res_path,
                    Some(job.id.to_string()),
                )
                .await?,
        )
    } else {
        pg_args.get("database").cloned()
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_value::<PgDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let annotations = windmill_common::worker::SqlAnnotations::parse(query);

    let sslmode = match database.sslmode.as_deref() {
        Some("allow") => "prefer".to_string(),
        Some("verify-ca") | Some("verify-full") => "require".to_string(),
        Some(s) => s.to_string(),
        None => "prefer".to_string(),
    };
    let database_string = format!(
        "postgres://{user}:{password}@{host}:{port}/{dbname}?sslmode={sslmode}",
        user = encode(&database.user.unwrap_or("postgres".to_string())),
        password = encode(&database.password.unwrap_or("".to_string())),
        host = encode(&database.host),
        port = database.port.unwrap_or(5432),
        dbname = database.dbname,
        sslmode = sslmode
    );
    let database_string_clone = database_string.clone();

    RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);
    LAST_QUERY.store(
        chrono::Utc::now().timestamp().try_into().unwrap_or(0),
        std::sync::atomic::Ordering::Relaxed,
    );
    let mtex;
    if !*CLOUD_HOSTED {
        mtex = Some(CONNECTION_CACHE.lock().await);
    } else {
        mtex = None;
    }

    let has_cached_con = mtex
        .as_ref()
        .is_some_and(|x| x.as_ref().is_some_and(|y| y.0 == database_string));
    let new_client = if has_cached_con {
        tracing::info!("Using cached connection");
        None
    } else if sslmode == "require" {
        tracing::info!("Creating new connection");
        let mut connector = TlsConnector::builder();
        if let Some(root_certificate_pem) = database.root_certificate_pem {
            if !root_certificate_pem.is_empty() {
                connector.add_root_certificate(
                    Certificate::from_pem(root_certificate_pem.as_bytes())
                        .map_err(|e| error::Error::BadConfig(format!("Invalid Certs: {e:#}")))?,
                );
            } else {
                connector.danger_accept_invalid_certs(true);
                connector.danger_accept_invalid_hostnames(true);
            }
        } else {
            connector
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        let (client, connection) = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            tokio_postgres::connect(
                &database_string,
                MakeTlsConnector::new(connector.build().map_err(to_anyhow)?),
            ),
        )
        .await
        .map_err(to_anyhow)?
        .map_err(to_anyhow)?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                let mut mtex = CONNECTION_CACHE.lock().await;
                *mtex = None;
                tracing::error!("connection error: {}", e);
            }
        });
        Some((client, handle))
    } else {
        tracing::info!("Creating new connection");
        let (client, connection) = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            tokio_postgres::connect(&database_string, NoTls),
        )
        .await
        .map_err(to_anyhow)?
        .map_err(to_anyhow)?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                let mut mtex = CONNECTION_CACHE.lock().await;
                *mtex = None;
                tracing::error!("connection error: {}", e);
            }
        });
        Some((client, handle))
    };

    let queries = parse_sql_blocks(query);

    let (client, handle) = if let Some((client, handle)) = new_client.as_ref() {
        (client, Some(handle))
    } else {
        let (_, client) = mtex.as_ref().unwrap().as_ref().unwrap();
        (client, None)
    };

    let sig = parse_pgsql_sig(&query).map_err(|x| Error::ExecutionErr(x.to_string()))?;
    let param_idx_to_arg_and_value = sig
        .args
        .iter()
        .filter_map(|x| x.oidx.map(|oidx| (oidx, (x, pg_args.get(&x.name)))))
        .collect::<HashMap<_, _>>();

    let size = AtomicUsize::new(0);
    let result_f = if queries.len() > 1 {
        let futures = queries
            .iter()
            .enumerate()
            .map(|(i, x)| {
                do_postgresql_inner(
                    x.to_string(),
                    &param_idx_to_arg_and_value,
                    client,
                    None,
                    &size,
                    annotations.return_last_result && i < queries.len() - 1,
                )
            })
            .collect::<error::Result<Vec<_>>>()?;

        let f = async {
            let mut res: Vec<Box<RawValue>> = vec![];
            for fut in futures {
                let r = fut.await?;
                res.push(r);
            }
            if annotations.return_last_result && res.len() > 0 {
                Ok(res.pop().unwrap())
            } else {
                Ok(to_raw_value(&res))
            }
        };

        f.boxed()
    } else {
        do_postgresql_inner(
            query.to_string(),
            &param_idx_to_arg_and_value,
            client,
            Some(column_order),
            &size,
            false,
        )?
    };

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
    )
    .await?;

    *mem_peak = size.load(Ordering::Relaxed) as i32;

    RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);

    if let Some(handle) = handle {
        if let Some(mut mtex) = mtex {
            let abort_handler = handle.abort_handle();

            if let Some(new_client) = new_client {
                *mtex = Some((database_string, new_client.0));
            }
            drop(mtex);
            LAST_QUERY.store(
                chrono::Utc::now().timestamp().try_into().unwrap_or(0),
                std::sync::atomic::Ordering::Relaxed,
            );

            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    let last_query = LAST_QUERY.load(std::sync::atomic::Ordering::Relaxed);
                    let now = chrono::Utc::now().timestamp().try_into().unwrap_or(0);

                    //we cache connection for 5 minutes at most
                    if last_query + 60 * 5 < now
                        && !RUNNING.load(std::sync::atomic::Ordering::Relaxed)
                    {
                        tracing::info!("Closing cache connection due to inactivity");
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

                    tracing::debug!("Keeping cached connection alive due to activity")
                }
                let mut mtex = CONNECTION_CACHE.lock().await;
                *mtex = None;
                abort_handler.abort();
            });
        } else {
            handle.abort();
        }
    }
    let raw_result = to_raw_value(&result);
    *mem_peak = (raw_result.get().len() / 1000) as i32;
    // And then check that we got back the same string we sent over.
    return Ok(raw_result);
}

#[derive(Debug)]
enum PgType {
    String(String),
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U32(u32),
    F32(f32),
    F64(f64),
    Uuid(Uuid),
    Decimal(Decimal),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Timestamp(chrono::NaiveDateTime),
    None(Option<bool>),
    Array(Vec<PgType>),
    Json(serde_json::Value),
    Bytea(Vec<u8>),
}

impl ToSql for PgType {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match *self {
            PgType::String(ref val) => val.to_sql(ty, out),
            PgType::Bool(ref val) => val.to_sql(ty, out),
            PgType::I8(ref val) => val.to_sql(ty, out),
            PgType::I16(ref val) => val.to_sql(ty, out),
            PgType::I32(ref val) => val.to_sql(ty, out),
            PgType::I64(ref val) => val.to_sql(ty, out),
            PgType::U32(ref val) => val.to_sql(ty, out),
            PgType::F32(ref val) => val.to_sql(ty, out),
            PgType::F64(ref val) => val.to_sql(ty, out),
            PgType::Uuid(ref val) => val.to_sql(ty, out),
            PgType::Decimal(ref val) => val.to_sql(ty, out),
            PgType::Date(ref val) => val.to_sql(ty, out),
            PgType::Time(ref val) => val.to_sql(ty, out),
            PgType::Timestamp(ref val) => val.to_sql(ty, out),
            PgType::None(ref val) => val.to_sql(ty, out),
            PgType::Array(ref val) => val.to_sql(ty, out),
            PgType::Json(ref val) => val.to_sql(ty, out),
            PgType::Bytea(ref val) => val.to_sql(ty, out),
        }
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    to_sql_checked!();
}

fn convert_val(value: &Value, arg_t: &String, typ: &Typ) -> windmill_common::error::Result<PgType> {
    match value {
        Value::Array(vec) if arg_t.ends_with("[]") => {
            let arg_t = arg_t.trim_end_matches("[]").to_string();
            let mut result = vec![];
            for val in vec {
                result.push(convert_val(val, &arg_t, typ)?);
            }
            Ok(PgType::Array(result))
        }
        Value::Null => Ok(PgType::None(None::<bool>)),
        Value::Bool(b) => Ok(PgType::Bool(b.clone())),
        Value::Number(n) if matches!(typ, Typ::Str(_)) => Ok(PgType::String(n.to_string())),
        Value::Number(n) if n.is_i64() && arg_t == "char" => {
            Ok(PgType::I8(n.as_i64().unwrap() as i8))
        }
        Value::Number(n) if n.is_i64() && (arg_t == "smallint" || arg_t == "smallserial") => {
            Ok(PgType::I16(n.as_i64().unwrap() as i16))
        }
        Value::Number(n)
            if n.is_i64()
                && (arg_t == "int"
                    || arg_t == "integer"
                    || arg_t == "int4"
                    || arg_t == "serial") =>
        {
            Ok(PgType::I32(n.as_i64().unwrap() as i32))
        }
        Value::Number(n) if n.is_i64() && (arg_t == "numeric" || arg_t == "decimal") => Ok(
            PgType::Decimal(Decimal::from_i64(n.as_i64().unwrap()).unwrap()),
        ),
        Value::Number(n) if n.is_i64() => Ok(PgType::I64(n.as_i64().unwrap())),
        Value::Number(n) if n.is_u64() && arg_t == "oid" => {
            Ok(PgType::U32(n.as_u64().unwrap() as u32))
        }
        Value::Number(n) if n.is_u64() && (arg_t == "bigint" || arg_t == "bigserial") => {
            Ok(PgType::I64(n.as_u64().unwrap() as i64))
        }
        Value::Number(n) if n.is_f64() && arg_t == "real" => {
            Ok(PgType::F32(n.as_f64().unwrap() as f32))
        }
        Value::Number(n) if n.is_f64() && arg_t == "double" => Ok(PgType::F64(n.as_f64().unwrap())),
        Value::Number(n) if n.is_f64() && (arg_t == "numeric" || arg_t == "decimal") => Ok(
            PgType::Decimal(Decimal::from_f64(n.as_f64().unwrap()).unwrap()),
        ),
        Value::Number(n) => Ok(PgType::F64(n.as_f64().unwrap())),
        Value::String(s) if arg_t == "uuid" => Ok(PgType::Uuid(Uuid::parse_str(s)?)),
        Value::String(s) if arg_t == "date" => {
            let date =
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            Ok(PgType::Date(date))
        }
        Value::String(s) if arg_t == "time" || arg_t == "timetz" => {
            let time =
                chrono::NaiveTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            Ok(PgType::Time(time))
        }
        Value::String(s) if arg_t == "timestamp" || arg_t == "timestamptz" => {
            let datetime = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ")
                .unwrap_or_default();
            Ok(PgType::Timestamp(datetime))
        }
        Value::String(s) if arg_t == "bytea" => {
            let bytes = engine::general_purpose::STANDARD
                .decode(s)
                .unwrap_or(vec![]);
            Ok(PgType::Bytea(bytes))
        }
        Value::Object(_) => Ok(PgType::Json(value.clone())),
        Value::String(s) => Ok(PgType::String(s.clone())),
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
        .try_get::<_, Option<Vec<T>>>(column_i)
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
                result.push(val_to_json_val(val)?);
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
