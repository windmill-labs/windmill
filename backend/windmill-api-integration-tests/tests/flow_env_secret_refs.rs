//! A secret reached through `flow_env` is stored on the step job as a reference (exact
//! `flow_env.X` read) or encrypted (computed from it), and still resolves when the step runs.
//! A subflow step, which resolves nothing in its args, gets a secret entry as a reference or
//! encrypted, and the resolved value of a non-secret one.
//!
//!   cargo test -p windmill-api-integration-tests --features deno_core --test flow_env_secret_refs
#![cfg(feature = "deno_core")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::*;

async fn api_create(port: u16, kind: &str, body: serde_json::Value) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/{kind}/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "{}",
        resp.text().await.unwrap_or_default()
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_flow_env_secret_kept_out_of_step_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let secret = "flow_env_secret_value_8k2q";
    for (path, value, is_secret) in [
        ("u/test-user/tok", secret, true),
        ("u/test-user/host", "example.com", false),
    ] {
        let body = json!({"path": path, "value": value, "is_secret": is_secret, "description": ""});
        api_create(port, "variables", body).await;
    }
    // A flow without steps returns its own inputs.
    let sub = json!({"path": "u/test-user/sub", "summary": "", "description": "",
        "schema": {"type": "object", "properties": {}}, "value": {"modules": []}});
    api_create(port, "flows", sub).await;

    let flow: FlowValue = serde_json::from_value(json!({
        "flow_env": {"TOKEN": "$var:u/test-user/tok", "HOST": "$var:u/test-user/host"},
        "modules": [
            {"id": "a", "value": {
                "type": "rawscript",
                "language": "bash",
                "content": "tok=\"$1\"\nhdr=\"$2\"\nurl=\"$3\"\necho \"{\\\"tok\\\":\\\"$tok\\\",\\\"hdr\\\":\\\"$hdr\\\",\\\"url\\\":\\\"$url\\\"}\" > result.json",
                "input_transforms": {
                    "tok": {"type": "javascript", "expr": "flow_env.TOKEN"},
                    "hdr": {"type": "javascript", "expr": "\"Bearer \" + flow_env.TOKEN"},
                    "url": {"type": "javascript", "expr": "\"https://\" + flow_env.HOST"}
                }
            }},
            {"id": "s", "value": {
                "type": "flow",
                "path": "u/test-user/sub",
                "input_transforms": {
                    "tok": {"type": "javascript", "expr": "flow_env.TOKEN"},
                    "hdr": {"type": "javascript", "expr": "\"Bearer \" + flow_env.TOKEN"},
                    "host": {"type": "javascript", "expr": "flow_env.HOST"}
                }
            }}
        ]
    }))?;

    let flow_job =
        RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
            .run_until_complete(&db, false, port)
            .await;
    let sub_args = flow_job.json_result().unwrap();
    assert_eq!(sub_args["tok"], "$var:u/test-user/tok");
    assert!(sub_args["hdr"].as_str().unwrap().starts_with("$encrypted:"));
    assert_eq!(sub_args["host"], "example.com");

    let (step_args, step_result): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT j.args, c.result FROM v2_job j JOIN v2_job_completed c ON c.id = j.id
         WHERE j.parent_job = $1 AND j.kind <> 'flow'",
    )
    .bind(flow_job.id)
    .fetch_one(&db)
    .await?;
    assert_eq!(
        step_result,
        json!({"tok": secret, "hdr": format!("Bearer {secret}"), "url": "https://example.com"})
    );
    assert_eq!(step_args["tok"], "$var:u/test-user/tok");
    assert!(step_args["hdr"]
        .as_str()
        .unwrap()
        .starts_with("$encrypted:"));
    assert_eq!(step_args["url"], "https://example.com");
    assert!(!step_args.to_string().contains(secret));

    Ok(())
}
