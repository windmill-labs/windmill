//! Regression test for the MCP token-exfiltration vulnerability.
//!
//! `GET /api/w/{w}/resources/mcp_tools/{path}` builds an MCP client from a
//! resource whose `token` field is a `$var:` reference. Before the fix the token
//! was resolved with `get_secret_value_as_admin` on the bare DB pool — no RLS,
//! no audit — so any workspace member who could read an MCP *resource* could
//! point its token at *any* secret variable in the workspace (e.g. one in an
//! admin-only folder) and have it decrypted and shipped as a bearer token.
//!
//! The fix resolves the token through the caller's permissioned path
//! (`get_value_internal` over the authed `user_db`), so the variable RLS — the
//! same gate as `variables/get_value` — applies and the secret read is audited.
//!
//! This test pins, against the `mcp_token_exfil` fixture:
//!   - a plain developer (test-user-3) who can read the MCP resource but has no
//!     access to the locked secret is DENIED (401) at token resolution, before
//!     any connection is attempted, and the secret never leaks;
//!   - an admin (test-user) clears the variable-RLS gate, the token resolves,
//!     and the request only fails later at the connect/SSRF step — proving the
//!     legitimate path still resolves the token (no over-blocking).
//!
//! SSRF rejection of an author-controlled URL is covered by the unit test in
//! `windmill-mcp` (`from_resource_rejects_ssrf_url`).
#![cfg(feature = "mcp")]

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const SECRET_VALUE: &str = "S3CRET-MCP-TOKEN-VALUE";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn get(base: &str, path: &str, token: &str) -> (reqwest::StatusCode, String) {
    let resp = client()
        .get(format!("{base}/{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("request");
    let status = resp.status();
    let body = resp.text().await.expect("body");
    (status, body)
}

#[sqlx::test(fixtures("base", "mcp_token_exfil"))]
async fn test_mcp_token_not_exfiltrated(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Insert the locked secret variable with a real, workspace-key-encrypted
    // value so an authorized read genuinely decrypts it.
    let mc = windmill_common::variables::build_crypt(&db, "test-workspace").await?;
    let encrypted = windmill_common::variables::encrypt(&mc, SECRET_VALUE);
    // Runtime-checked query (not the `query!` macro) so no offline `.sqlx` cache
    // entry is needed for this test-only insert.
    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
         VALUES ('test-workspace', 'f/locked/secret_token', $1, true, 'Locked secret', '{}')",
    )
    .bind(&encrypted)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/resources/mcp_tools");
    let path = "u/test-user-3/evil_mcp";

    // ---- CORE REGRESSION: the developer can read the resource but must NOT be
    //      able to resolve the locked secret. They are denied (401) at the
    //      variable-RLS gate, before any MCP connection is attempted, and the
    //      secret never appears in the response.
    let (status, body) = get(&base, path, "SECRET_TOKEN_3").await;
    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "developer must be denied resolving a secret they can't read (got {status}): {body}"
    );
    assert!(
        !body.contains(SECRET_VALUE),
        "the locked secret must never leak to the developer: {body}"
    );
    assert!(
        body.contains("don't have access"),
        "denial should come from the variable-RLS gate, not a connection error: {body}"
    );
    // Pre-fix, the token was decrypted as admin and the handler proceeded to the
    // connection step; that path must no longer be reached for the developer.
    assert!(
        !body.contains("Failed to connect to MCP server"),
        "developer must be blocked before the connection step (would mean the token was resolved): {body}"
    );

    // ---- NO OVER-BLOCKING: an admin clears the variable-RLS gate, so the token
    //      resolves and the request only fails later at the connect/SSRF step.
    //      A different failure mode (not 401, reaches the connection) proves the
    //      legitimate read still works.
    let (status, body) = get(&base, path, "SECRET_TOKEN").await;
    assert_ne!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "admin must clear the variable-RLS gate (got {status}): {body}"
    );
    assert!(
        body.contains("Failed to connect to MCP server"),
        "admin should resolve the token and only fail at the connect/SSRF step: {body}"
    );

    Ok(())
}
