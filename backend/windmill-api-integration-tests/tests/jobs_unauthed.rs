use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

/// Assert that a response is NOT a router-level 404.
/// Router-level 404 has empty body (from static asset fallback).
/// Handler-level 404/500 has a non-empty body, proving the route was matched.
fn assert_route_matched(status: u16, body: &str, endpoint: &str) {
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 (empty body) for {} -- route pattern not matched",
        endpoint,
    );
}

const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000000";
const FAKE_SECRET: &str = "aabb";

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_unauthed_routes_no_auth(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs_u");

    // --- Suspended job endpoints (Option<ApiAuthed>) ---

    let resp = client()
        .get(format!("{base}/resume/{FAKE_UUID}/1/{FAKE_SECRET}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /resume");

    let resp = client()
        .post(format!("{base}/resume/{FAKE_UUID}/1/{FAKE_SECRET}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "POST /resume");

    let resp = client()
        .get(format!("{base}/cancel/{FAKE_UUID}/1/{FAKE_SECRET}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /cancel");

    let resp = client()
        .post(format!("{base}/cancel/{FAKE_UUID}/1/{FAKE_SECRET}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "POST /cancel");

    let resp = client()
        .get(format!("{base}/get_flow/{FAKE_UUID}/1/{FAKE_SECRET}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /get_flow");

    // --- Job query endpoints (OptAuthed layer) ---

    let resp = client()
        .get(format!("{base}/get_root_job_id/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_root_job_id",
    );

    let resp = client()
        .get(format!("{base}/get/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /get");

    let resp = client()
        .get(format!("{base}/get_logs/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /get_logs");

    let resp = client()
        .get(format!("{base}/get_completed_logs_tail/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_completed_logs_tail",
    );

    let resp = client()
        .get(format!("{base}/get_args/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /get_args");

    let resp = client()
        .post(format!("{base}/queue/get_started_at_by_ids"))
        .json(&json!([FAKE_UUID]))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/get_started_at_by_ids",
    );

    let resp = client()
        .get(format!("{base}/get_flow_debug_info/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_flow_debug_info",
    );

    // --- Completed job endpoints ---

    let resp = client()
        .get(format!("{base}/completed/get/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get",
    );

    let resp = client()
        .get(format!("{base}/completed/get_result/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_result",
    );

    let resp = client()
        .get(format!("{base}/completed/get_result_maybe/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_result_maybe",
    );

    let resp = client()
        .get(format!("{base}/completed/get_timing/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_timing",
    );

    // --- Update/SSE endpoints ---

    let resp = client()
        .get(format!("{base}/getupdate/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /getupdate",
    );

    // --- Log file (wildcard path) ---

    let resp = client()
        .get(format!("{base}/get_log_file/{FAKE_UUID}/test.txt"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_log_file",
    );

    // --- Queue operations ---

    let resp = client()
        .post(format!("{base}/queue/cancel/{FAKE_UUID}"))
        .json(&json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/cancel",
    );

    let resp = client()
        .post(format!(
            "{base}/queue/cancel_persistent/u/test-user/fake_script"
        ))
        .json(&json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/cancel_persistent",
    );

    let resp = client()
        .post(format!("{base}/queue/force_cancel/{FAKE_UUID}"))
        .json(&json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/force_cancel",
    );

    // --- Flow operations ---

    let resp = client()
        .post(format!("{base}/flow/resume_suspended/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /flow/resume_suspended",
    );

    let resp = client()
        .get(format!("{base}/flow/approval_info/{FAKE_UUID}"))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /flow/approval_info",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_unauthed_routes_with_auth(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs_u");

    // Verify Option<ApiAuthed> works in the Some case too

    let resp = authed(client().get(format!("{base}/resume/{FAKE_UUID}/1/{FAKE_SECRET}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /resume (authed)",
    );

    let resp = authed(client().post(format!("{base}/cancel/{FAKE_UUID}/1/{FAKE_SECRET}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /cancel (authed)",
    );

    let resp = authed(client().get(format!("{base}/get_flow/{FAKE_UUID}/1/{FAKE_SECRET}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_flow (authed)",
    );

    let resp = authed(client().get(format!("{base}/get/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get (authed)",
    );

    let resp = authed(client().get(format!("{base}/completed/get/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get (authed)",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result_maybe/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /completed/get_result_maybe (authed)",
    );

    let resp = authed(client().post(format!("{base}/queue/cancel/{FAKE_UUID}")))
        .json(&json!({"reason": "test"}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /queue/cancel (authed)",
    );

    let resp = authed(client().get(format!("{base}/flow/approval_info/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /flow/approval_info (authed)",
    );

    Ok(())
}
