use base64::{engine, Engine as _};
use chrono::Datelike;
use core::fmt::Write;
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Response;
use serde_json::{json, value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use windmill_common::error::to_anyhow;

use windmill_common::jobs::QueuedJob;
use windmill_common::{error::Error, worker::to_raw_value};
use windmill_parser_sql::{parse_db_resource, parse_snowflake_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, HTTP_CLIENT};

use serde::{Deserialize, Serialize};

use crate::common::run_future_with_polling_update_job_poller;
use crate::{common::build_args_values, AuthedClientBackgroundTask};

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
    public_key: String,
    private_key: String,
    username: String,
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
    async fn get_snowflake_response<T: for<'a> Deserialize<'a>>(
        self,
    ) -> windmill_common::error::Result<T>;
}

impl SnowflakeResponseExt for Result<Response, reqwest::Error> {
    async fn get_snowflake_response<T: for<'a> Deserialize<'a>>(
        self,
    ) -> windmill_common::error::Result<T> {
        match self {
            Ok(response) => match response.error_for_status_ref() {
                Ok(_) => response
                    .json::<T>()
                    .await
                    .map_err(|e| Error::ExecutionErr(e.to_string())),
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
}

fn do_snowflake_inner<'a>(
    query: &'a str,
    job_args: &HashMap<String, Value>,
    mut body: serde_json::Map<String, Value>,
    account_identifier: &'a str,
    token: &'a str,
    column_order: Option<&'a mut Option<Vec<String>>>,
) -> windmill_common::error::Result<BoxFuture<'a, windmill_common::error::Result<Box<RawValue>>>> {
    body.insert("statement".to_string(), json!(query));

    let mut bindings = serde_json::Map::new();
    let sig = parse_snowflake_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let mut i = 1;
    for arg in &sig {
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
        let response = HTTP_CLIENT
            .post(format!(
                "https://{}.snowflakecomputing.com/api/v2/statements/",
                account_identifier.to_uppercase()
            ))
            .bearer_auth(token)
            .header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT")
            .json(&body)
            .send()
            .await
            .get_snowflake_response::<SnowflakeResponse>()
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
                let response = HTTP_CLIENT
                    .get(url)
                    .bearer_auth(token)
                    .header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT")
                    .query(&[("partition", idx.to_string())])
                    .send()
                    .await
                    .get_snowflake_response::<SnowflakeDataOnlyResponse>()
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
                            row_map
                                .insert(row_type.name.clone(), parse_val(&val, &row_type.r#type));
                        });
                    row_map
                })
                .collect::<Vec<_>>(),
        );

        Ok(rows)
    };

    Ok(result_f.boxed())
}

pub async fn do_snowflake(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let snowflake_args = build_args_values(job, client, db).await?;

    let inline_db_res_path = parse_db_resource(&query);

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        Some(
            client
                .get_authed()
                .await
                .get_resource_value_interpolated::<serde_json::Value>(
                    &inline_db_res_path,
                    Some(job.id.to_string()),
                )
                .await?,
        )
    } else {
        snowflake_args.get("database").cloned()
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_value::<SnowflakeDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let qualified_username = format!(
        "{}.{}",
        database.account_identifier.split('.').next().unwrap_or(""), // get first part of account identifier
        database.username
    )
    .to_uppercase();

    let public_key = pem::parse(database.public_key.as_bytes()).map_err(|e| {
        Error::ExecutionErr(format!("Failed to parse public key: {}", e.to_string()))
    })?;
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

    let private_key = EncodingKey::from_rsa_pem(database.private_key.as_bytes()).map_err(|e| {
        Error::ExecutionErr(format!("Failed to parse private key: {}", e.to_string()))
    })?;

    let token = encode(&Header::new(Algorithm::RS256), &claims, &private_key)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

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
    body.insert("timeout".to_string(), json!(10)); // in seconds

    let queries = parse_sql_blocks(query);

    let result_f = if queries.len() > 1 {
        let futures = queries
            .iter()
            .map(|x| {
                do_snowflake_inner(
                    x,
                    &snowflake_args,
                    body.clone(),
                    &database.account_identifier,
                    &token,
                    None,
                )
            })
            .collect::<windmill_common::error::Result<Vec<_>>>()?;

        let f = async {
            let mut res: Vec<Box<RawValue>> = vec![];
            for fut in futures {
                let r = fut.await?;
                res.push(r);
            }
            Ok(to_raw_value(&res))
        };

        f.boxed()
    } else {
        do_snowflake_inner(
            query,
            &snowflake_args,
            body.clone(),
            &database.account_identifier,
            &token,
            Some(column_order),
        )?
    };
    let r = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        db,
        mem_peak,
        canceled_by,
        result_f.map_err(to_anyhow),
        worker_name,
        &job.workspace_id,
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
