use base64::Engine;
use mysql_async::{
    consts::ColumnType, prelude::*, FromValueError, OptsBuilder, Params, Row, SslOpts,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use windmill_common::{
    error::{to_anyhow, Error},
    jobs::QueuedJob,
};
use windmill_parser_sql::parse_mysql_sig;

use crate::{transform_json_value, AuthedClient, JobCompleted};

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
    job: QueuedJob,
    client: &AuthedClient,
    query: &str,
) -> windmill_common::error::Result<JobCompleted> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };

    let mysql_args: serde_json::Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let database = serde_json::from_value::<MysqlDatabase>(
        mysql_args.get("database").unwrap_or(&json!({})).clone(),
    )
    .map_err(|e: serde_json::Error| Error::ExecutionErr(e.to_string()))?;

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

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());
    let mut statement_values: Vec<mysql_async::Value> = vec![];

    let sig = parse_mysql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "text".to_string());
        let mysql_v = match args.get(arg.name.as_str()).unwrap_or_else(|| &json!(null)) {
            Value::Null => mysql_async::Value::NULL,
            Value::Bool(b) => mysql_async::Value::Int(if *b { 1 } else { 0 }),
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
        statement_values.push(mysql_v);
    }
    let rows: Vec<Row> = conn
        .exec(query, Params::Positional(statement_values))
        .await
        .map_err(to_anyhow)?;
    let rows = rows
        .into_iter()
        .map(|x| convert_row_to_value(x))
        .collect::<Vec<serde_json::Value>>();

    drop(conn);

    pool.disconnect().await.map_err(to_anyhow)?;

    // And then check that we got back the same string we sent over.
    return Ok(JobCompleted { job: job, result: json!(rows), logs: "".to_string(), success: true });
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
        d @ mysql_async::Value::Date(_, _, _, _, _, _, _) => json!(d.as_sql(true)),
        t @ mysql_async::Value::Time(_, _, _, _, _, _) => json!(t.as_sql(true)),
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
