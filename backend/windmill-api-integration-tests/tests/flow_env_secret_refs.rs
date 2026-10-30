//! A secret reached through `flow_env` is stored on the step job as a reference (exact
//! `flow_env.X` read) or encrypted (computed from it), and still resolves when the step runs.
//!
//!   cargo test -p windmill-api-integration-tests --features deno_core --test flow_env_secret_refs

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::*;

async fn create_variable(port: u16, path: &str, value: &str, is_secret: bool) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/variables/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({"path": path, "value": value, "is_secret": is_secret, "description": ""}))
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

#[cfg(feature = "deno_core")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_flow_env_secret_kept_out_of_step_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let secret = "flow_env_secret_value_8k2q";
    create_variable(port, "u/test-user/tok", secret, true).await;
    create_variable(port, "u/test-user/host", "example.com", false).await;

    let flow: FlowValue = serde_json::from_value(json!({
        "flow_env": {"TOKEN": "$var:u/test-user/tok", "HOST": "$var:u/test-user/host"},
        "modules": [{"id": "a", "value": {
            "type": "rawscript",
            "language": "bash",
            "content": "tok=\"$1\"\nhdr=\"$2\"\nurl=\"$3\"\necho \"{\\\"tok\\\":\\\"$tok\\\",\\\"hdr\\\":\\\"$hdr\\\",\\\"url\\\":\\\"$url\\\"}\" > result.json",
            "input_transforms": {
                "tok": {"type": "javascript", "expr": "flow_env.TOKEN"},
                "hdr": {"type": "javascript", "expr": "\"Bearer \" + flow_env.TOKEN"},
                "url": {"type": "javascript", "expr": "\"https://\" + flow_env.HOST"}
            }
        }}]
    }))?;

    let flow_job =
        RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
            .run_until_complete(&db, false, port)
            .await;
    assert_eq!(
        flow_job.json_result().unwrap(),
        json!({"tok": secret, "hdr": format!("Bearer {secret}"), "url": "https://example.com"})
    );

    let step_args: serde_json::Value =
        sqlx::query_scalar("SELECT args FROM v2_job WHERE parent_job = $1")
            .bind(flow_job.id)
            .fetch_one(&db)
            .await?;
    assert_eq!(step_args["tok"], "$var:u/test-user/tok");
    assert!(step_args["hdr"]
        .as_str()
        .unwrap()
        .starts_with("$encrypted:"));
    assert_eq!(step_args["url"], "https://example.com");
    assert!(!step_args.to_string().contains(secret));

    Ok(())
}
