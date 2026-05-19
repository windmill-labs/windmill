//! Regression test for the app component preview authorization bypass.
//!
//! `POST /api/w/:workspace/apps_u/execute_component/:path` runs in "preview"
//! mode whenever the client supplies `force_viewer_static_fields`. In that
//! mode it accepts request-supplied `raw_code` and enqueues it as a
//! `Viewer`-mode job — i.e. it is the app-editor equivalent of
//! `/jobs/run/preview`. The bug was that this branch did not re-apply the
//! guards `/jobs/run/preview` enforces for arbitrary code execution, so an
//! authenticated Operator (a run-only user who must not be able to create
//! scripts/apps or run preview jobs) could enqueue arbitrary worker code with
//! a single request, escaping the Operator restriction entirely.
//!
//! This test pins down:
//!   - an Operator is rejected from preview mode (the core fix; pre-fix this
//!     enqueued a job and returned 200),
//!   - a regular non-operator member can still run an editor preview (the fix
//!     must not over-block the legitimate editor flow),
//!   - preview is confined to paths the caller can read (defense-in-depth
//!     against scoped tokens / cross-namespace preview), and
//!   - run mode (no `force_viewer_static_fields`) is unaffected by the guard.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// A preview request: `force_viewer_static_fields` present + inline `raw_code`.
/// This is the exact shape an attacker (or the editor) sends.
fn preview_body(app_path: &str) -> serde_json::Value {
    json!({
        "args": {},
        "component": "comp",
        "raw_code": {
            "language": "deno",
            "content": "export function main() { return \"pwned\"; }",
            "path": format!("{}/comp", app_path)
        },
        "force_viewer_static_fields": {}
    })
}

#[sqlx::test(fixtures("base", "app_preview_auth"))]
async fn test_app_preview_authorization(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/apps_u/execute_component");

    // 1. CORE REGRESSION: an Operator sends a preview request in their own
    //    namespace (so the *only* thing that can reject them is the Operator
    //    check itself). Pre-fix this returned 200 with an enqueued job UUID;
    //    post-fix it must be rejected.
    let resp = authed(
        client().post(format!("{base}/u/operator-user/myapp")),
        "OPERATOR_TOKEN",
    )
    .json(&preview_body("u/operator-user/myapp"))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 401,
        "Operator must be rejected from app preview (got {status}): {body}"
    );
    assert!(
        body.contains("Operators cannot run preview jobs"),
        "rejection must be the operator guard, got: {body}"
    );

    // 2. The fix must NOT over-block the legitimate editor flow: a regular
    //    non-operator member previewing in their own namespace still works
    //    (the endpoint returns the enqueued job UUID before any worker runs).
    let resp = authed(
        client().post(format!("{base}/u/test-user-2/myapp")),
        "SECRET_TOKEN_2",
    )
    .json(&preview_body("u/test-user-2/myapp"))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert!(
        status.is_success(),
        "non-operator editor preview must still succeed (got {status}): {body}"
    );
    assert!(
        uuid::Uuid::parse_str(body.trim()).is_ok(),
        "successful preview must return a job UUID, got: {body}"
    );

    // 3. Inline `raw_code` preview is deliberately NOT path-gated: a
    //    non-operator can already run arbitrary inline code via
    //    `/jobs/run/preview`, so the app URL path string is irrelevant for the
    //    inline case. This pins that decision so an over-restrictive path check
    //    is not re-added for inline previews.
    let resp = authed(
        client().post(format!("{base}/u/test-user/secretapp")),
        "SECRET_TOKEN_2",
    )
    .json(&preview_body("u/test-user/secretapp"))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert!(
        status.is_success(),
        "inline raw_code preview must not be path-gated (got {status}): {body}"
    );
    assert!(
        uuid::Uuid::parse_str(body.trim()).is_ok(),
        "inline preview should enqueue a job UUID, got: {body}"
    );

    // 4. Run mode (no `force_viewer_static_fields`) is unaffected by the new
    //    preview guard: an Operator hitting a deployed-app path still follows
    //    the pre-existing policy lookup (here: the app does not exist -> 404),
    //    proving the guard only gates preview mode.
    let resp = authed(
        client().post(format!("{base}/u/operator-user/nonexistent")),
        "OPERATOR_TOKEN",
    )
    .json(&json!({
        "args": {},
        "component": "comp",
        "raw_code": {
            "language": "deno",
            "content": "export function main() { return 1; }",
            "path": "u/operator-user/nonexistent/comp"
        }
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 404,
        "run mode must be unchanged (deployed app lookup -> 404, not the preview guard); got {status}: {body}"
    );

    // 5. Defense-in-depth: the guard must check the *runnable* being previewed,
    //    not just the app URL path. A caller pairs an allowed app path
    //    (`u/test-user-2/myapp`, own namespace) with a `path` pointing at a
    //    deployed runnable in another user's namespace. Without checking the
    //    runnable path this would resolve `script/u/test-user/private` with the
    //    root DB handle and enqueue it; it must be rejected by the path check.
    let resp = authed(
        client().post(format!("{base}/u/test-user-2/myapp")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "args": {},
        "component": "comp",
        "path": "script/u/test-user/private",
        "force_viewer_static_fields": {}
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "preview targeting a runnable outside the caller's namespace must be rejected even with an allowed app path (got {status}): {body}"
    );

    // 6. Defense-in-depth: a persisted inline-script preview selects code by the
    //    caller-controlled `app_script` id. Pairing an allowed app path with an
    //    id owned by another (private) app must be rejected — without the
    //    id-ownership check the worker would fetch and run that app's code.
    let resp = authed(
        client().post(format!("{base}/u/test-user-2/myapp")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "args": {},
        "component": "comp",
        "id": 999777,
        "raw_code": {
            "language": "deno",
            "content": "export function main() { return 1; }",
            "path": "u/test-user-2/myapp/comp"
        },
        "force_viewer_static_fields": {}
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "preview with an app_script id owned by another app must be rejected (got {status}): {body}"
    );

    // 7. The id-ownership check must NOT over-block a legitimate persisted
    //    inline-script preview: an id owned by an app in the caller's own
    //    namespace passes the guard and enqueues (returns a job UUID).
    let resp = authed(
        client().post(format!("{base}/u/test-user-2/ownapp")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "args": {},
        "component": "comp",
        "id": 999778,
        "raw_code": {
            "language": "deno",
            "content": "export function main() { return 1; }",
            "path": "u/test-user-2/ownapp/comp"
        },
        "force_viewer_static_fields": {}
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert!(
        status.is_success(),
        "persisted preview for an app the caller owns must still succeed (got {status}): {body}"
    );
    assert!(
        uuid::Uuid::parse_str(body.trim()).is_ok(),
        "successful persisted preview must return a job UUID, got: {body}"
    );

    // 8. Scope escalation: a token scoped to `apps:run` (but not `jobs:run`)
    //    can reach this route (it maps to the `apps` scope domain) and is not an
    //    Operator, but must NOT be able to enqueue arbitrary preview `raw_code`.
    //    `/jobs/run/preview` requires `jobs:run` for exactly this reason; the
    //    app preview path must enforce the same. Without the `jobs:run` check
    //    this enqueues a job (returns a UUID); with it, it is rejected (403).
    let resp = authed(
        client().post(format!("{base}/u/test-user-2/myapp")),
        "APPS_RUN_TOKEN",
    )
    .json(&preview_body("u/test-user-2/myapp"))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 403,
        "apps:run-scoped token must not escalate to arbitrary preview code (got {status}): {body}"
    );
    assert!(
        body.contains("jobs:run"),
        "rejection must be the jobs:run scope gate, got: {body}"
    );

    Ok(())
}
