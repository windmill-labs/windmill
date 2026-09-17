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
//! The guard on both routes has one exemption: `wmill.datatable()` called from
//! inside a job the operator is running. Operators can only run deployed code,
//! so a request the job's WM_TOKEN authenticates comes from code a non-operator
//! authored, and the exemption is limited to the request shape the helper sends
//! (PostgreSQL against a `datatable://` database) so a leaked WM_TOKEN cannot
//! be replayed to run anything else.
//!
//! This test pins down:
//!   - an Operator's own token is rejected by the operator guard (the core fix;
//!     pre-fix this reached the inline executor instead of returning 401),
//!   - a regular non-operator passes the guard (the fix must not over-block the
//!     legitimate inline preview flow): in the test harness the worker inline
//!     utils are not registered, so a caller past the guard gets the distinct
//!     "worker inline functions" error rather than the operator rejection,
//!   - an Operator's job token passes the guard for a datatable query while its
//!     job is running, on the inline route and on the `/jobs/run/preview`
//!     fallback the SDKs use when the worker has no internal server,
//!   - the same token is rejected for any other payload (in-process DuckDB, or a
//!     `-- database` directive redirecting the query, whether written literally or
//!     reached through a `WM_INTERNAL_DB` marker) and for a deferred run,
//!   - an Operator's job token for a job that is not running, whether finished or
//!     merely queued, is rejected.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::auth::create_jwt_token;
use windmill_common::db::Authed;
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

/// The request `wmill.datatable("main")` sends: PostgreSQL against `datatable://main`.
fn datatable_query_body() -> serde_json::Value {
    json!({
        "language": "postgresql",
        "content": "SELECT 1 AS x;",
        "args": { "database": "datatable://main" }
    })
}

/// Mint the WM_TOKEN a job hands its own code: an internally-signed job JWT
/// (note the `job_id` claim) for the fixture's operator, exactly as the worker
/// issues it when the operator runs a deployed script.
async fn operator_job_token(job_id: uuid::Uuid) -> String {
    let authed = Authed {
        email: "operator@windmill.dev".to_string(),
        username: "operator-user".to_string(),
        is_admin: false,
        is_operator: true,
        groups: vec![],
        folders: vec![],
        scopes: None,
        token_prefix: None,
    };
    create_jwt_token(
        authed,
        "test-workspace",
        3600,
        Some(job_id),
        Some("ephemeral-script".to_string()),
        None,
        None,
    )
    .await
    .expect("mint operator job token")
}

const OPERATOR_GUARD_MSG: &str = "Operators cannot run preview jobs";

/// The fixture's deployed-script jobs of the operator: one running, one queued.
const RUNNING_JOB_ID: &str = "2aa0c0de-0000-4000-8000-000000000001";
const QUEUED_JOB_ID: &str = "2aa0c0de-0000-4000-8000-000000000002";

async fn post(url: &str, token: &str, body: &serde_json::Value) -> (u16, String) {
    let resp = authed(client().post(url), token)
        .json(body)
        .send()
        .await
        .expect("request");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body");
    (status, body)
}

#[sqlx::test(fixtures("base", "inline_preview_auth"))]
async fn test_inline_preview_authorization(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // The server decodes WM_TOKENs with the same in-process JWT secret, so
    // setting it once lets us mint valid ones below.
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url = format!("http://localhost:{port}/api/w/test-workspace/jobs/run_inline/preview");

    // 1. CORE REGRESSION: an Operator must be rejected by the operator guard.
    //    Pre-fix this fell through to the inline executor (arbitrary code
    //    execution); post-fix it returns 401 with the operator guard message.
    let (status, body) = post(&url, "OPERATOR_TOKEN", &inline_preview_body()).await;
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
    let (status, body) = post(&url, "SECRET_TOKEN_2", &inline_preview_body()).await;
    assert_ne!(
        status, 401,
        "non-operator must not be blocked by the operator guard (got {status}): {body}"
    );
    assert!(
        !body.contains(OPERATOR_GUARD_MSG),
        "non-operator must not hit the operator guard, got: {body}"
    );

    // 3. The WM_TOKEN of a deployed-script job the Operator is running passes the
    //    guard for a datatable query: this is `wmill.datatable()` called from
    //    inside that job. As in 2, the harness then fails with the "worker inline
    //    functions" error.
    let running_job_token =
        operator_job_token(uuid::Uuid::parse_str(RUNNING_JOB_ID).unwrap()).await;
    let (status, body) = post(&url, &running_job_token, &datatable_query_body()).await;
    assert_ne!(
        status, 401,
        "operator job token of a running job must pass the guard for a datatable query (got {status}): {body}"
    );
    assert!(
        !body.contains(OPERATOR_GUARD_MSG),
        "operator job token of a running job must not hit the operator guard, got: {body}"
    );

    // 4. The same token is rejected for any other payload: the exemption covers
    //    the datatable request shape only, never in-process DuckDB, and never a
    //    `-- database` directive, which the executor honors over `args.database`.
    let mut redirected = datatable_query_body();
    redirected["content"] = json!("-- database u/test-user/other_db\nSELECT 1 AS x;");
    let mut to_s3 = datatable_query_body();
    to_s3["content"] = json!("-- s3\nSELECT 1 AS x;");
    let mut resource_db = datatable_query_body();
    resource_db["args"]["database"] = json!("$res:u/test-user/other_db");
    // A marker is a single line the directive regexes cannot match; the directive only
    // appears once the executor expands it, so the guard must check the expansion.
    let mut marker = datatable_query_body();
    marker["content"] = json!(concat!(
        r#"-- WM_INTERNAL_DB_SELECT {"table":"t","columnDefs":[{"field":"id","datatype":"int4"}],"#,
        r#""whereClause":"true\n-- database u/test-user/other_db\n AND true"}"#
    ));
    for (label, payload) in [
        ("DuckDB", inline_preview_body()),
        ("database directive", redirected),
        ("s3 directive", to_s3),
        ("resource database", resource_db),
        ("marker-expanded database directive", marker),
    ] {
        let (status, body) = post(&url, &running_job_token, &payload).await;
        assert_eq!(
            status, 401,
            "operator job token must be rejected for a {label} payload (got {status}): {body}"
        );
        assert!(
            body.contains(OPERATOR_GUARD_MSG),
            "rejection for a {label} payload must be the operator guard, got: {body}"
        );
    }

    // 5. An Operator's job token whose job is not running is rejected like the
    //    operator's own token, whether the job is over (no queue row) or merely
    //    queued: a WM_TOKEN that leaked through logs cannot be replayed once the
    //    job is over.
    for (label, job_id) in [
        ("finished", uuid::Uuid::new_v4()),
        ("queued", uuid::Uuid::parse_str(QUEUED_JOB_ID).unwrap()),
    ] {
        let token = operator_job_token(job_id).await;
        let (status, body) = post(&url, &token, &datatable_query_body()).await;
        assert_eq!(
            status, 401,
            "operator job token of a {label} job must be rejected (got {status}): {body}"
        );
        assert!(
            body.contains(OPERATOR_GUARD_MSG),
            "rejection for a {label} job must be the operator guard, got: {body}"
        );
    }

    // 6. The SDKs fall back to `/jobs/run/preview` when the worker has no internal
    //    server (agent workers). The same exemption applies there: the running
    //    job's token queues the datatable query (201 with the job id), the
    //    operator's own token is still refused.
    let fallback_url = format!("http://localhost:{port}/api/w/test-workspace/jobs/run/preview");
    let (status, body) = post(&fallback_url, &running_job_token, &datatable_query_body()).await;
    assert_eq!(
        status, 201,
        "operator job token of a running job must queue a datatable preview (got {status}): {body}"
    );
    let (status, body) = post(&fallback_url, "OPERATOR_TOKEN", &datatable_query_body()).await;
    assert_eq!(
        status, 401,
        "Operator must be rejected from the preview fallback (got {status}): {body}"
    );
    assert!(
        body.contains(OPERATOR_GUARD_MSG),
        "rejection must be the operator guard, got: {body}"
    );

    // 7. A deferred run on the fallback would outlive the running job the
    //    exemption keys off, so the running job's token cannot schedule one.
    for deferral in [
        "scheduled_in_secs=86400",
        "scheduled_for=2099-01-01T00:00:00Z",
    ] {
        let deferred_url = format!("{fallback_url}?{deferral}");
        let (status, body) = post(&deferred_url, &running_job_token, &datatable_query_body()).await;
        assert_eq!(
            status, 401,
            "operator job token must not schedule a deferred preview with {deferral} (got {status}): {body}"
        );
        assert!(
            body.contains(OPERATOR_GUARD_MSG),
            "rejection for {deferral} must be the operator guard, got: {body}"
        );
    }

    Ok(())
}
