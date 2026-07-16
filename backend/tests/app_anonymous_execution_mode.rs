//! Test for the `RestrictAnonymousAppDeployment` workspace protection rule.
//!
//! By default (no rule), any user with write access can deploy an app with
//! `execution_mode: anonymous` (no login required) — the historical
//! behavior. When a workspace protection ruleset enables
//! `RestrictAnonymousAppDeployment`, only workspace admins and the
//! ruleset's bypass users/groups can create an anonymous app or flip an
//! existing app to anonymous. To avoid breaking existing workflows,
//! restricted users can still redeploy an app that is already anonymous
//! (no exposure change) and can downgrade it back to `publisher`
//! (exposure reduction).
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)
//!   test-user-2 (non-admin, token SECRET_TOKEN_2)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::invalidate_protection_rules_cache;
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const USER_TOKEN: &str = "SECRET_TOKEN_2";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn app_payload(path: &str, execution_mode: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "Test app",
        "value": {},
        "policy": { "execution_mode": execution_mode, "triggerables": {} }
    })
}

fn policy_update(execution_mode: &str) -> serde_json::Value {
    json!({
        "policy": { "execution_mode": execution_mode, "triggerables": {} }
    })
}

/// Single test to avoid interference through the process-global protection
/// rules cache (keyed by workspace id, shared across parallel tests).
#[sqlx::test(fixtures("base"))]
async fn test_restrict_anonymous_app_deployment_rule(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    invalidate_protection_rules_cache("test-workspace");

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // ========================================
    // 1. Default behavior (no rule): a non-admin can create an anonymous
    //    app and flip an app to anonymous, as before.
    // ========================================

    let resp = authed(client().post(format!("{ws}/apps/create")), USER_TOKEN)
        .json(&app_payload("u/test-user-2/anon_default", "anonymous"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "without the rule, non-admin creating an anonymous app must succeed: {}",
        resp.text().await?
    );

    let resp = authed(client().post(format!("{ws}/apps/create")), USER_TOKEN)
        .json(&app_payload("u/test-user-2/test_app", "publisher"))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);

    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "without the rule, non-admin flipping an app to anonymous must succeed: {}",
        resp.text().await?
    );

    // back to publisher for the gated scenarios below
    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&policy_update("publisher"))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    // ========================================
    // 2. Admin enables the RestrictAnonymousAppDeployment rule.
    // ========================================

    let resp = authed(
        client().post(format!("{ws}/workspaces/protection_rules")),
        ADMIN_TOKEN,
    )
    .json(&json!({
        "name": "no-public-apps",
        "rules": ["RestrictAnonymousAppDeployment"],
        "bypass_users": [],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "admin should create the protection rule: {}",
        resp.text().await?
    );

    // ========================================
    // 3. With the rule, a non-admin cannot create an anonymous app...
    // ========================================

    let resp = authed(client().post(format!("{ws}/apps/create")), USER_TOKEN)
        .json(&app_payload("u/test-user-2/anon_blocked", "anonymous"))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 403,
        "with the rule, non-admin creating an anonymous app must be rejected: {body}"
    );
    assert!(
        body.contains("no-public-apps"),
        "error should name the blocking ruleset, got: {body}"
    );

    // ... nor flip an existing app to anonymous ...
    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "with the rule, non-admin flipping an app to anonymous must be rejected"
    );

    // ... while a publisher create still works.
    let resp = authed(client().post(format!("{ws}/apps/create")), USER_TOKEN)
        .json(&app_payload("u/test-user-2/pub_ok", "publisher"))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);

    // ========================================
    // 4. An admin is never blocked by the rule.
    // ========================================

    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        ADMIN_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "admin flipping an app to anonymous must succeed: {}",
        resp.text().await?
    );

    // ========================================
    // 5. A restricted user can still redeploy an app that is already
    //    anonymous — keeping it anonymous does not change its exposure, and
    //    blocking it would prevent non-admin editors from deploying public
    //    apps at all (the frontend always sends the full policy on deploy).
    // ========================================

    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&json!({
        "value": { "edited": true },
        "policy": { "execution_mode": "anonymous", "triggerables": {} }
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "restricted user redeploying an already-anonymous app must succeed: {}",
        resp.text().await?
    );

    // ========================================
    // 6. A restricted user can downgrade the app back to publisher
    //    (reduces exposure), but cannot re-flip it to anonymous.
    // ========================================

    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&policy_update("publisher"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "restricted user downgrading anonymous to publisher must succeed: {}",
        resp.text().await?
    );

    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "restricted user re-flipping to anonymous must be rejected"
    );

    // ========================================
    // 7. Bypass users are exempt from the rule.
    // ========================================

    let resp = authed(
        client().post(format!("{ws}/workspaces/protection_rules/no-public-apps")),
        ADMIN_TOKEN,
    )
    .json(&json!({
        "rules": ["RestrictAnonymousAppDeployment"],
        "bypass_users": ["test-user-2"],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "admin should update the protection rule: {}",
        resp.text().await?
    );

    let resp = authed(
        client().post(format!("{ws}/apps/update/u/test-user-2/test_app")),
        USER_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "bypass user flipping an app to anonymous must succeed: {}",
        resp.text().await?
    );

    Ok(())
}

/// Seed a minimal deployed script so `execute_component` can resolve `script/<path>`.
async fn seed_script(db: &Pool<Postgres>, path: &str, content: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content,
                                created_by, on_behalf_of_email, language, tag, lock)
           VALUES ('test-workspace', hashtext($2)::bigint, $1, '', '', $2, 'test-user', 'test@windmill.dev',
                   'deno'::script_lang, 'deno', '')"#,
    )
    .bind(path)
    .bind(content)
    .execute(db)
    .await?;
    // Isolated test DBs reuse script paths; the process-global deployed-script cache
    // is keyed by (workspace, path), so disable it to resolve against this test's DB.
    windmill_common::DEPLOYED_SCRIPT_CACHE_DISABLED
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// Anonymous (no token) versioned execute of component `c` -> `script/u/test-user/noop`.
async fn execute_anon_versioned(
    ws: &str,
    app: &str,
    version: i64,
) -> anyhow::Result<reqwest::Response> {
    Ok(client()
        .post(format!("{ws}/apps_u/execute_component/{app}"))
        .json(&json!({ "version": version, "component": "c", "path": "script/u/test-user/noop", "args": {} }))
        .send()
        .await?)
}

/// Create an anonymous app wired to run `script/<trigger>` via component `c`; return
/// its version id.
async fn create_anon_app(
    ws: &str,
    db: &Pool<Postgres>,
    path: &str,
    trigger: &str,
) -> anyhow::Result<i64> {
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": path,
            "summary": "",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables_v2": { format!("c:script/{trigger}"): { "static_inputs": {}, "one_of_inputs": {} } }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create {path}: {}", resp.text().await?);
    Ok(sqlx::query_scalar(
        "SELECT versions[array_upper(versions, 1)] FROM app WHERE path = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(path)
    .fetch_one(db)
    .await?)
}

/// Dispatch pending `notify_app_policy_change` events the way the server poller
/// (`process_notify_event`) does, invalidating the affected app policy caches.
async fn drain_policy_invalidations(db: &Pool<Postgres>) -> anyhow::Result<usize> {
    let events = windmill_common::notify_events::poll_notify_events(db, 0).await?;
    let mut n = 0;
    for e in &events {
        if e.channel == "notify_app_policy_change" {
            if let Some((ws, path)) = e.payload.split_once(':') {
                windmill_api::invalidate_app_policy_cache(ws, path);
                n += 1;
            }
        }
    }
    Ok(n)
}

/// Revoking public access or deleting an app must reject a subsequent anonymous
/// execute once the `notify_app_policy_change` invalidation is processed — the
/// cache must not keep a revoked app publicly executable (GHSA-r5v4-cxh9-7qhq).
#[sqlx::test(fixtures("base"))]
async fn test_policy_change_invalidates_cache(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    const PATH: &str = "u/test-user/pub_app";
    seed_script(&db, "u/test-user/noop", "export function main() {}").await?;
    let version = create_anon_app(&ws, &db, PATH, "u/test-user/noop").await?;

    // Prime the cache with the anonymous policy.
    let resp = execute_anon_versioned(&ws, PATH, version).await?;
    assert_eq!(
        resp.status(),
        200,
        "anonymous execute while public: {}",
        resp.text().await?
    );

    // Revoke public access (anonymous -> publisher), keeping the runnable so the
    // rejection is the execution_mode gate, not a missing triggerable.
    let resp = authed(client().post(format!("{ws}/apps/update/{PATH}")), ADMIN_TOKEN)
        .json(&json!({
            "policy": {
                "execution_mode": "publisher",
                "triggerables_v2": { "c:script/u/test-user/noop": { "static_inputs": {}, "one_of_inputs": {} } }
            }
        }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "revoke to publisher: {}",
        resp.text().await?
    );

    // The policy change must emit an invalidation event; dispatch it as the poller would.
    assert_eq!(
        drain_policy_invalidations(&db).await?,
        1,
        "policy update must emit one invalidation"
    );

    let resp = execute_anon_versioned(&ws, PATH, version).await?;
    let body = resp.text().await?;
    assert!(
        body.contains("publisher execution mode requires authentication"),
        "revoked app must reject anonymous execution, got: {body}"
    );

    // Deleting the app must likewise invalidate and then reject.
    let resp = authed(
        client().delete(format!("{ws}/apps/delete/{PATH}")),
        ADMIN_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete app: {}", resp.text().await?);
    assert!(
        drain_policy_invalidations(&db).await? >= 1,
        "delete must emit an invalidation"
    );

    let resp = execute_anon_versioned(&ws, PATH, version).await?;
    assert!(
        !resp.status().is_success(),
        "deleted app must reject execution, got {}",
        resp.status()
    );

    Ok(())
}

/// The cache is keyed by `(workspace, path)`, so a version id executed against a
/// different app resolves that app's own policy, never the primed one
/// (GHSA-r5v4-cxh9-7qhq).
#[sqlx::test(fixtures("base"))]
async fn test_versioned_execute_not_replayable_across_apps(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    seed_script(&db, "u/test-user/noop", "export function main() {}").await?;
    // App A may run script/noop; app B is wired to a different runnable.
    let version_a = create_anon_app(&ws, &db, "u/test-user/app_a", "u/test-user/noop").await?;
    create_anon_app(&ws, &db, "u/test-user/app_b", "u/test-user/other").await?;

    let resp = execute_anon_versioned(&ws, "u/test-user/app_a", version_a).await?;
    assert_eq!(resp.status(), 200, "prime app_a: {}", resp.text().await?);

    // Replaying app A's version id against app B must resolve B's own policy.
    let resp = execute_anon_versioned(&ws, "u/test-user/app_b", version_a).await?;
    assert!(
        !resp.status().is_success(),
        "app_b must not run app_a's runnable, got {}",
        resp.status()
    );

    Ok(())
}

/// A version change (a redeploy) is a client-visible freshness signal: the policy
/// cache refetches when the caller's version differs from the cached one, so the
/// new policy takes effect immediately without waiting for the notify invalidation
/// (GHSA-r5v4-cxh9-7qhq).
#[sqlx::test(fixtures("base"))]
async fn test_version_change_refetches_policy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    const PATH: &str = "u/test-user/redeployed_app";
    seed_script(&db, "u/test-user/noop", "export function main() {}").await?;
    let version = create_anon_app(&ws, &db, PATH, "u/test-user/noop").await?;

    // Prime the cache under `version`.
    let resp = execute_anon_versioned(&ws, PATH, version).await?;
    assert_eq!(resp.status(), 200, "prime: {}", resp.text().await?);

    // Revoke public access WITHOUT dispatching the notify invalidation.
    let resp = authed(client().post(format!("{ws}/apps/update/{PATH}")), ADMIN_TOKEN)
        .json(&json!({
            "policy": {
                "execution_mode": "publisher",
                "triggerables_v2": { "c:script/u/test-user/noop": { "static_inputs": {}, "one_of_inputs": {} } }
            }
        }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "revoke to publisher: {}",
        resp.text().await?
    );

    // Executing with a bumped version (as a redeploy would) refetches despite the
    // stale cache entry — no invalidation was processed.
    let resp = execute_anon_versioned(&ws, PATH, version + 1).await?;
    let body = resp.text().await?;
    assert!(
        body.contains("publisher execution mode requires authentication"),
        "a version change must refetch the live policy, got: {body}"
    );

    Ok(())
}
