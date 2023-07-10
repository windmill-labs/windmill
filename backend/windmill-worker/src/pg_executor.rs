use anyhow::Context;
use chrono::Utc;
use futures::TryStreamExt;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::Deserialize;
use serde_json::Map;
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, NoTls, Row};
use tokio_postgres::{
    types::{FromSql, Type},
    Column,
};

use windmill_common::error::Error;
use windmill_common::{error::to_anyhow, jobs::QueuedJob};
use windmill_parser_sql::parse_sql_sig;

use crate::{get_content, transform_json_value, AuthedClient, JobCompleted};

#[derive(Deserialize)]
struct PgDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    sslmode: Option<String>,
    dbname: String,
}

pub async fn do_postgresql(
    job: QueuedJob,
    client: &AuthedClient,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<JobCompleted> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };

    let pg_args: serde_json::Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let database =
        serde_json::from_value::<PgDatabase>(pg_args.get("database").unwrap_or(&json!({})).clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let sslmode = database.sslmode.unwrap_or("prefer".to_string());
    let database = format!(
        "postgres://{user}:{password}@{host}:{port}/{dbname}?sslmode={sslmode}",
        user = database.user.unwrap_or("postgres".to_string()),
        password = database.password.unwrap_or("".to_string()),
        host = database.host,
        port = database.port.unwrap_or(5432),
        dbname = database.dbname,
        sslmode = sslmode
    );
    let (client, handle) = if sslmode == "require" {
        let (client, connection) = tokio_postgres::connect(
            &database,
            MakeTlsConnector::new(
                TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .build()
                    .map_err(to_anyhow)?,
            ),
        )
        .await
        .map_err(to_anyhow)?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        (client, handle)
    } else {
        let (client, connection) = tokio_postgres::connect(&database, NoTls)
            .await
            .map_err(to_anyhow)?;
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        (client, handle)
    };

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());
    let mut i = 1;
    let mut statement_values: Vec<serde_json::Value> = vec![];

    loop {
        if args.get(&format!("${}", i)).is_none() {
            break;
        }
        statement_values.push(args.get(&format!("${}", i)).unwrap().to_owned());
        i += 1;
    }
    let query = get_content(&job, db).await?;

    let sig = parse_sql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let query_params = statement_values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let arg_t = &sig[i]
                .otyp
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing otyp for pg arg"))?
                .to_owned();

            let boxed: windmill_common::error::Result<Box<dyn ToSql + Sync + Send>> = match value {
                Value::Null => Ok(Box::new(None::<bool>)),
                Value::Bool(b) => Ok(Box::new(b.clone())),
                Value::Number(n) if n.is_i64() && arg_t == "char" => {
                    Ok(Box::new(n.as_i64().unwrap() as i8))
                }
                Value::Number(n)
                    if n.is_i64() && (arg_t == "smallint" || arg_t == "smallserial") =>
                {
                    Ok(Box::new(n.as_i64().unwrap() as i16))
                }
                Value::Number(n) if n.is_i64() && (arg_t == "int" || arg_t == "serial") => {
                    Ok(Box::new(n.as_i64().unwrap() as i32))
                }
                Value::Number(n) if n.is_i64() => Ok(Box::new(n.as_i64().unwrap())),
                Value::Number(n) if n.is_u64() && arg_t == "oid" => {
                    Ok(Box::new(n.as_u64().unwrap() as u32))
                }
                Value::Number(n) if n.is_u64() && (arg_t == "bigint" || arg_t == "bigserial") => {
                    Ok(Box::new(n.as_u64().unwrap() as i64))
                }
                Value::Number(n) if n.is_f64() && arg_t == "real" => {
                    Ok(Box::new(n.as_f64().unwrap() as f32))
                }
                Value::Number(n) if n.is_f64() && arg_t == "double" => {
                    Ok(Box::new(n.as_f64().unwrap()))
                }
                Value::Number(n) => Ok(Box::new(n.as_f64().unwrap())),
                Value::String(s) => Ok(Box::new(s.clone())),
                _ => Err(Error::ExecutionErr(format!(
                    "Unsupported type in query: {:?} and signature {arg_t:?}",
                    value
                ))),
            };
            boxed
        })
        .collect::<windmill_common::error::Result<Vec<_>>>()?;
    // Now we can execute a simple statement that just returns its parameter.
    let rows = client
        .query_raw(&query, query_params)
        .await
        .map_err(to_anyhow)?;

    let result = json!(rows
        .try_collect::<Vec<Row>>()
        .await
        .map_err(to_anyhow)?
        .into_iter()
        .map(postgres_row_to_json_value)
        .collect::<Result<Vec<_>, _>>()?);

    handle.abort();
    // And then check that we got back the same string we sent over.
    return Ok(JobCompleted { job: job, result, logs: "".to_string(), success: true });
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
        Type::TIMESTAMPTZ => get_basic(row, column, column_i, |a: chrono::DateTime<Utc>| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::UUID => get_basic(row, column, column_i, |a: uuid::Uuid| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        // Type::DATE => get_basic(row, column, column_i, |a: chrono::NaiveDate| {
        //     Ok(JSONValue::String(a.to_string()))
        // })?,
        Type::JSON | Type::JSONB => get_basic(row, column, column_i, |a: JSONValue| Ok(a))?,
        Type::FLOAT4 => get_basic(row, column, column_i, |a: f32| {
            Ok(f64_to_json_number(a.into())?)
        })?,
        Type::FLOAT8 => get_basic(row, column, column_i, |a: f64| Ok(f64_to_json_number(a)?))?,
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
            "conversion issue for value at column_name:{} with type {:?}",
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
                "conversion issue for array at column_name:{}",
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
