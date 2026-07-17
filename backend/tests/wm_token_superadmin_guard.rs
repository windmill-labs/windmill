//! A WM_TOKEN (job JWT) running as a superadmin must not be able to perform
//! global user/token management — promotion, password reset, user creation,
//! token creation/impersonation, offboarding, or exporting the user table.
//! A non-admin `wm_deployers` member can mint
//! such a token implicitly via an app/flow `on_behalf_of`, so trusting it would
//! let them establish *persistent* superadmin. A real superadmin who needs this
//! from a script must use a dedicated superadmin API token (which only a real
//! superadmin can create), not `$WM_TOKEN`.
//!
//! The fixture provides `test@windmill.dev` (instance superadmin, token
//! `SECRET_TOKEN`) and `test2@windmill.dev` (non-superadmin, `SECRET_TOKEN_2`).

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

/// Mint a WM_TOKEN: an internally-signed job JWT (note the `job_id` claim) for
/// `email`, exactly as a running app/flow job is issued.
async fn wm_token(email: &str, is_admin: bool) -> String {
    let authed = Authed {
        email: email.to_string(),
        username: "runner".to_string(),
        is_admin,
        is_operator: false,
        groups: vec![],
        folders: vec![],
        scopes: None,
        token_prefix: None,
    };
    create_jwt_token(
        authed,
        "test-workspace",
        3600,
        Some(uuid::Uuid::new_v4()),
        Some("app".to_string()),
        None,
        None,
    )
    .await
    .expect("mint wm_token")
}

#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_cannot_manage_superadmin_users(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // The server decodes WM_TOKENs with the same in-process JWT secret, so
    // setting it once lets us mint a valid one below.
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");

    // A superadmin-capable WM_TOKEN — the exact thing a deployer obtains via an
    // app on_behalf_of pointed at a superadmin.
    let sa_wm = wm_token("test@windmill.dev", true).await;

    // 1. Cannot mint a (superadmin) token.
    let resp = authed(client().post(format!("{base}/tokens/create")), &sa_wm)
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not create tokens: {}",
        resp.text().await?
    );

    // 2. Cannot impersonate (mint a token as another user).
    let resp = authed(client().post(format!("{base}/tokens/impersonate")), &sa_wm)
        .json(&json!({ "impersonate_email": "test2@windmill.dev" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not impersonate: {}",
        resp.text().await?
    );

    // 3. Cannot promote a user to superadmin.
    let resp = authed(
        client().post(format!("{base}/update/test2@windmill.dev")),
        &sa_wm,
    )
    .json(&json!({ "is_super_admin": true }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not promote users: {}",
        resp.text().await?
    );

    // 4. Cannot reset its own (the superadmin's) password.
    let resp = authed(client().post(format!("{base}/setpassword")), &sa_wm)
        .json(&json!({ "password": "hunter2" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not reset passwords: {}",
        resp.text().await?
    );

    // 4b. Cannot delete a user.
    let resp = authed(
        client().delete(format!("{base}/delete/test2@windmill.dev")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not delete users: {}",
        resp.text().await?
    );

    // 4c. Cannot change a user's login type.
    let resp = authed(
        client().post(format!("{base}/set_login_type/test2@windmill.dev")),
        &sa_wm,
    )
    .json(&json!({ "login_type": "password" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not change login type: {}",
        resp.text().await?
    );

    // 4d. Cannot offboard a global user (deletes user, tokens, password, invites,
    //     instance-group membership and reassigns their assets).
    let resp = authed(
        client().post(format!("{base}/offboard/test2@windmill.dev")),
        &sa_wm,
    )
    .json(&json!({}))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not offboard users: {}",
        resp.text().await?
    );

    // 4e. Cannot export the global user table (leaks every user's password_hash).
    let resp = authed(client().get(format!("{base}/export")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not export global users: {}",
        resp.text().await?
    );

    // 5. Escape hatch / no false positive: a real superadmin API token
    //    (SECRET_TOKEN, no job_id) can still create tokens.
    let resp = authed(
        client().post(format!("{base}/tokens/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "label": "ci" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "a real superadmin token must still create tokens: {}",
        resp.text().await?
    );

    // 6. No collateral: a non-superadmin WM_TOKEN can still create its own
    //    token — the guard only fires for superadmin-capable job tokens.
    let user_wm = wm_token("test2@windmill.dev", false).await;
    let resp = authed(client().post(format!("{base}/tokens/create")), &user_wm)
        .json(&json!({ "label": "from-script" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "non-superadmin WM_TOKEN must still create its own token: {}",
        resp.text().await?
    );

    Ok(())
}

/// A WM_TOKEN running as a superadmin must be rejected by *any* `require_super_admin`
/// route, not just the handful that call `forbid_superadmin_job_token`. `GET
/// /api/settings/list_global` is gated solely by `require_super_admin`, so it
/// exercises the token-layer guard (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_rejected_by_require_super_admin(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    // The exact token a deployer obtains via an app on_behalf_of pointed at a superadmin.
    let sa_wm = wm_token("test@windmill.dev", true).await;
    let resp = authed(client().get(format!("{base}/settings/list_global")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not reach a require_super_admin route: {}",
        resp.text().await?
    );

    // No false positive: a real superadmin API token (no job_id) still reaches it.
    let resp = authed(
        client().get(format!("{base}/settings/list_global")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real superadmin token must still reach the route: {}",
        resp.text().await?
    );

    Ok(())
}

/// Direct `is_super_admin_email` authorization gates (not routed through
/// `require_super_admin`) must also reject a superadmin `WM_TOKEN`. Covers the two
/// bypass classes the CI review flagged: destructive `delete_workspace`, and the
/// `CUSTOM_INSTANCE_DB` credential lookup whose guard must read the *authenticated*
/// `job_id`, not the caller-supplied `?job_id` query param (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_rejected_by_direct_super_admin_gates(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    let sa_wm = wm_token("test@windmill.dev", true).await;

    // 1. Global workspace deletion (destructive) — must be forbidden.
    let resp = authed(
        client().delete(format!("{base}/workspaces/delete/test-workspace")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not delete a workspace: {}",
        resp.text().await?
    );
    // The workspace must still exist.
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = 'test-workspace')")
            .fetch_one(&db)
            .await?;
    assert!(
        exists,
        "rejected delete must not have removed the workspace"
    );

    // 2. CUSTOM_INSTANCE_DB credential lookup, WITHOUT the ?job_id query param —
    //    the guard must reject based on the authenticated token's job_id.
    let resp = authed(
        client().get(format!(
            "{base}/w/test-workspace/resources/get_value_interpolated/CUSTOM_INSTANCE_DB/anydb"
        )),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not resolve CUSTOM_INSTANCE_DB (no creds leak): {}",
        resp.text().await?
    );

    Ok(())
}

/// The instance-level `devops` role must be capped like superadmin.
/// `is_devops_email` returns true for superadmin emails, so every
/// `require_devops_role` route (worker management, instance config, service logs)
/// is reachable by exactly the same superadmin `WM_TOKEN` unless it is capped too
/// (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_rejected_by_require_devops_role(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    let sa_wm = wm_token("test@windmill.dev", true).await;
    let resp = authed(client().get(format!("{base}/service_logs/list_files")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not reach a require_devops_role route: {}",
        resp.text().await?
    );

    // No false positive: a real superadmin API token (no job_id) still reaches it.
    let resp = authed(
        client().get(format!("{base}/service_logs/list_files")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real superadmin token must still reach the devops route: {}",
        resp.text().await?
    );

    // The advisory's own PoC route: the full user directory, gated solely by
    // `require_super_admin` with no per-route job-token denylist.
    let resp = authed(
        client().get(format!("{base}/users/list_as_super_admin")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not list all users: {}",
        resp.text().await?
    );

    Ok(())
}

/// A job token must not clear an *admin-or-devops* gate via the devops branch.
/// `require_admin_or_devops` (the EE critical-alerts endpoints) grants when the
/// caller is a workspace admin OR an instance `devops`; since `is_devops_email`
/// is true for superadmins, a WM_TOKEN running on-behalf of a superadmin who is
/// NOT a member of the target workspace would otherwise gain workspace-scoped
/// devops access to a workspace it has no admin rights in (GHSA-hfh4-cx4h-3fcr).
/// The workspace-admin branch stays allowed — that is the cap ceiling.
#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_rejected_by_admin_or_devops_gate(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    // superadmin-external is a superadmin but not a member of test-workspace, so
    // its workspace-level is_admin is false — the exact exploit precondition.
    let sa_wm = wm_token("superadmin-external@windmill.dev", false).await;
    let resp = authed(client().get(format!("{base}/critical_alerts")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not clear the admin-or-devops gate on a workspace it isn't admin of: {}",
        resp.text().await?
    );

    // No false positive: the same superadmin's real API token (not a job token)
    // still clears the gate via the devops branch.
    let resp = authed(
        client().get(format!("{base}/critical_alerts")),
        "EXTERNAL_SUPERADMIN_TOKEN",
    )
    .send()
    .await?;
    assert_ne!(
        resp.status(),
        403,
        "a real superadmin token must still clear the admin-or-devops gate: {}",
        resp.text().await?
    );

    Ok(())
}
