//! Tests for the `guest` app execution mode.
//!
//! A guest (`ExecutionMode::Guest`) has no account and so no ACL of its own: its
//! token's scopes are its entire grant. These tests pin the three things that would
//! silently undo it:
//!
//!   * what makes a token a guest — the server-minted label, never a scope anyone
//!     could type into `users/tokens/create`;
//!   * the confinement — a guest reaches the one app it was let in for and nothing
//!     else;
//!   * the switches — an app's own `execution_mode: guest` is inert unless the
//!     workspace and the instance allow guests, checked at the door rather than only
//!     where a policy is written (git-sync and the CLI push policies past every UI);
//!     the allowance on top of them has a binary of its own.
//!
//! The token is inserted directly: how a guest session is minted is the identity
//! provider's business (EE), what one can do is this file's.
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const GUEST_TOKEN: &str = "GUEST_SECRET_TOKEN";
const APP_PATH: &str = "u/test-user/guest_app";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

async fn enable_guests(port: u16, ws: &str) -> anyhow::Result<()> {
    authed(
        client().post(format!(
            "http://localhost:{port}/api/w/{ws}/workspaces/edit_guest_access"
        )),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": true }))
    .send()
    .await?;
    Ok(())
}

fn guest_scopes() -> Vec<String> {
    vec![
        "guest".to_string(),
        "jobs:read".to_string(),
        "resources:run".to_string(),
        "users:read".to_string(),
        "folders:read".to_string(),
        format!("apps:read:{APP_PATH}"),
        format!("apps:run:{APP_PATH}"),
    ]
}

/// Insert a guest session for `test-workspace`, scoped to `APP_PATH`. Mirrors
/// `create_guest_session_token`: the server-minted label, the narrow reads, the two
/// path-scoped app grants, the workspace pin, and an expiry — a derived token's
/// lifetime is capped at it, so a guest session without one cannot mint.
async fn insert_guest_token(db: &Pool<Postgres>, workspace: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, scopes, workspace_id, expiration)
         VALUES (encode(sha256($1::bytea), 'hex'), 'GUEST_SECR', $2, 'guest@example.com',
                 'guest_session', $3, $4, now() + interval '8 hours')",
    )
    .bind(GUEST_TOKEN.as_bytes())
    .bind(GUEST_TOKEN)
    .bind(guest_scopes())
    .bind(workspace)
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn guest_session_is_confined_to_its_app(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    insert_guest_token(&db, "test-workspace").await?;

    // Its own identity resolves, and reports the role rather than falling through to
    // the non-member branch that hands out a `superadmin` shape.
    let resp = authed(client().get(format!("{ws}/users/whoami")), GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "guest whoami must resolve");
    let me: serde_json::Value = resp.json().await?;
    assert_eq!(
        me["role"],
        json!("guest"),
        "guest must not read as superadmin"
    );
    assert_eq!(me["operator"], json!(true));
    assert_eq!(me["is_admin"], json!(false));

    // `resources/list_names` and the type schemas stay open — a guest drives an app,
    // and app pickers need them — so the line to pin is the value-returning route.
    for route in [
        "jobs/list",
        "scripts/list",
        "flows/list",
        "variables/list",
        "resources/get_value/u/test-user/secret",
        "apps/list",
    ] {
        let resp = authed(client().get(format!("{ws}/{route}")), GUEST_TOKEN)
            .send()
            .await?;
        assert_eq!(
            resp.status(),
            403,
            "guest must be denied {route}, got {}",
            resp.status()
        );
    }

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn guest_token_does_not_cross_workspaces(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // A second workspace with an app at the SAME path: without the token's workspace
    // pin, `apps:run:<path>` would unlock it too, since a path is not unique across
    // workspaces.
    sqlx::query(
        "INSERT INTO workspace (id, name, owner) VALUES ('other-ws', 'other-ws', 'test-user')",
    )
    .execute(&db)
    .await?;
    sqlx::query("INSERT INTO workspace_settings (workspace_id) VALUES ('other-ws')")
        .execute(&db)
        .await?;

    insert_guest_token(&db, "test-workspace").await?;

    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/other-ws/apps/get/p/{APP_PATH}"
        )),
        GUEST_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "a guest token pinned to one workspace must not authenticate against another"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn guest_entry_needs_both_the_app_mode_and_the_workspace_switch(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP_PATH,
            "summary": "Guest app",
            "value": {},
            "policy": { "execution_mode": "guest", "triggerables": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);

    let secret: String = authed(
        client().get(format!("{ws}/apps/secret_of/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .text()
    .await?;

    // The app says guest, the workspace has not opted in: inert.
    let resp = client()
        .get(format!("{ws}/apps_u/guest_entry/{secret}"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        404,
        "a guest app in a workspace that has not enabled guests must not advertise entry"
    );

    authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": true }))
    .send()
    .await?;

    // Unauthenticated on purpose: this is what a signed-out visitor reads.
    let resp = client()
        .get(format!("{ws}/apps_u/guest_entry/{secret}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let entry: serde_json::Value = resp.json().await?;
    assert_eq!(entry["app_path"], json!(APP_PATH));

    // Turning the switch back off closes the door again even though the app's own
    // policy is unchanged.
    authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": false }))
    .send()
    .await?;
    let resp = client()
        .get(format!("{ws}/apps_u/guest_entry/{secret}"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        404,
        "turning guests off must stop advertising entry for an app already set to guest"
    );

    Ok(())
}

/// The guest grant is the server-minted label, never the `guest` scope. Scopes on a
/// user-created token are whatever the caller typed, so if the scope granted anything
/// then any member of any workspace could mint themselves non-member access to every
/// guest-mode app on the instance.
#[sqlx::test(fixtures("base"))]
async fn a_self_declared_guest_scope_grants_nothing(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // `users/tokens/create` must refuse the label outright...
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/users/tokens/create")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "label": "guest_session", "scopes": guest_scopes() }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "the guest session label must be server-minted only"
    );

    // ...and so must relabelling an ordinary token into it, or the pin-less user
    // token would become a guest session that authenticates in every workspace.
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/users/tokens/create")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "label": "mine", "scopes": guest_scopes() }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let prefix: String = sqlx::query_scalar(
        "SELECT token_prefix FROM token WHERE email = 'test@windmill.dev' AND label = 'mine'",
    )
    .fetch_one(&db)
    .await?;
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/users/tokens/update_label/{prefix}"
        )),
        ADMIN_TOKEN,
    )
    .json(&json!({ "label": "guest_session" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "relabelling into the guest namespace must be refused: {}",
        resp.text().await?
    );

    // ...and a token that carries the scopes under any other label authenticates as
    // nothing in a workspace its owner is not a member of.
    // An email with no `usr` row anywhere: exactly the identity the guest arm exists
    // to admit, and the one a forged scope must not admit.
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, scopes)
         VALUES (encode(sha256($1::bytea), 'hex'), 'FORGED_SCO', $2, 'outsider@example.com',
                 'forged', $3)",
    )
    .bind(b"FORGED_SCOPES".as_slice())
    .bind("FORGED_SCOPES")
    .bind(guest_scopes())
    .execute(&db)
    .await?;

    let resp = authed(client().get(format!("{ws}/users/whoami")), "FORGED_SCOPES")
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "declaring the guest scope must not turn a non-member into an identity"
    );

    Ok(())
}

/// A guest-mode policy that names one runnable, so an `execute_component` request
/// gets past the triggerables lookup and reaches the guest gate. `sandbox` is what
/// makes the embed-token endpoint actually mint a token.
fn guest_app_with_runnable(path: &str, sandbox: bool) -> serde_json::Value {
    app_with_runnable(path, "guest", sandbox)
}

fn app_with_runnable(path: &str, execution_mode: &str, sandbox: bool) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "App",
        "value": {},
        "policy": {
            "execution_mode": execution_mode,
            "sandbox": sandbox,
            "triggerables_v2": {
                "script/u/test-user/noop": { "static_inputs": {}, "one_of_inputs": {} }
            }
        }
    })
}

fn execute(port: u16, ws: &str, app: &str, token: &str) -> reqwest::RequestBuilder {
    authed(
        client().post(format!(
            "http://localhost:{port}/api/w/{ws}/apps_u/execute_component/{app}"
        )),
        token,
    )
    .json(&json!({
        "component": "a",
        "path": "script/u/test-user/noop",
        "args": {}
    }))
}

/// The workspace switch is enforced at the auth door for every guest request, not
/// remembered per handler. This is what stands between a `guest` policy pushed by
/// git-sync and execution once an admin has turned guests off — and it closes the
/// app to sessions already issued.
#[sqlx::test(fixtures("base"))]
async fn the_door_re_checks_the_workspace_switch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(APP_PATH, false))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    insert_guest_token(&db, "test-workspace").await?;

    // Switch off: the session does not authenticate at all, even though the app's
    // policy says guest and the session was (in this fixture) issued regardless. On
    // the authed route that is a 401; on the optional-auth run route the rejected
    // token reads as no token, and a guest-mode app then refuses the anonymous
    // caller — a denial either way.
    let resp = authed(client().get(format!("{ws}/users/whoami")), GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "a guest must not authenticate while guests are off"
    );
    let resp = execute(port, "test-workspace", APP_PATH, GUEST_TOKEN)
        .send()
        .await?;
    assert!(
        resp.status().is_client_error() && resp.status() != 404,
        "a guest must not run while guests are off, got {}",
        resp.status()
    );

    // Switch on: through the door. What follows the run is the runnable lookup,
    // which fails on the nonexistent script — the point is that it is no longer a
    // denial.
    enable_guests(port, "test-workspace").await?;
    let resp = execute(port, "test-workspace", APP_PATH, GUEST_TOKEN)
        .send()
        .await?;
    assert!(
        resp.status() != 401 && resp.status() != 403,
        "with guests on, the door must let the run through: {}",
        resp.status()
    );

    Ok(())
}

/// The path scope is what keeps a guest to the one app it was let in for: the route
/// layer is resource-blind for `apps:run`, so this line is drawn in the handler.
#[sqlx::test(fixtures("base"))]
async fn guest_cannot_run_another_guest_app(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": true }))
    .send()
    .await?;
    let other = "u/test-user/other_guest_app";
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(other, false))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    insert_guest_token(&db, "test-workspace").await?; // scoped to APP_PATH, not `other`

    let resp = execute(port, "test-workspace", other, GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a guest session scoped to one app must not run another, even one open to guests"
    );

    Ok(())
}

/// The app path is spliced into the session's scopes, whose grammar reserves `:`, `,`
/// and `*`: a path carrying one would scope the guest to more than the one app it was
/// let in for, so the mint refuses it before anything else. Anything else in a path
/// (spaces, `@`) is literal to that grammar and stays admissible.
#[sqlx::test(fixtures("base"))]
async fn a_scope_metacharacter_in_the_app_path_is_refused(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let mint = |path: &'static str| {
        let db = db.clone();
        async move {
            let mut tx = db.begin().await?;
            let minted = windmill_api_users::users::create_guest_session_token(
                "guest@example.com",
                "test-workspace",
                path,
                &mut tx,
                tower_cookies::Cookies::default(),
            )
            .await;
            anyhow::Ok(minted)
        }
    };
    for path in [
        "u/test-user/entry,u/test-user/hidden",
        "u/test-user/*",
        "u/test-user/entry:run",
    ] {
        let minted = mint(path).await?;
        assert!(
            matches!(minted, Err(windmill_common::error::Error::BadRequest(ref m)) if m.contains("cannot be scoped")),
            "{path}: {minted:?}"
        );
    }
    for path in ["u/test-user/My App", "u/admin@windmill.dev/x"] {
        let minted = mint(path).await?;
        assert!(
            !matches!(minted, Err(windmill_common::error::Error::BadRequest(ref m)) if m.contains("cannot be scoped")),
            "{path} is literal to the scope grammar and must get past the guard: {minted:?}"
        );
    }
    Ok(())
}

/// A guest reads the jobs it launched and nothing else: with no membership behind it,
/// it must stop where an app embed token stops, before the share-token and ACL grants
/// a member would get, and with the same "not found" so it cannot probe for jobs.
#[sqlx::test(fixtures("base"))]
async fn a_guest_cannot_read_a_job_it_did_not_launch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    insert_guest_token(&db, "test-workspace").await?;
    let resp = authed(client().post(format!("{ws}/scripts/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": "u/test-user/noop",
            "summary": "",
            "description": "",
            "content": "echo 42",
            "language": "bash",
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let resp = authed(
        client().post(format!("{ws}/jobs/run/p/u/test-user/noop")),
        ADMIN_TOKEN,
    )
    .json(&json!({}))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let job_id = resp.text().await?;

    let resp = authed(
        client().get(format!("{ws}/jobs_u/getupdate/{job_id}")),
        GUEST_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        404,
        "another caller's job is not found for a guest: {}",
        resp.text().await?
    );

    Ok(())
}

/// Guests mode cannot land on a path the scope grammar cannot hold, however it gets
/// there: set at creation, set on update, or a rename of an app already in that mode.
#[sqlx::test(fixtures("base"))]
async fn guests_mode_needs_a_scopable_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable("u/test-user/a:b", false))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "created into Guests on a `:` path");

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(APP_PATH, false))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let resp = authed(
        client().post(format!("{ws}/apps/update/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "path": "u/test-user/a,b" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 400, "renamed to a `,` path while in Guests");
    let resp = authed(
        client().post(format!("{ws}/apps/update/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "path": "u/test-user/My App" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a space is literal: {}",
        resp.text().await?
    );

    // Set on update: an app that already sits on such a path cannot be switched.
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": "u/test-user/x:y",
            "summary": "App",
            "value": {},
            "policy": { "execution_mode": "publisher", "triggerables_v2": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user/x:y")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "policy": { "execution_mode": "guest", "triggerables_v2": {} } }))
    .send()
    .await?;
    assert_eq!(resp.status(), 400, "switched to Guests on a `:` path");

    Ok(())
}

/// Renaming a workspace copies its settings; the guest switch and the guest JWT key must
/// travel with them, or the rename silently shuts every guest app or drops the key.
#[sqlx::test(fixtures("base"))]
async fn a_workspace_rename_keeps_the_guest_switch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    enable_guests(port, "test-workspace").await?;
    sqlx::query(
        "INSERT INTO guest_activity (email, workspace_id, day)
         VALUES ('guest@example.com', 'test-workspace', CURRENT_DATE)",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "UPDATE workspace_settings SET guest_jwt_public_key = 'test-pem-key' WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/workspaces/change_workspace_id"
        )),
        ADMIN_TOKEN,
    )
    .json(&json!({ "new_id": "test-workspace-2", "new_name": "Test workspace 2" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let enabled: bool = sqlx::query_scalar(
        "SELECT guest_access_enabled FROM workspace_settings WHERE workspace_id = 'test-workspace-2'",
    )
    .fetch_one(&db)
    .await?;
    assert!(enabled, "the guest switch travels with the workspace");
    let jwt_key: Option<String> = sqlx::query_scalar(
        "SELECT guest_jwt_public_key FROM workspace_settings WHERE workspace_id = 'test-workspace-2'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        jwt_key.as_deref(),
        Some("test-pem-key"),
        "the guest JWT key travels with the workspace"
    );
    let moved: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM guest_activity WHERE workspace_id = 'test-workspace-2')
            AND NOT EXISTS(SELECT 1 FROM guest_activity WHERE workspace_id = 'test-workspace')",
    )
    .fetch_one(&db)
    .await?;
    assert!(moved, "the guests seen in the workspace follow its new id");

    Ok(())
}

/// The superadmin switch sits above every workspace's: off, no guest session stands and
/// no app discovers as open, whatever the workspace and the app say.
#[sqlx::test(fixtures("base"))]
async fn the_instance_switch_closes_every_workspace(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(APP_PATH, false))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    insert_guest_token(&db, "test-workspace").await?;
    let set_instance_switch = |disabled: bool| {
        authed(
            client().post(format!(
                "http://localhost:{port}/api/settings/global/guest_access_disabled"
            )),
            ADMIN_TOKEN,
        )
        .json(&json!({ "value": disabled }))
        .send()
    };

    let secret: String = authed(
        client().get(format!("{ws}/apps/secret_of/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .text()
    .await?;

    set_instance_switch(true).await?.error_for_status()?;
    let resp = authed(client().get(format!("{ws}/users/whoami")), GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "the instance switch closes an issued session"
    );
    let resp = client()
        .get(format!("{ws}/apps_u/guest_entry/{secret}"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        404,
        "and nothing discovers as open to guests"
    );

    set_instance_switch(false).await?.error_for_status()?;
    let resp = authed(client().get(format!("{ws}/users/whoami")), GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "back on, the session stands again");

    Ok(())
}

/// An account holder is never a guest, and that holds after the mint too: a session
/// minted before the account existed ends at the door the moment one does, so an
/// account provisioned in a race with the mint cannot outlive the rule.
#[sqlx::test(fixtures("base"))]
async fn an_account_ends_the_guest_session(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    insert_guest_token(&db, "test-workspace").await?;
    let resp = authed(client().get(format!("{ws}/users/whoami")), GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "guest whoami must resolve");

    sqlx::query(
        "INSERT INTO password (email, password_hash, login_type, super_admin, verified, name)
         VALUES ('guest@example.com', 'not-a-real-hash', 'password', false, true, 'Guest')",
    )
    .execute(&db)
    .await?;
    let resp = authed(client().get(format!("{ws}/users/whoami")), GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "an account created after the mint ends the guest session at the door"
    );

    Ok(())
}

/// An upload goes through an app's `s3_inputs` policy or not at all for a guest: the
/// legacy branch for an app without one uploads with the caller's own standing, which a
/// guest has none of, and an app path with no row must not slip past the confinement.
#[cfg(feature = "parquet")]
#[sqlx::test(fixtures("base"))]
async fn a_guest_cannot_upload_outside_a_policy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(APP_PATH, false)) // no `s3_inputs`
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    insert_guest_token(&db, "test-workspace").await?;

    let upload = |app: &str| {
        authed(
            client().post(format!(
                "{ws}/apps_u/upload_s3_file/{app}?file_key=anything"
            )),
            GUEST_TOKEN,
        )
        .body("x")
        .send()
    };
    let resp = upload("u/test-user/no_such_app").await?;
    assert_eq!(
        resp.status(),
        403,
        "a path with no app must not escape the guest's confinement: {}",
        resp.text().await?
    );
    let resp = upload(APP_PATH).await?;
    assert_eq!(
        resp.status(),
        400,
        "without an upload policy a guest is refused like an anonymous caller: {}",
        resp.text().await?
    );

    Ok(())
}

/// An anonymous app is open to anyone, a guest included, and the guest uses it as
/// itself: the component run and the result read that follows are one identity, so
/// the read's launched-by-me grant matches. Acting as nobody for the run and as the
/// guest for the read would start a job whose result the page can never fetch.
#[sqlx::test(fixtures("base"))]
async fn a_guest_uses_an_anonymous_app_as_itself(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    let resp = authed(client().post(format!("{ws}/scripts/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": "u/test-user/noop",
            "summary": "",
            "description": "",
            "content": "echo 42",
            "language": "bash",
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let anon = "u/test-user/anon_app";
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&app_with_runnable(anon, "anonymous", false))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    insert_guest_token(&db, "test-workspace").await?; // scoped to APP_PATH, not `anon`

    let resp = execute(port, "test-workspace", anon, GUEST_TOKEN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let job_id = resp.text().await?;

    let resp = authed(
        client().get(format!("{ws}/jobs_u/getupdate/{job_id}")),
        GUEST_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "the guest that started the run must be able to read it back: {}",
        resp.text().await?
    );

    Ok(())
}

/// The embed token a guest mints for a sandboxed app is the one credential handed to
/// untrusted app JS. It must be a guest twice over — resolve like its minter (the
/// label) and be governed like its minter (the sentinel) — or every guest control
/// silently skips the most exposed credential there is.
#[sqlx::test(fixtures("base"))]
async fn a_guest_minted_embed_token_stays_a_guest(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(APP_PATH, true))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let secret: String = authed(
        client().get(format!("{ws}/apps/secret_of/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .text()
    .await?;
    insert_guest_token(&db, "test-workspace").await?;

    // The guest page mints the iframe's token from the guest session.
    let resp = authed(
        client().get(format!("{ws}/apps_u/embed_token/{secret}")),
        GUEST_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a guest must be able to mint: {}",
        resp.text().await?
    );
    let body: serde_json::Value = resp.json().await?;
    let embed = body["token"]
        .as_str()
        .expect("mint must return a token for an authenticated guest")
        .to_string();

    // Its lifetime is capped at the session that minted it: the requested embed
    // validity (12h) is longer than the guest session's (8h in this fixture), and the
    // session's expiry is a guest's only revocation.
    let parent_exp: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT expiration FROM token WHERE token_prefix = 'GUEST_SECR'")
            .fetch_one(&db)
            .await?;
    let child_exp: chrono::DateTime<chrono::Utc> = body["expiration"]
        .as_str()
        .and_then(|e| e.parse().ok())
        .expect("mint must return the token's expiration");
    assert!(
        child_exp <= parent_exp,
        "a guest's embed token must not outlive the session that minted it ({child_exp} > {parent_exp})"
    );

    // Resolves — and as a guest, not as the non-member superadmin shape.
    let resp = authed(client().get(format!("{ws}/users/whoami")), &embed)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "the minted token must authenticate");
    let me: serde_json::Value = resp.json().await?;
    assert_eq!(me["role"], json!("guest"));

    // Governed: the workspace switch closes it at the door, iframe or not.
    authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": false }))
    .send()
    .await?;
    let resp = authed(client().get(format!("{ws}/users/whoami")), &embed)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "turning guests off must stop a guest's embed token authenticating"
    );
    let resp = execute(port, "test-workspace", APP_PATH, &embed)
        .send()
        .await?;
    assert!(
        resp.status().is_client_error() && resp.status() != 404,
        "and running components, got {}",
        resp.status()
    );
    enable_guests(port, "test-workspace").await?;

    // And its scopes are not something the guest's email can later rewrite. The
    // guest session itself cannot reach `/users/*` (workspace pin), so model the real
    // threat: the same email after promotion, holding an ordinary unpinned session.
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label)
         VALUES (encode(sha256($1::bytea), 'hex'), 'PROMOTED_S', $2, 'guest@example.com',
                 'session')",
    )
    .bind(b"PROMOTED_SESSION".as_slice())
    .bind("PROMOTED_SESSION")
    .execute(&db)
    .await?;
    for prefix in [&embed[..10], &GUEST_TOKEN[..10]] {
        let resp = authed(
            client().post(format!(
                "http://localhost:{port}/api/users/tokens/update_scopes/{prefix}"
            )),
            "PROMOTED_SESSION",
        )
        .json(&json!({ "scopes": null }))
        .send()
        .await?;
        assert_eq!(
            resp.status(),
            404,
            "a promoted account must not be able to rescope its old guest credentials"
        );
    }

    Ok(())
}

/// The label is the single source of truth: a guest-labelled credential is governed
/// as a guest even if its scopes carry no sentinel. Otherwise every mint that derives
/// a token from a guest session is one forgotten `push` away from an ungoverned
/// non-member credential.
#[sqlx::test(fixtures("base"))]
async fn a_guest_label_is_governed_without_the_sentinel(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    enable_guests(port, "test-workspace").await?;
    let scopes: Vec<String> = guest_scopes()
        .into_iter()
        .filter(|s| s != "guest")
        .collect();
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, scopes, workspace_id, expiration)
         VALUES (encode(sha256($1::bytea), 'hex'), 'NOSENTINE_', $2, 'guest@example.com',
                 'guest_session', $3, 'test-workspace', now() + interval '8 hours')",
    )
    .bind(b"NOSENTINEL".as_slice())
    .bind("NOSENTINEL")
    .bind(scopes)
    .execute(&db)
    .await?;

    let resp = authed(client().get(format!("{ws}/users/whoami")), "NOSENTINEL")
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let me: serde_json::Value = resp.json().await?;
    assert_eq!(
        me["role"],
        json!("guest"),
        "the label alone must make a credential a guest"
    );
    let resp = authed(client().get(format!("{ws}/jobs/list")), "NOSENTINEL")
        .send()
        .await?;
    assert_eq!(resp.status(), 403, "and confine it like one");

    Ok(())
}

/// A guest is someone with no account at all — including a deactivated one. The
/// sign-in path's own account lookup filters on `disabled = false`, so a disabled
/// account reads as absent there; the mint has to refuse on its own or deactivation
/// (manual or SCIM, whose revocation is "delete the tokens") walks straight back in.
#[sqlx::test(fixtures("base"))]
async fn a_disabled_account_cannot_become_a_guest(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": true }))
    .send()
    .await?;
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&guest_app_with_runnable(APP_PATH, false))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    sqlx::query(
        "INSERT INTO password (email, password_hash, login_type, super_admin, verified, disabled)
         VALUES ('gone@example.com', 'x', 'password', false, true, true)",
    )
    .execute(&db)
    .await?;

    let mut tx = db.begin().await?;
    let cookies = tower_cookies::Cookies::default();
    let minted = windmill_api_users::users::create_guest_session_token(
        "gone@example.com",
        "test-workspace",
        APP_PATH,
        &mut tx,
        cookies,
    )
    .await;
    assert!(
        matches!(minted, Err(windmill_common::error::Error::NotAuthorized(_))),
        "a deactivated account must be refused a guest session, got {minted:?}"
    );

    Ok(())
}
