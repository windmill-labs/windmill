use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use futures::TryStreamExt;
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Map;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tokio_postgres::types::IsNull;
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
use windmill_parser_sql::parse_pgsql_sig;

use crate::common::build_args_values;
use crate::AuthedClientBackgroundTask;
use bytes::BytesMut;
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

pub async fn do_postgresql(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<Box<RawValue>> {
    let pg_args = build_args_values(job, client, db).await?;

    let database = if let Some(db) = pg_args.get("database") {
        serde_json::from_value::<PgDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };
    let sslmode = database.sslmode.unwrap_or("prefer".to_string());
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
                        .map_err(|e| error::Error::BadConfig(format!("Invalid Certs: {e}")))?,
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

        let (client, connection) = tokio_postgres::connect(
            &database_string,
            MakeTlsConnector::new(connector.build().map_err(to_anyhow)?),
        )
        .await
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
        let (client, connection) = tokio_postgres::connect(&database_string, NoTls)
            .await
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

    let mut statement_values: Vec<serde_json::Value> = vec![];

    let sig = parse_pgsql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    for arg in &sig {
        statement_values.push(
            pg_args
                .get(&arg.name)
                .map(|x| x.to_owned())
                .unwrap_or_else(|| serde_json::Value::Null),
        );
    }

    let query_params = statement_values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let arg_t = &sig[i]
                .otyp
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing otyp for pg arg"))?
                .to_owned();
            convert_val(value, arg_t)
        })
        .collect::<windmill_common::error::Result<Vec<_>>>()?;

    let (client, handle) = if let Some((client, handle)) = new_client.as_ref() {
        (client, Some(handle))
    } else {
        let (_, client) = mtex.as_ref().unwrap().as_ref().unwrap();
        (client, None)
    };
    // Now we can execute a simple statement that just returns its parameter.
    let rows = client
        .query_raw(query, query_params)
        .await
        .map_err(to_anyhow)?;

    let result = json!(rows
        .try_collect::<Vec<Row>>()
        .await
        .map_err(to_anyhow)?
        .into_iter()
        .map(postgres_row_to_json_value)
        .collect::<Result<Vec<_>, _>>()?);
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
    // And then check that we got back the same string we sent over.
    return Ok(to_raw_value(&result));
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
        }
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    to_sql_checked!();
}

fn convert_val(value: &Value, arg_t: &String) -> windmill_common::error::Result<PgType> {
    match value {
        Value::Array(vec) if arg_t.ends_with("[]") => {
            let arg_t = arg_t.trim_end_matches("[]").to_string();
            let mut result = vec![];
            for val in vec {
                result.push(convert_val(val, &arg_t)?);
            }
            Ok(PgType::Array(result))
        }
        Value::Null => Ok(PgType::None(None::<bool>)),
        Value::Bool(b) => Ok(PgType::Bool(b.clone())),
        Value::Number(n) if n.is_i64() && arg_t == "char" => {
            Ok(PgType::I8(n.as_i64().unwrap() as i8))
        }
        Value::Number(n) if n.is_i64() && (arg_t == "smallint" || arg_t == "smallserial") => {
            Ok(PgType::I16(n.as_i64().unwrap() as i16))
        }
        Value::Number(n) if n.is_i64() && (arg_t == "int" || arg_t == "serial") => {
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
        Value::String(s) if arg_t == "time" => {
            let time =
                chrono::NaiveTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            Ok(PgType::Time(time))
        }
        Value::String(s) if arg_t == "timestamp" => {
            let datetime = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ")
                .unwrap_or_default();
            Ok(PgType::Timestamp(datetime))
        }
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
        Type::TIMESTAMPTZ => get_basic(row, column, column_i, |a: chrono::DateTime<Utc>| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::UUID => get_basic(row, column, column_i, |a: uuid::Uuid| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::INET => get_basic(row, column, column_i, |a: IpAddr| {
            Ok(JSONValue::String(a.to_string()))
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
        // these types require a custom StringCollector struct as an intermediary (see struct at bottom)
        Type::TS_VECTOR => get_basic(row, column, column_i, |a: StringCollector| {
            Ok(JSONValue::String(a.0))
        })?,

        // array types
        Type::BOOL_ARRAY => get_array(row, column, column_i, |a: bool| Ok(JSONValue::Bool(a)))?,
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
