use std::collections::HashMap;

use futures::{stream, TryStreamExt};
use serde_json::{json, value::RawValue};
use sqlx::types::Json;
use windmill_common::worker::{to_raw_value, Connection};
use windmill_common::{error::Error, worker::CLOUD_HOSTED};
use windmill_parser_graphql::parse_graphql_sig;
use windmill_queue::{CanceledBy, MiniPulledJob};

use serde::Deserialize;

use crate::common::{build_http_client, resolve_job_timeout, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::{common::build_args_map, AuthedClient};

#[derive(Deserialize)]
struct GraphqlApi {
    bearer_token: Option<String>,
    base_url: String,
    custom_headers: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct GraphqlResponse {
    data: Option<Box<RawValue>>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

pub async fn do_graphql(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    occupation_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, conn).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let api = if let Some(db) = job_args.as_ref().and_then(|x| x.get("api")) {
        serde_json::from_str::<GraphqlApi>(db.get())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(windmill_common::error::Error::BadRequest(
            "Missing api argument".to_string(),
        ));
    };

    // variables is job_args except for api
    let mut variables = HashMap::new();
    let sig = parse_graphql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;
    if let Some(job_args) = job_args {
        for arg in &sig {
            match job_args.get(&arg.name) {
                Some(x) => {
                    variables.insert(arg.name.clone(), x.to_owned());
                }
                None if arg.default.is_some() => {
                    variables.insert(
                        arg.name.clone(),
                        to_raw_value(arg.default.as_ref().unwrap()),
                    );
                }
                _ => {}
            }
        }
    }
    let (timeout_duration, _, _) =
        resolve_job_timeout(&conn, &job.workspace_id, job.id, job.timeout).await;

    let http_client = build_http_client(timeout_duration)?;

    let mut request = http_client.post(api.base_url).json(&json!({
        "query": query,
        "variables": variables
    }));

    if let Some(token) = &api.bearer_token {
        if token.len() > 0 {
            request = request.bearer_auth(token.as_str());
        }
    }

    if let Some(headers) = &api.custom_headers {
        for (k, v) in headers {
            request = request.header(k, v);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    let result_stream = response.bytes_stream();

    let mut i = 0;
    let is_cloud_hosted = *CLOUD_HOSTED;

    let result_f = async {
        let result_f = result_stream
            .map_err(|x| Error::ExecutionErr(x.to_string()))
            .try_fold(Vec::new(), |mut acc, x| async move {
                i += x.len();
                if (is_cloud_hosted && i > 2_000_000) || (i > 500_000_000) {
                    return Err(Error::ExecutionErr(format!(
                        "Response too large: {i} bytes"
                    )));
                }
                acc.extend_from_slice(&x);
                Ok(acc)
            });

        let result = serde_json::from_slice::<GraphqlResponse>(
            &result_f
                .await
                .map_err(|_| Error::ExecutionErr("15 timeout for http request".to_string()))?,
        )
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

        if let Some(errors) = result.errors {
            return Err(Error::ExecutionErr(
                errors
                    .into_iter()
                    .map(|x| x.message)
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
        }

        // And then check that we got back the same string we sent over.
        Ok(result
            .data
            .unwrap_or_else(|| serde_json::from_str("{}").unwrap()))
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
        &mut Some(occupation_metrics),
        Box::pin(stream::once(async { 0 })),
    )
    .await?;

    *mem_peak = (r.get().len() / 1000) as i32;
    Ok(r)
}
