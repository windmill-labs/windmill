//! A WM_TOKEN (job JWT) is minted for one job in one workspace and carries that
//! job's full user privileges. Routes that name no workspace are instance-wide,
//! so it must not reach them: doing so would trade an ephemeral, workspace-bound
//! credential for a permanent one (`tokens/create` mints a workspace-less API
//! token that never expires), for instance configuration, or for global user
//! management.
//!
//! A non-admin `wm_deployers` member can mint such a token implicitly via an
//! app/flow `on_behalf_of`, so the identity it carries need not be their own. A
//! real superadmin who needs a global endpoint from a script must use a
//! dedicated API token (which only a real superadmin can create), not
//! `$WM_TOKEN`.
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
async fn test_wm_token_is_confined_to_its_workspace(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // The server decodes WM_TOKENs with the same in-process JWT secret, so
    // setting it once lets us mint a valid one below.
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api");
    let base = format!("{api}/users");

    // A superadmin-capable WM_TOKEN — the exact thing a deployer obtains via an
    // app on_behalf_of pointed at a superadmin.
    let sa_wm = wm_token("test@windmill.dev", true).await;
    // ...and one for a plain user: neither may leave its workspace.
    let user_wm = wm_token("test2@windmill.dev", false).await;

    // 1. Cannot mint a (superadmin) token.
    let resp = authed(client().post(format!("{base}/tokens/create")), &sa_wm)
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
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
        403,
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
        403,
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
        403,
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
        403,
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
        403,
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
        403,
        "superadmin WM_TOKEN must not offboard users: {}",
        resp.text().await?
    );

    // 4e. Cannot export the global user table (leaks every user's password_hash).
    let resp = authed(client().get(format!("{base}/export")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not export global users: {}",
        resp.text().await?
    );

    // 5. Not only user management: any workspace-less route is out of reach,
    //    including one open to every authenticated user.
    let resp = authed(client().get(format!("{api}/workers/list")), &user_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "WM_TOKEN must not enumerate instance workers: {}",
        resp.text().await?
    );

    // 6. A plain user's WM_TOKEN cannot mint itself a permanent, workspace-less
    //    token either — the confinement does not depend on being a superadmin.
    let resp = authed(client().post(format!("{base}/tokens/create")), &user_wm)
        .json(&json!({ "label": "from-script" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "WM_TOKEN must not create tokens: {}",
        resp.text().await?
    );

    // 7. Escape hatch / no false positive: a real API token (SECRET_TOKEN, no
    //    job_id) still reaches both.
    let resp = authed(client().post(format!("{base}/tokens/create")), "SECRET_TOKEN")
        .json(&json!({ "label": "ci" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "a real superadmin token must still create tokens: {}",
        resp.text().await?
    );
    let resp = authed(client().get(format!("{api}/workers/list")), "SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real token must still list workers: {}",
        resp.text().await?
    );

    // 8. No collateral on the routes a job legitimately needs: its own workspace,
    //    and the workspace-less endpoint the clients' `whoami()` calls.
    let resp = authed(client().get(format!("{base}/whoami")), &user_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "WM_TOKEN must still resolve its own identity: {}",
        resp.text().await?
    );
    let resp = authed(
        client().get(format!("{api}/w/test-workspace/scripts/list")),
        &user_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "WM_TOKEN must still work inside its own workspace: {}",
        resp.text().await?
    );

    // 9. ...and the writes it keeps. `wmill workspace add` checks this before it will
    //    accept the credentials it was given, so a job that points the CLI at its own
    //    instance depends on it.
    let resp = authed(
        client().post(format!("{api}/workspaces/exists")),
        &user_wm,
    )
    .json(&json!({ "id": "test-workspace" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "WM_TOKEN must still reach the workspace-exists check `wmill workspace add` makes: {}",
        resp.text().await?
    );

    //    The resource editor's object-storage "Test connection" runs as a preview job that
    //    POSTs its config here. The route only exists under `parquet`; without it the
    //    status would be a 404 and the assertion would hold for the wrong reason. The body
    //    is deliberately not a valid `ObjectSettings`, so reaching the handler's own
    //    extractors is exactly a 422 — anything else means the request never got there.
    #[cfg(feature = "parquet")]
    {
        let resp = authed(
            client().post(format!("{api}/settings/test_object_storage_config")),
            &user_wm,
        )
        .json(&json!({}))
        .send()
        .await?;
        assert_eq!(
            resp.status(),
            422,
            "WM_TOKEN must still reach the object-storage connection test: {}",
            resp.text().await?
        );
    }

    Ok(())
}
