use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::variables::generate_approval_token;
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

/// Insert a minimal completed job directly into the database for testing.
async fn insert_completed_job(db: &Pool<Postgres>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, tag, args)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'script', 'deno', '{}'::jsonb)",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
         VALUES ($1, 'test-workspace', 100, '42'::jsonb, 'success')",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();

    id
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_unauthed_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs_u");

    let job_id = insert_completed_job(&db).await;

    // --- No-data endpoints ---

    let resp = authed(client().post(format!("{base}/queue/get_started_at_by_ids")))
        .json(&json!([]))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/get_started_at_by_ids",
    );

    // --- Completed job endpoints (unauthed service, with auth header) ---

    let resp = authed(client().get(format!("{base}/get/{job_id}")))
        .send()
        .await?;
    assert_2xx(resp.status().as_u16(), &resp.text().await?, "GET /get");

    let resp = authed(client().get(format!("{base}/get_logs/{job_id}")))
        .send()
        .await?;
    assert_2xx(resp.status().as_u16(), &resp.text().await?, "GET /get_logs");

    let resp = authed(client().get(format!("{base}/get_completed_logs_tail/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_completed_logs_tail",
    );

    let resp = authed(client().get(format!("{base}/get_args/{job_id}")))
        .send()
        .await?;
    assert_2xx(resp.status().as_u16(), &resp.text().await?, "GET /get_args");

    let resp = authed(client().get(format!("{base}/completed/get/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_result",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result_maybe/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_result_maybe",
    );

    let resp = authed(client().get(format!("{base}/completed/get_timing/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_timing",
    );

    let resp = authed(client().get(format!("{base}/getupdate/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /getupdate",
    );

    Ok(())
}

const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000000";
const FAKE_SECRET: &str = "aabb";

/// Reachability tests for endpoints that need complex runtime.
/// These just verify the route matches (handler runs), not 2xx.
fn assert_route_reachable(status: u16, body: &str, endpoint: &str) {
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for {endpoint}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_unauthed_complex_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs_u");

    let resp = authed(client().get(format!("{base}/resume/{FAKE_UUID}/1/{FAKE_SECRET}")))
        .send()
        .await?;
    assert_route_reachable(resp.status().as_u16(), &resp.text().await?, "GET /resume");

    let resp = authed(client().post(format!("{base}/cancel/{FAKE_UUID}/1/{FAKE_SECRET}")))
        .send()
        .await?;
    assert_route_reachable(resp.status().as_u16(), &resp.text().await?, "POST /cancel");

    let resp = authed(client().get(format!("{base}/get_flow/{FAKE_UUID}/1/{FAKE_SECRET}")))
        .send()
        .await?;
    assert_route_reachable(resp.status().as_u16(), &resp.text().await?, "GET /get_flow");

    let resp = authed(client().post(format!("{base}/queue/cancel/{FAKE_UUID}")))
        .json(&serde_json::json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/cancel",
    );

    let resp = authed(client().post(format!("{base}/queue/force_cancel/{FAKE_UUID}")))
        .json(&serde_json::json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/force_cancel",
    );

    let resp = authed(client().post(format!("{base}/flow/resume_suspended/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /flow/resume_suspended",
    );

    let resp = authed(client().get(format!("{base}/flow/approval_info/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /flow/approval_info",
    );

    let resp = authed(client().get(format!("{base}/get_root_job_id/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_root_job_id",
    );

    let resp = authed(client().get(format!("{base}/get_flow_debug_info/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_flow_debug_info",
    );

    let resp = authed(client().get(format!("{base}/get_log_file/{FAKE_UUID}/test.txt")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_log_file",
    );

    let resp = authed(client().post(format!("{base}/queue/cancel_persistent/u/test-user/fake")))
        .json(&serde_json::json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/cancel_persistent",
    );

    Ok(())
}

/// Build a minimal FlowValue JSON with a suspend step (with resume_form) followed by an identity
/// step. The suspend step is at index 0, the identity step at index 1. After the suspend step
/// completes, `flow_status.step` = 1 and `approval_step = 0` points to the form.
fn flow_value_with_suspend_form() -> serde_json::Value {
    json!({
        "modules": [
            {
                "id": "a",
                "value": {"type": "identity"},
                "suspend": {
                    "required_events": 1,
                    "resume_form": {
                        "schema": {
                            "properties": {
                                "reason": {"type": "string", "description": "Approval reason"}
                            },
                            "order": ["reason"]
                        }
                    }
                }
            },
            {
                "id": "b",
                "value": {"type": "identity"}
            }
        ],
        "same_worker": false
    })
}

/// Build the flow_status JSON for a flow suspended at step 1 (step 0 completed with suspend).
fn flow_status_suspended_at_step_1(step_job_id: Uuid) -> serde_json::Value {
    json!({
        "step": 1,
        "modules": [
            {"type": "Success", "id": "a", "job": step_job_id, "skipped": false},
            {"type": "WaitingForEvents", "id": "b", "count": 1, "job": step_job_id}
        ],
        "failure_module": {
            "parent_module": null,
            "type": "WaitingForPriorSteps",
            "id": "failure"
        },
        "cleanup_module": {"flow_jobs_to_clean": []}
    })
}

/// Insert a v2_job_queue + v2_job_status pair (v2_job_status has FK to v2_job_queue).
async fn insert_queue_and_status(
    db: &Pool<Postgres>,
    flow_id: Uuid,
    flow_status: &serde_json::Value,
) {
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for)
         VALUES ($1, 'test-workspace', now())",
    )
    .bind(flow_id)
    .execute(db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO v2_job_status (id, flow_status)
         VALUES ($1, $2)",
    )
    .bind(flow_id)
    .bind(flow_status)
    .execute(db)
    .await
    .unwrap();
}

/// Insert a flow job with raw_flow stored in v2_job (RawFlow / FlowPreview path).
async fn insert_suspended_flow_with_raw_flow(
    db: &Pool<Postgres>,
    raw_flow: &serde_json::Value,
) -> Uuid {
    let flow_id = Uuid::new_v4();
    let step_job_id = Uuid::new_v4();
    let flow_status = flow_status_suspended_at_step_1(step_job_id);

    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, tag, raw_flow)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'flowpreview', 'flow', $2)",
    )
    .bind(flow_id)
    .bind(raw_flow)
    .execute(db)
    .await
    .unwrap();

    insert_queue_and_status(db, flow_id, &flow_status).await;

    flow_id
}

/// Insert a flow job WITHOUT raw_flow but with a matching flow_node entry (FlowNode path).
async fn insert_suspended_flow_node(db: &Pool<Postgres>, flow_value: &serde_json::Value) -> Uuid {
    let flow_id = Uuid::new_v4();
    let step_job_id = Uuid::new_v4();
    let flow_status = flow_status_suspended_at_step_1(step_job_id);

    // Insert the flow_node entry first to get its id
    let node_id: i64 = sqlx::query_scalar(
        "INSERT INTO flow_node (workspace_id, hash, path, flow)
         VALUES ('test-workspace', 12345, 'f/test/flow', $1)
         RETURNING id",
    )
    .bind(flow_value)
    .fetch_one(db)
    .await
    .unwrap();

    // Insert the job with kind=flownode and runnable_id pointing to the flow_node
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, tag, runnable_id)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'flownode', 'flow', $2)",
    )
    .bind(flow_id)
    .bind(node_id)
    .execute(db)
    .await
    .unwrap();

    insert_queue_and_status(db, flow_id, &flow_status).await;

    flow_id
}

/// Insert a flow job WITHOUT raw_flow but with runnable_id pointing to flow_node,
/// and NO flow_node entry — simulates the broken state before the fix.
async fn insert_suspended_flow_node_without_node_entry(db: &Pool<Postgres>) -> Uuid {
    let flow_id = Uuid::new_v4();
    let step_job_id = Uuid::new_v4();
    let flow_status = flow_status_suspended_at_step_1(step_job_id);

    // Use a non-existent runnable_id — simulates FlowNode with raw_flow=NULL and no fallback
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, tag, runnable_id)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'flownode', 'flow', 99999999)",
    )
    .bind(flow_id)
    .execute(db)
    .await
    .unwrap();

    insert_queue_and_status(db, flow_id, &flow_status).await;

    flow_id
}

async fn get_approval_info_response(
    port: u16,
    db: &Pool<Postgres>,
    job_id: Uuid,
) -> serde_json::Value {
    let token = generate_approval_token("test-workspace", job_id, db)
        .await
        .unwrap();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs_u");
    let resp = client()
        .get(format!("{base}/flow/approval_info/{job_id}?token={token}"))
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap();
    assert!(
        (200..300).contains(&status),
        "approval_info returned {status}: {body}",
    );
    serde_json::from_str(&body).unwrap()
}

/// Test: approval_info returns form_schema for a top-level suspend (raw_flow stored in v2_job).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_approval_info_form_schema_from_raw_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow_value = flow_value_with_suspend_form();
    let flow_id = insert_suspended_flow_with_raw_flow(&db, &flow_value).await;

    let info = get_approval_info_response(port, &db, flow_id).await;
    assert!(
        info.get("form_schema").is_some(),
        "form_schema should be present for raw_flow path, got: {info}",
    );

    Ok(())
}

/// Test: approval_info returns form_schema for a FlowNode sub-flow (graph-based branch/loop).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_approval_info_form_schema_from_flow_node(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // We need a flow path to exist for the flow_node FK
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, versions, value, edited_by, edited_at, schema)
         VALUES ('test-workspace', 'f/test/flow', '', '', '{}', '{}'::jsonb, 'test-user', now(), '{}'::jsonb)",
    )
    .execute(&db)
    .await?;

    let flow_value = flow_value_with_suspend_form();
    let flow_id = insert_suspended_flow_node(&db, &flow_value).await;

    let info = get_approval_info_response(port, &db, flow_id).await;
    assert!(
        info.get("form_schema").is_some(),
        "form_schema should be present for flow_node path, got: {info}",
    );

    Ok(())
}

/// Test: approval_info returns no form_schema when FlowNode has no matching entry
/// (simulates the pre-fix behavior where flow_node fallback didn't exist).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_approval_info_no_form_when_flow_node_missing(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow_id = insert_suspended_flow_node_without_node_entry(&db).await;

    let info = get_approval_info_response(port, &db, flow_id).await;
    assert!(
        info.get("form_schema").is_none(),
        "form_schema should be absent when no flow definition found, got: {info}",
    );

    Ok(())
}
