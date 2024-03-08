use futures::TryFutureExt;
use serde_json::{json, value::RawValue, Value};
use windmill_common::error::to_anyhow;
use windmill_common::jobs::QueuedJob;
use windmill_common::{error::Error, worker::to_raw_value};
use windmill_parser_sql::{parse_bigquery_sig, parse_db_resource};
use windmill_queue::{CanceledBy, HTTP_CLIENT};

use serde::Deserialize;

use crate::common::run_future_with_polling_update_job_poller;
use crate::{
    common::{build_args_values, resolve_job_timeout},
    AuthedClientBackgroundTask,
};

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
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
) -> windmill_common::error::Result<Box<RawValue>> {
    let bigquery_args = build_args_values(job, client, db).await?;

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
        bigquery_args.get("database").cloned()
    };

    let database = if let Some(db) = db_arg {
        db.to_string()
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let service_account = CustomServiceAccount::from_json(&database)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let authentication_manager = AuthenticationManager::from(service_account);
    let scopes = &["https://www.googleapis.com/auth/bigquery"];
    let token = authentication_manager
        .get_token(scopes)
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let mut statement_values: Vec<Value> = vec![];

    let sig = parse_bigquery_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    for arg in &sig {
        let arg_t = arg.otyp.clone().unwrap_or_else(|| "string".to_string());
        let arg_n = arg.clone().name;
        let arg_v = bigquery_args.get(&arg.name).cloned().unwrap_or(json!(""));
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
                    "arrayValues": bigquery_args
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

    let timeout_ms = i32::try_from(
        resolve_job_timeout(&db, &job.workspace_id, job.id, job.timeout)
            .await
            .0
            .as_millis(),
    )
    .unwrap_or(200000);

    let result_f = async {
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
                "timeoutMs": timeout_ms,
                "queryParameters": statement_values,
            }))
            .send()
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!("Could not send query to BigQuery API: {}", e))
            })?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let result = response.json::<BigqueryResponse>().await.map_err(|e| {
                    Error::ExecutionErr(format!(
                        "BigQuery API response could not be parsed: {}",
                        e.to_string()
                    ))
                })?;

                if !result.jobComplete {
                    return Err(Error::ExecutionErr(
                        "BigQuery API did not answer query in time".to_string(),
                    ));
                }

                if result.rows.is_none() || result.rows.as_ref().unwrap().len() == 0 {
                    return Ok(serde_json::from_str("[]").unwrap());
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
                    .collect::<Vec<_>>();

                return Ok(to_raw_value(&rows));
            }
            Err(e) => match response.json::<BigqueryErrorResponse>().await {
                Ok(bq_err) => Err(Error::ExecutionErr(format!(
                    "Error from BigQuery API: {}",
                    bq_err.error.message
                )))
                .map_err(to_anyhow)?,
                Err(_) => Err(Error::ExecutionErr(format!(
                    "Error from BigQuery API could not be parsed: {}",
                    e.to_string()
                )))
                .map_err(to_anyhow)?,
            },
        }
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

fn convert_val(arg_t: String, arg_v: Value) -> Value {
    match arg_t.as_str() {
        "timestamp" | "datetime" | "date" | "time" => {
            let mut v: String = arg_v.as_str().unwrap_or("").to_owned();

            match arg_t.as_str() {
                "timestamp" | "datetime" => {
                    v = v.trim_end_matches("Z").to_string();
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
                            v = time.trim_end_matches("Z").to_string();
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
