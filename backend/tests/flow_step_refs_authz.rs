//! Regression test for the flow step/sub-flow reference ACL bypass (WIN-2412).
//!
//! Step references (`script` by path or hash, `flow` sub-flows) are resolved with a
//! privileged connection when the flow runs, so whoever saves the flow decides which
//! runnable a step reaches. Without a gate at save time, any workspace member could
//! deploy — or preview — a flow whose step points at a folder-protected script and read
//! its output, or inherit its `on_behalf_of` identity, through the flow.
//!
//! Pinned against the `flow_step_refs_authz` fixture (a script and a flow in a folder only
//! the admin can read):
//!   - a plain member is denied on create, update and preview, including when the
//!     reference is nested inside a branch or is the failure module, and the response
//!     names the runnable it refused;
//!   - the admin, who can read it, is not blocked (no over-blocking);
//!   - a member referencing a runnable they *can* read still deploys.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const WS: &str = "test-workspace";
const ADMIN: &str = "SECRET_TOKEN";
const MEMBER: &str = "SECRET_TOKEN_3";

async fn post(
    base: &str,
    path: &str,
    token: &str,
    body: serde_json::Value,
) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/w/{WS}/{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("request");
    let status = resp.status();
    (status, resp.text().await.expect("body"))
}

fn new_flow(path: &str, value: serde_json::Value) -> serde_json::Value {
    json!({ "path": path, "summary": "", "description": "", "value": value, "schema": {} })
}

fn script_step(path: &str) -> serde_json::Value {
    json!({ "modules": [{ "id": "a", "value": { "type": "script", "path": path, "input_transforms": {} } }] })
}

#[sqlx::test(fixtures("base", "flow_step_refs_authz"))]
async fn test_flow_step_refs_require_caller_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}", server.addr.port());

    // ---- CORE REGRESSION: a member cannot deploy a step pointing at a script they
    //      cannot read, wherever in the flow the reference sits.
    for (case, value) in [
        ("top-level script step", script_step("f/private/hidden")),
        (
            "script step nested in a branch",
            json!({ "modules": [{ "id": "a", "value": { "type": "branchall", "branches": [
                { "summary": "", "modules": [{ "id": "b", "value": { "type": "script", "path": "f/private/hidden", "input_transforms": {} } }] }
            ] } }] }),
        ),
        (
            "failure module",
            json!({
                "modules": [],
                "failure_module": { "id": "failure", "value": { "type": "script", "path": "f/private/hidden", "input_transforms": {} } }
            }),
        ),
        (
            "sub-flow step",
            json!({ "modules": [{ "id": "a", "value": { "type": "flow", "path": "f/private/hiddenflow", "input_transforms": {} } }] }),
        ),
    ] {
        let (status, body) = post(
            &base,
            "flows/create",
            MEMBER,
            new_flow("f/shared/attempt", value),
        )
        .await;
        assert_eq!(
            status,
            reqwest::StatusCode::UNAUTHORIZED,
            "member must be denied deploying a {case} they cannot read: {body}"
        );
        assert!(
            body.contains("f/private/hidden"),
            "the denial should name the runnable it refused ({case}): {body}"
        );
    }

    // Preview runs a request-supplied flow value, which never goes through the deploy
    // path — it must be gated on its own.
    let (status, body) = post(
        &base,
        "jobs/run/preview_flow",
        MEMBER,
        json!({ "value": script_step("f/private/hidden"), "path": "f/shared/attempt", "args": {} }),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "member must be denied previewing a step they cannot read: {body}"
    );

    // ---- NO OVER-BLOCKING: a readable reference still deploys, for the member and for
    //      the admin who can read the private folder.
    let (status, body) = post(
        &base,
        "flows/create",
        MEMBER,
        new_flow("f/shared/allowed", script_step("f/shared/visible")),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::CREATED,
        "a member referencing a script they can read must still deploy: {body}"
    );

    let (status, body) = post(
        &base,
        "flows/create",
        ADMIN,
        new_flow("f/private/delegating", script_step("f/private/hidden")),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::CREATED,
        "the admin can read the private script and must not be blocked: {body}"
    );

    // An update re-points an already-deployed flow, so it is gated like a create.
    let (status, body) = post(
        &base,
        "flows/update/f/shared/allowed",
        MEMBER,
        new_flow("f/shared/allowed", script_step("f/private/hidden")),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "member must be denied re-pointing a flow at a script they cannot read: {body}"
    );

    Ok(())
}
