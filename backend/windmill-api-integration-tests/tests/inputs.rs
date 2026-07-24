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

// A scheduled runnable with a dynamic-skip handler (or a scheduled script with native
// retry) runs as a `singlestepflow`, not `flow`/`script` (see windmill-queue schedule.rs).
// Both a flow's and a script's history must surface their own singlestepflow runs, but a
// script and flow may share a path, so each side must match only the wrapped kind that
// belongs to it — the other must not leak in.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_input_history_singlestepflow_flow_vs_script(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Both rows share runnable_path 'f/test/scheduled'.
    let flow_job = insert_singlestepflow(&db, "flow").await?;
    let script_job = insert_singlestepflow(&db, "script").await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    let history_ids = |runnable_type: &'static str| {
        let base = base.clone();
        async move {
            let resp = authed(client().get(format!(
                "{base}/history?runnable_id=f/test/scheduled&runnable_type={runnable_type}"
            )))
            .send()
            .await?;
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            assert_2xx(status, &body, "GET /inputs/history");
            let inputs: Vec<serde_json::Value> = serde_json::from_str(&body)?;
            anyhow::Ok(
                inputs
                    .iter()
                    .filter_map(|i| i.get("id").and_then(|v| v.as_str()).map(String::from))
                    .collect::<std::collections::HashSet<_>>(),
            )
        }
    };

    let flow_hist = history_ids("FlowPath").await?;
    assert!(
        flow_hist.contains(&flow_job.to_string()),
        "flow-wrapped singlestepflow missing from flow history: {flow_hist:?}",
    );
    assert!(
        !flow_hist.contains(&script_job.to_string()),
        "script-wrapped singlestepflow leaked into flow history: {flow_hist:?}",
    );

    let script_hist = history_ids("ScriptPath").await?;
    assert!(
        script_hist.contains(&script_job.to_string()),
        "script-wrapped singlestepflow missing from script history: {script_hist:?}",
    );
    assert!(
        !script_hist.contains(&flow_job.to_string()),
        "flow-wrapped singlestepflow leaked into script history: {script_hist:?}",
    );

    Ok(())
}

// Insert a completed root singlestepflow at path f/test/scheduled wrapping `wrapped_type`
// ('flow' or 'script') as its single module — mirrors the schedule.rs wrapper shape.
async fn insert_singlestepflow(
    db: &Pool<Postgres>,
    wrapped_type: &str,
) -> anyhow::Result<uuid::Uuid> {
    let id = uuid::Uuid::new_v4();
    let raw_flow = json!({ "modules": [{ "id": "a", "value": { "type": wrapped_type } }] });
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, tag, created_by, permissioned_as, \
         permissioned_as_email, kind, runnable_path, raw_flow, same_worker, visible_to_owner) \
         VALUES ($1, 'test-workspace', 'flow', 'test-user', 'u/test-user', \
         'test@windmill.dev', 'singlestepflow', 'f/test/scheduled', $2, false, true)",
    )
    .bind(id)
    .bind(sqlx::types::Json(&raw_flow))
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status) \
         VALUES ($1, 'test-workspace', 1, false, 'success')",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(id)
}
