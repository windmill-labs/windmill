//! Integration tests for workspace protection rulesets.
//!
//! Tests verify that DisableDirectDeployment and RestrictDeployToDeployers
//! protection rules correctly block/allow operations based on user permissions.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::invalidate_protection_rules_cache;

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_script(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "content": "export async function main() { return 42; }",
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

fn new_flow(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "value": { "modules": [] },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

/// Comprehensive test for protection rules functionality.
/// Tests all essential cases in a single test to avoid cache interference.
#[sqlx::test(fixtures("base"))]
async fn test_protection_rules(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    invalidate_protection_rules_cache("test-workspace");

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // ========================================
    // 1. Without protection rule, non-admin can create scripts and flows
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_script("u/test-user-2/script_no_rule", "No rule"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script without rule: {}",
        resp.text().await?
    );

    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_flow("u/test-user-2/flow_no_rule", "No rule"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create flow without rule: {}",
        resp.text().await?
    );

    // ========================================
    // 2. Non-admin cannot create protection rules
    // ========================================

    let resp = authed(
        client().post(format!("{base}/workspaces/protection_rules")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "name": "unauthorized-rule",
        "rules": ["DisableDirectDeployment"],
        "bypass_users": [],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "Non-admin should not create rules: {}",
        resp.status()
    );

    // ========================================
    // 3. Admin creates protection rule
    // ========================================

    let resp = authed(
        client().post(format!("{base}/workspaces/protection_rules")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "name": "test-rule",
        "rules": ["DisableDirectDeployment"],
        "bypass_users": [],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should create rule: {}",
        resp.text().await?
    );

    // ========================================
    // 4. With rule, non-admin is blocked from creating scripts/flows
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_script("u/test-user-2/blocked_script", "Blocked"))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "Non-admin should be blocked from scripts: {}",
        resp.status()
    );
    let body = resp.text().await?;
    assert!(
        body.contains("blocked") || body.contains("Blocked"),
        "Error should mention blocking: {}",
        body
    );

    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_flow("u/test-user-2/blocked_flow", "Blocked"))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "Non-admin should be blocked from flows: {}",
        resp.status()
    );

    // ========================================
    // 5. Admin bypasses protection rule
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script("u/test-user/admin_script", "Admin"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should bypass rule: {}",
        resp.text().await?
    );

    // ========================================
    // 6. Update rule to bypass test-user-2
    // ========================================

    let resp = authed(
        client().post(format!("{base}/workspaces/protection_rules/test-rule")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "rules": ["DisableDirectDeployment"],
        "bypass_users": ["test-user-2"],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Should update rule: {}",
        resp.text().await?
    );

    // Invalidate cache to pick up the update
    invalidate_protection_rules_cache("test-workspace");

    // ========================================
    // 7. Bypassed user can now create
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_script("u/test-user-2/bypassed_script", "Bypassed"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Bypassed user should create: {}",
        resp.text().await?
    );

    // ========================================
    // 8. Non-bypassed user (test-user-3) is still blocked
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_3",
    )
    .json(&new_script("u/test-user-3/still_blocked", "Blocked"))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "Non-bypassed user should be blocked: {}",
        resp.status()
    );

    // ========================================
    // 9. Delete rule
    // ========================================

    let resp = authed(
        client().delete(format!("{base}/workspaces/protection_rules/test-rule")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Should delete rule: {}",
        resp.text().await?
    );

    // Invalidate cache to pick up the deletion
    invalidate_protection_rules_cache("test-workspace");

    // ========================================
    // 10. After deletion, non-admin can create again
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_3",
    )
    .json(&new_script("u/test-user-3/after_delete", "After delete"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create after rule deletion: {}",
        resp.text().await?
    );

    // ========================================
    // 11. Verify rule list is empty
    // ========================================

    let resp = authed(
        client().get(format!("{base}/workspaces/protection_rules")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let rules: Vec<serde_json::Value> = resp.json().await?;
    assert!(rules.is_empty(), "Should have no rules after deletion");

    Ok(())
}

/// Test the `RestrictDeployToDeployers` rule.
///
/// Admins and members of the `wm_deployers` group can deploy, everyone else
/// is blocked. Uses a dedicated workspace + token prefixes so it doesn't
/// race against `test_protection_rules` on the shared PROTECTION_RULES_CACHE
/// and AUTH_CACHE lazy_statics.
#[sqlx::test(fixtures("restrict_deploy_to_deployers"))]
async fn test_restrict_deploy_to_deployers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    invalidate_protection_rules_cache("rdd-ws");

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/rdd-ws");

    // Admin creates the rule.
    let resp = authed(
        client().post(format!("{base}/workspaces/protection_rules")),
        "RDD_ADMIN_TOKEN",
    )
    .json(&json!({
        "name": "deployers-only",
        "rules": ["RestrictDeployToDeployers"],
        "bypass_users": [],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should create rule: {}",
        resp.text().await?
    );

    // Admin can still deploy.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "RDD_ADMIN_TOKEN",
    )
    .json(&new_script("u/rdd-admin/admin_deploys", "admin"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should deploy: {}",
        resp.text().await?
    );

    // wm_deployers member can deploy.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "RDD_DEPLOYER_TOKEN",
    )
    .json(&new_script("u/rdd-deployer/deployer_deploys", "deployer"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "wm_deployers member should deploy: {}",
        resp.text().await?
    );

    // Regular non-deployer is blocked.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "RDD_USER_TOKEN",
    )
    .json(&new_script("u/rdd-user/blocked", "blocked"))
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "non-deployer should be blocked: {}",
        resp.status()
    );
    let body = resp.text().await?;
    assert!(
        body.contains("wm_deployers"),
        "Error should mention wm_deployers: {}",
        body
    );

    // Extend the rule with the user as a bypass_user — they can then deploy.
    let resp = authed(
        client().post(format!("{base}/workspaces/protection_rules/deployers-only")),
        "RDD_ADMIN_TOKEN",
    )
    .json(&json!({
        "rules": ["RestrictDeployToDeployers"],
        "bypass_users": ["rdd-user"],
        "bypass_groups": []
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "Should update rule");
    invalidate_protection_rules_cache("rdd-ws");

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "RDD_USER_TOKEN",
    )
    .json(&new_script("u/rdd-user/bypassed", "bypassed"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "bypass_users should allow deploy: {}",
        resp.text().await?
    );

    Ok(())
}
