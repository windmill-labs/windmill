use std::collections::HashMap;

use base64::Engine;
use mysql_async::{
    consts::ColumnType, prelude::*, FromValueError, OptsBuilder, Params, Row, SslOpts,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue, Value};
use sqlx::types::Json;
use windmill_common::{
    error::{to_anyhow, Error},
    jobs::QueuedJob,
};
use windmill_parser_sql::{parse_mysql_sig, RE_ARG_MYSQL_NAMED};

use crate::{common::build_args_map, AuthedClientBackgroundTask};

#[derive(Deserialize)]
struct MysqlDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    database: String,
    ssl: Option<bool>,
}

pub async fn do_mysql(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let database = if let Some(db) = job_args.and_then(|x| x.get("database")) {
        serde_json::from_str::<MysqlDatabase>(db.get())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let opts = OptsBuilder::default()
        .db_name(Some(database.database))
        .user(database.user)
        .pass(database.password)
        .ip_or_hostname(database.host)
        .tcp_port(database.port.unwrap_or(3306));

    let opts = if database.ssl.unwrap_or(false) {
        opts.ssl_opts({
            SslOpts::default()
                .with_danger_skip_domain_validation(true)
                .with_danger_accept_invalid_certs(true)
        })
    } else {
        opts
    };

    let pool = mysql_async::Pool::new(opts);
    let mut conn = pool.get_conn().await.map_err(to_anyhow)?;

    let sig = parse_mysql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let using_named_params = RE_ARG_MYSQL_NAMED.captures_iter(&query).count() > 0;

    let mut statement_values: Params = match using_named_params {
        true => Params::Named(HashMap::new()),
        false => Params::Positional(vec![]),
    };
    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "text".to_string());
        let arg_n = arg.clone().name;
        let mysql_v = match job
            .args
            .as_ref()
            .and_then(|x| {
                x.get(arg.name.as_str())
                    .map(|x| serde_json::from_str::<serde_json::Value>(x.get()).ok())
            })
            .flatten()
            .unwrap_or_else(|| json!(null))
        {
            Value::Null => mysql_async::Value::NULL,
            Value::Bool(b) => mysql_async::Value::Int(if b { 1 } else { 0 }),
            Value::String(s)
                if arg_t == "timestamp"
                    || arg_t == "datetime"
                    || arg_t == "date"
                    || arg_t == "time" =>
            {
                string_date_to_mysql_date(&s)
            }
            Value::String(s) => mysql_async::Value::Bytes(s.as_bytes().to_vec()),
            Value::Number(n)
                if n.is_i64() && (arg_t == "int" || arg_t == "integer" || arg_t == "smallint") =>
            {
                mysql_async::Value::Int(n.as_i64().unwrap())
            }
            Value::Number(n) if n.is_u64() && arg_t == "uint" => {
                mysql_async::Value::UInt(n.as_u64().unwrap())
            }
            Value::Number(n) if n.is_f64() && arg_t == "float" => {
                (n.as_f64().unwrap() as f32).into()
            }
            Value::Number(n) if n.is_f64() && arg_t == "real" => n.as_f64().into(),
            value @ _ => {
                return Err(Error::ExecutionErr(format!(
                    "Unsupported type in query: {:?} and signature {arg_t:?}",
                    value
                )))
            }
        };
        match &mut statement_values {
            Params::Positional(v) => v.push(mysql_v),
            Params::Named(m) => {
                m.insert(arg_n.into_bytes(), mysql_v);
            }
            _ => {}
        }
    }
    let rows: Vec<Row> = conn
        .exec(
            query,
            match statement_values {
                Params::Positional(v) => Params::Positional(v),
                Params::Named(m) => Params::Named(m),
                _ => Params::Empty,
            },
        )
        .await
        .map_err(to_anyhow)?;
    let rows = rows
        .into_iter()
        .map(|x| convert_row_to_value(x))
        .collect::<Vec<serde_json::Value>>();

    drop(conn);

    pool.disconnect().await.map_err(to_anyhow)?;

    // And then check that we got back the same string we sent over.
    return Ok(windmill_common::worker::to_raw_value(&json!(rows)));
}

fn string_date_to_mysql_date(s: &str) -> mysql_async::Value {
    // 2023-12-01T16:18:00.000Z
    let re = regex::Regex::new(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d+)Z").unwrap();
    let caps = re.captures(s);

    if let Some(caps) = caps {
        mysql_async::Value::Date(
            caps.get(1).unwrap().as_str().parse().unwrap_or_default(),
            caps.get(2).unwrap().as_str().parse().unwrap_or_default(),
            caps.get(3).unwrap().as_str().parse().unwrap_or_default(),
            caps.get(4).unwrap().as_str().parse().unwrap_or_default(),
            caps.get(5).unwrap().as_str().parse().unwrap_or_default(),
            caps.get(6).unwrap().as_str().parse().unwrap_or_default(),
            caps.get(7).unwrap().as_str().parse().unwrap_or_default(),
        )
    } else {
        mysql_async::Value::Date(0, 0, 0, 0, 0, 0, 0)
    }
}

fn convert_row_to_value(row: Row) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for (key, value) in row.clone().columns_ref().iter().zip(row.unwrap()) {
        map.insert(
            key.name_str().to_string(),
            convert_mysql_value_to_json(value, key.column_type()),
        );
    }
    serde_json::Value::Object(map)
}

fn conversion_error<T: Serialize>(r: Result<T, FromValueError>) -> serde_json::Value {
    match r {
        Ok(v) => json!(v),
        Err(e) => json!(format!("Error converting value: {:?}", e)),
    }
}
fn convert_mysql_value_to_json(v: mysql_async::Value, c: ColumnType) -> serde_json::Value {
    return match v {
        mysql_async::Value::NULL => serde_json::Value::Null,
        mysql_async::Value::Bytes(b) if c.is_character_type() => {
            json!(String::from_utf8_lossy(&b).to_string())
        }
        mysql_async::Value::Int(n) => json!(n),
        mysql_async::Value::UInt(n) => json!(n),
        mysql_async::Value::Float(n) => json!(n),
        mysql_async::Value::Double(n) => json!(n),
        d @ mysql_async::Value::Date(_, _, _, _, _, _, _) => {
            json!(d.as_sql(true).trim_matches('\''))
        }
        t @ mysql_async::Value::Time(_, _, _, _, _, _) => json!(t.as_sql(true).trim_matches('\'')),
        _ => match c {
            ColumnType::MYSQL_TYPE_FLOAT | ColumnType::MYSQL_TYPE_DOUBLE => {
                conversion_error(f64::from_value_opt(v))
            }
            ColumnType::MYSQL_TYPE_TINY
            | ColumnType::MYSQL_TYPE_SHORT
            | ColumnType::MYSQL_TYPE_LONG
            | ColumnType::MYSQL_TYPE_LONGLONG => conversion_error(i64::from_value_opt(v)),
            ColumnType::MYSQL_TYPE_BIT
            | ColumnType::MYSQL_TYPE_BLOB
            | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
            | ColumnType::MYSQL_TYPE_LONG_BLOB
            | ColumnType::MYSQL_TYPE_TINY_BLOB => json!(base64::engine::general_purpose::STANDARD
                .encode(Vec::from_value_opt(v).unwrap_or_else(|_| vec![]))),
            | ColumnType::MYSQL_TYPE_DATETIME => {
                json!(String::from_value_opt(v).unwrap_or_else(|_| "".to_string()))
            }
            _ => json!(format!("Unsupported type {:?}", c)),
        },
    };
}
