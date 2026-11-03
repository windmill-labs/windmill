//! `jobs/recent_runs`: it aggregates over the workspace through `user_db`, which applies RLS but
//! not a token's `if_jobs:filter_tags` scope, so the handler filters on it itself; and it reports a
//! resolved failure apart from an unresolved one.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

// From the `jobs_read_auth` fixture, both runs of test-user-2: one tagged `deno`, one `flow`.
const DENO_JOB: &str = "14141414-1414-1414-1414-141414141414";
const FLOW_JOB: &str = "15151515-1515-1515-1515-151515151515";

async fn recent_runs(port: u16, token: &str) -> String {
    recent_runs_query(port, token, "").await
}

async fn recent_runs_query(port: u16, token: &str, query: &str) -> String {
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/recent_runs{query}"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("request");
    assert!(resp.status().is_success(), "{}", resp.status());
    resp.text().await.expect("body")
}

#[sqlx::test(fixtures("base", "jobs_read_auth"))]
async fn test_recent_runs_honors_tag_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let unscoped = recent_runs(port, "SECRET_TOKEN_2").await;
    assert!(unscoped.contains(DENO_JOB), "{unscoped}");
    assert!(unscoped.contains(FLOW_JOB), "{unscoped}");

    // SCOPED_DENO_TOKEN is test-user-2 restricted to `if_jobs:filter_tags:deno`.
    let scoped = recent_runs(port, "SCOPED_DENO_TOKEN").await;
    assert!(scoped.contains(DENO_JOB), "{scoped}");
    assert!(!scoped.contains(FLOW_JOB), "{scoped}");
    Ok(())
}

async fn insert_root_run(
    db: &Pool<Postgres>,
    id: &str,
    status: &str,
    resolved: bool,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag, visible_to_owner)
         VALUES ($1::uuid, 'test-workspace', 'test-user-2', 'u/test-user-2', 'test2@windmill.dev',
            'script', 'deno', 'u/test-user-2/flaky', 'deno', true)",
    )
    .bind(id)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status)
         VALUES ($1::uuid, 'test-workspace', 10, $2::job_status)",
    )
    .bind(id)
    .bind(status)
    .execute(db)
    .await?;
    if resolved {
        sqlx::query(
            "INSERT INTO job_resolution (job_id, workspace_id) VALUES ($1::uuid, 'test-workspace')",
        )
        .bind(id)
        .execute(db)
        .await?;
    }
    Ok(())
}

/// A resolved failure is `resolved`, not `failure`: Home's failed counts leave it out, and the
/// failed ids asked for one item are the failures that item counts.
#[sqlx::test(fixtures("base", "jobs_read_auth"))]
async fn test_recent_runs_resolved_failures(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    insert_root_run(&db, "a0000000-0000-0000-0000-000000000001", "failure", true).await?;
    insert_root_run(&db, "a0000000-0000-0000-0000-000000000002", "failure", false).await?;
    insert_root_run(&db, "a0000000-0000-0000-0000-000000000003", "canceled", true).await?;
    // A failed single-step flow wrapping a flow at the same path: the flow's failure, not the
    // script's, though a job listing by path and kind cannot tell them apart.
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag, visible_to_owner, raw_flow)
         VALUES ('a0000000-0000-0000-0000-000000000004', 'test-workspace', 'test-user-2',
            'u/test-user-2', 'test2@windmill.dev', 'singlestepflow', 'deno', 'u/test-user-2/flaky',
            'deno', true,
            '{\"modules\": [{\"id\": \"a\", \"value\": {\"type\": \"flow\", \"path\": \"u/test-user-2/flaky\"}}]}')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status)
         VALUES ('a0000000-0000-0000-0000-000000000004', 'test-workspace', 10, 'failure')",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;

    let body: serde_json::Value = serde_json::from_str(
        &recent_runs_query(
            server.addr.port(),
            "SECRET_TOKEN_2",
            "?failed_ids_kind=script&failed_ids_path=u/test-user-2/flaky",
        )
        .await,
    )?;
    let item = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["path"] == "u/test-user-2/flaky" && i["kind"] == "script")
        .expect("item");
    let status_of = |id: &str| {
        item["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .map(|r| r["status"].clone())
    };
    assert_eq!(status_of("a0000000-0000-0000-0000-000000000001"), Some("resolved".into()));
    assert_eq!(status_of("a0000000-0000-0000-0000-000000000002"), Some("failure".into()));
    assert_eq!(status_of("a0000000-0000-0000-0000-000000000003"), Some("canceled".into()));
    assert_eq!(item["failed_runs"], 1, "{body}");
    // Only the item named in the query carries its ids, and they are its own unresolved failures.
    assert_eq!(
        item["failed_run_ids"],
        serde_json::json!(["a0000000-0000-0000-0000-000000000002"]),
        "{body}"
    );
    let flow = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["path"] == "u/test-user-2/flaky" && i["kind"] == "flow")
        .expect("flow item");
    assert!(flow.get("failed_run_ids").is_none(), "{body}");
    assert_eq!(body["failed_runs"], 2, "{body}");
    Ok(())
}
