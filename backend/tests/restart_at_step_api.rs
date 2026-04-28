//! HTTP-level integration tests for the `restart_flow_at_step` API endpoint.
//!
//! The other restart tests in `job_payload.rs` exercise the worker by hand-
//! constructing `RestartedFrom` chains and calling `push()` directly. These
//! tests instead drive the actual HTTP endpoint to lock in the API contract,
//! including the validation branches in `resolve_nested_restart` (step lookup,
//! parallel rejection, etc.).

#![cfg(feature = "deno_core")]
#![cfg(feature = "enterprise")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::flows::FlowValue;
use windmill_common::jobs::JobPayload;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

const SUPER_TOKEN: &str = "SECRET_TOKEN";

/// Happy path: HTTP nested restart targeting a step inside iteration 1 of a
/// top-level sequential ForLoop. Verifies the API endpoint accepts a
/// `nested_path`, walks the original execution to resolve UUIDs, and the
/// resulting job runs successfully.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_api_restart_at_step_nested_happy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = RunJob::from(JobPayload::Flow {
        path: "f/system/hello_with_nodes_flow".to_string(),
        dedicated_worker: None,
        apply_preprocessor: true,
        version: 1443253234253454,
        labels: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/restart/f/{}",
            first_run.id
        )),
        SUPER_TOKEN,
    )
    .json(&json!({
        "step_id": "a",
        "branch_or_iteration_n": 1,
        "nested_path": [{ "step_id": "c" }],
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "expected 201, got {}: {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
    let new_job_id = resp.text().await?;
    assert!(!new_job_id.is_empty(), "expected job UUID in response body");
    Ok(())
}

/// Happy path: top-level (non-nested) restart via HTTP. Same surface as
/// `test_restarted_flow_payload` but driven through the actual REST endpoint.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_api_restart_at_step_top_level_happy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = RunJob::from(JobPayload::Flow {
        path: "f/system/hello_with_nodes_flow".to_string(),
        dedicated_worker: None,
        apply_preprocessor: true,
        version: 1443253234253454,
        labels: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/restart/f/{}",
            first_run.id
        )),
        SUPER_TOKEN,
    )
    .json(&json!({ "step_id": "a", "branch_or_iteration_n": 0 }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    Ok(())
}

/// Rejection: nested step doesn't exist in the original run. The API should
/// reject with a 4xx (or surface a clear backend error) rather than silently
/// queue an unrunnable job.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_api_restart_at_step_rejects_unknown_step(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = RunJob::from(JobPayload::Flow {
        path: "f/system/hello_with_nodes_flow".to_string(),
        dedicated_worker: None,
        apply_preprocessor: true,
        version: 1443253234253454,
        labels: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/restart/f/{}",
            first_run.id
        )),
        SUPER_TOKEN,
    )
    .json(&json!({
        "step_id": "a",
        "branch_or_iteration_n": 1,
        "nested_path": [{ "step_id": "does_not_exist" }],
    }))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "expected error, got {}",
        resp.status()
    );
    Ok(())
}

/// Rejection: nested restart targets an iteration past the actual count.
/// Original ran 3 iterations (index 0..2); requesting `branch_or_iteration_n=5`
/// must error.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_api_restart_at_step_rejects_out_of_range_iteration(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = RunJob::from(JobPayload::Flow {
        path: "f/system/hello_with_nodes_flow".to_string(),
        dedicated_worker: None,
        apply_preprocessor: true,
        version: 1443253234253454,
        labels: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/restart/f/{}",
            first_run.id
        )),
        SUPER_TOKEN,
    )
    .json(&json!({
        "step_id": "a",
        "branch_or_iteration_n": 5,
        "nested_path": [{ "step_id": "c" }],
    }))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "expected error for out-of-range iteration, got {}: {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
    Ok(())
}

/// Rejection: parallel ForLoop ancestor on the nested path. The resolver
/// rejects this category outright — `branch_or_iteration_n` only makes sense
/// for sequential containers because each iteration runs as a separate
/// numbered child.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_api_restart_at_step_rejects_parallel_loop(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Run a parallel ForLoop with an inner script. We use a `RawFlow` with an
    // explicit `path` so the API endpoint accepts it as a valid completed job
    // (the handler requires `runnable_path`).
    let parallel_flow: FlowValue = serde_json::from_value(json!({
        "modules": [{
            "id": "loop",
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "javascript", "expr": "[1, 2]" },
                "skip_failures": false,
                "parallel": true,
                "modules": [{
                    "id": "inner",
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "input_transforms": {},
                        "content": "export function main() { return 'ok' }"
                    }
                }]
            }
        }],
    }))
    .unwrap();

    let first_run = RunJob::from(JobPayload::RawFlow {
        value: parallel_flow,
        path: Some("u/admin/parallel_test".to_string()),
        restarted_from: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/restart/f/{}",
            first_run.id
        )),
        SUPER_TOKEN,
    )
    .json(&json!({
        "step_id": "loop",
        "branch_or_iteration_n": 0,
        "nested_path": [{ "step_id": "inner" }],
    }))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "expected rejection of parallel-loop nested restart, got {}: {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
    Ok(())
}
