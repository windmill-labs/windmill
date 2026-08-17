//! Regression test for the inline preview authorization bypass (GHSA-pp5h-96x3-3wqq).
//!
//! `POST /api/w/:workspace/jobs/run_inline/preview` -> `run_inline_preview_script`
//! runs request-supplied code inline (in-process via DuckDB), i.e. it is an
//! arbitrary-code-execution sibling of `/jobs/run/preview`. The bug was that
//! this handler was missing the Operator guard that `run_preview_script`
//! enforces, so an authenticated Operator (a run-only user who must not be able
//! to run preview jobs) could execute arbitrary code in a single request. This
//! was the incomplete-fix residual of CVE-2026-22683, whose v1.615.0 patch only
//! covered the entity-CRUD endpoints and left this direct inline-exec sink open.
//!
//! This test pins down:
//!   - an Operator is rejected by the operator guard (the core fix; pre-fix this
//!     reached the inline executor instead of returning 401), and
//!   - a regular non-operator passes the guard (the fix must not over-block the
//!     legitimate inline preview flow): in the test harness the worker inline
//!     utils are not registered, so a caller past the guard gets the distinct
//!     "worker inline functions" error rather than the operator rejection.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// An inline preview request: request-supplied `content` to run via DuckDB.
fn inline_preview_body() -> serde_json::Value {
    json!({
        "language": "duckdb",
        "content": "SELECT content FROM read_text(['/etc/passwd']);",
        "args": {}
    })
}

const OPERATOR_GUARD_MSG: &str = "Operators cannot run preview jobs";

#[sqlx::test(fixtures("base", "inline_preview_auth"))]
async fn test_inline_preview_authorization(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url = format!("http://localhost:{port}/api/w/test-workspace/jobs/run_inline/preview");

    // 1. CORE REGRESSION: an Operator must be rejected by the operator guard.
    //    Pre-fix this fell through to the inline executor (arbitrary code
    //    execution); post-fix it returns 401 with the operator guard message.
    let resp = authed(client().post(&url), "OPERATOR_TOKEN")
        .json(&inline_preview_body())
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 401,
        "Operator must be rejected from inline preview (got {status}): {body}"
    );
    assert!(
        body.contains(OPERATOR_GUARD_MSG),
        "rejection must be the operator guard, got: {body}"
    );

    // 2. The fix must NOT over-block a legitimate non-operator: a regular member
    //    passes the operator + scope checks. The test harness does not register
    //    the worker inline utils, so the request proceeds past the guard and
    //    fails later with the distinct "worker inline functions" error — proving
    //    the operator guard did not reject it.
    let resp = authed(client().post(&url), "SECRET_TOKEN_2")
        .json(&inline_preview_body())
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_ne!(
        status, 401,
        "non-operator must not be blocked by the operator guard (got {status}): {body}"
    );
    assert!(
        !body.contains(OPERATOR_GUARD_MSG),
        "non-operator must not hit the operator guard, got: {body}"
    );

    Ok(())
}
