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

#[allow(dead_code)]
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

/// Resolving is authorized by row-level security on `v2_job` alone: `v2_job_completed`
/// has RLS disabled, so a regression here silently lets any member annotate any run.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_resolve_completed_jobs_scoping(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    async fn seed(db: &Pool<Postgres>, owner: &str, status: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
                                 kind, tag, runnable_path, visible_to_owner)
             VALUES ($1, 'test-workspace', $2, $3, $4, 'script', 'deno', $5, true)",
        )
        .bind(id)
        .bind(owner)
        .bind(format!("u/{owner}"))
        .bind(format!("{owner}@windmill.dev"))
        .bind(format!("u/{owner}/some_script"))
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
             VALUES ($1, 'test-workspace', 100, '42'::jsonb, $2::job_status)",
        )
        .bind(id)
        .bind(status)
        .execute(db)
        .await
        .unwrap();
        id
    }

    let mine = seed(&db, "test-user-2", "failure").await;
    let theirs = seed(&db, "test-user", "failure").await;
    let mine_succeeded = seed(&db, "test-user-2", "success").await;

    // test-user-2 is a plain non-admin, non-operator member of the workspace.
    let member = |b: reqwest::RequestBuilder| b.header("Authorization", "Bearer SECRET_TOKEN_2");

    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine, theirs, mine_succeeded], "note": "expected" }))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /jobs/completed/resolve");
    let resolved: Vec<Uuid> = serde_json::from_str(&body)?;
    assert_eq!(
        resolved,
        vec![mine],
        "only the caller's own failed run may be resolved"
    );

    let row: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT c.status::text, r.resolved_by, r.note
           FROM v2_job_completed c JOIN job_resolution r ON r.job_id = c.id
          WHERE c.id = $1",
    )
    .bind(mine)
    .fetch_one(&db)
    .await?;
    assert_eq!(row.0, "failure", "resolving must not change job status");
    assert_eq!(row.1.as_deref(), Some("test-user-2"));
    assert_eq!(row.2.as_deref(), Some("expected"));

    let resp = member(client().post(format!("{base}/completed/unresolve")))
        .json(&json!({ "job_ids": [mine, theirs] }))
        .send()
        .await?;
    let body = resp.text().await?;
    let unresolved: Vec<Uuid> = serde_json::from_str(&body)?;
    assert_eq!(unresolved, vec![mine]);

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM job_resolution")
        .fetch_one(&db)
        .await?;
    assert_eq!(remaining, 0);

    Ok(())
}
