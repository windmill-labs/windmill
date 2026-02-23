use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use futures::StreamExt;
use regex::Regex;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;
use tiberius::{
    AuthMethod, Client, ColumnData, Config, EncryptionLevel, FromSqlOwned, Query, Row, SqlBrowser,
};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use uuid::Uuid;
use windmill_common::utils::merge_raw_values_to_object;
use windmill_common::worker::SqlResultCollectionStrategy;
use windmill_common::{
    error::{self, to_anyhow, Error},
    utils::empty_as_none,
    worker::{to_raw_value, Connection},
};
use windmill_object_store::convert_json_line_stream;
use windmill_parser_sql::{parse_db_resource, parse_mssql_sig, parse_s3_mode};
use windmill_queue::MiniPulledJob;
use windmill_queue::{append_logs, CanceledBy};

use crate::common::{
    build_args_values, get_reserved_variables, s3_mode_args_to_worker_data, OccupancyMetrics,
};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use windmill_common::client::AuthedClient;

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
    #[serde(default, deserialize_with = "empty_as_none")]
    ca_cert: Option<String>,
    encrypt: Option<bool>,
    integrated_auth: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AadToken {
    #[serde(default, deserialize_with = "empty_as_none")]
    token: Option<String>,
}

lazy_static::lazy_static! {
    static ref RE_MSSQL_READONLY_INTENT: Regex = Regex::new(r#"(?mi)^-- ApplicationIntent=ReadOnly *(?:\r|\n|$)"#).unwrap();
}

pub async fn do_mssql(
    job: &MiniPulledJob,
    authed_client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    job_dir: &str,
    parent_runnable_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    let mssql_args = build_args_values(job, authed_client, conn).await?;

    let inline_db_res_path = parse_db_resource(&query);
    let s3 = parse_s3_mode(&query)?
        .map(|s3| s3_mode_args_to_worker_data(s3, authed_client.clone(), job));

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        Some(
            authed_client
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
    let collection_strategy = if annotations.return_last_result {
        SqlResultCollectionStrategy::LastStatementAllRows
    } else if annotations.result_collection == SqlResultCollectionStrategy::Legacy {
        SqlResultCollectionStrategy::AllStatementsAllRows
    } else {
        annotations.result_collection
    };

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
    if database.integrated_auth.unwrap_or(false) {
        #[cfg(any(feature = "mssql-kerberos", feature = "mssql-winauth"))]
        {
            config.authentication(AuthMethod::Integrated);
            #[cfg(feature = "mssql-kerberos")]
            let logs = format!("\nUsing Integrated Authentication (Kerberos/GSSAPI)");
            #[cfg(feature = "mssql-winauth")]
            let logs = format!("\nUsing Integrated Authentication (Windows SSPI)");
            append_logs(&job.id, &job.workspace_id, logs, conn).await;
        }
        #[cfg(not(any(feature = "mssql-kerberos", feature = "mssql-winauth")))]
        {
            return Err(Error::BadRequest(
                "Integrated authentication is not available in this build. Requires mssql-kerberos (Linux) or mssql-winauth (Windows) feature.".to_string(),
            ));
        }
    } else if let Some(token_value) = &database.aad_token {
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
            "No authentication method configured. Set integrated_auth, aad_token, or user/password.".to_string(),
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

    config.encryption(if database.encrypt.unwrap_or(true) {
        EncryptionLevel::Required
    } else {
        EncryptionLevel::NotSupported
    });

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

    let reserved_variables =
        get_reserved_variables(job, &authed_client.token, conn, parent_runnable_path).await?;

    let (query, args_to_skip) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &mssql_args, &reserved_variables)?;

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
                    let raw_value = row_to_json(row.map_err(to_anyhow)?).map_err(to_anyhow);
                    let json = raw_value.and_then(|raw_value| serde_json::from_str(raw_value.get()).map_err(to_anyhow));
                    json
                });
                while let Some(row) = stream.next().await {
                    yield row;
                }
            };

            let stream = convert_json_line_stream(rows_stream.boxed(), s3.format).await?;
            s3.upload(stream.boxed()).await?;

            Ok(to_raw_value(&s3.to_return_s3_obj()))
        } else {
            let stream = prepared_query.query(&mut client).await.map_err(to_anyhow)?;
            let results = stream.into_results().await.map_err(to_anyhow)?;
            let len = results.len();
            let mut json_results = vec![];
            for (i, statement_result) in results.into_iter().enumerate() {
                if collection_strategy.collect_last_statement_only(len) && i < len - 1 {
                    continue;
                }
                let mut json_rows = vec![];
                for row in statement_result {
                    json_rows.push(row_to_json(row)?);
                    if collection_strategy.collect_first_row_only() {
                        break;
                    }
                }
                json_results.push(json_rows);
            }
            collection_strategy.collect(json_results)
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

fn row_to_json(row: Row) -> Result<Box<RawValue>, Error> {
    let cols = row
        .columns()
        .iter()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    let mut entries = Vec::new();
    for (col, val) in cols.iter().zip(row.into_iter()) {
        entries.push((col.name().to_string(), sql_to_json_value(val)?));
    }
    Ok(merge_raw_values_to_object(entries.as_slice()))
}

fn sql_to_json_value(val: ColumnData) -> Result<Box<RawValue>, Error> {
    let null = || RawValue::from_string("null".to_string()).unwrap();
    let val = match val {
        ColumnData::Bit(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::U8(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::I16(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::I32(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::I64(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::String(x) => x.map(|x| to_raw_value(&x.to_string())).unwrap_or_else(null),
        ColumnData::Binary(x) => x
            .map(|x| to_raw_value(&general_purpose::STANDARD.encode(x.as_ref())))
            .unwrap_or_else(null),
        ColumnData::F32(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::F64(x) => x.map(|x| to_raw_value(&x)).unwrap_or_else(null),
        ColumnData::Guid(x) => x.map(|x| to_raw_value(&x.to_string())).unwrap_or_else(null),
        ColumnData::Xml(x) => x.map(|x| to_raw_value(&x.to_string())).unwrap_or_else(null),
        ColumnData::Numeric(x) => x
            .map(|x| numeric_to_raw_value(&x))
            .transpose()?
            .unwrap_or_else(null),
        ColumnData::DateTime(x) => NaiveDateTime::from_sql_owned(ColumnData::DateTime(x))
            .map_err(to_anyhow)?
            .map(|x| to_raw_value(&x.to_string()))
            .unwrap_or_else(null),
        ColumnData::DateTime2(x) => NaiveDateTime::from_sql_owned(ColumnData::DateTime2(x))
            .map_err(to_anyhow)?
            .map(|x| to_raw_value(&x.to_string()))
            .unwrap_or_else(null),
        ColumnData::SmallDateTime(x) => NaiveDateTime::from_sql_owned(ColumnData::SmallDateTime(x))
            .map_err(to_anyhow)?
            .map(|x| to_raw_value(&x.to_string()))
            .unwrap_or_else(null),
        ColumnData::Time(x) => NaiveTime::from_sql_owned(ColumnData::Time(x))
            .map_err(to_anyhow)?
            .map(|x| to_raw_value(&x.to_string()))
            .unwrap_or_else(null),
        ColumnData::Date(x) => NaiveDate::from_sql_owned(ColumnData::Date(x))
            .map_err(to_anyhow)?
            .map(|x| to_raw_value(&x.to_string()))
            .unwrap_or_else(null),
        ColumnData::DateTimeOffset(x) => {
            DateTime::<Utc>::from_sql_owned(ColumnData::DateTimeOffset(x))
                .map_err(to_anyhow)?
                .map(|x| to_raw_value(&x.to_string()))
                .unwrap_or_else(null)
        }
    };
    Ok(val)
}

fn numeric_to_raw_value(numeric: &tiberius::numeric::Numeric) -> Result<Box<RawValue>, Error> {
    let sign = if numeric.value().is_negative() {
        "-"
    } else {
        ""
    };
    let int_part = numeric.int_part().abs();
    let dec_part = numeric.dec_part().abs();

    let str = if dec_part == 0 {
        format!("{}{}", sign, int_part)
    } else {
        format!(
            "{}{}.{:0pad$}",
            sign,
            int_part,
            dec_part,
            pad = numeric.scale() as usize
        )
    };

    Ok(RawValue::from_string(str).map_err(to_anyhow)?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tiberius::numeric::Numeric;

    #[test]
    fn test_sql_to_json_value_numeric_null() {
        let result = sql_to_json_value(ColumnData::Numeric(None)).unwrap();
        assert_eq!(result.get(), "null");
    }

    #[test]
    fn test_sql_to_json_value_numeric_integer() {
        let numeric = Numeric::new_with_scale(12345, 0);
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "12345");
    }

    #[test]
    fn test_sql_to_json_value_numeric_decimal() {
        let numeric = Numeric::new_with_scale(123456, 2); // Represents 1234.56
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "1234.56");
    }

    #[test]
    fn test_sql_to_json_value_numeric_negative() {
        let numeric = Numeric::new_with_scale(-98765, 2); // Represents -987.65
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "-987.65");
    }

    #[test]
    fn test_sql_to_json_value_numeric_negative_integer() {
        let numeric = Numeric::new_with_scale(-98765, 0);
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "-98765");
    }

    #[test]
    fn test_sql_to_json_value_numeric_high_precision() {
        let numeric = Numeric::new_with_scale(123456789012345, 10); // High precision
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "12345.6789012345");
    }

    #[test]
    fn test_sql_to_json_value_numeric_negative_fractional_only() {
        // -0.4: int_part() is 0, so old code lost the negative sign
        let numeric = Numeric::new_with_scale(-4, 1);
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "-0.4");
    }

    #[test]
    fn test_sql_to_json_value_numeric_7_69() {
        let numeric = Numeric::new_with_scale(769, 2);
        let result = sql_to_json_value(ColumnData::Numeric(Some(numeric))).unwrap();
        assert_eq!(result.get(), "7.69");
    }
}
