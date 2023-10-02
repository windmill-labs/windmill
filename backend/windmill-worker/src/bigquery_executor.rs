use serde_json::{json, Value};
use windmill_common::error::Error;
use windmill_common::jobs::QueuedJob;
use windmill_parser_sql::parse_bigquery_sig;
use windmill_queue::HTTP_CLIENT;

use serde::Deserialize;

use crate::{common::transform_json_value, AuthedClient};

use gcp_auth::{AuthenticationManager, CustomServiceAccount};

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct BigqueryResponse {
    rows: Option<Vec<BigqueryResponseRow>>,
    totalRows: Option<Value>,
    schema: Option<BigqueryResponseSchema>,
    jobComplete: bool,
}

#[derive(Deserialize)]
struct BigqueryResponseRow {
    f: Vec<BigqueryResponseValue>,
}

#[derive(Deserialize)]
struct BigqueryResponseValue {
    v: Value,
}

#[derive(Deserialize)]
struct BigqueryResponseSchema {
    fields: Vec<BigqueryResponseSchemaField>,
}

#[derive(Deserialize)]
struct BigqueryResponseSchemaField {
    name: String,
    r#type: String,
    fields: Option<Vec<BigqueryResponseSchemaField>>,
}

#[derive(Deserialize)]
struct BigqueryErrorResponse {
    error: BigqueryError,
}

#[derive(Deserialize)]
struct BigqueryError {
    message: String,
}

pub async fn do_bigquery(
    job: QueuedJob,
    client: &AuthedClient,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<serde_json::Value> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone(), &job, db).await?)
    } else {
        None
    };

    let bigquery_args: Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let database = bigquery_args
        .get("database")
        .unwrap_or(&json!({}))
        .to_string();

    if database == "{}" {
        return Err(Error::ExecutionErr("Invalid database".to_string()));
    }

    let service_account = CustomServiceAccount::from_json(&database)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let authentication_manager = AuthenticationManager::from(service_account);
    let scopes = &["https://www.googleapis.com/auth/bigquery"];
    let token = authentication_manager
        .get_token(scopes)
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());

    let mut statement_values: Vec<Value> = vec![];

    let sig = parse_bigquery_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "string".to_string());
        let arg_n = arg.clone().name;
        let arg_v = args.get(&arg.name).cloned().unwrap_or(json!(""));
        let bigquery_v = if arg_t.ends_with("[]") {
            let base_type = arg_t.strip_suffix("[]").unwrap_or(&arg_t);
            json!({
                "name": arg.name,
                "parameterType": {
                    "type": "ARRAY",
                    "arrayType": {
                        "type": base_type.to_uppercase()
                    }
                },
                "parameterValue": {
                    "arrayValues": args
                        .get(&arg.name)
                        .unwrap_or(&json!([]))
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|x| {
                            convert_val(base_type.to_string(), x.clone())
                        })
                        .collect::<Vec<Value>>()
                }
            })
        } else {
            json!({
                "name": arg_n,
                "parameterType": {
                    "type": arg_t.to_uppercase()
                },
                "parameterValue": {
                    "value": convert_val(arg_t, arg_v),
                }
            })
        };

        statement_values.push(bigquery_v);
    }

    let response = HTTP_CLIENT
        .post(
            "https://bigquery.googleapis.com/bigquery/v2/projects/".to_string()
                + authentication_manager
                    .project_id()
                    .await
                    .map_err(|e| Error::ExecutionErr(e.to_string()))?
                    .as_str()
                + "/queries",
        )
        .bearer_auth(token.as_str())
        .json(&json!({
            "query": query,
            "useLegacySql": false,
            "maxResults": 10000,
            "timeoutMs": 10000, // default
            "queryParameters": statement_values,
        }))
        .send()
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    match response.error_for_status_ref() {
        Ok(_) => {
            let result = response
                .json::<BigqueryResponse>()
                .await
                .map_err(|e| Error::ExecutionErr(e.to_string()))?;

            if !result.jobComplete {
                return Err(Error::ExecutionErr(
                    "BigQuery API did not answer query in time".to_string(),
                ));
            }

            if result.rows.is_none() || result.rows.as_ref().unwrap().len() == 0 {
                return Ok(Value::Array(vec![]));
            }

            if result.schema.is_none() {
                return Err(Error::ExecutionErr(
                    "Incomplete response from BigQuery API".to_string(),
                ));
            }

            if result
                .totalRows
                .unwrap_or(json!(""))
                .as_str()
                .unwrap_or("")
                .parse::<i64>()
                .unwrap_or(0)
                > 10000
            {
                return Err(Error::ExecutionErr(
                    "More than 10000 rows were requested, use LIMIT 10000 to limit the number of rows".to_string(),
                ));
            }

            let rows = result
                .rows
                .unwrap()
                .iter()
                .map(|row| {
                    let mut row_map = serde_json::Map::new();
                    row.f
                        .iter()
                        .zip(result.schema.as_ref().unwrap().fields.iter())
                        .for_each(|(field, schema)| {
                            row_map.insert(
                                schema.name.clone(),
                                parse_val(&field.v, &schema.r#type, &schema),
                            );
                        });
                    Value::from(row_map)
                })
                .collect();

            return Ok(rows);
        }
        Err(e) => match response.json::<BigqueryErrorResponse>().await {
            Ok(bq_err) => return Err(Error::ExecutionErr(bq_err.error.message)),
            Err(_) => return Err(Error::ExecutionErr(e.to_string())),
        },
    }
}

fn convert_val(arg_t: String, arg_v: Value) -> Value {
    match arg_t.as_str() {
        "timestamp" | "datetime" | "date" | "time" => {
            let mut v: String = arg_v.as_str().unwrap_or("").to_owned();

            match arg_t.as_str() {
                "timestamp" | "datetime" => {
                    v = v + ":00";
                }
                "date" => {
                    let arr = v.split("T").collect::<Vec<&str>>();
                    match arr.as_slice() {
                        [date, _] => {
                            v = date.to_string();
                        }
                        _ => {}
                    }
                }
                "time" => {
                    let arr = v.split("T").collect::<Vec<&str>>();
                    match arr.as_slice() {
                        [_, time] => {
                            v = time.to_string() + ":00";
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            json!({ "value": json!(v) })
        }
        _ => {
            let mut v = arg_v;

            if !v.is_string() {
                // if not string, convert to string for api request
                v = json!(v.to_string());
            }

            json!({
                    "value": v,
                }
            )
        }
    }
}

fn parse_val(value: &Value, typ: &str, schema: &BigqueryResponseSchemaField) -> Value {
    let str_value = value.as_str().unwrap_or("").to_string();

    if value.is_array() {
        return Value::Array(
            value
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|x| {
                    parse_val(
                        &serde_json::from_value::<BigqueryResponseValue>(x.clone())
                            .ok()
                            .unwrap_or(BigqueryResponseValue { v: json!({}) })
                            .v,
                        typ,
                        schema,
                    )
                })
                .collect::<Vec<Value>>(),
        );
    }
    match typ.to_lowercase().as_str() {
        "struct" | "record" => {
            let mut nested_row_map = serde_json::Map::new();
            serde_json::from_value::<BigqueryResponseRow>(value.clone())
                .ok()
                .unwrap_or(BigqueryResponseRow { f: vec![] })
                .f
                .iter()
                .zip(schema.fields.as_ref().clone().unwrap_or(&vec![]).iter())
                .for_each(|(f, s)| {
                    nested_row_map.insert(s.name.clone(), parse_val(&f.v, &s.r#type, &s));
                });
            Value::from(nested_row_map)
        }
        "bool" | "boolean" => json!(str_value.parse::<bool>().ok().unwrap_or(false)),
        "float" | "float64" => json!(str_value.parse::<f64>().ok().unwrap_or(0.0)),
        "int64" | "integer" | "timestamp" => json!(str_value.parse::<i64>().ok().unwrap_or(0)),
        "json" => serde_json::from_str(&str_value).ok().unwrap_or(json!({})),
        _ => value.clone(),
    }
}
