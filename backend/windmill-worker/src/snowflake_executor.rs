use base64::{engine, Engine as _};
use core::fmt::Write;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use pem;
use serde_json::{json, value::RawValue, Value};
use sha2::{Digest, Sha256};

use windmill_common::jobs::QueuedJob;
use windmill_common::{error::Error, worker::to_raw_value};
use windmill_parser_sql::parse_snowflake_sig;
use windmill_queue::HTTP_CLIENT;

use serde::{Deserialize, Serialize};

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

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SnowflakeResponse {
    data: Vec<Vec<Value>>,
    resultSetMetaData: SnowflakeResultSetMetaData,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SnowflakeResultSetMetaData {
    numRows: i64,
    rowType: Vec<SnowflakeRowType>,
}

#[derive(Deserialize)]
struct SnowflakeRowType {
    name: String,
    r#type: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct SnowflakeError {
    message: String,
}

pub async fn do_snowflake(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let snowflake_args = build_args_values(job, client, db).await?;

    let database = serde_json::from_value::<SnowflakeDatabase>(
        snowflake_args.get("database").unwrap_or(&json!({})).clone(),
    )
    .map_err(|e| Error::ExecutionErr(e.to_string()))?;

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
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
    };

    let private_key = EncodingKey::from_rsa_pem(database.private_key.as_bytes()).map_err(|e| {
        Error::ExecutionErr(format!("Failed to parse private key: {}", e.to_string()))
    })?;

    let token = encode(&Header::new(Algorithm::RS256), &claims, &private_key)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    tracing::debug!("Snowflake token: {}", token);

    let mut bindings = serde_json::Map::new();
    let sig = parse_snowflake_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let mut i = 1;
    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "string".to_string());
        let arg_v = snowflake_args.get(&arg.name).cloned().unwrap_or(json!(""));
        let snowflake_v = convert_typ_val(arg_t, arg_v);

        bindings.insert(i.to_string(), snowflake_v);
        i += 1;
    }

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
    body.insert("statement".to_string(), json!(query));
    body.insert("timeout".to_string(), json!(10)); // in seconds

    if i > 1 {
        body.insert("bindings".to_string(), json!(bindings));
    }

    let response = HTTP_CLIENT
        .post(format!(
            "https://{}.snowflakecomputing.com/api/v2/statements/",
            database.account_identifier.to_uppercase()
        ))
        .bearer_auth(token)
        .header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    match response.error_for_status_ref() {
        Ok(_) => {
            let result = response
                .json::<SnowflakeResponse>()
                .await
                .map_err(|e| Error::ExecutionErr(e.to_string()))?;

            if result.resultSetMetaData.numRows > 10000 {
                return Err(Error::ExecutionErr(
                    "More than 10000 rows were requested, use LIMIT 10000 to limit the number of rows".to_string(),
                ));
            }

            let rows = to_raw_value(
                &result
                    .data
                    .iter()
                    .map(|row| {
                        let mut row_map = serde_json::Map::new();
                        row.iter()
                            .zip(result.resultSetMetaData.rowType.iter())
                            .for_each(|(val, row_type)| {
                                row_map.insert(
                                    row_type.name.clone(),
                                    parse_val(&val, &row_type.r#type),
                                );
                            });
                        Value::from(row_map)
                    })
                    .collect::<Vec<_>>(),
            );

            Ok(rows)
        }
        Err(e) => {
            let resp = response.text().await.unwrap_or("".to_string());
            match serde_json::from_str::<SnowflakeError>(&resp) {
                Ok(sf_err) => Err(Error::ExecutionErr(sf_err.message)),
                Err(_) => Err(Error::ExecutionErr(e.to_string())),
            }
        }
    }
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
    match typ.to_lowercase().as_str() {
        "boolean" => json!(str_value.parse::<bool>().ok().unwrap_or(false)),
        "real" | "time" | "timestamp_ltz" | "timestamp_ntz" => {
            json!(str_value.parse::<f64>().ok().unwrap_or(0.0))
        }
        "fixed" | "date" | "number" => json!(str_value.parse::<i64>().ok().unwrap_or(0)),
        _ => value.clone(),
    }
}
