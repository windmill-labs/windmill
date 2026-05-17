//! Regression test for GHSA-vxc5-w28p-m9xw.
//!
//! The workspace job preview endpoints (`/jobs/run/preview`, `/jobs/run/preview_flow`,
//! `/jobs/run/preview_bundle`, `/jobs/run/dynamic_select`) run arbitrary,
//! request-supplied code. They only called `require_path_read_access_for_preview`
//! (a folder/namespace *read* check that is a no-op when `path` is null), so a token
//! scoped to a single deployed script/flow could escape its scope and execute
//! arbitrary code. The fix adds an explicit `jobs:run` scope check to those handlers.

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

async fn insert_scoped_token(db: &Pool<Postgres>, token: &str, scopes: &[&str]) {
    // Mirrors the token rows created by the `base` fixture: non-super-admin token
    // belonging to the non-admin user `test2@windmill.dev` of `test-workspace`.
    let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
    sqlx::query(
        "INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin, scopes) \
         VALUES (encode(sha256($1::bytea), 'hex'), 'SECRET_TOK', $1, 'test2@windmill.dev', \
         'scoped token', false, $2)",
    )
    .bind(token)
    .bind(&scopes)
    .execute(db)
    .await
    .expect("insert scoped token");
}

/// A token scoped to a specific deployed script must NOT be able to run arbitrary
/// preview code. The scope passes the route-level middleware (route kind = scripts),
/// so a 403 here proves the in-handler `check_scopes("jobs:run")` is doing its job.
#[sqlx::test(fixtures("base"))]
async fn scoped_token_cannot_run_arbitrary_preview(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    insert_scoped_token(
        &db,
        "POC_SCOPED_TOKEN",
        &["jobs:run:scripts:f/locked/onlythis"],
    )
    .await;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "POC_SCOPED_TOKEN".to_string(),
    );

    let resp = client
        .client()
        .post(format!(
            "{}/w/test-workspace/jobs/run/preview",
            client.baseurl()
        ))
        .json(&json!({
            "content": "export async function main() { return \"pwned\"; }",
            "language": "deno",
            "path": null,
            "args": {}
        }))
        .send()
        .await?;

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "a script-scoped token must be denied (403) on /jobs/run/preview, got {}",
        resp.status()
    );
    let body = resp.text().await?;
    assert!(
        body.contains("jobs:run"),
        "403 should reference the required jobs:run scope, body: {body}"
    );

    Ok(())
}

/// A token granted the broad `jobs:run` scope (no resource kind) must still be
/// allowed through the scope check — the fix must not over-block legitimate
/// broadly-authorized tokens.
#[sqlx::test(fixtures("base"))]
async fn broad_jobs_run_token_not_blocked_on_preview(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    insert_scoped_token(&db, "POC_BROAD_TOKEN", &["jobs:run"]).await;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "POC_BROAD_TOKEN".to_string(),
    );

    let resp = client
        .client()
        .post(format!(
            "{}/w/test-workspace/jobs/run/preview",
            client.baseurl()
        ))
        .json(&json!({
            "content": "export async function main() { return 1; }",
            "language": "deno",
            "path": null,
            "args": {}
        }))
        .send()
        .await?;

    assert_ne!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "a token with the broad jobs:run scope must not be blocked on /jobs/run/preview"
    );

    Ok(())
}
