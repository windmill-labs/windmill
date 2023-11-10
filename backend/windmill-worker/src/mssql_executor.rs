use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use tiberius::{AuthMethod, Client, ColumnData, Config, FromSqlOwned, Query, Row};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use uuid::Uuid;
use windmill_common::error::{self, Error};
use windmill_common::worker::to_raw_value;
use windmill_common::{error::to_anyhow, jobs::QueuedJob};
use windmill_parser_sql::parse_mssql_sig;

use crate::common::build_args_values;
use crate::AuthedClientBackgroundTask;

#[derive(Deserialize)]
struct MssqlDatabase {
    host: String,
    user: String,
    password: String,
    port: Option<u16>,
    dbname: String,
}

pub async fn do_mssql(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<Box<RawValue>> {
    let mssql_args = build_args_values(job, client, db).await?;

    let database = if let Some(db) = mssql_args.get("database") {
        serde_json::from_value::<MssqlDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let mut config = Config::new();

    config.host(database.host);
    config.port(database.port.unwrap_or(1433));
    config.database(database.dbname);

    // Using SQL Server authentication.
    config.authentication(AuthMethod::sql_server(database.user, database.password));
    config.trust_cert(); // on production, it is not a good idea to do this

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    // To be able to use Tokio's tcp, we're using the `compat_write` from
    // the `TokioAsyncWriteCompatExt` to get a stream compatible with the
    // traits from the `futures` crate.
    let mut client = Client::connect(config, tcp.compat_write())
        .await
        .map_err(to_anyhow)?;

    let sig = parse_mssql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let mut prepared_query = Query::new(query.to_owned());
    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "string".to_string());
        let arg_v = mssql_args
            .get(&arg.name)
            .cloned()
            .unwrap_or(serde_json::json!(""));
        json_value_to_sql(&mut prepared_query, &arg_v, &arg_t)?;
    }

    // A response to a query is a stream of data, that must be
    // polled to the end before querying again. Using streams allows
    // fetching data in an asynchronous manner, if needed.
    let stream = prepared_query.query(&mut client).await.map_err(to_anyhow)?;
    let rows = stream
        .into_results()
        .await
        .map_err(to_anyhow)?
        .into_iter()
        .map(|rows| {
            let result = rows
                .into_iter()
                .map(|row| row_to_json(row))
                .collect::<Result<Vec<Map<String, Value>>, Error>>();
            result
        })
        .collect::<Result<Vec<Vec<Map<String, Value>>>, Error>>()?;

    return Ok(to_raw_value(&rows));
}

fn json_value_to_sql<'a>(
    query: &mut Query,
    value: &Value,
    arg_t: &String,
) -> windmill_common::error::Result<()> {
    match value {
        Value::Null => {
            query.bind(None::<String>);
        }
        Value::Bool(b) => {
            query.bind(b.to_owned());
        }
        Value::Number(n) if n.is_u64() && arg_t == "tinyint" => {
            query.bind(n.as_u64().unwrap().to_owned() as u8)
        }
        Value::Number(n) if n.is_i64() && arg_t == "smallint" => {
            query.bind(n.as_i64().unwrap().to_owned() as i16)
        }
        Value::Number(n) if n.is_i64() && arg_t == "int" => {
            query.bind(n.as_i64().unwrap().to_owned() as i32)
        }
        Value::Number(n) if n.is_i64() && arg_t == "bigint" => {
            query.bind(n.as_i64().unwrap().to_owned() as i64)
        }
        Value::Number(n) if n.is_f64() && arg_t == "real" => {
            query.bind(n.as_f64().unwrap().to_owned() as f32)
        }
        Value::Number(n) => query.bind(n.as_f64().unwrap().to_owned()),
        Value::String(s) if arg_t == "uuid" => query.bind(Uuid::parse_str(s).map_err(to_anyhow)?),
        Value::String(s) if arg_t == "binary" || arg_t == "varbinary" || arg_t == "image" => {
            query.bind(general_purpose::STANDARD.decode(s).map_err(to_anyhow)?)
        }
        Value::String(s) if arg_t == "date" => {
            let date = NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            query.bind(date)
        }
        Value::String(s) if arg_t == "time" => {
            let time = NaiveTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            query.bind(time)
        }
        Value::String(s)
            if arg_t == "datetime" || arg_t == "datetime2" || arg_t == "smalldatetime" =>
        {
            let datetime =
                NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ").unwrap_or_default();
            query.bind(datetime)
        }
        Value::String(s) if arg_t == "datetimeoffset" => {
            let datetime = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ")
                .unwrap_or_default()
                .and_utc();
            query.bind(datetime)
        }
        Value::String(s) => query.bind(s.to_owned()),
        _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported type in query: {:?} and signature {arg_t:?}",
                value
            )))
        }
    };
    Ok(())
}

fn row_to_json(row: Row) -> Result<Map<String, Value>, Error> {
    let cols = row
        .columns()
        .iter()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    let mut map = Map::new();
    for (col, val) in cols.iter().zip(row.into_iter()) {
        map.insert(col.name().to_string(), sql_to_json_value(val)?);
    }
    Ok(map)
}

fn value_or_null<T>(
    val: Option<T>,
    convert: impl Fn(T) -> Result<Value, Error>,
) -> Result<Value, Error> {
    val.map_or(Ok(Value::Null), convert)
}

fn sql_to_json_value(val: ColumnData) -> Result<Value, Error> {
    match val {
        ColumnData::Bit(x) => value_or_null(x, |x| Ok(Value::Bool(x))),
        ColumnData::U8(x) => value_or_null(x, |x| Ok(Value::Number(x.into()))),
        ColumnData::I16(x) => value_or_null(x, |x| Ok(Value::Number(x.into()))),
        ColumnData::I32(x) => value_or_null(x, |x| Ok(Value::Number(x.into()))),
        ColumnData::I64(x) => value_or_null(x, |x| Ok(Value::Number(x.into()))),
        ColumnData::String(x) => value_or_null(x, |x| Ok(Value::String(x.to_string()))),
        ColumnData::Binary(x) => value_or_null(x, |x| {
            Ok(Value::String(general_purpose::STANDARD.encode(x.as_ref())))
        }),
        ColumnData::F32(x) => value_or_null(x, |x| {
            Ok(Value::Number(
                serde_json::Number::from_f64(x.into())
                    .ok_or(anyhow::anyhow!("invalid json-float"))?,
            ))
        }),
        ColumnData::F64(x) => value_or_null(x, |x| {
            Ok(Value::Number(
                serde_json::Number::from_f64(x).ok_or(anyhow::anyhow!("invalid json-float"))?,
            ))
        }),
        ColumnData::Guid(x) => value_or_null(x, |x| Ok(Value::String(x.to_string()))),
        ColumnData::Xml(x) => value_or_null(x, |x| Ok(Value::String(x.to_string()))),
        ColumnData::Numeric(x) => value_or_null(x, |x| {
            Ok(Value::Number(
                serde_json::Number::from_f64(x.into())
                    .ok_or(anyhow::anyhow!("invalid json-float"))?,
            ))
        }),
        ColumnData::DateTime(x) => value_or_null(
            NaiveDateTime::from_sql_owned(ColumnData::DateTime(x)).map_err(to_anyhow)?,
            |x| Ok(Value::String(x.to_string())),
        ),
        ColumnData::DateTime2(x) => value_or_null(
            NaiveDateTime::from_sql_owned(ColumnData::DateTime2(x)).map_err(to_anyhow)?,
            |x| Ok(Value::String(x.to_string())),
        ),
        ColumnData::SmallDateTime(x) => value_or_null(
            NaiveDateTime::from_sql_owned(ColumnData::SmallDateTime(x)).map_err(to_anyhow)?,
            |x| Ok(Value::String(x.to_string())),
        ),
        ColumnData::Time(x) => value_or_null(
            NaiveTime::from_sql_owned(ColumnData::Time(x)).map_err(to_anyhow)?,
            |x| Ok(Value::String(x.to_string())),
        ),
        ColumnData::Date(x) => value_or_null(
            NaiveDate::from_sql_owned(ColumnData::Date(x)).map_err(to_anyhow)?,
            |x| Ok(Value::String(x.to_string())),
        ),
        ColumnData::DateTimeOffset(x) => value_or_null(
            DateTime::<Utc>::from_sql_owned(ColumnData::DateTimeOffset(x)).map_err(to_anyhow)?,
            |x| Ok(Value::String(x.to_string())),
        ),
    }
}
