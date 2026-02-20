#![cfg(all(feature = "private", feature = "agent_worker_server"))]

use windmill_test_utils::*;
use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
};

fn bun_code(code: &str) -> RawCode {
    RawCode {
        hash: None,
        content: code.to_string(),
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_simple_script(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        "export function main() { return 42; }",
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    assert_eq!(result.json_result(), Some(json!(42)));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_script_with_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        "export function main(x: number, y: number) { return x + y; }",
    )))
    .arg("x", json!(10))
    .arg("y", json!(32))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    assert_eq!(result.json_result(), Some(json!(42)));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_script_with_logs(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        r#"export function main() {
    console.log("hello from agent worker");
    console.log("processing step 1");
    console.log("processing step 2");
    return "done";
}"#,
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    assert_eq!(result.json_result(), Some(json!("done")));

    let logs = sqlx::query_scalar::<_, String>(
        "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(result.id)
    .fetch_optional(&db)
    .await?;

    let logs = logs.expect("logs should exist");
    assert!(
        logs.contains("hello from agent worker"),
        "logs should contain the printed output, got: {logs}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_script_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        "export function main() { throw new Error('test error'); }",
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(!result.success, "job should fail");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_complex_result(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        r#"export function main() {
    return {
        items: [1, 2, 3],
        metadata: { key: "value" },
        count: 3,
    };
}"#,
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    let json = result.json_result().unwrap();
    assert_eq!(json["items"], json!([1, 2, 3]));
    assert_eq!(json["metadata"]["key"], json!("value"));
    assert_eq!(json["count"], json!(3));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_token_creation(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, _port, _server) = init_client_agent_mode(db.clone()).await;

    // client.baseurl() already includes /api
    let resp = client
        .client()
        .post(format!(
            "{}/agent_workers/create_agent_token",
            client.baseurl()
        ))
        .json(&json!({
            "worker_group": "lifecycle-test",
            "tags": ["bun", "flow", "dependency"],
            "exp": usize::MAX
        }))
        .send()
        .await?;

    assert!(
        resp.status().is_success(),
        "create_agent_token should succeed, got: {}",
        resp.status()
    );

    let token = resp.text().await?;
    let token = token.trim_matches('"');
    assert!(
        token.starts_with("jwt_agent_"),
        "token should start with jwt_agent_ prefix, got: {token}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_token_and_ping(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, port, _server) = init_client_agent_mode(db.clone()).await;

    let resp = client
        .client()
        .post(format!(
            "{}/agent_workers/create_agent_token",
            client.baseurl()
        ))
        .json(&json!({
            "worker_group": "lifecycle-test",
            "tags": ["bun", "flow", "dependency"],
            "exp": usize::MAX
        }))
        .send()
        .await?;

    assert!(resp.status().is_success());
    let token = resp.text().await?;
    let token = token.trim_matches('"');

    let suffix = windmill_common::utils::create_default_worker_suffix("lifecycle-test");
    let base_url = format!("http://localhost:{port}");
    let http_client =
        windmill_common::agent_workers::build_agent_http_client(&suffix, &token, &base_url);

    // Initial ping inserts the worker record into the database
    let resp = http_client
        .client
        .post(format!("{}/api/agent_workers/update_ping", base_url))
        .json(&json!({
            "worker_instance": "test-instance",
            "ip": "127.0.0.1",
            "tags": ["bun"],
            "version": "test",
            "vcpus": 4,
            "memory": 8192,
            "ping_type": "Initial"
        }))
        .send()
        .await?;

    assert!(
        resp.status().is_success(),
        "initial ping should succeed, got: {}",
        resp.status()
    );

    // Verify the ping was recorded in the database
    let worker_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM worker_ping WHERE worker_instance = 'test-instance'",
    )
    .fetch_one(&db)
    .await?;

    assert!(worker_count > 0, "worker ping should be recorded in database");

    // MainLoop ping updates the existing record
    let resp = http_client
        .client
        .post(format!("{}/api/agent_workers/update_ping", base_url))
        .json(&json!({
            "tags": ["bun"],
            "vcpus": 4,
            "memory": 8192,
            "jobs_executed": 0,
            "ping_type": "MainLoop"
        }))
        .send()
        .await?;

    assert!(
        resp.status().is_success(),
        "main loop ping should succeed, got: {}",
        resp.status()
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_multiple_jobs_sequential(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    for i in 0..3 {
        let result = RunJob::from(JobPayload::Code(bun_code(&format!(
            "export function main() {{ return {i}; }}"
        ))))
        .run_until_complete(&db, false, port)
        .await;

        assert!(result.success, "job {i} should succeed");
        assert_eq!(result.json_result(), Some(json!(i)));
    }

    Ok(())
}
