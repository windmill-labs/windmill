use std::collections::HashMap;

use serde_json::{json, value::RawValue};
use sqlx::types::Json;
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
    job: &mut QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let api = if let Some(db) = job_args.as_ref().and_then(|x| x.get("api")) {
        serde_json::from_str::<GraphqlApi>(db.get())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let mut request = HTTP_CLIENT.post(api.base_url).json(&json!({
        "query": query,
        "variables": job_args
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
