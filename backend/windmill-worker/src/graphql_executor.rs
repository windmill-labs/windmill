use std::collections::HashMap;

use serde_json::{json, value::RawValue};
use windmill_common::error::Error;
use windmill_common::jobs::QueuedJob;
use windmill_queue::HTTP_CLIENT;

use serde::Deserialize;

use crate::{common::build_args_map, AuthedClientBackgroundTask};

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
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let graphql_args = build_args_map(job, client, db).await?;

    let api = if let Some(db) = graphql_args.get("api") {
        serde_json::from_str::<GraphqlApi>(db.get())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let args = &job.args;

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
    return Ok(result
        .data
        .unwrap_or_else(|| serde_json::from_str("{}").unwrap()));
}
