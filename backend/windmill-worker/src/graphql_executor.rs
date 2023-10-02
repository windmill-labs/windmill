use std::collections::HashMap;

use serde_json::{json, Value};
use windmill_common::error::Error;
use windmill_common::jobs::QueuedJob;
use windmill_queue::HTTP_CLIENT;

use serde::Deserialize;

use crate::{common::transform_json_value, AuthedClient};

#[derive(Deserialize)]
struct GraphqlApi {
    bearer_token: Option<String>,
    base_url: String,
    custom_headers: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct GraphqlResponse {
    data: Option<Value>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

pub async fn do_graphql(
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

    let graphql_args: serde_json::Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let api =
        serde_json::from_value::<GraphqlApi>(graphql_args.get("api").unwrap_or(&json!({})).clone())
            .map_err(|e: serde_json::Error| Error::ExecutionErr(e.to_string()))?;

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());

    let mut request = HTTP_CLIENT.post(api.base_url).json(&json!({
        "query": query,
        "variables": args
    }));

    if let Some(token) = &api.bearer_token {
        request = request.bearer_auth(token.as_str());
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

    let result = response
        .json::<GraphqlResponse>()
        .await
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
    return Ok(result.data.unwrap_or(json!({})));
}
