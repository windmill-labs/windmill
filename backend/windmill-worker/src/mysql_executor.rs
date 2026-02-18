use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use base64::Engine;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use itertools::Itertools;
use mysql_async::{
    consts::ColumnType, prelude::*, FromValueError, OptsBuilder, Params, Row, SslOpts,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue, Value};
use std::str::FromStr;
use tokio::sync::Mutex;
use windmill_common::{
    client::AuthedClient,
    error::{to_anyhow, Error},
    worker::{to_raw_value, Connection, SqlResultCollectionStrategy},
};
use windmill_object_store::convert_json_line_stream;
use windmill_parser_sql::{
    parse_db_resource, parse_mysql_sig, parse_s3_mode, parse_sql_blocks,
    parse_sql_statement_named_params, RE_ARG_MYSQL_NAMED,
};
use windmill_queue::CanceledBy;
use windmill_queue::MiniPulledJob;

use crate::{
    common::{
        build_args_values, get_reserved_variables, s3_mode_args_to_worker_data, OccupancyMetrics,
        S3ModeWorkerData,
    },
    handle_child::run_future_with_polling_update_job_poller,
    sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args,
};

#[derive(Deserialize)]
pub struct MysqlDatabase {
    pub host: String,
    pub user: Option<String>,
    pub password: Option<String>,
    pub port: Option<u16>,
    pub database: String,
    pub ssl: Option<bool>,
}

fn do_mysql_inner<'a>(
    query: &'a str,
    all_statement_values: &Params,
    conn: Arc<Mutex<mysql_async::Conn>>,
    column_order: Option<&'a mut Option<Vec<String>>>,
    skip_collect: bool,
    first_row_only: bool,
    s3: Option<S3ModeWorkerData>,
) -> windmill_common::error::Result<BoxFuture<'a, windmill_common::error::Result<Vec<Box<RawValue>>>>>
{
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
        if skip_collect {
            conn.lock()
                .await
                .exec_drop(query, statement_values)
                .await
                .map_err(to_anyhow)?;

            Ok(vec![])
        } else if let Some(ref s3) = s3 {
            let query = query.to_string();
            let rows_stream = async_stream::stream! {
                let mut conn = conn.lock().await;
                let mut result = match conn.exec_iter(query, statement_values).await.map_err(to_anyhow) {
                    Ok(result) => result,
                    Err(e) => {
                        yield Err(anyhow!("Error executing query: {:?}", e));
                        return;
                    }
                };
                loop {
                    let row = result.next().await;
                    match row {
                        Ok(Some(row)) => {
                            yield Ok(convert_row_to_value(row));
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(e) => {
                            yield Err(anyhow!("Error fetching row: {:?}", e));
                            return;
                        }
                    }
                }
            };

            let stream = convert_json_line_stream(rows_stream.boxed(), s3.format).await?;
            s3.upload(stream.boxed()).await?;

            Ok(vec![to_raw_value(&s3.to_return_s3_obj())])
        } else {
            let rows: Vec<Row> = if first_row_only {
                conn.lock()
                    .await
                    .exec_first(query, statement_values)
                    .await
                    .map_err(to_anyhow)?
                    .into_iter()
                    .collect()
            } else {
                conn.lock()
                    .await
                    .exec(query, statement_values)
                    .await
                    .map_err(to_anyhow)?
            };

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
                .map(|x| to_raw_value(&convert_row_to_value(x)))
                .collect::<Vec<_>>())
        }
    };

    Ok(result_f.boxed())
}

pub async fn do_mysql(
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
        serde_json::from_value::<MysqlDatabase>(db)
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

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let (query, args_to_skip) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args, &reserved_variables)?;

    let using_named_params = RE_ARG_MYSQL_NAMED.captures_iter(query).count() > 0;

    let mut statement_values: Params = match using_named_params {
        true => Params::Named(HashMap::new()),
        false => Params::Positional(vec![]),
    };
    for arg in &sig {
        if args_to_skip.contains(&arg.name) {
            continue;
        }
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "text".to_string());
        let arg_n = arg.name.clone();
        let mysql_v = match job_args
            .get(arg.name.as_str())
            .unwrap_or_else(|| &json!(null))
        {
            Value::Null => mysql_async::Value::NULL,
            Value::Bool(b) => mysql_async::Value::Int(if *b { 1 } else { 0 }),
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
    let mysql_conn = pool.get_conn().await.map_err(to_anyhow)?;
    let conn_a = Arc::new(Mutex::new(mysql_conn));

    let queries = parse_sql_blocks(query);

    let conn_a_ref = &conn_a;
    let result_f = async move {
        let mut results = vec![];
        for (i, query) in queries.iter().enumerate() {
            let result = do_mysql_inner(
                query,
                &statement_values,
                conn_a_ref.clone(),
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

    drop(conn_a);

    pool.disconnect().await.map_err(to_anyhow)?;

    *mem_peak = (result.get().len() / 1000) as i32;

    // And then check that we got back the same string we sent over.
    return Ok(result);
}

// 2023-12-01T16:18:00.000Z
static DATE_REGEX_TZ: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d+)Z").unwrap()
});
// 2025-04-21 10:08:00
static DATE_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})").unwrap());
// 2025-01-05
static DATE_REGEX_DATE_ONLY: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").unwrap());

fn string_date_to_mysql_date(s: &str) -> mysql_async::Value {
    // Try ISO format with timezone (most specific)
    if let Some(caps) = DATE_REGEX_TZ.captures(s) {
        return mysql_async::Value::Date(
            get_capture_by_index(&caps, 1),
            get_capture_by_index(&caps, 2),
            get_capture_by_index(&caps, 3),
            get_capture_by_index(&caps, 4),
            get_capture_by_index(&caps, 5),
            get_capture_by_index(&caps, 6),
            get_capture_by_index(&caps, 7),
        );
    }

    // Try datetime without timezone
    if let Some(caps) = DATE_REGEX.captures(s) {
        return mysql_async::Value::Date(
            get_capture_by_index(&caps, 1),
            get_capture_by_index(&caps, 2),
            get_capture_by_index(&caps, 3),
            get_capture_by_index(&caps, 4),
            get_capture_by_index(&caps, 5),
            get_capture_by_index(&caps, 6),
            0,
        );
    }

    // Try date-only format (YYYY-MM-DD)
    if let Some(caps) = DATE_REGEX_DATE_ONLY.captures(s) {
        return mysql_async::Value::Date(
            get_capture_by_index(&caps, 1),
            get_capture_by_index(&caps, 2),
            get_capture_by_index(&caps, 3),
            0,
            0,
            0,
            0,
        );
    }

    // Fallback for invalid format
    mysql_async::Value::Date(0, 0, 0, 0, 0, 0, 0)
}

fn get_capture_by_index<T: FromStr + Default>(caps: &regex::Captures, n: usize) -> T {
    caps.get(n)
        .and_then(|s| s.as_str().parse::<T>().ok())
        .unwrap_or_default()
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
