use std::collections::HashMap;
use std::env;

use duckdb::types::TimeUnit;
use duckdb::{params_from_iter, Row};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use tokio::fs::remove_file;
use tokio::task;
use uuid::Uuid;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::s3_helpers::{
    DuckdbConnectionSettingsQueryV2, DuckdbConnectionSettingsResponse, S3Object,
};
use windmill_common::worker::{to_raw_value, Connection};
use windmill_parser_sql::{parse_duckdb_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::common::{build_args_values, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
#[cfg(feature = "mysql")]
use crate::mysql_executor::MysqlDatabase;
use crate::pg_executor::PgDatabase;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use windmill_common::client::AuthedClient;

fn do_duckdb_inner(
    conn: &duckdb::Connection,
    query: &str,
    job_args: &HashMap<String, duckdb::types::Value>,
    skip_collect: bool,
    column_order: &mut Option<Vec<String>>,
) -> Result<Box<RawValue>> {
    let mut rows_vec = vec![];

    let (query, job_args) = interpolate_named_args(query, &job_args);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let mut rows = stmt
        .query(params_from_iter(job_args))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    if skip_collect {
        return Ok(to_raw_value(&json!([])));
    }

    // Statement needs to be stepped at least once or stmt.column_names() will panic
    let mut column_names = None;
    loop {
        let row = rows.next();
        match row {
            Ok(Some(row)) => {
                // Set column names if not already set
                let stmt = row.as_ref();
                let column_names = match column_names.as_ref() {
                    Some(column_names) => column_names,
                    None => {
                        column_names = Some(stmt.column_names());
                        column_names.as_ref().unwrap()
                    }
                };

                let row = row_to_value(row, &column_names.as_slice())
                    .map_err(|e| Error::ExecutionErr(e.to_string()))?;
                rows_vec.push(row);
            }
            Ok(None) => break,
            Err(e) => {
                return Err(Error::ExecutionErr(e.to_string()));
            }
        }
    }

    if let (Some(column_order), Some(column_names)) = (column_order.as_mut(), column_names) {
        *column_order = column_names.clone();
    }

    return Ok(to_raw_value(&rows_vec));
}

pub async fn do_duckdb(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order_ref: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>> {
    let result_f = async {
        let mut duckdb_connection_settings_cache =
            HashMap::<Option<String>, DuckdbConnectionSettingsResponse>::new();

        let sig = parse_duckdb_sig(query)?.args;
        let mut job_args = build_args_values(job, client, conn).await?;

        let (query, _) = &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args)?;

        let (_query_with_transformed_s3_uris, mut used_storages) =
            transform_s3_uris(query, client, &mut duckdb_connection_settings_cache).await?;
        let query = _query_with_transformed_s3_uris.as_deref().unwrap_or(query);

        let job_args = {
            let mut m: HashMap<String, duckdb::types::Value> = HashMap::new();
            for sig_arg in sig.into_iter() {
                let json_value = job_args
                    .remove(&sig_arg.name)
                    .or_else(|| sig_arg.default)
                    .unwrap_or_else(|| json!(null));

                if matches!(&sig_arg.otyp.as_ref().map(String::as_str), Some("s3object")) {
                    let s3_obj = serde_json::from_value::<S3Object>(json_value).map_err(|e| {
                        Error::ExecutionErr(format!("Failed to deserialize S3Object: {}", e))
                    })?;
                    let duckdb_conn_settings: DuckdbConnectionSettingsResponse =
                        get_duckdb_connection_settings(
                            &s3_obj.storage,
                            &mut duckdb_connection_settings_cache,
                            client,
                        )
                        .await?;

                    let uri =
                        duckdb_conn_settings_to_s3_network_uri(&duckdb_conn_settings, &s3_obj.s3)?;
                    m.insert(sig_arg.name, duckdb::types::Value::Text(uri));
                    used_storages.insert(s3_obj.storage, duckdb_conn_settings);
                } else {
                    let duckdb_value = json_value_to_duckdb_value(
                        &json_value,
                        sig_arg
                            .otyp
                            .clone()
                            .unwrap_or_else(|| "text".to_string())
                            .as_str(),
                    )?;
                    m.insert(sig_arg.name, duckdb_value);
                }
            }
            m
        };

        let query_block_list = parse_sql_blocks(query);

        // Replace windmill resource ATTACH statements with the real instructions
        let query_block_list = {
            let mut v = vec![];
            for query_block in query_block_list.iter() {
                let query_block = remove_comments(&query_block);
                match parse_attach_db_resource(query_block) {
                    Some(parsed) => v.extend(
                        transform_attach_db_resource_query(&parsed, &job.id, client).await?,
                    ),
                    None => match transform_attach_ducklake(&query_block, &job.id, client).await? {
                        Some((ducklake_query, used_storage)) => {
                            v.extend(ducklake_query);
                            used_storages.insert(used_storage.0, used_storage.1);
                        }
                        None => v.push(query_block.to_string()),
                    },
                };
            }
            v
        };

        // duckdb::Connection is not Send so we do it in a single blocking task
        let (result, column_order) = task::spawn_blocking(move || {
            let conn = duckdb::Connection::open_in_memory()
                .map_err(|e| Error::ConnectingToDatabase(e.to_string()))?;

            for (_, DuckdbConnectionSettingsResponse { connection_settings_str, .. }) in
                used_storages.into_iter()
            {
                conn.execute_batch(&connection_settings_str)
                    .map_err(|e| Error::ExecutionErr(e.to_string()))?;
            }

            let mut result: Option<Box<RawValue>> = None;
            let mut column_order = None;

            for (query_block_index, query_block) in query_block_list.iter().enumerate() {
                result = Some(
                    do_duckdb_inner(
                        &conn,
                        query_block.as_str(),
                        &job_args,
                        query_block_index != query_block_list.len() - 1,
                        &mut column_order,
                    )
                    .map_err(|e| Error::ExecutionErr(e.to_string()))?,
                );
            }
            let result = result.unwrap_or_else(|| to_raw_value(&json!([])));
            Ok::<_, Error>((result, column_order))
        })
        .await
        .map_err(to_anyhow)??;

        *column_order_ref = column_order;

        // BigQuery cleanup
        let bq_credentials_path = make_bq_credentials_path(&job.id);
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        if matches!(tokio::fs::try_exists(&bq_credentials_path).await, Ok(true)) {
            remove_file(&bq_credentials_path).await.map_err(to_anyhow)?;
        }
        Ok(result)
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

    Ok(result)
}

fn row_to_value(row: &Row<'_>, column_names: &[String]) -> Result<Box<RawValue>> {
    let mut obj = serde_json::Map::new();
    for (i, key) in column_names.iter().enumerate() {
        let value: duckdb::types::Value =
            row.get(i).map_err(|e| Error::ExecutionErr(e.to_string()))?;
        let json_value = match value {
            duckdb::types::Value::Null => serde_json::Value::Null,
            duckdb::types::Value::Boolean(b) => serde_json::Value::Bool(b),
            duckdb::types::Value::TinyInt(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::SmallInt(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::Int(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::BigInt(i) => serde_json::Value::Number(i.into()),
            duckdb::types::Value::HugeInt(i) => serde_json::Value::String(i.to_string()),
            duckdb::types::Value::UTinyInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::USmallInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::UInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::UBigInt(u) => serde_json::Value::Number(u.into()),
            duckdb::types::Value::Float(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(f as f64)
                    .ok_or_else(|| Error::ExecutionErr("Could not convert to f64".to_string()))?,
            ),
            duckdb::types::Value::Double(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(f)
                    .ok_or_else(|| Error::ExecutionErr("Could not convert to f64".to_string()))?,
            ),
            duckdb::types::Value::Decimal(d) => serde_json::Value::String(d.to_string()),
            duckdb::types::Value::Timestamp(_, ts) => serde_json::Value::String(ts.to_string()),
            duckdb::types::Value::Text(s) => serde_json::Value::String(s),
            duckdb::types::Value::Blob(b) => serde_json::Value::Array(
                b.into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            ),
            duckdb::types::Value::Date32(d) => serde_json::Value::Number(d.into()),
            duckdb::types::Value::Time64(_, t) => serde_json::Value::String(t.to_string()),
            duckdb::types::Value::Interval { months, days, nanos } => serde_json::json!({
                "months": months,
                "days": days,
                "nanos": nanos
            }),
            duckdb::types::Value::List(values) => serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|v| serde_json::Value::String(format!("{:?}", v)))
                    .collect(),
            ),
            duckdb::types::Value::Enum(e) => serde_json::Value::String(e),
            duckdb::types::Value::Struct(fields) => serde_json::Value::Object(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(format!("{:?}", v))))
                    .collect(),
            ),
            duckdb::types::Value::Array(values) => serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|v| serde_json::Value::String(format!("{:?}", v)))
                    .collect(),
            ),
            duckdb::types::Value::Map(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| {
                        (
                            format!("{:?}", k),
                            serde_json::Value::String(format!("{:?}", v)),
                        )
                    })
                    .collect(),
            ),
            duckdb::types::Value::Union(value) => {
                serde_json::Value::String(format!("{:?}", *value))
            }
        };
        obj.insert(key.clone(), json_value);
    }
    serde_json::value::to_raw_value(&obj).map_err(|e| e.into())
}

fn json_value_to_duckdb_value(
    json_value: &serde_json::Value,
    arg_type: &str,
) -> Result<duckdb::types::Value> {
    let arg_type = arg_type.to_lowercase();
    let duckdb_value = match json_value {
        serde_json::Value::Null => duckdb::types::Value::Null,
        serde_json::Value::Bool(b) => duckdb::types::Value::Boolean(*b),

        serde_json::Value::String(s)
            if matches!(
                arg_type.as_str(),
                "timestamp" | "timestamptz" | "timestamp with time zone" | "datetime"
            ) =>
        {
            string_to_duckdb_timestamp(&s)?
        }
        serde_json::Value::String(s) if arg_type.as_str() == "date" => string_to_duckdb_date(&s)?,
        serde_json::Value::String(s) if arg_type.as_str() == "time" => string_to_duckdb_time(&s)?,
        serde_json::Value::String(s) => duckdb::types::Value::Text(s.clone()),

        serde_json::Value::Number(n) if n.is_i64() => {
            let v = n.as_i64().unwrap();
            match arg_type.as_str() {
                "tinyint" | "int1" => duckdb::types::Value::TinyInt(v as i8),
                "smallint" | "int2" | "short" => duckdb::types::Value::SmallInt(v as i16),
                "integer" | "int4" | "int" | "signed" => duckdb::types::Value::Int(v as i32),
                "bigint" | "int8" | "long" => duckdb::types::Value::BigInt(v),
                "hugeint" => duckdb::types::Value::HugeInt(v as i128),
                "float" | "float4" | "real" => duckdb::types::Value::Float(v as f32),
                "double" | "float8" => duckdb::types::Value::Double(v as f64),
                _ => duckdb::types::Value::BigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_u64() => {
            let v = n.as_u64().unwrap();
            match arg_type.as_str() {
                "utinyint" => duckdb::types::Value::UTinyInt(v as u8),
                "usmallint" => duckdb::types::Value::USmallInt(v as u16),
                "uinteger" => duckdb::types::Value::UInt(v as u32),
                "ubigint" | "uhugeint" => duckdb::types::Value::UBigInt(v),
                _ => duckdb::types::Value::UBigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_f64() => {
            let v = n.as_f64().unwrap();
            match arg_type.as_str() {
                "float" | "float4" | "real" => duckdb::types::Value::Float(v as f32),
                "double" | "float8" => duckdb::types::Value::Double(v),
                "decimal" | "numeric" => {
                    duckdb::types::Value::Decimal(Decimal::from_f64(v).ok_or_else(|| {
                        Error::ExecutionErr("Could not convert f64 to Decimal".to_string())
                    })?)
                }
                _ => duckdb::types::Value::Double(v), // default fallback
            }
        }

        serde_json::Value::Array(arr) => {
            duckdb::types::Value::Text(serde_json::to_string(arr).map_err(to_anyhow)?)
        }
        serde_json::Value::Object(map) => {
            duckdb::types::Value::Text(serde_json::to_string(map).map_err(to_anyhow)?)
        }

        value @ _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported type in query: {:?} and signature {arg_type:?}",
                value
            )))
        }
    };
    Ok(duckdb_value)
}

fn string_to_duckdb_timestamp(s: &str) -> Result<duckdb::types::Value> {
    let ts = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e: chrono::ParseError| Error::ExecutionErr(e.to_string()))?;
    Ok(duckdb::types::Value::Timestamp(
        TimeUnit::Millisecond,
        ts.timestamp_millis(),
    ))
}

fn string_to_duckdb_date(s: &str) -> Result<duckdb::types::Value> {
    use chrono::Datelike;
    let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
    Ok(duckdb::types::Value::Date32(date.num_days_from_ce()))
}

fn string_to_duckdb_time(s: &str) -> Result<duckdb::types::Value> {
    use chrono::Timelike;
    let time = chrono::NaiveTime::parse_from_str(s, "%H:%M:%S").unwrap();
    Ok(duckdb::types::Value::Time64(
        TimeUnit::Microsecond,
        time.num_seconds_from_midnight() as i64,
    ))
}

struct ParsedAttachDbResource<'a> {
    resource_path: &'a str,
    name: &'a str,
    db_type: &'a str,
    extra_args: Option<&'a str>,
}
fn parse_attach_db_resource<'a>(query: &'a str) -> Option<ParsedAttachDbResource<'a>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"ATTACH '(\$res:|res://)([^']+)' AS (\S+) \(TYPE (\w+)(.*)\)").unwrap();
    }

    for cap in RE.captures_iter(query) {
        if let (Some(resource_path), Some(name), Some(db_type)) =
            (cap.get(2), cap.get(3), cap.get(4))
        {
            let extra_args = cap.get(5).map(|m| query[m.start()..m.end()].trim());
            return Some(ParsedAttachDbResource {
                resource_path: query[resource_path.start()..resource_path.end()].trim(),
                name: query[name.start()..name.end()].trim(),
                db_type: query[db_type.start()..db_type.end()].trim(),
                extra_args,
            });
        }
    }
    None
}

fn format_attach_db_conn_str(db_resource: Value, db_type: &str, job_id: &Uuid) -> Result<String> {
    let s = match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" => {
            let res: PgDatabase = serde_json::from_value(db_resource)?;
            format!(
                "dbname={} {} host={} {} {}",
                res.dbname,
                res.user.map(|u| format!("user={}", u)).unwrap_or_default(),
                res.host,
                res.password
                    .map(|p| format!("password={}", p))
                    .unwrap_or_default(),
                res.port.map(|p| format!("port={}", p)).unwrap_or_default(),
            )
        }
        "mysql" => {
            #[cfg(not(feature = "mysql"))]
            return Err(Error::ExecutionErr(
                "MySQL feature is not enabled".to_string(),
            ));

            #[cfg(feature = "mysql")]
            {
                let resource: MysqlDatabase = serde_json::from_value(db_resource)?;
                format!(
                    "database={} host={} ssl_mode={} {} {} {}",
                    resource.database,
                    resource.host,
                    resource
                        .ssl
                        .map(|ssl| if ssl { "required" } else { "disabled" })
                        .unwrap_or("preferred"),
                    resource
                        .password
                        .map(|p| format!("password={}", p))
                        .unwrap_or_default(),
                    resource
                        .port
                        .map(|p| format!("port={}", p))
                        .unwrap_or_default(),
                    resource
                        .user
                        .map(|u| format!("user={}", u))
                        .unwrap_or_default(),
                )
            }
        }
        "bigquery" => {
            let resource: Value = serde_json::from_value(db_resource)?;
            // duckdb's bigquery extension requires a json file as credentials
            // TODO : make this cleaner
            let bq_credentials_path = make_bq_credentials_path(job_id);
            env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &bq_credentials_path);
            std::fs::write(&bq_credentials_path, resource.to_string()).map_err(|e| {
                Error::ExecutionErr(format!(
                    "Failed to write BigQuery credentials to {}: {}",
                    &bq_credentials_path, e
                ))
            })?;
            let project_id: String = serde_json::from_value(
                resource
                    .get("project_id")
                    .ok_or_else(|| {
                        Error::ExecutionErr("BigQuery resource must contain project_id".to_string())
                    })?
                    .to_owned(),
            )
            .map_err(|_e| Error::ExecutionErr("failed project_id deserialize".to_string()))?;
            format!("project={}", project_id,)
        }
        _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported db type in DuckDB ATTACH: {db_type}",
            )))
        }
    };
    Ok(s)
}

async fn fetch_attach_db_conn_str(
    parsed: &ParsedAttachDbResource<'_>,
    job_id: &Uuid,
    client: &AuthedClient,
) -> Result<String> {
    let db_resource: Value = client
        .get_resource_value_interpolated(parsed.resource_path, Some(job_id.to_string()))
        .await?;
    format_attach_db_conn_str(db_resource, parsed.db_type, job_id)
}

fn get_attach_db_install_str(db_type: &str) -> Result<&str> {
    match db_type.to_lowercase().as_str() {
        "postgres" => Ok("INSTALL postgres;"),
        "mysql" => {
            #[cfg(not(feature = "mysql"))]
            return Err(Error::ExecutionErr(
                "MySQL feature is not enabled".to_string(),
            ));
            #[cfg(feature = "mysql")]
            Ok("INSTALL mysql;")
        }
        "bigquery" => Ok("INSTALL bigquery FROM community;"),
        _ => Err(Error::ExecutionErr(format!(
            "Unsupported db type in DuckDB ATTACH: {}",
            db_type
        ))),
    }
}

async fn transform_attach_db_resource_query(
    parsed: &ParsedAttachDbResource<'_>,
    job_id: &Uuid,
    client: &AuthedClient,
) -> Result<Vec<String>> {
    let attach_str = format!(
        "ATTACH '{}' as {} (TYPE {}{});",
        fetch_attach_db_conn_str(&parsed, &job_id, &client).await?,
        parsed.name,
        parsed.db_type,
        parsed.extra_args.unwrap_or("")
    )
    .to_string();

    Ok(vec![
        get_attach_db_install_str(parsed.db_type)?.to_string(),
        format!("LOAD {};", parsed.db_type),
        attach_str,
    ])
}

async fn transform_attach_ducklake(
    query: &str,
    job_id: &Uuid,
    client: &AuthedClient,
) -> Result<
    Option<(
        Vec<String>,
        (Option<String>, DuckdbConnectionSettingsResponse), // (storage_name, storage_settings)
    )>,
> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"ATTACH 'ducklake://([^':]+)' AS ([^ ;]+)").unwrap();
    }
    let Some(cap) = RE.captures(query) else {
        return Ok(None);
    };
    let ducklake_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
    let alias_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

    let ducklake_config = client.get_ducklake(ducklake_name).await?;
    let db_type = ducklake_config.catalog.resource_type.as_ref();

    let db_conn_str =
        format_attach_db_conn_str(ducklake_config.catalog_resource, db_type, &job_id)?;

    let used_storage = ducklake_config.storage_settings;
    let s3_conn_str =
        duckdb_conn_settings_to_s3_network_uri(&used_storage, &ducklake_config.storage.path)?;

    let attach_str = format!(
        "ATTACH 'ducklake:{db_type}:{db_conn_str}' AS {alias_name} (DATA_PATH '{s3_conn_str}');",
    );

    let install_db_ext_str = get_attach_db_install_str(db_type)?;
    Ok(Some((
        vec![
            "INSTALL ducklake;".to_string(),
            install_db_ext_str.to_string(),
            attach_str,
        ],
        (ducklake_config.storage.storage, used_storage),
    )))
}

// Returns the transformed query and the set of storages used
async fn transform_s3_uris(
    query: &str,
    client: &AuthedClient,
    duckdb_connection_settings_cache: &mut DuckDbConnectionSettingsCache,
) -> Result<(
    Option<String>,
    HashMap<Option<String>, DuckdbConnectionSettingsResponse>,
)> {
    let mut transformed_query = None;
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"'s3://([^'/]*)/([^']*)'").unwrap();
    }
    let mut used_storages = HashMap::new();
    for cap in RE.captures_iter(query) {
        if let (storage, Some(s3_path)) = (cap.get(1), cap.get(2)) {
            let s3_path = s3_path.as_str();
            let storage = match storage.map(|m| m.as_str()) {
                Some("") | None => None,
                Some(s) => Some(s.to_string()),
            };
            let original_str_lit =
                format!("'s3://{}/{}'", storage.as_deref().unwrap_or(""), s3_path);
            let duckdb_conn_settings =
                get_duckdb_connection_settings(&storage, duckdb_connection_settings_cache, client)
                    .await?;
            let url = duckdb_conn_settings_to_s3_network_uri(&duckdb_conn_settings, s3_path)?;
            let url = format!("'{url}'");
            transformed_query = Some(
                transformed_query
                    .unwrap_or(query.to_string())
                    .replace(&original_str_lit, &url),
            );
            used_storages.insert(storage, duckdb_conn_settings);
        }
    }
    Ok((transformed_query, used_storages))
}

pub fn duckdb_conn_settings_to_s3_network_uri(
    s: &DuckdbConnectionSettingsResponse,
    s3_path: &str,
) -> Result<String> {
    match &s {
        DuckdbConnectionSettingsResponse { s3_bucket: Some(bucket), .. } => {
            Ok(format!("s3://{bucket}/{s3_path}"))
        }
        DuckdbConnectionSettingsResponse { azure_container_path: Some(base), .. } => {
            Ok(format!("{base}/{s3_path})"))
        }
        _ => {
            Err(Error::ExecutionErr(
                "DuckDB connection settings response must have either s3_bucket or azure_container_path".to_string(),
            ))
        }
    }
}

// BigQuery extension requires a json file as credentials
// The file path is set as an env var by do_duckdb
// It is created by transform_attach_db_resource_query (when bigquery is detected)
// and deleted by do_duckdb after the query is executed
fn make_bq_credentials_path(job_id: &Uuid) -> String {
    format!("/tmp/service-account-credentials-{}.json", job_id)
}

// duckdb-rs does not support named parameters,
// and it raises an error when passing unused arguments. We cannot prepare batch statements
// but only single SQL statements so it doesn't work when all arguments are not used by
// every single statement.
fn interpolate_named_args<'a>(
    query: &str,
    args: &'a HashMap<String, duckdb::types::Value>,
) -> (String, Vec<&'a duckdb::types::Value>) {
    let mut query = query.to_string();

    let mut values = vec![];
    for (arg_name, arg_value) in args {
        let pat = format!("${}", arg_name);
        if !query.contains(&pat) {
            continue;
        }
        values.push(arg_value);
        query = query.replace(&pat, &format!("${}", values.len()));
    }
    (query, values)
}

// input should contain a single statement. remove all comments before and after it
fn remove_comments(stmt: &str) -> &str {
    let mut in_stmt = false;
    let mut in_comment = false;
    let mut start = None;
    let mut end = stmt.len();

    let mut c = ' ';
    for (next_i, next_char) in stmt.char_indices() {
        if next_i > 0 {
            let i = next_i - 1;
            if !in_comment && in_stmt && c == ';' {
                end = i + 1;
                break;
            } else if in_comment && c == '\n' {
                in_comment = false;
            } else if c == '-' && next_char == '-' {
                in_comment = true;
            } else if !in_comment && !c.is_whitespace() && start == None {
                start = Some(i);
                in_stmt = true;
            }
        }
        c = next_char;
    }

    return &stmt[start.unwrap_or(0)..end];
}

async fn get_duckdb_connection_settings(
    storage: &Option<String>,
    cache: &mut DuckDbConnectionSettingsCache,
    client: &AuthedClient,
) -> Result<DuckdbConnectionSettingsResponse> {
    if let Some(settings) = cache.get(storage) {
        return Ok(settings.clone());
    } else {
        let settings = client
            .get_duckdb_connection_settings(&DuckdbConnectionSettingsQueryV2 {
                s3_resource_path: None,
                storage: storage.clone(),
            })
            .await?;
        cache.insert(storage.clone(), settings.clone());
        return Ok(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_remove_comments_single_line() {
        let sql = "-- This is a comment\nSELECT * FROM table;";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_multi_line() {
        let sql = "-- This is a comment\nSELECT * FROM table;\n-- Another comment";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_inline_comment() {
        let sql = "   SELECT * FROM table;    -- This is an inline comment  ";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_no_comments() {
        let sql = "SELECT * FROM table;";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_empty_string() {
        let sql = "";
        assert_eq!(remove_comments(sql), "");
    }
    #[test]
    fn test_remove_comments_with_whitespace() {
        let sql = "   -- Comment\n  -- Comment2\n  -- Comment3\n   SELECT\n\n * FROM\n table\n;\n\n -- end comment   ";
        assert_eq!(remove_comments(sql), "SELECT\n\n * FROM\n table\n;");
    }
}

type DuckDbConnectionSettingsCache = HashMap<Option<String>, DuckdbConnectionSettingsResponse>;
