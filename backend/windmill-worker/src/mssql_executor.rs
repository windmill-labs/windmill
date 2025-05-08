use std::convert::Infallible;

use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use futures::StreamExt;
use regex::Regex;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use tiberius::{AuthMethod, Client, ColumnData, Config, FromSqlOwned, Query, Row, SqlBrowser};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use uuid::Uuid;
use windmill_common::s3_helpers::convert_json_line_stream;
use windmill_common::{
    error::{self, to_anyhow, Error},
    utils::empty_string_as_none,
    worker::{to_raw_value, Connection},
};
use windmill_parser_sql::{parse_db_resource, parse_mssql_sig, parse_s3_mode};
use windmill_queue::MiniPulledJob;
use windmill_queue::{append_logs, CanceledBy};

use crate::common::{build_args_values, s3_mode_args_to_worker_data, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::AuthedClient;

use serde::Deserializer;

#[derive(Deserialize)]
struct MssqlDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    dbname: String,
    instance_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_aad_token")]
    aad_token: Option<AadToken>,
    trust_cert: Option<bool>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    ca_cert: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AadToken {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    token: Option<String>,
}

lazy_static::lazy_static! {
    static ref RE_MSSQL_READONLY_INTENT: Regex = Regex::new(r#"(?mi)^-- ApplicationIntent=ReadOnly *(?:\r|\n|$)"#).unwrap();
}

pub async fn do_mssql(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    job_dir: &str,
) -> error::Result<Box<RawValue>> {
    let mssql_args = build_args_values(job, client, conn).await?;

    let inline_db_res_path = parse_db_resource(&query);
    let s3 = parse_s3_mode(&query).map(|s3| s3_mode_args_to_worker_data(s3, client.clone(), job));

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
        mssql_args.get("database").cloned()
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_value::<MssqlDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let annotations = windmill_common::worker::SqlAnnotations::parse(query);

    let mut config = Config::new();

    let host_ref = &database.host;
    let port_ref = database.port;

    config.host(host_ref.clone());
    config.database(database.dbname);
    let use_instance_name = database.instance_name.as_ref().is_some_and(|x| x != "");
    if use_instance_name {
        config.instance_name(database.instance_name.unwrap());
    }
    if let Some(port) = database.port {
        config.port(port);
    }

    let readonly_intent = RE_MSSQL_READONLY_INTENT.is_match(query);
    config.readonly(readonly_intent);

    if readonly_intent {
        let logs = format!("\nSetting ApplicationIntent to ReadOnly");
        append_logs(&job.id, &job.workspace_id, logs, conn).await;
    }

    // Handle authentication based on available credentials
    if let Some(token_value) = &database.aad_token {
        if let Some(token) = &token_value.token {
            config.authentication(AuthMethod::aad_token(token));
        } else {
            return Err(Error::BadRequest(
                "Invalid AAD token format - expected { token: string }".to_string(),
            ));
        }
    } else if let (Some(user), Some(password)) = (&database.user, &database.password) {
        config.authentication(AuthMethod::sql_server(user.clone(), password.clone()));
    } else {
        return Err(Error::BadRequest(
            "Neither AAD token nor username/password credentials are set".to_string(),
        ));
    }

    // Handle certificate trust configuration
    if database.trust_cert.unwrap_or(true) {
        // If trust_cert is true, ignore ca_cert and trust any certificate
        config.trust_cert();
        tracing::info!("MSSQL: disabling certificate validation");
    } else if let Some(ca_cert) = &database.ca_cert {
        // Only use ca_cert if trust_cert is false
        let cert_path = format!("{}/ca_cert.pem", job_dir);

        std::fs::write(&cert_path, ca_cert)
            .map_err(|e| Error::ExecutionErr(format!("Failed to write CA certificate: {}", e)))?;

        // Use the CA certificate for trust
        config.trust_cert_ca(cert_path);
        tracing::info!("MSSQL: using provided CA certificate for trust");
    }

    let tcp = if use_instance_name {
        TcpStream::connect_named(&config).await.map_err(to_anyhow)? // named instance
    } else {
        TcpStream::connect(config.get_addr()).await?
    };
    tcp.set_nodelay(true)?;

    // NOTE Azure default behavior with SQL Server is to redirect:
    // https://learn.microsoft.com/en-us/azure/azure-sql/database/connectivity-architecture?view=azuresql#connection-policy
    // https://github.com/prisma/tiberius?tab=readme-ov-file#redirects
    let mut client = match Client::connect(config.clone(), tcp.compat_write()).await {
        Ok(client) => {
            tracing::debug!("Connected to host: {:#?}, port: {:#?}", host_ref, port_ref);
            client
        }
        Err(tiberius::error::Error::Routing { host, port }) => {
            tracing::debug!("Redirecting to host: {:#?}, port: {:#?}", host, port);
            config.host(&host);
            config.port(port);

            let tcp = TcpStream::connect(config.get_addr()).await?;
            tcp.set_nodelay(true)?;

            Client::connect(config, tcp.compat_write())
                .await
                .map_err(to_anyhow)?
        }
        Err(e) => return Err(to_anyhow(e).into()),
    };

    let sig = parse_mssql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let (query, args_to_skip) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &mssql_args)?;

    let mut prepared_query = Query::new(query.to_owned());
    for arg in &sig {
        if args_to_skip.contains(&arg.name) {
            continue;
        }
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "string".to_string());
        let arg_v = mssql_args
            .get(&arg.name)
            .cloned()
            .unwrap_or(serde_json::json!(""));
        json_value_to_sql(&mut prepared_query, &arg_v, &arg_t)?;
    }

    let result_f = async {
        // A response to a query is a stream of data, that must be
        // polled to the end before querying again. Using streams allows
        // fetching data in an asynchronous manner, if needed.

        if let Some(s3) = s3 {
            let rows_stream = async_stream::stream! {
                let mut stream = prepared_query.query(&mut client).await.map_err(to_anyhow)?.into_row_stream().map(|row| {
                    row_to_json(row.map_err(to_anyhow)?).map_err(to_anyhow)
                });
                while let Some(row) = stream.next().await {
                    yield row;
                }
            };

            let stream = convert_json_line_stream(rows_stream.boxed(), s3.format)
                .await?
                .map(|chunk| Ok::<_, Infallible>(chunk));
            s3.upload(stream).await?;

            Ok(serde_json::value::to_raw_value(&s3.object_key)?)
        } else {
            let stream = prepared_query.query(&mut client).await.map_err(to_anyhow)?;
            let results = stream.into_results().await.map_err(to_anyhow)?;
            let len = results.len();
            let mut json_results = vec![];
            for (i, statement_result) in results.into_iter().enumerate() {
                if annotations.return_last_result && i < len - 1 {
                    continue;
                }
                let mut json_rows = vec![];
                for row in statement_result {
                    let row = row_to_json(row)?;
                    json_rows.push(row);
                }
                json_results.push(json_rows);
            }
            if annotations.return_last_result && json_results.len() > 0 {
                Ok(to_raw_value(&json_results.pop().unwrap()))
            } else {
                Ok(to_raw_value(&json_results))
            }
        }
    };

    let raw_result = run_future_with_polling_update_job_poller(
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

    *mem_peak = (raw_result.get().len() / 1000) as i32;

    Ok(raw_result)
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

fn row_to_json(row: Row) -> Result<Value, Error> {
    let cols = row
        .columns()
        .iter()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    let mut map = Map::new();
    for (col, val) in cols.iter().zip(row.into_iter()) {
        map.insert(col.name().to_string(), sql_to_json_value(val)?);
    }
    Ok(Value::Object(map))
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

fn deserialize_aad_token<'de, D>(deserializer: D) -> Result<Option<AadToken>, D::Error>
where
    D: Deserializer<'de>,
{
    let result = AadToken::deserialize(deserializer);

    match result {
        Ok(token) if token.token.is_some() => Ok(Some(token)),
        _ => Ok(None),
    }
}
