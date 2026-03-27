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

fn assert_route_reachable(status: u16, body: &str, endpoint: &str) {
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for {endpoint}",
    );
}

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

async fn create_script(port: u16) -> String {
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/test_job_script",
            "summary": "test",
            "description": "",
            "content": "export function main() { return 42; }",
            "language": "deno",
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "create script: {}",
        resp.status()
    );
    "u/test-user/test_job_script".to_string()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_list_and_count(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // --- List/count endpoints (2xx with empty results) ---

    let resp = authed(client().get(format!("{base}/list"))).send().await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/list",
    );

    let resp = authed(client().get(format!("{base}/queue/list")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/queue/list",
    );

    let resp = authed(client().get(format!("{base}/queue/count")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/queue/count",
    );

    let resp = authed(client().get(format!("{base}/completed/list")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/list",
    );

    let resp = authed(client().get(format!("{base}/completed/count")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/count",
    );

    // --- Global endpoints ---

    let resp = client()
        .get(format!("http://localhost:{port}/api/jobs/db_clock"))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/db_clock",
    );

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/jobs/completed/count_by_tag"
    )))
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/count_by_tag",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_completed_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    let job_id = insert_completed_job(&db).await;

    let resp = authed(client().get(format!("{base}/completed/get/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get_result",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result_maybe/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get_result_maybe",
    );

    let resp = authed(client().get(format!("{base}/completed/get_timing/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get_timing",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_run_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // Run preview — no pre-existing script needed
    let resp = authed(client().post(format!("{base}/run/preview")))
        .json(&json!({
            "content": "export function main() { return 1; }",
            "language": "deno",
            "args": {}
        }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/preview",
    );

    // Run preview flow
    let resp = authed(client().post(format!("{base}/run/preview_flow")))
        .json(&json!({
            "value": {"modules": []},
            "args": {}
        }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/preview_flow",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");
    let fake = Uuid::nil();

    // These need complex runtime but should hit the handler (not 404)

    let resp = authed(client().post(format!("{base}/flow/resume/{fake}")))
        .json(&json!({}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/flow/resume",
    );

    let resp = authed(client().get(format!("{base}/job_signature/{fake}/1")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/job_signature",
    );

    let resp = authed(client().get(format!("{base}/resume_urls/{fake}/1")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/resume_urls",
    );

    let resp = authed(client().get(format!("{base}/result_by_id/{fake}/step1")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/result_by_id",
    );

    let resp = authed(client().post(format!("{base}/restart/f/{fake}")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/restart/f",
    );

    let resp = authed(client().post(format!("{base}/run/workflow_as_code/{fake}/main")))
        .json(&json!({}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/workflow_as_code",
    );

    Ok(())
}
