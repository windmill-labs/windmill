use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_inputs_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    // GET /history with fake runnable → 200 empty array
    let resp = authed(client().get(format!(
        "{base}/history?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/history");

    // GET /list with fake runnable → 200 empty array
    let resp = authed(client().get(format!(
        "{base}/list?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/list");

    // POST /create → 200, returns UUID
    let resp = authed(client().post(format!(
        "{base}/create?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .json(&json!({"name": "test_input", "args": {}}))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /inputs/create");
    let input_id: String = serde_json::from_str(&body)?;

    // GET /{id}/args → 200
    let resp = authed(client().get(format!("{base}/{input_id}/args")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/{id}/args");

    // POST /delete/{id} → 200
    let resp = authed(client().post(format!("{base}/delete/{input_id}")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /inputs/delete/{id}");

    Ok(())
}

// A scheduled/triggered flow whose schedule has a dynamic-skip handler runs as a
// `singlestepflow`, not `flow` (see windmill-queue schedule.rs). The run-history
// sidebar must surface those runs, so `get_input_history` includes that kind for
// flow runnables.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_input_history_includes_singlestepflow(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, tag, created_by, permissioned_as, \
         permissioned_as_email, kind, runnable_path, same_worker, visible_to_owner) \
         VALUES ($1, 'test-workspace', 'flow', 'test-user', 'u/test-user', \
         'test@windmill.dev', 'singlestepflow', 'f/test/scheduled_flow', false, true)",
    )
    .bind(job_id)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status) \
         VALUES ($1, 'test-workspace', 1, false, 'success')",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    let resp = authed(client().get(format!(
        "{base}/history?runnable_id=f/test/scheduled_flow&runnable_type=FlowPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/history");

    let inputs: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    assert!(
        inputs
            .iter()
            .any(|i| i.get("id").and_then(|v| v.as_str()) == Some(&job_id.to_string())),
        "singlestepflow run missing from flow input history: {body}",
    );

    Ok(())
}
