//! A WM_TOKEN (job JWT) is minted for one job in one workspace and carries that
//! job's full user privileges. Two independent caps hold it there, and this file
//! covers both:
//!
//! - Confinement to the job's workspace, enforced in the auth middleware. A route
//!   that names no workspace is instance-wide, so reaching one would trade an
//!   ephemeral, workspace-bound credential for a permanent one (`tokens/create`
//!   mints a workspace-less API token that never expires), for instance
//!   configuration, or for global user management. Refusals are `403`.
//! - A ceiling of workspace admin whatever identity the token borrows, enforced at
//!   the privilege gates themselves (`require_super_admin`, `require_devops_role`,
//!   `require_instance_admin`, GHSA-hfh4-cx4h-3fcr). Refusals are `401`.
//!
//! The middleware runs first, so on a workspace-less route it answers before the
//! gate behind it ever runs: those cases pin the outer cap, and the gates are
//! pinned by the workspace-scoped cases, which the middleware lets through.
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
use windmill_api_auth::ApiAuthed;
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
    let resp = authed(client().post(format!("{api}/workspaces/exists")), &user_wm)
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
    //    POSTs its config here. The body is deliberately not a valid `ObjectSettings`, so
    //    reaching the handler's own extractors is exactly a 422 — a 403 means confinement
    //    refused it. The route is only mounted under `parquet`, which would make this a 404,
    //    so the case is gated on the feature rather than left to fail where it can't run.
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

    // 10. The rest of the allowlist: routes that answer from the caller's own account or
    //     from the request body alone.
    for route in [
        "users/email",
        "users/usage",
        "users/tutorial_progress",
        "workspaces/allowed_domain_auto_invite",
    ] {
        let resp = authed(client().get(format!("{api}/{route}")), &user_wm)
            .send()
            .await?;
        assert_eq!(
            resp.status(),
            200,
            "WM_TOKEN must still read its own {route}: {}",
            resp.text().await?
        );
    }
    // ...and the one `settings/global` key on the allowlist, which the CLI reads before
    // creating a user on a git-sync push. `ws_base_url` is the control: the handler leaves
    // it as ungated as `automate_username_creation`, so only the allowlist stops it.
    let resp = authed(
        client().get(format!("{api}/settings/global/automate_username_creation")),
        &user_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "WM_TOKEN must still read automate_username_creation: {}",
        resp.text().await?
    );
    let resp = authed(
        client().get(format!("{api}/settings/global/ws_base_url")),
        &user_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "WM_TOKEN must not read any other global setting: {}",
        resp.text().await?
    );
    let resp = authed(client().post(format!("{api}/schedules/preview")), &user_wm)
        .json(&json!({ "schedule": "0 0 12 * * *", "timezone": "UTC" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "WM_TOKEN must still preview a cron expression: {}",
        resp.text().await?
    );
    let resp = authed(client().post(format!("{base}/tutorial_progress")), &user_wm)
        .json(&json!({ "progress": 1, "skipped_all": false }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "WM_TOKEN must still record its own tutorial progress: {}",
        resp.text().await?
    );

    // 11. ...and the caller-scoped reads deliberately left out of it, each because it
    //     names another workspace or the identity's credentials.
    for route in ["users/list_invites", "users/tokens/list", "workspaces/list"] {
        let resp = authed(client().get(format!("{api}/{route}")), &user_wm)
            .send()
            .await?;
        assert_eq!(
            resp.status(),
            403,
            "{route} must stay confined: {}",
            resp.text().await?
        );
    }

    Ok(())
}

/// A WM_TOKEN running as a superadmin must be rejected by *any* `require_super_admin`
/// route, not just the handful that call `forbid_superadmin_job_token`
/// (GHSA-hfh4-cx4h-3fcr). Both shapes of such a route are covered: a workspace-less
/// one, which workspace confinement answers first, and a workspace-scoped one, which
/// reaches the gate itself.
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
        403,
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

    // `GET /api/w/{workspace}/users/list_addable` is gated solely by
    // `require_super_admin` too, but names a workspace, so the request runs the gate
    // instead of stopping at confinement.
    let resp = authed(
        client().get(format!("{base}/w/test-workspace/users/list_addable")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not clear require_super_admin inside its own workspace: {}",
        resp.text().await?
    );

    let resp = authed(
        client().get(format!("{base}/w/test-workspace/users/list_addable")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real superadmin token must still clear the gate: {}",
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
    let resp = authed(
        client().get(format!("{base}/service_logs/list_files")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
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
        403,
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

/// Instance-global routes with no workspace binding that gate on the caller's own
/// `is_admin` claim must reject a WM_TOKEN — `is_admin` is a workspace-admin claim
/// (also true for superadmins), and a job token is capped at workspace admin, so it
/// must not wield that claim as instance authorization (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_rejected_by_instance_admin_gates(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    // A worker-group config carrying a static env value that must stay masked.
    sqlx::query("INSERT INTO config (name, config) VALUES ('worker__wm2082grp', $1)")
        .bind(json!({ "env_vars_static": { "LEAKY": "supersecretvalue" } }))
        .execute(&db)
        .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    // The exact token a deployer obtains via an app on_behalf_of pointed at a
    // superadmin: is_admin=true, but carrying a job_id.
    let sa_wm = wm_token("test@windmill.dev", true).await;

    // 1. Arbitrary workspace unarchive (mutation on any workspace by id).
    let resp = authed(
        client().post(format!("{base}/workspaces/unarchive/test-workspace")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not unarchive an arbitrary workspace: {}",
        resp.text().await?
    );

    // 2. Global concurrency-group pruning.
    let resp = authed(
        client().delete(format!("{base}/concurrency_groups/prune/anykey")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not prune a global concurrency group: {}",
        resp.text().await?
    );

    // 3. The sibling listing spans every workspace's concurrency keys, so it is
    //    gated the same way as the prune above.
    let resp = authed(
        client().get(format!("{base}/concurrency_groups/list")),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not list global concurrency groups: {}",
        resp.text().await?
    );

    // 4. Worker-group config: refused outright, so the static env value it would
    //    otherwise obfuscate never reaches a job token. Asserting the status rather
    //    than the absence of the secret keeps the case honest — an error body
    //    trivially satisfies "does not contain the secret".
    let resp = authed(
        client().get(format!("{base}/configs/list_worker_groups")),
        &sa_wm,
    )
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 403,
        "superadmin WM_TOKEN must not read the worker-group config: {body}"
    );
    assert!(
        !body.contains("supersecretvalue"),
        "the refusal must not carry the static env value: {body}"
    );

    // No false positive: a real superadmin API token (no job_id) still sees the
    // unobfuscated value — the cap keys off the job token, not the identity.
    let body = authed(
        client().get(format!("{base}/configs/list_worker_groups")),
        "SECRET_TOKEN",
    )
    .send()
    .await?
    .text()
    .await?;
    assert!(
        body.contains("supersecretvalue"),
        "a real superadmin token must still see the unobfuscated worker-group config: {body}"
    );

    Ok(())
}

/// Capping a `WM_TOKEN` at the gates is only durable if the token cannot trade
/// itself for one without the `job_id` those gates key off. Both credential-minting
/// routes must therefore refuse an elevated job token: `refresh_token` (which mints
/// a database-backed session token and returns it in `Set-Cookie`) and
/// `tokens/create` for the `devops` tier, whose routes are capped just like
/// superadmin's (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_cannot_mint_a_provenance_free_credential(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");

    // 1. Session refresh: a superadmin-identity job token must not obtain a session
    //    token, which would authenticate with no job provenance at all.
    let sa_wm = wm_token("test@windmill.dev", true).await;
    let resp = authed(client().get(format!("{base}/refresh_token")), &sa_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not refresh into a session token: {}",
        resp.text().await?
    );

    // No false positive: a real superadmin API token still refreshes.
    let resp = authed(
        client().get(format!("{base}/refresh_token")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real superadmin token must still refresh: {}",
        resp.text().await?
    );

    // 2. Token mint, devops tier: `require_devops_role` rejects this job token, so
    //    minting one that would pass it by email must be refused too.
    let devops_wm = wm_token("devops@windmill.dev", false).await;
    let resp = authed(client().post(format!("{base}/tokens/create")), &devops_wm)
        .json(&json!({ "label": "from-script" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "devops WM_TOKEN must not mint a token: {}",
        resp.text().await?
    );

    // 3. Choosing the password of the elevated account it runs as would let the
    //    holder log in for a session that carries no job provenance at all.
    let resp = authed(client().post(format!("{base}/setpassword")), &devops_wm)
        .json(&json!({ "password": "hunter2" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "devops WM_TOKEN must not set its account password: {}",
        resp.text().await?
    );

    Ok(())
}

/// The MCP OAuth approval is a third credential mint: the code it stores is
/// exchanged for a database token holding only an email, so an elevated job token
/// approving a client would obtain a credential with no `job_id` and re-enter the
/// API through the gateway uncapped (GHSA-hfh4-cx4h-3fcr). The guard sits in the
/// shared inner fn, ahead of client validation, so it fires without a registered
/// client.
#[cfg(feature = "mcp")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_cannot_mint_via_mcp_oauth_approval(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    let approval = json!({
        "client_id": "wm2082-client",
        "redirect_uri": "http://localhost/callback",
        "scope": "mcp:all",
        "state": "s",
        "code_challenge": "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        "code_challenge_method": "S256",
    });

    let sa_wm = wm_token("test@windmill.dev", true).await;
    let resp = authed(
        client().post(format!("{base}/w/test-workspace/mcp/oauth/server/approve")),
        &sa_wm,
    )
    .json(&approval)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not approve an MCP OAuth client: {}",
        resp.text().await?
    );

    // The gateway route reaches the same mint and must be capped identically.
    let mut gateway_approval = approval.clone();
    gateway_approval["workspace_id"] = json!("test-workspace");
    let resp = authed(
        client().post(format!("{base}/mcp/gateway/oauth/server/approve")),
        &sa_wm,
    )
    .json(&gateway_approval)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not approve through the MCP gateway: {}",
        resp.text().await?
    );

    Ok(())
}

/// The two links that let a narrowly-scoped mint become a general credential: the
/// sandboxed app embed mint (a 12h database token with no job provenance) and
/// `tokens/update_scopes`, which an unscoped job token could use to clear the
/// scopes of any token sharing its email (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_cannot_mint_or_widen_an_app_embed_token(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    // A sandboxed app the superadmin identity can read — the mint's precondition.
    sqlx::query(
        "INSERT INTO app (id, workspace_id, path, summary, versions, policy, extra_perms)
         VALUES (9001, 'test-workspace', 'u/test-user/embedded', 'Embedded', '{}',
                 '{\"execution_mode\": \"viewer\", \"sandbox\": true}', '{}')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO app_version (id, app_id, value, created_by, created_at)
         VALUES (9001, 9001, '{\"grid\": []}', 'test-user', NOW())",
    )
    .execute(&db)
    .await?;
    sqlx::query("UPDATE app SET versions = ARRAY[9001::bigint] WHERE id = 9001")
        .execute(&db)
        .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    let sa_wm = wm_token("test@windmill.dev", true).await;
    let resp = authed(
        client().get(format!(
            "{base}/w/test-workspace/apps/embed_token/p/u/test-user/embedded"
        )),
        &sa_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "superadmin WM_TOKEN must not mint an app embed token: {}",
        resp.text().await?
    );

    // Even a token minted some other way must stay narrow: widening is refused.
    let resp = authed(
        client().post(format!("{base}/users/tokens/update_scopes/SECRET_T")),
        &sa_wm,
    )
    .json(&json!({ "scopes": serde_json::Value::Null }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "superadmin WM_TOKEN must not widen a token's scopes: {}",
        resp.text().await?
    );

    Ok(())
}

/// Destroying the account or credentials of the identity a job runs as is never the
/// runnable's work, and a `wm_deployers` member may point `on_behalf_of` at any real
/// user — so these reject every job token, elevated or not (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_cannot_destroy_its_on_behalf_account(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");

    // An ordinary member's identity: the cap here does not depend on elevation.
    let user_wm = wm_token("test2@windmill.dev", false).await;

    let resp = authed(client().post(format!("{base}/leave_instance")), &user_wm)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "WM_TOKEN must not delete the account it runs as: {}",
        resp.text().await?
    );

    // The prefix of that identity's real fixture token, so without the guard the
    // delete would land rather than silently match nothing.
    let resp = authed(
        client().delete(format!("{base}/tokens/delete/SECRET_TOK")),
        &user_wm,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "WM_TOKEN must not revoke that identity's tokens: {}",
        resp.text().await?
    );

    // Ejecting the identity from a workspace is the same primitive, on both routes
    // that expose it (one keyed by username, one by email).
    for route in [
        "w/test-workspace/users/leave",
        "w/test-workspace/workspaces/leave",
    ] {
        let resp = authed(
            client().post(format!("http://localhost:{port}/api/{route}")),
            &user_wm,
        )
        .send()
        .await?;
        assert_eq!(
            resp.status(),
            401,
            "WM_TOKEN must not leave a workspace as {route}: {}",
            resp.text().await?
        );
    }
    let membership: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM usr WHERE workspace_id = 'test-workspace' AND email = 'test2@windmill.dev'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(membership, 1, "the workspace membership must survive");

    // The account and its credentials are untouched, not merely the response refused.
    let account_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM password WHERE email = 'test2@windmill.dev'")
            .fetch_one(&db)
            .await?;
    assert_eq!(account_rows, 1, "the password row must survive");
    let token_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM token WHERE email = 'test2@windmill.dev'")
            .fetch_one(&db)
            .await?;
    assert!(token_rows > 0, "the identity's tokens must survive");

    Ok(())
}

/// `load_workspace_authed` grants an admin claim in a workspace the caller may have
/// no relationship with, and carries `job_id` into the result — so deriving it from
/// the on-behalf email would hand a WM_TOKEN admin over every workspace on the
/// instance, and with it the cross-workspace diff (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_wm_token_gets_no_admin_claim_in_a_foreign_workspace(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    // A workspace the superadmin identity is not a member of.
    sqlx::query(
        "INSERT INTO workspace (id, name, owner) VALUES ('other-workspace', 'Other', 'test-user')",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");

    let sa_wm = wm_token("test@windmill.dev", true).await;
    let resp = authed(
        client().get(format!(
            "{base}/w/test-workspace/workspaces/compare/other-workspace"
        )),
        &sa_wm,
    )
    .send()
    .await?;
    assert_ne!(
        resp.status(),
        200,
        "superadmin WM_TOKEN must not diff a workspace it does not belong to"
    );

    // No false positive: a real superadmin token still holds the claim.
    let resp = authed(
        client().get(format!(
            "{base}/w/test-workspace/workspaces/compare/other-workspace"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a real superadmin token must still diff across workspaces: {}",
        resp.text().await?
    );

    Ok(())
}

/// The guards below cap a job token on routes that name no workspace, which confinement
/// now answers first — so no request can reach them and no HTTP case would notice if they
/// stopped capping. They are called directly for that reason; deleting them because the
/// suite is green elsewhere would leave the inner cap unpinned (GHSA-hfh4-cx4h-3fcr).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_privilege_gates_reject_a_job_token_directly(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    fn authed_as(email: &str, job_id: Option<uuid::Uuid>) -> ApiAuthed {
        ApiAuthed {
            email: email.to_string(),
            username: "runner".to_string(),
            is_admin: true,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes: None,
            username_override: None,
            username_override_is_token_label: false,
            is_session_token: false,
            token_prefix: None,
            read_only: false,
            job_id,
            credential_expiry: None,
        }
    }

    let job = authed_as("test@windmill.dev", Some(uuid::Uuid::new_v4()));
    let not_job = authed_as("test@windmill.dev", None);

    assert!(
        windmill_api_auth::require_devops_role(&db, &job)
            .await
            .is_err(),
        "a job token must not hold the devops role"
    );
    assert!(
        windmill_api_auth::require_instance_admin(&job).is_err(),
        "a job token must not hold instance admin"
    );

    // The identity is a real superadmin, so without the job provenance both gates pass —
    // proving the rejections above key off `job_id` and not the fixture's user.
    assert!(
        windmill_api_auth::require_devops_role(&db, &not_job)
            .await
            .is_ok(),
        "a superadmin API token must still hold the devops role"
    );
    assert!(
        windmill_api_auth::require_instance_admin(&not_job).is_ok(),
        "a superadmin API token must still hold instance admin"
    );

    // The boolean sibling, which `list_worker_groups` consults to decide whether to
    // obfuscate rather than to refuse: reading `true` there returns `env_vars_static` in
    // the clear, so this one fails by leaking rather than by letting a request through.
    assert!(
        !windmill_api_auth::is_instance_admin(&job),
        "a job token must not read as instance admin"
    );
    assert!(
        windmill_api_auth::is_instance_admin(&not_job),
        "an admin API token must still read as instance admin"
    );

    // `forbid_superadmin_job_token` guards the same class of route but keys on two things
    // at once, so all three combinations are worth pinning: it fires only for a job token
    // whose identity is a superadmin.
    assert!(
        windmill_api_auth::forbid_superadmin_job_token(&db, &job.email, job.job_id)
            .await
            .is_err(),
        "a superadmin job token must be forbidden"
    );
    assert!(
        windmill_api_auth::forbid_superadmin_job_token(&db, &not_job.email, None)
            .await
            .is_ok(),
        "a superadmin API token carries no job provenance to forbid"
    );
    assert!(
        windmill_api_auth::forbid_superadmin_job_token(
            &db,
            "test2@windmill.dev",
            Some(uuid::Uuid::new_v4())
        )
        .await
        .is_ok(),
        "a job token running as a non-superadmin is not what this gate withholds"
    );

    // `forbid_elevated_job_token` is the same shape one tier wider — `is_devops_email` is
    // true for superadmins too. The embed-token case above reaches it, but only ever with
    // an elevated identity; these pin the other two combinations, so collapsing the gate
    // into a blanket job-token refusal would be caught here rather than by whoever next
    // creates a token from a script.
    assert!(
        windmill_api_auth::forbid_elevated_job_token(&db, "devops@windmill.dev", job.job_id)
            .await
            .is_err(),
        "a devops job token must not mint a credential"
    );
    assert!(
        windmill_api_auth::forbid_elevated_job_token(&db, &job.email, job.job_id)
            .await
            .is_err(),
        "a superadmin job token must not mint a credential"
    );
    assert!(
        windmill_api_auth::forbid_elevated_job_token(
            &db,
            "test2@windmill.dev",
            Some(uuid::Uuid::new_v4())
        )
        .await
        .is_ok(),
        "an unelevated job token is left to the confinement check, not refused here"
    );

    Ok(())
}
