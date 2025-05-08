use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;

use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, TryFutureExt};
use reqwest::Client;
use serde_json::{json, value::RawValue, Value};
use windmill_common::error::to_anyhow;
use windmill_common::more_serde::json_stream_values;
use windmill_common::s3_helpers::convert_json_line_stream;
use windmill_common::worker::Connection;
use windmill_common::{error::Error, worker::to_raw_value};
use windmill_parser_sql::{
    parse_bigquery_sig, parse_db_resource, parse_s3_mode, parse_sql_blocks,
    parse_sql_statement_named_params, s3_mode_extension, S3ModeFormat,
};
use windmill_queue::CanceledBy;

use serde::Deserialize;

use crate::common::{build_http_client, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::{
    common::{build_args_values, resolve_job_timeout},
    AuthedClient,
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

#[derive(Clone)]
struct S3Mode {
    client: AuthedClient,
    object_key: String,
    format: S3ModeFormat,
    storage: Option<String>,
    workspace_id: String,
}

fn do_bigquery_inner<'a>(
    query: &'a str,
    all_statement_values: &'a HashMap<String, Value>,
    project_id: &'a str,
    token: &'a str,
    timeout_ms: u64,
    column_order: Option<&'a mut Option<Vec<String>>>,
    skip_collect: bool,
    http_client: &'a Client,
    s3: Option<S3Mode>,
) -> windmill_common::error::Result<BoxFuture<'a, windmill_common::error::Result<Box<RawValue>>>> {
    let param_names = parse_sql_statement_named_params(query, '@');

    let statement_values = all_statement_values
        .iter()
        .filter_map(|(name, val)| {
            if param_names.contains(name) {
                Some(val)
            } else {
                None
            }
        })
        .collect::<Vec<&Value>>();

    let result_f = async move {
        let response = http_client
            .post(
                "https://bigquery.googleapis.com/bigquery/v2/projects/".to_string()
                    + project_id
                    + "/queries",
            )
            .bearer_auth(token)
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
                if skip_collect {
                    return Ok(to_raw_value(&Value::Array(vec![])));
                } else if let Some(ref s3) = s3 {
                    let rows_stream = json_stream_values(response.bytes_stream(), |sender| {
                        RootMpscDeserializer { sender }
                    })
                    .await?
                    .map(|chunk| Ok::<_, Infallible>(chunk));

                    let stream = convert_json_line_stream(rows_stream.boxed(), s3.format)
                        .await?
                        .map(|chunk| Ok::<_, Infallible>(chunk));

                    s3.client
                        .upload_s3_file(
                            s3.workspace_id.as_str(),
                            s3.object_key.clone(),
                            s3.storage.clone(),
                            stream,
                        )
                        .await?;

                    Ok(serde_json::value::to_raw_value(&s3.object_key)?)
                } else {
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

                    if let Some(column_order) = column_order {
                        *column_order = Some(
                            result
                                .schema
                                .as_ref()
                                .unwrap()
                                .fields
                                .iter()
                                .map(|x| x.name.clone())
                                .collect::<Vec<String>>(),
                        );
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

                    Ok(to_raw_value(&rows))
                }
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

    Ok(result_f.boxed())
}

use windmill_queue::MiniPulledJob;

pub async fn do_bigquery(
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
    let bigquery_args = build_args_values(job, client, conn).await?;

    let inline_db_res_path = parse_db_resource(&query);
    let s3 = parse_s3_mode(&query).map(|s3_mode| S3Mode {
        client: client.clone(),
        storage: s3_mode.storage,
        object_key: format!(
            "{}/{}.{}",
            s3_mode.prefix.unwrap_or_else(|| format!(
                "wmill_datalake/{}",
                job.runnable_path
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unknown_script")
            )),
            job.id,
            s3_mode_extension(s3_mode.format)
        ),
        format: s3_mode.format,
        workspace_id: job.workspace_id.clone(),
    });

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
        bigquery_args.get("database").cloned()
    };

    let database = if let Some(db) = db_arg {
        db.to_string()
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let annotations = windmill_common::worker::SqlAnnotations::parse(query);

    let service_account = CustomServiceAccount::from_json(&database)
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let authentication_manager = AuthenticationManager::from(service_account);
    let scopes = &["https://www.googleapis.com/auth/bigquery"];
    let token = authentication_manager
        .get_token(scopes)
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let (timeout_duration, _, _) =
        resolve_job_timeout(&conn, &job.workspace_id, job.id, job.timeout).await;
    let timeout_ms = timeout_duration.as_millis() as u64;
    let http_client = build_http_client(timeout_duration)?;

    let project_id = authentication_manager
        .project_id()
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let sig = parse_bigquery_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let (query, args_to_skip) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &bigquery_args)?;

    let queries = parse_sql_blocks(query);

    let mut statement_values: HashMap<String, Value> = HashMap::new();

    for arg in &sig {
        if args_to_skip.contains(&arg.name) {
            continue;
        }
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

        statement_values.insert(arg_n, bigquery_v);
    }

    let result_f = if queries.len() > 1 {
        let futures = queries
            .iter()
            .enumerate()
            .map(|(i, x)| {
                do_bigquery_inner(
                    x,
                    &statement_values,
                    &project_id,
                    token.as_str(),
                    timeout_ms,
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
        do_bigquery_inner(
            query,
            &statement_values,
            &project_id,
            token.as_str(),
            timeout_ms,
            Some(column_order),
            false,
            &http_client,
            s3,
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

// This deserializer takes a bigquery response as a stream and sends each row to an mpsc
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
                formatter.write_str("bigquery response json object")
            }
            fn visit_map<A>(mut self, mut map: A) -> Result<(), A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                while let Some(key) = map.next_key::<String>()? {
                    // We assume schema always comes before rows
                    if key == "schema" {
                        let schema: BigqueryResponseSchema = map.next_value()?;
                        for field in schema.fields {
                            self.col_defs.push(field.name);
                        }
                    } else if key == "rows" {
                        let () = map.next_value_seed(RowsMpscDeserializer {
                            sender: self.sender,
                            col_defs: &self.col_defs,
                        })?;
                    } else {
                        // Ignore other keys
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
                while let Some(elem) = seq.next_element::<BigqueryResponseRow>()? {
                    let mut row = BTreeMap::<&str, Value>::new();
                    for (i, field) in elem.f.iter().enumerate() {
                        let col_name = self.col_defs.get(i).map(|s| s.as_str()).unwrap_or("");
                        row.insert(col_name, field.v.clone());
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
