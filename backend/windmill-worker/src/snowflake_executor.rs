use base64::{engine, Engine as _};
use chrono::Datelike;
use core::fmt::Write;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, TryFutureExt};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::{Client, Response};
use serde_json::{json, value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use windmill_common::error::to_anyhow;
use windmill_common::more_serde::json_stream_values;
use windmill_common::s3_helpers::convert_json_line_stream;
use windmill_common::worker::Connection;

use windmill_common::{error::Error, worker::to_raw_value};
use windmill_parser_sql::{
    parse_db_resource, parse_s3_mode, parse_snowflake_sig, parse_sql_blocks,
};
use windmill_queue::{CanceledBy, MiniPulledJob, HTTP_CLIENT};

use serde::{Deserialize, Serialize};

use crate::common::{
    build_http_client, resolve_job_timeout, s3_mode_args_to_worker_data, OccupancyMetrics,
    S3ModeWorkerData,
};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::{common::build_args_values, AuthedClient};

#[derive(Serialize)]
struct Claims {
    iss: String,
    sub: String,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct SnowflakeDatabase {
    account_identifier: String,
    public_key: Option<String>,
    private_key: Option<String>,
    username: Option<String>,
    database: Option<String>,
    schema: Option<String>,
    warehouse: Option<String>,
    role: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SnowflakeResponse {
    data: Vec<Vec<Value>>,
    resultSetMetaData: SnowflakeResultSetMetaData,
    statementHandle: String,
}

#[derive(Deserialize, Debug)]
struct SnowflakeDataOnlyResponse {
    data: Vec<Vec<Value>>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SnowflakeResultSetMetaData {
    numRows: i64,
    rowType: Vec<SnowflakeRowType>,
    partitionInfo: Vec<Value>,
}

#[derive(Deserialize, Debug)]
struct SnowflakeRowType {
    name: String,
    r#type: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct SnowflakeError {
    message: String,
}

trait SnowflakeResponseExt {
    async fn parse_snowflake_response<T: for<'a> Deserialize<'a>>(
        self,
    ) -> windmill_common::error::Result<T>;
}

async fn handle_snowflake_result(
    result: Result<Response, reqwest::Error>,
) -> windmill_common::error::Result<Response> {
    match result {
        Ok(response) => match response.error_for_status_ref() {
            Ok(_) => Ok(response),
            Err(e) => {
                let resp = response.text().await.unwrap_or("".to_string());
                match serde_json::from_str::<SnowflakeError>(&resp) {
                    Ok(sf_err) => return Err(Error::ExecutionErr(sf_err.message)),
                    Err(_) => return Err(Error::ExecutionErr(e.to_string())),
                }
            }
        },
        Err(e) => Err(Error::ExecutionErr(format!(
            "Could not send request: {:?}",
            e
        ))),
    }
}

impl SnowflakeResponseExt for Result<Response, reqwest::Error> {
    async fn parse_snowflake_response<T: for<'a> Deserialize<'a>>(
        self,
    ) -> windmill_common::error::Result<T> {
        let response = handle_snowflake_result(self).await?;
        response
            .json::<T>()
            .await
            .map_err(|e| Error::ExecutionErr(e.to_string()))
    }
}

fn do_snowflake_inner<'a>(
    query: &'a str,
    job_args: &HashMap<String, Value>,
    mut body: serde_json::Map<String, Value>,
    account_identifier: &'a str,
    token: &'a str,
    token_is_keypair: bool,
    column_order: Option<&'a mut Option<Vec<String>>>,
    skip_collect: bool,
    http_client: &'a Client,
    s3: Option<S3ModeWorkerData>,
) -> windmill_common::error::Result<BoxFuture<'a, windmill_common::error::Result<Box<RawValue>>>> {
    let sig = parse_snowflake_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let (query, args_to_skip) = &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args)?;

    body.insert("statement".to_string(), json!(query));

    let mut bindings = serde_json::Map::new();

    let mut i = 1;
    for arg in &sig {
        if args_to_skip.contains(&arg.name) {
            continue;
        }
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "string".to_string());
        let arg_v = job_args.get(&arg.name).cloned().unwrap_or(json!(""));
        let snowflake_v = convert_typ_val(arg_t, arg_v);

        bindings.insert(i.to_string(), snowflake_v);
        i += 1;
    }

    if i > 1 {
        body.insert("bindings".to_string(), json!(bindings));
    }

    let result_f = async move {
        let mut request = http_client
            .post(format!(
                "https://{}.snowflakecomputing.com/api/v2/statements/",
                account_identifier.to_uppercase()
            ))
            .bearer_auth(token)
            .json(&body);

        if token_is_keypair {
            request = request.header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT");
        }

        let result = request.send().await;

        if skip_collect {
            handle_snowflake_result(result).await?;
            Ok(to_raw_value(&Value::Array(vec![])))
        } else if let Some(ref s3) = s3 {
            // do not do parse_snowflake_response as it will call .json() and
            // load the entire response into memory
            let result = result.map_err(|e| {
                Error::ExecutionErr(format!("Could not send request to Snowflake: {:?}", e))
            })?;

            let rows_stream = json_stream_values(result.bytes_stream(), |sender| {
                RootMpscDeserializer { sender }
            })
            .await?
            .boxed()
            .map(|chunk| Ok::<_, Infallible>(chunk));

            let stream = convert_json_line_stream(rows_stream, s3.format).await?;
            s3.upload(stream.boxed()).await?;

            Ok(serde_json::value::to_raw_value(&s3.object_key)?)
        } else {
            let response = result
                .parse_snowflake_response::<SnowflakeResponse>()
                .await?;

            if response.resultSetMetaData.numRows > 10000 {
                return Err(Error::ExecutionErr(
                    "More than 10000 rows were requested, use LIMIT 10000 to limit the number of rows"
                        .to_string(),
                ));
            }
            if let Some(column_order) = column_order {
                *column_order = Some(
                    response
                        .resultSetMetaData
                        .rowType
                        .iter()
                        .map(|x| x.name.clone())
                        .collect::<Vec<String>>(),
                );
            }

            let mut rows = response.data;

            if response.resultSetMetaData.partitionInfo.len() > 1 {
                for idx in 1..response.resultSetMetaData.partitionInfo.len() {
                    let url = format!(
                        "https://{}.snowflakecomputing.com/api/v2/statements/{}",
                        account_identifier.to_uppercase(),
                        response.statementHandle
                    );
                    let mut request = HTTP_CLIENT
                        .get(url)
                        .bearer_auth(token)
                        .query(&[("partition", idx.to_string())]);

                    if token_is_keypair {
                        request =
                            request.header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT");
                    }

                    let response = request
                        .send()
                        .await
                        .parse_snowflake_response::<SnowflakeDataOnlyResponse>()
                        .await?;

                    rows.extend(response.data);
                }
            }

            let rows = to_raw_value(
                &rows
                    .iter()
                    .map(|row| {
                        let mut row_map = serde_json::Map::new();
                        row.iter()
                            .zip(response.resultSetMetaData.rowType.iter())
                            .for_each(|(val, row_type)| {
                                row_map.insert(
                                    row_type.name.clone(),
                                    parse_val(&val, &row_type.r#type),
                                );
                            });
                        row_map
                    })
                    .collect::<Vec<_>>(),
            );

            Ok(rows)
        }
    };

    Ok(result_f.boxed())
}

pub async fn do_snowflake(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let snowflake_args = build_args_values(job, client, conn).await?;

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
        snowflake_args.get("database").cloned()
    };

    let database = if let Some(ref db) = db_arg {
        serde_json::from_value::<SnowflakeDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let annotations = windmill_common::worker::SqlAnnotations::parse(query);

    // Check if the token is present in db_arg and use it if available
    let (token, token_is_keypair) = if let Some(token) = db_arg
        .as_ref()
        .and_then(|db| db.get("token"))
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
    {
        tracing::debug!("Using oauth token from db_arg");
        (token.to_string(), false)
    } else {
        tracing::debug!("Generating new oauth token");

        let qualified_username = format!(
            "{}.{}",
            database.account_identifier.split('.').next().unwrap_or(""),
            database.username.as_deref().unwrap_or("")
        )
        .to_uppercase();

        let public_key = match database.public_key.as_deref() {
            Some(key) => pem::parse(key.as_bytes()).map_err(|e| {
                Error::ExecutionErr(format!("Failed to parse public key: {}", e.to_string()))
            })?,
            None => return Err(Error::ExecutionErr("Public key is missing".to_string())),
        };
        let mut public_key_hash = Sha256::new();
        public_key_hash.update(public_key.contents());

        let public_key_fp = engine::general_purpose::STANDARD.encode(public_key_hash.finalize());

        let iss = format!("{}.SHA256:{}", qualified_username, public_key_fp);

        let claims = Claims {
            iss: iss,
            sub: qualified_username,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::try_hours(1).unwrap()).timestamp(),
        };

        let private_key = match database.private_key.as_deref() {
            Some(key) => EncodingKey::from_rsa_pem(key.as_bytes()).map_err(|e| {
                Error::ExecutionErr(format!("Failed to parse private key: {}", e.to_string()))
            })?,
            None => return Err(Error::ExecutionErr("Private key is missing".to_string())),
        };

        (
            encode(&Header::new(Algorithm::RS256), &claims, &private_key)
                .map_err(|e| Error::ExecutionErr(e.to_string()))?,
            true,
        )
    };

    tracing::debug!("Snowflake token: {}", token);

    let mut body = serde_json::Map::new();
    if database.schema.is_some() {
        body.insert(
            "schema".to_string(),
            json!(database.schema.unwrap().to_uppercase()),
        );
    }
    if database.warehouse.is_some() {
        body.insert(
            "warehouse".to_string(),
            json!(database.warehouse.unwrap().to_uppercase()),
        );
    }
    if database.role.is_some() {
        body.insert(
            "role".to_string(),
            json!(database.role.unwrap().to_uppercase()),
        );
    }
    if database.database.is_some() {
        body.insert(
            "database".to_string(),
            json!(database.database.unwrap().to_uppercase()),
        );
    }
    let timeout = resolve_job_timeout(&conn, &job.workspace_id, job.id, job.timeout)
        .await
        .0
        .as_secs();
    body.insert("timeout".to_string(), json!(timeout));

    let queries = parse_sql_blocks(query);

    let (timeout_duration, _, _) =
        resolve_job_timeout(&conn, &job.workspace_id, job.id, job.timeout).await;

    let http_client = build_http_client(timeout_duration)?;

    let result_f = if queries.len() > 1 {
        let futures = queries
            .iter()
            .enumerate()
            .map(|(i, x)| {
                do_snowflake_inner(
                    x,
                    &snowflake_args,
                    body.clone(),
                    &database.account_identifier,
                    &token,
                    token_is_keypair,
                    None,
                    annotations.return_last_result && i < queries.len() - 1,
                    &http_client,
                    s3.clone(),
                )
            })
            .collect::<windmill_common::error::Result<Vec<_>>>()?;

        let f = async {
            let mut res: Vec<Box<RawValue>> = vec![];
            for fut in futures {
                let r = fut.await?;
                res.push(r);
            }
            if annotations.return_last_result && res.len() > 0 {
                Ok(res.pop().unwrap())
            } else {
                Ok(to_raw_value(&res))
            }
        };

        f.boxed()
    } else {
        do_snowflake_inner(
            query,
            &snowflake_args,
            body.clone(),
            &database.account_identifier,
            &token,
            token_is_keypair,
            Some(column_order),
            false,
            &http_client,
            s3.clone(),
        )?
    };
    let r = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        result_f.map_err(to_anyhow),
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;
    *mem_peak = (r.get().len() / 1000) as i32;
    Ok(r)
}

fn convert_typ_val(arg_t: String, arg_v: Value) -> Value {
    match arg_t.as_str() {
        "date" => {
            let arr = arg_v
                .as_str()
                .unwrap_or("")
                .split("T")
                .collect::<Vec<&str>>();
            json!({
                "type": "TEXT",
                "value": match arr.as_slice() {
                    [date, _] => {
                        json!(date)
                    }
                    _ => {
                        arg_v
                    }
                }
            })
        }
        "time" => {
            let arr = arg_v
                .as_str()
                .unwrap_or("")
                .split("T")
                .collect::<Vec<&str>>();
            json!({
                "type": "TEXT",
                "value": match arr.as_slice() {
                    [_, time] => {
                        json!(time)
                    }
                    _ => {
                        arg_v
                    }
                }
            })
        }
        "binary" => {
            // convert base64 to hex as expected by snowflake
            let bytes = engine::general_purpose::STANDARD
                .decode(arg_v.as_str().unwrap_or(""))
                .unwrap_or(vec![]);
            let mut hex = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                write!(hex, "{:02X}", byte).unwrap_or(());
            }
            json!({
                "type": "TEXT",
                "value": hex
            })
        }
        _ => {
            let mut v = arg_v;

            if !v.is_string() {
                // if not string, convert to string for api request
                v = json!(v.to_string());
            }

            json!({
                "type": "TEXT", // snowflake infer type from schema
                "value": v
            })
        }
    }
}

fn parse_val(value: &Value, typ: &str) -> Value {
    let str_value = value.as_str().unwrap_or("").to_string();
    let val = match typ.to_lowercase().as_str() {
        "boolean" => str_value.parse::<bool>().ok().map(|v| json!(v)),
        "real" => str_value.parse::<f64>().ok().map(|v| json!(v)),
        "timestamp_ltz" | "timestamp_ntz" => str_value
            .parse::<f64>()
            .ok()
            .map(|v| {
                chrono::DateTime::from_timestamp(v.round() as i64, 0)
                    .map(|d| json!(d.format("%Y-%m-%d %H:%M:%S").to_string()))
            })
            .flatten(),
        "time" => str_value
            .parse::<f64>()
            .ok()
            .map(|v| {
                chrono::NaiveTime::from_num_seconds_from_midnight_opt(v.round() as u32, 0)
                    .map(|d| json!(d.format("%H:%M:%S").to_string()))
            })
            .flatten(),
        "date" => str_value
            .parse::<i32>()
            .ok()
            .map(|v| {
                chrono::NaiveDate::from_num_days_from_ce_opt(
                    v + chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                        .unwrap()
                        .num_days_from_ce(),
                )
                .map(|d| json!(d.format("%Y-%m-%d").to_string()))
            })
            .flatten(),
        "fixed" | "number" => str_value
            .parse::<i64>()
            .ok()
            .map(|v| json!(v))
            .or(str_value.parse::<f64>().ok().map(|v| json!(v))),
        _ => Some(value.clone()),
    };

    if let Some(val) = val {
        val
    } else {
        json!(format!(
            "ERR: Could not parse {} argument with value {}",
            typ, str_value
        ))
    }
}

// This deserializer takes a snowflake response as a stream and sends each row to an mpsc
// channel as a json record without storing the full input json in memory.
struct RootMpscDeserializer {
    sender: tokio::sync::mpsc::Sender<serde_json::Value>,
}

impl<'de> serde::de::DeserializeSeed<'de> for RootMpscDeserializer {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RootVisitor<'a> {
            sender: &'a tokio::sync::mpsc::Sender<serde_json::Value>,
            col_defs: Vec<String>,
        }

        impl<'de, 'a> serde::de::Visitor<'de> for RootVisitor<'a> {
            type Value = ();
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("data field from snowflake response")
            }
            fn visit_map<A>(mut self, mut map: A) -> Result<(), A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                while let Some(key) = map.next_key::<String>()? {
                    if key == "resultSetMetaData" {
                        let result_set_metadata: SnowflakeResultSetMetaData = map.next_value()?;
                        self.col_defs = result_set_metadata
                            .rowType
                            .iter()
                            .map(|x| x.name.clone())
                            .collect::<Vec<String>>();
                    } else if key == "data" {
                        let () = map.next_value_seed(RowsMpscDeserializer {
                            sender: self.sender,
                            col_defs: &self.col_defs,
                        })?;
                    } else {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
                Ok(())
            }
        }

        deserializer.deserialize_map(RootVisitor { sender: &self.sender, col_defs: vec![] })
    }
}

struct RowsMpscDeserializer<'a> {
    sender: &'a tokio::sync::mpsc::Sender<serde_json::Value>,
    col_defs: &'a Vec<String>,
}

impl<'de, 'a> serde::de::DeserializeSeed<'de> for RowsMpscDeserializer<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RowsVisitor<'a> {
            sender: &'a tokio::sync::mpsc::Sender<serde_json::Value>,
            col_defs: &'a Vec<String>,
        }

        impl<'de, 'a> serde::de::Visitor<'de> for RowsVisitor<'a> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of rows")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                while let Some(elem) = seq.next_element::<Vec<Value>>()? {
                    let mut row = BTreeMap::<&str, Value>::new();
                    for (i, field) in elem.iter().enumerate() {
                        let col_name = self.col_defs.get(i).map(|s| s.as_str()).unwrap_or("");
                        row.insert(col_name, field.clone());
                    }
                    let row = serde_json::to_value(row).map_err(|err| {
                        serde::de::Error::custom(format!("Map parse failed: {err}"))
                    })?;
                    self.sender.blocking_send(row).map_err(|err| {
                        serde::de::Error::custom(format!("Queue send failed: {err}"))
                    })?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(RowsVisitor { sender: &self.sender, col_defs: &self.col_defs })
    }
}
