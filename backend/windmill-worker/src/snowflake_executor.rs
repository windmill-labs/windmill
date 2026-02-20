use base64::{engine, Engine as _};
use chrono::Datelike;
use core::fmt::Write;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, TryStreamExt};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::{Client, Response};
use serde_json::{json, value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use windmill_common::error::to_anyhow;
use windmill_object_store::convert_json_line_stream;
use windmill_common::worker::{Connection, SqlResultCollectionStrategy};

use windmill_common::{error::Error, worker::to_raw_value};
use windmill_parser_sql::{
    parse_db_resource, parse_s3_mode, parse_snowflake_sig, parse_sql_blocks,
};
use windmill_queue::{CanceledBy, MiniPulledJob, HTTP_CLIENT};

use serde::{Deserialize, Serialize};

use crate::common::{build_args_values, get_reserved_variables};
use crate::common::{
    build_http_client, resolve_job_timeout, s3_mode_args_to_worker_data, OccupancyMetrics,
    S3ModeWorkerData,
};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use windmill_common::client::AuthedClient;

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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SnowflakeAsyncResponse {
    statement_handle: String,
}

async fn poll_snowflake_async_query(
    http_client: &Client,
    account_identifier: &str,
    statement_handle: &str,
    token: &str,
    token_is_keypair: bool,
    deadline: std::time::Instant,
) -> windmill_common::error::Result<SnowflakeResponse> {
    let url = format!(
        "https://{}.snowflakecomputing.com/api/v2/statements/{}",
        account_identifier.to_uppercase(),
        statement_handle
    );

    loop {
        if std::time::Instant::now() > deadline {
            return Err(Error::ExecutionErr(
                "Snowflake query timed out while polling for results".to_string(),
            ));
        }

        let mut request = http_client.get(&url).bearer_auth(token);
        if token_is_keypair {
            request = request.header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT");
        }

        let response = request.send().await.map_err(|e| {
            Error::ExecutionErr(format!("Could not poll Snowflake status: {:?}", e))
        })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::ExecutionErr(format!("error reading poll response body: {}", e)))?;

        tracing::debug!(
            "Snowflake poll response status: {}, body: {}",
            status,
            &body[..body.len().min(500)]
        );

        if status == reqwest::StatusCode::ACCEPTED {
            // Still running, wait and poll again
            tracing::info!("Snowflake query still running, polling again in 1s...");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }

        if !status.is_success() {
            return Err(Error::ExecutionErr(format!(
                "Snowflake poll returned error status {}: {}",
                status,
                &body[..body.len().min(500)]
            )));
        }

        // Query completed, parse the response
        let response: SnowflakeResponse = serde_json::from_str(&body).map_err(|e| {
            Error::ExecutionErr(format!(
                "error decoding poll response: {}. Status: {}. Body preview: {}",
                e,
                status,
                &body[..body.len().min(500)]
            ))
        })?;

        return Ok(response);
    }
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

fn do_snowflake_inner<'a>(
    query: &'a str,
    job_args: &HashMap<String, Value>,
    mut body: serde_json::Map<String, Value>,
    account_identifier: &'a str,
    token: &'a str,
    token_is_keypair: bool,
    column_order: Option<&'a mut Option<Vec<String>>>,
    skip_collect: bool,
    first_row_only: bool,
    http_client: &'a Client,
    s3: Option<S3ModeWorkerData>,
    reserved_variables: &HashMap<String, String>,
    deadline: std::time::Instant,
) -> windmill_common::error::Result<BoxFuture<'a, windmill_common::error::Result<Vec<Box<RawValue>>>>>
{
    let sig = parse_snowflake_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let (query, args_to_skip) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args, reserved_variables)?;

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
            // Still need to handle async (202) responses even when not collecting results
            let raw_response = handle_snowflake_result(result).await?;
            let status = raw_response.status();

            if status == reqwest::StatusCode::ACCEPTED {
                let body = raw_response.text().await.map_err(|e| {
                    Error::ExecutionErr(format!("error reading response body: {}", e))
                })?;
                let async_resp: SnowflakeAsyncResponse =
                    serde_json::from_str(&body).map_err(|e| {
                        Error::ExecutionErr(format!(
                            "error decoding async response: {}. Body preview: {}",
                            e,
                            &body[..body.len().min(500)]
                        ))
                    })?;

                tracing::info!(
                    "Snowflake statement running asynchronously, polling for completion (handle: {})",
                    async_resp.statement_handle
                );

                // Poll until complete, but discard the results
                poll_snowflake_async_query(
                    http_client,
                    account_identifier,
                    &async_resp.statement_handle,
                    token,
                    token_is_keypair,
                    deadline,
                )
                .await?;
            }

            Ok(vec![])
        } else {
            // Handle both sync (200) and async (202) responses
            let raw_response = handle_snowflake_result(result).await?;
            let status = raw_response.status();
            let body = raw_response
                .text()
                .await
                .map_err(|e| Error::ExecutionErr(format!("error reading response body: {}", e)))?;

            tracing::debug!(
                "Snowflake response status: {}, body: {}",
                status,
                &body[..body.len().min(1000)]
            );

            let response = if status == reqwest::StatusCode::ACCEPTED {
                // Async execution - need to poll for results
                let async_resp: SnowflakeAsyncResponse =
                    serde_json::from_str(&body).map_err(|e| {
                        Error::ExecutionErr(format!(
                            "error decoding async response: {}. Body preview: {}",
                            e,
                            &body[..body.len().min(500)]
                        ))
                    })?;

                tracing::info!(
                    "Snowflake query running asynchronously, polling for results (handle: {})",
                    async_resp.statement_handle
                );

                poll_snowflake_async_query(
                    http_client,
                    account_identifier,
                    &async_resp.statement_handle,
                    token,
                    token_is_keypair,
                    deadline,
                )
                .await?
            } else {
                // Sync execution - parse directly
                serde_json::from_str::<SnowflakeResponse>(&body).map_err(|e| {
                    Error::ExecutionErr(format!(
                        "error decoding response body: {}. Status: {}. Body preview: {}",
                        e,
                        status,
                        &body[..body.len().min(500)]
                    ))
                })?
            };

            if s3.is_none() && response.resultSetMetaData.numRows > 10000 {
                return Err(Error::ExecutionErr(
                    "More than 10000 rows were requested, use LIMIT 10000 to limit the number of rows or use S3 streaming for larger datasets: https://windmill.dev/docs/core_concepts/sql_to_s3_streaming"
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

            // Clones are because, in s3 mode, reqwest::Body::wrap_stream requires the stream to be
            // 'static even though it doesn't make sense to be in our case since the request is
            // awaited and the stream is fully read before the function returns.
            // Turns out it is a real pain to trick the compiler, even using unsafe
            let cloned_account_identifier: String = account_identifier.to_string();
            let cloned_token = token.to_string();

            let rows_stream = async_stream::stream! {
                for row in response.data {
                    yield Ok::<Vec<Value>, windmill_common::error::Error>(row);
                }

                if response.resultSetMetaData.partitionInfo.len() > 1 {
                    for idx in 1..response.resultSetMetaData.partitionInfo.len() {
                        let url = format!(
                            "https://{}.snowflakecomputing.com/api/v2/statements/{}",
                            cloned_account_identifier.to_uppercase(),
                            response.statementHandle
                        );
                        let mut request = HTTP_CLIENT
                            .get(url)
                            .bearer_auth(cloned_token.as_str())
                            .query(&[("partition", idx.to_string())]);

                        if token_is_keypair {
                            request =
                                request.header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT");
                        }

                        let result = request.send().await;
                        let raw_response = match handle_snowflake_result(result).await {
                            Ok(r) => r,
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        };
                        let status = raw_response.status();
                        let body = match raw_response.text().await {
                            Ok(b) => b,
                            Err(e) => {
                                yield Err(Error::ExecutionErr(format!("error reading partition response: {}", e)));
                                return;
                            }
                        };

                        // Handle async (202) response for partition fetch
                        let partition_data: SnowflakeDataOnlyResponse = if status == reqwest::StatusCode::ACCEPTED {
                            // Poll until complete - partition fetches should be fast, but handle async just in case
                            let mut poll_body = body;
                            loop {
                                if std::time::Instant::now() > deadline {
                                    yield Err(Error::ExecutionErr(
                                        "Snowflake partition fetch timed out while polling".to_string(),
                                    ));
                                    return;
                                }

                                let async_resp: SnowflakeAsyncResponse = match serde_json::from_str(&poll_body) {
                                    Ok(r) => r,
                                    Err(e) => {
                                        yield Err(Error::ExecutionErr(format!(
                                            "error decoding async partition response: {}",
                                            e
                                        )));
                                        return;
                                    }
                                };

                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                                let poll_url = format!(
                                    "https://{}.snowflakecomputing.com/api/v2/statements/{}",
                                    cloned_account_identifier.to_uppercase(),
                                    async_resp.statement_handle
                                );
                                let mut poll_request = HTTP_CLIENT.get(&poll_url).bearer_auth(cloned_token.as_str());
                                if token_is_keypair {
                                    poll_request = poll_request.header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT");
                                }

                                let poll_response = match poll_request.send().await {
                                    Ok(r) => r,
                                    Err(e) => {
                                        yield Err(Error::ExecutionErr(format!("partition poll error: {:?}", e)));
                                        return;
                                    }
                                };

                                let poll_status = poll_response.status();
                                poll_body = match poll_response.text().await {
                                    Ok(b) => b,
                                    Err(e) => {
                                        yield Err(Error::ExecutionErr(format!("error reading partition poll response: {}", e)));
                                        return;
                                    }
                                };

                                if poll_status == reqwest::StatusCode::ACCEPTED {
                                    continue;
                                }

                                if !poll_status.is_success() {
                                    yield Err(Error::ExecutionErr(format!(
                                        "partition poll returned error: {}",
                                        &poll_body[..poll_body.len().min(500)]
                                    )));
                                    return;
                                }

                                match serde_json::from_str(&poll_body) {
                                    Ok(r) => break r,
                                    Err(e) => {
                                        yield Err(Error::ExecutionErr(format!(
                                            "error decoding partition poll response: {}",
                                            e
                                        )));
                                        return;
                                    }
                                }
                            }
                        } else {
                            match serde_json::from_str(&body) {
                                Ok(r) => r,
                                Err(e) => {
                                    yield Err(Error::ExecutionErr(format!(
                                        "error decoding partition response: {}. Body: {}",
                                        e,
                                        &body[..body.len().min(500)]
                                    )));
                                    return;
                                }
                            }
                        };

                        for row in partition_data.data {
                            yield Ok(row);
                        }
                    }
                }
            };

            let rows_stream = rows_stream.map_ok(move |row| {
                let mut row_map = serde_json::Map::new();
                row.iter()
                    .zip(response.resultSetMetaData.rowType.iter())
                    .for_each(|(val, row_type)| {
                        row_map.insert(row_type.name.clone(), parse_val(&val, &row_type.r#type));
                    });
                row_map
            });

            let rows_stream = rows_stream.take(if first_row_only { 1 } else { usize::MAX });

            if let Some(s3) = s3 {
                let rows_stream =
                    rows_stream.map(|r| serde_json::value::to_value(&r?).map_err(to_anyhow));
                let stream = convert_json_line_stream(rows_stream.boxed(), s3.format).await?;
                s3.upload(stream.boxed()).await?;
                Ok(vec![to_raw_value(&s3.to_return_s3_obj())])
            } else {
                let rows = rows_stream
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .map(|x| x.map(|v| to_raw_value(&v)))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            }
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
    parent_runnable_path: Option<String>,
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
    let collection_strategy = if annotations.return_last_result {
        SqlResultCollectionStrategy::LastStatementAllRows
    } else {
        annotations.result_collection
    };

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

    let deadline = std::time::Instant::now() + timeout_duration;

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let result_f = async move {
        let mut results = vec![];
        for (i, q) in queries.iter().enumerate() {
            let result = do_snowflake_inner(
                q,
                &snowflake_args,
                body.clone(),
                &database.account_identifier,
                &token,
                token_is_keypair,
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
                &http_client,
                s3.clone(),
                &reserved_variables,
                deadline,
            )?
            .await?;
            results.push(result);
        }

        collection_strategy.collect(results)
    };

    let r = run_future_with_polling_update_job_poller(
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
