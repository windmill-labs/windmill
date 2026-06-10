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
