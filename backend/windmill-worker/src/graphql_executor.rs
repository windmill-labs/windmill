use serde_json::{json, Value};
use windmill_common::error::Error;
use windmill_common::jobs::QueuedJob;
use windmill_queue::HTTP_CLIENT;

use serde::Deserialize;

use crate::{transform_json_value, AuthedClient, JobCompleted};

#[derive(Deserialize)]
struct GraphqlDatabase {
    bearer_token: Option<String>,
    base_url: String,
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
) -> windmill_common::error::Result<JobCompleted> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };

    let graphql_args: serde_json::Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let database = serde_json::from_value::<GraphqlDatabase>(
        graphql_args.get("database").unwrap_or(&json!({})).clone(),
    )
    .map_err(|e: serde_json::Error| Error::ExecutionErr(e.to_string()))?;

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());

    let mut request = HTTP_CLIENT.post(database.base_url).json(&json!({
        "query": query,
        "variables": args
    }));

    if let Some(token) = &database.bearer_token {
        request = request.bearer_auth(token.as_str());
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
    return Ok(JobCompleted {
        job: job,
        result: result.data.unwrap_or(json!({})),
        logs: "".to_string(),
        success: true,
    });
}
