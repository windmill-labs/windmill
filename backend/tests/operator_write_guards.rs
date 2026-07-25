//! Operators are a read-only + execute-only role, but only the script / flow /
//! app handlers used to enforce that on writes. RLS does not carry
//! `is_operator` (only `is_admin` reaches `set_session_context`) and
//! `check_scopes` is a no-op for a normal session, so an Operator could create
//! and modify groups, folders, variables, resources, resource types and
//! schedules through direct API calls — including overwriting shared `g/all/*`
//! credentials.
//!
//! This pins the guard on every such write route: an Operator is rejected with
//! 401 "Operators cannot ...", a regular member is not caught by it, and the
//! token a job runs with is exempt. The trigger routes share the same guard in
//! `windmill_trigger::handler` but are feature-gated per trigger kind, so they
//! are not exercised here.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{auth::create_jwt_token, db::Authed};
use windmill_test_utils::*;

const GUARD_PREFIX: &str = "Operators cannot";

/// (method, path suffix under /api/w/test-workspace, body)
fn write_routes() -> Vec<(&'static str, String, serde_json::Value)> {
    vec![
        ("POST", "groups/create".into(), json!({"name": "g1"})),
        ("POST", "groups/update/all".into(), json!({"summary": "x"})),
        (
            "POST",
            "groups/adduser/all".into(),
            json!({"username": "test-user-2"}),
        ),
        (
            "POST",
            "groups/removeuser/all".into(),
            json!({"username": "test-user-2"}),
        ),
        ("DELETE", "groups/delete/all".into(), json!(null)),
        ("POST", "folders/create".into(), json!({"name": "f1"})),
        ("POST", "folders/update/f1".into(), json!({"summary": "x"})),
        (
            "POST",
            "folders/addowner/f1".into(),
            json!({"owner": "u/operator-user", "write": true}),
        ),
        (
            "POST",
            "folders/removeowner/f1".into(),
            json!({"owner": "u/test-user"}),
        ),
        ("DELETE", "folders/delete/f1".into(), json!(null)),
        (
            "POST",
            "variables/create".into(),
            json!({"path": "g/all/v", "value": "x", "is_secret": false, "description": ""}),
        ),
        (
            "POST",
            "variables/update/g/all/v".into(),
            json!({"value": "x"}),
        ),
        ("DELETE", "variables/delete/g/all/v".into(), json!(null)),
        (
            "DELETE",
            "variables/delete_bulk".into(),
            json!({"paths": ["g/all/v"]}),
        ),
        (
            "POST",
            "resources/create".into(),
            json!({"path": "g/all/r", "value": {}, "resource_type": "postgresql"}),
        ),
        (
            "POST",
            "resources/update/g/all/r".into(),
            json!({"description": "x"}),
        ),
        (
            "POST",
            "resources/update_value/g/all/r".into(),
            json!({"value": {}}),
        ),
        ("DELETE", "resources/delete/g/all/r".into(), json!(null)),
        (
            "DELETE",
            "resources/delete_bulk".into(),
            json!({"paths": ["g/all/r"]}),
        ),
        (
            "POST",
            "resources/type/create".into(),
            json!({"name": "rt1", "schema": {}}),
        ),
        (
            "POST",
            "resources/type/update/rt1".into(),
            json!({"description": "x"}),
        ),
        ("DELETE", "resources/type/delete/rt1".into(), json!(null)),
        (
            "POST",
            "schedules/create".into(),
            json!({"path": "u/operator-user/s", "schedule": "0 0 * * * *", "timezone": "UTC",
                   "script_path": "u/test-user/noop", "is_flow": false, "args": {}}),
        ),
        (
            "POST",
            "schedules/update/u/test-user/s".into(),
            json!({"schedule": "0 0 * * * *", "timezone": "UTC", "args": {}}),
        ),
        (
            "DELETE",
            "schedules/delete/u/test-user/s".into(),
            json!(null),
        ),
        (
            "POST",
            "schedules/setenabled/u/test-user/s".into(),
            json!({"enabled": false}),
        ),
    ]
}

async fn send(
    port: u16,
    token: &str,
    method: &str,
    path: &str,
    body: &serde_json::Value,
) -> anyhow::Result<(u16, String)> {
    let client = reqwest::Client::new();
    let url = format!("http://localhost:{port}/api/w/test-workspace/{path}");
    let mut req = match method {
        "POST" => client.post(&url),
        _ => client.delete(&url),
    }
    .header("Authorization", format!("Bearer {token}"));
    if !body.is_null() {
        req = req.json(body);
    }
    let resp = req.send().await?;
    let status = resp.status().as_u16();
    Ok((status, resp.text().await?))
}

#[sqlx::test(fixtures("base", "operator_write_guards"))]
async fn operators_cannot_write_workspace_objects(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for (method, path, body) in write_routes() {
        let (status, resp) = send(port, "OPERATOR_TOKEN", method, &path, &body).await?;
        assert_eq!(
            status, 401,
            "{method} {path}: operator must be rejected, got {status}: {resp}"
        );
        assert!(
            resp.contains(GUARD_PREFIX),
            "{method} {path}: rejection must be the operator guard, got: {resp}"
        );

        // The guard must not over-block a regular member: whatever else the
        // request runs into (permissions, missing object), it must not be the
        // operator guard.
        let (_, resp) = send(port, "SECRET_TOKEN_2", method, &path, &body).await?;
        assert!(
            !resp.contains(GUARD_PREFIX),
            "{method} {path}: non-operator must not hit the operator guard, got: {resp}"
        );
    }

    Ok(())
}

/// The token a job runs with carries the operator's `is_operator`, so the guard
/// must let it through — otherwise `wmill.setState` / `setVariable` /
/// `setResource`, which hit these very routes, break for every operator-launched
/// run.
#[sqlx::test(fixtures("base", "operator_write_guards"))]
async fn operator_job_token_can_still_write(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let job_token = create_jwt_token(
        Authed {
            email: "operator@windmill.dev".to_string(),
            username: "operator-user".to_string(),
            is_admin: false,
            is_operator: true,
            groups: vec!["all".to_string()],
            folders: vec![],
            scopes: None,
            token_prefix: None,
        },
        "test-workspace",
        3600,
        Some(uuid::Uuid::new_v4()),
        Some("ephemeral-script".to_string()),
        None,
        None,
    )
    .await?;

    let (status, resp) = send(
        port,
        &job_token,
        "POST",
        "resources/create",
        &json!({"path": "u/operator-user/state", "value": {"n": 1}, "resource_type": "state"}),
    )
    .await?;
    assert_eq!(
        status, 201,
        "operator job token must be able to write its own state, got {status}: {resp}"
    );

    Ok(())
}
