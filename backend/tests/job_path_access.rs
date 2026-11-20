//! Job routes that act on a script or flow path rather than a job id must hold the caller to
//! the access the path's own routes require: `queue/cancel_persistent` to the per-job cancel
//! check, `run/flow_dependencies` to write access on the flow path.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

const W_ID: &str = "test-workspace";

async fn post_json(
    url: &str,
    token: &str,
    body: serde_json::Value,
) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("request");
    let status = resp.status();
    (status, resp.text().await.expect("body"))
}

#[sqlx::test(fixtures("base"))]
async fn cancel_persistent_requires_cancel_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let path = "u/test-user-2/loop";
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, \
            language, lock, restart_unless_cancelled) \
         VALUES ($1, 1, $2, '', '', 'echo', 'test-user-2', 'bash', '', true)",
    )
    .bind(W_ID)
    .bind(path)
    .execute(&db)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, runnable_id, runnable_path, script_lang, tag, \
            visible_to_owner) \
         VALUES ($1, $2, 'test-user-2', now(), 'u/test-user-2', 'test2@windmill.dev', 'script', \
            1, $3, 'bash', 'bash', true)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(path)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at, tag) \
         VALUES ($1, $2, now(), true, now(), 'bash')",
    )
    .bind(id)
    .bind(W_ID)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let (status, body) = post_json(
        &format!("http://localhost:{port}/api/w/{W_ID}/jobs_u/queue/cancel_persistent/{path}"),
        "SECRET_TOKEN_3",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "a member who cannot see the run must not cancel it: {body}"
    );
    let canceled_by: Option<String> =
        sqlx::query_scalar("SELECT canceled_by FROM v2_job_queue WHERE id = $1")
            .bind(id)
            .fetch_one(&db)
            .await?;
    assert_eq!(canceled_by, None, "the perpetual run is left running");
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn flow_dependencies_requires_write_on_the_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let (status, body) = post_json(
        &format!("http://localhost:{port}/api/w/{W_ID}/jobs/run/flow_dependencies_async"),
        "SECRET_TOKEN_3",
        serde_json::json!({ "path": "u/test-user-2/flow", "flow_value": { "modules": [] } }),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "a developer must not run a dependency job for a flow path they cannot write: {body}"
    );
    let queued: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM v2_job WHERE workspace_id = $1 AND runnable_path = $2",
    )
    .bind(W_ID)
    .bind("u/test-user-2/flow")
    .fetch_one(&db)
    .await?;
    assert_eq!(queued, 0, "no dependency job is queued");
    Ok(())
}
