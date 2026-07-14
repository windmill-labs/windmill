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

    // 4f. Cannot read the full user directory. This route is only guarded by
    //     `require_super_admin` (no per-route job-token denylist), so it proves
    //     the cap is baked into `require_super_admin` itself — the advisory PoC.
    let resp = authed(client().get(format!("{base}/list_as_super_admin")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not list all users: {}",
        resp.text().await?
    );

    // 4g. Cannot read instance settings (unredacted SMTP/OAuth secrets, license
    //     key, SSO config). Also guarded only by `require_super_admin`.
    let settings_base = format!("http://localhost:{port}/api/settings");
    let resp = authed(
        client().get(format!("{settings_base}/global/license_key")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not read instance settings: {}",
        resp.text().await?
    );

    // 4h. No false positive: a real superadmin API token still reads the
    //     directory (the cap keys off the job token, not the identity).
    let resp = authed(
        client().get(format!("{base}/list_as_super_admin")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real superadmin token must still list users: {}",
        resp.text().await?
    );

    // 4i. The instance-level `devops` role is also capped: a superadmin (hence
    //     devops) WM_TOKEN must not reach a require_devops_role route.
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/service_logs/list_files"
        )),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not reach a devops route: {}",
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
