use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
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
