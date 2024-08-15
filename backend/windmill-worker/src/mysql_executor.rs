use std::{collections::HashMap, sync::Arc};

use base64::Engine;
use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;
use mysql_async::{
    consts::ColumnType, prelude::*, FromValueError, OptsBuilder, Params, Row, SslOpts,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue, Value};
use sqlx::types::Json;
use tokio::sync::Mutex;
use windmill_common::{
    error::{to_anyhow, Error},
    jobs::QueuedJob,
};
use windmill_parser_sql::{
    parse_db_resource, parse_mysql_sig, parse_sql_blocks, parse_sql_statement_named_params,
    RE_ARG_MYSQL_NAMED,
};
use windmill_queue::CanceledBy;

use crate::{
    common::{build_args_map, run_future_with_polling_update_job_poller},
    AuthedClientBackgroundTask,
};

#[derive(Deserialize)]
struct MysqlDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    database: String,
    ssl: Option<bool>,
}

pub fn do_mysql_inner<'a>(
    query: &'a str,
    all_statement_values: &Params,
    conn: Arc<Mutex<mysql_async::Conn>>,
    column_order: Option<&'a mut Option<Vec<String>>>,
) -> windmill_common::error::Result<BoxFuture<'a, anyhow::Result<Vec<Value>>>> {
    let param_names = parse_sql_statement_named_params(query, ':')
        .into_iter()
        .map(|x| x.into_bytes())
        .collect_vec();

    let statement_values = if let Params::Named(m) = all_statement_values {
        Params::Named(
            m.into_iter()
                .filter(|(k, _)| param_names.contains(&k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    } else {
        all_statement_values.clone()
    };

    let result_f = async move {
        let rows: Vec<Row> = conn
            .lock()
            .await
            .exec(query, statement_values)
            .await
            .map_err(to_anyhow)?;

        if let Some(column_order) = column_order {
            *column_order = Some(
                rows.first()
                    .map(|x| {
                        x.columns()
                            .iter()
                            .map(|x| x.name_str().to_string())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default(),
            );
        }

        Ok(rows
            .into_iter()
            .map(|x| convert_row_to_value(x))
            .collect::<Vec<serde_json::Value>>())
            as Result<Vec<serde_json::Value>, anyhow::Error>
    };

    Ok(result_f.boxed())
}

pub async fn do_mysql(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
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

    let sig = parse_mysql_sig(query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let using_named_params = RE_ARG_MYSQL_NAMED.captures_iter(query).count() > 0;

    let mut statement_values: Params = match using_named_params {
        true => Params::Named(HashMap::new()),
        false => Params::Positional(vec![]),
    };
    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "text".to_string());
        let arg_n = arg.name.clone();
        let mysql_v = match job_args
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
                if n.is_i64()
                    && (arg_t == "int"
                        || arg_t == "integer"
                        || arg_t == "smallint"
                        || arg_t == "bigint") =>
            {
                mysql_async::Value::Int(n.as_i64().unwrap())
            }
            Value::Number(n) if n.is_f64() && arg_t == "float" => {
                (n.as_f64().unwrap() as f32).into()
            }
            Value::Number(n) if n.is_i64() && arg_t == "float" => {
                (n.as_i64().unwrap() as f32).into()
            }
            Value::Number(n) if n.is_u64() && arg_t == "uint" => {
                mysql_async::Value::UInt(n.as_u64().unwrap())
            }
            Value::Number(n)
                if n.is_f64() && (arg_t == "real" || arg_t == "dec" || arg_t == "fixed") =>
            {
                n.as_f64().unwrap().into()
            }
            Value::Number(n)
                if n.is_i64() && (arg_t == "real" || arg_t == "dec" || arg_t == "fixed") =>
            {
                (n.as_i64().unwrap() as f64).into()
            }
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

    let pool = mysql_async::Pool::new(opts);
    let conn = pool.get_conn().await.map_err(to_anyhow)?;
    let conn_a = Arc::new(Mutex::new(conn));

    let queries = parse_sql_blocks(query);

    let result_f = if queries.len() > 1 {
        let futures = queries
            .iter()
            .map(|x| do_mysql_inner(x, &statement_values, conn_a.clone(), None))
            .collect::<windmill_common::error::Result<Vec<_>>>()?;

        let f = async {
            let mut res: Vec<serde_json::Value> = vec![];
            for fut in futures {
                let r = fut.await?;
                res.push(serde_json::to_value(r).map_err(to_anyhow)?);
            }
            Ok(res)
        };

        f.boxed()
    } else {
        do_mysql_inner(query, &statement_values, conn_a.clone(), Some(column_order))?
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
    )
    .await?;

    drop(conn_a);

    pool.disconnect().await.map_err(to_anyhow)?;

    let raw_result = windmill_common::worker::to_raw_value(&json!(result));
    *mem_peak = (raw_result.get().len() / 1000) as i32;

    // And then check that we got back the same string we sent over.
    return Ok(raw_result);
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
            ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL => {
                conversion_error(rust_decimal::Decimal::from_value_opt(v))
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
