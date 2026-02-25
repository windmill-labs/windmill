//! Integration tests for preserve_on_behalf_of functionality.
//!
//! Tests verify that when deploying scripts, flows, apps, schedules, and triggers:
//! - Admin users can preserve the original on_behalf_of/email values
//! - Users in the wm_deployers group can preserve these values
//! - Regular users cannot preserve and their email is used instead

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

// ============================================================================
// Script Tests
// ============================================================================

fn new_script_with_on_behalf_of(
    path: &str,
    on_behalf_of_email: Option<&str>,
    preserve: bool,
) -> serde_json::Value {
    let mut script = json!({
        "path": path,
        "summary": "Test script",
        "description": "",
        "content": "export async function main() { return 42; }",
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    });
    if let Some(email) = on_behalf_of_email {
        script["on_behalf_of_email"] = json!(email);
    }
    if preserve {
        script["preserve_on_behalf_of"] = json!(true);
    }
    script
}

// ============================================================================
// Flow Tests
// ============================================================================

fn new_flow_with_on_behalf_of(
    path: &str,
    on_behalf_of_email: Option<&str>,
    preserve: bool,
) -> serde_json::Value {
    let mut flow = json!({
        "path": path,
        "summary": "Test flow",
        "description": "",
        "value": { "modules": [] },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    });
    if let Some(email) = on_behalf_of_email {
        flow["on_behalf_of_email"] = json!(email);
    }
    if preserve {
        flow["preserve_on_behalf_of"] = json!(true);
    }
    flow
}

// ============================================================================
// App Tests
// ============================================================================

fn new_app_with_on_behalf_of(
    path: &str,
    on_behalf_of: Option<&str>,
    on_behalf_of_email: Option<&str>,
    preserve: bool,
) -> serde_json::Value {
    let mut policy = json!({
        "execution_mode": "anonymous",
        "triggerables": {}
    });
    if let Some(obo) = on_behalf_of {
        policy["on_behalf_of"] = json!(obo);
    }
    if let Some(email) = on_behalf_of_email {
        policy["on_behalf_of_email"] = json!(email);
    }

    let mut app = json!({
        "path": path,
        "summary": "Test app",
        "value": {
            "type": "rawapp",
            "inline_script": null
        },
        "policy": policy
    });
    if preserve {
        app["preserve_on_behalf_of"] = json!(true);
    }
    app
}

// ============================================================================
// HTTP Trigger Helpers
// ============================================================================

#[cfg(feature = "http_trigger")]
fn new_http_trigger(
    path: &str,
    script_path: &str,
    route_path: &str,
    email: Option<&str>,
    preserve: bool,
) -> serde_json::Value {
    let mut trigger = json!({
        "path": path,
        "script_path": script_path,
        "is_flow": false,
        "route_path": route_path,
        "request_type": "async",
        "authentication_method": "none",
        "http_method": "post",
        "is_static_website": false,
        "workspaced_route": false,
        "wrap_body": false,
        "raw_string": false
    });
    if let Some(e) = email {
        trigger["email"] = json!(e);
    }
    if preserve {
        trigger["preserve_email"] = json!(true);
    }
    trigger
}

// ============================================================================
// WebSocket Trigger Helpers
// ============================================================================

#[cfg(feature = "websocket")]
fn new_websocket_trigger(
    path: &str,
    script_path: &str,
    email: Option<&str>,
    preserve: bool,
) -> serde_json::Value {
    let mut trigger = json!({
        "path": path,
        "script_path": script_path,
        "is_flow": false,
        "url": "wss://echo.websocket.org",
        "filters": [],
        "can_return_message": false,
        "can_return_error_result": false
    });
    if let Some(e) = email {
        trigger["email"] = json!(e);
    }
    if preserve {
        trigger["preserve_email"] = json!(true);
    }
    trigger
}

// ============================================================================
// Comprehensive Test
// ============================================================================

/// Comprehensive test for preserve_on_behalf_of functionality.
/// Tests all entity types in a single test to minimize overhead.
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_preserve_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // ========================================
    // 1. Script: Admin preserves on_behalf_of_email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user/script_admin_preserve",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2",
        "u/test-user/script_admin_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Admin should preserve on_behalf_of_email"
    );

    // ========================================
    // 2. Script: Deployer (wm_deployers group) preserves on_behalf_of_email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/deployer-user/script_deployer_preserve",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Deployer should create script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/script_deployer_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Deployer should preserve on_behalf_of_email"
    );

    // ========================================
    // 3. Script: Non-admin without wm_deployers cannot preserve
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user-2/script_no_preserve",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Non-admin should create script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/script_no_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("test2@windmill.dev"),
        "Non-admin should have their own email as on_behalf_of_email"
    );

    // ========================================
    // 4. Flow: Admin preserves on_behalf_of_email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN",
    )
    .json(&new_flow_with_on_behalf_of(
        "u/test-user/flow_admin_preserve",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create flow: {}",
        resp.text().await?
    );

    let flow = sqlx::query!(
        "SELECT on_behalf_of_email FROM flow WHERE path = $1 AND workspace_id = $2",
        "u/test-user/flow_admin_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        flow.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Admin should preserve flow on_behalf_of_email"
    );

    // ========================================
    // 5. Flow: Deployer preserves on_behalf_of_email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&new_flow_with_on_behalf_of(
        "u/deployer-user/flow_deployer_preserve",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Deployer should create flow: {}",
        resp.text().await?
    );

    let flow = sqlx::query!(
        "SELECT on_behalf_of_email FROM flow WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/flow_deployer_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        flow.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Deployer should preserve flow on_behalf_of_email"
    );

    // ========================================
    // 6. Flow: Non-admin cannot preserve
    // ========================================

    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_flow_with_on_behalf_of(
        "u/test-user-2/flow_no_preserve",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Non-admin should create flow: {}",
        resp.text().await?
    );

    let flow = sqlx::query!(
        "SELECT on_behalf_of_email FROM flow WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/flow_no_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        flow.on_behalf_of_email.as_deref(),
        Some("test2@windmill.dev"),
        "Non-admin should have their own email as flow on_behalf_of_email"
    );

    // ========================================
    // 7. App: Admin preserves on_behalf_of
    // ========================================

    let resp = authed(client().post(format!("{base}/apps/create")), "SECRET_TOKEN")
        .json(&new_app_with_on_behalf_of(
            "u/test-user/app_admin_preserve",
            Some("u/original-user"),
            Some("original@windmill.dev"),
            true,
        ))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create app: {}",
        resp.text().await?
    );

    let app = sqlx::query!(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2",
        "u/test-user/app_admin_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    let policy = app.policy;
    assert_eq!(
        policy.get("on_behalf_of").and_then(|v| v.as_str()),
        Some("u/original-user"),
        "Admin should preserve app on_behalf_of"
    );
    assert_eq!(
        policy.get("on_behalf_of_email").and_then(|v| v.as_str()),
        Some("original@windmill.dev"),
        "Admin should preserve app on_behalf_of_email"
    );

    // ========================================
    // 8. App: Deployer preserves on_behalf_of
    // ========================================

    let resp = authed(
        client().post(format!("{base}/apps/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&new_app_with_on_behalf_of(
        "u/deployer-user/app_deployer_preserve",
        Some("u/original-user"),
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Deployer should create app: {}",
        resp.text().await?
    );

    let app = sqlx::query!(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/app_deployer_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    let policy = app.policy;
    assert_eq!(
        policy.get("on_behalf_of").and_then(|v| v.as_str()),
        Some("u/original-user"),
        "Deployer should preserve app on_behalf_of"
    );
    assert_eq!(
        policy.get("on_behalf_of_email").and_then(|v| v.as_str()),
        Some("original@windmill.dev"),
        "Deployer should preserve app on_behalf_of_email"
    );

    // ========================================
    // 9. App: Non-admin cannot preserve
    // ========================================

    let resp = authed(
        client().post(format!("{base}/apps/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_app_with_on_behalf_of(
        "u/test-user-2/app_no_preserve",
        Some("u/original-user"),
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Non-admin should create app: {}",
        resp.text().await?
    );

    let app = sqlx::query!(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/app_no_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    let policy = app.policy;
    assert_eq!(
        policy.get("on_behalf_of").and_then(|v| v.as_str()),
        Some("u/test-user-2"),
        "Non-admin should have their own permissioned_as as app on_behalf_of"
    );
    assert_eq!(
        policy.get("on_behalf_of_email").and_then(|v| v.as_str()),
        Some("test2@windmill.dev"),
        "Non-admin should have their own email as app on_behalf_of_email"
    );

    // ========================================
    // 10. Schedule: Admin preserves email and edited_by
    // ========================================

    // First create a script for the schedule to reference
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user/scheduled_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create scheduled script: {}",
        resp.text().await?
    );

    let resp = authed(
        client().post(format!("{base}/schedules/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/schedule_admin_preserve",
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC",
        "script_path": "u/test-user/scheduled_script",
        "is_flow": false,
        "enabled": false,
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should create schedule: {}",
        resp.text().await?
    );

    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/test-user/schedule_admin_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        schedule.email, "original@windmill.dev",
        "Admin should preserve schedule email"
    );
    assert_eq!(
        schedule.edited_by, "original-user",
        "Admin should preserve schedule edited_by (looked up from email)"
    );

    // ========================================
    // 11. Schedule: Deployer preserves email and edited_by
    // ========================================

    // Create script for deployer
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/deployer-user/scheduled_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create scheduled script: {}",
        resp.text().await?
    );

    let resp = authed(
        client().post(format!("{base}/schedules/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&json!({
        "path": "u/deployer-user/schedule_deployer_preserve",
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC",
        "script_path": "u/deployer-user/scheduled_script",
        "is_flow": false,
        "enabled": false,
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Deployer should create schedule: {}",
        resp.text().await?
    );

    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/schedule_deployer_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        schedule.email, "original@windmill.dev",
        "Deployer should preserve schedule email"
    );
    assert_eq!(
        schedule.edited_by, "original-user",
        "Deployer should preserve schedule edited_by"
    );

    // ========================================
    // 12. Schedule: Non-admin cannot preserve
    // ========================================

    // Create script for test-user-2
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user-2/scheduled_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create scheduled script: {}",
        resp.text().await?
    );

    let resp = authed(
        client().post(format!("{base}/schedules/create")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "path": "u/test-user-2/schedule_no_preserve",
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC",
        "script_path": "u/test-user-2/scheduled_script",
        "is_flow": false,
        "enabled": false,
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Non-admin should create schedule: {}",
        resp.text().await?
    );

    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/schedule_no_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        schedule.email, "test2@windmill.dev",
        "Non-admin should have their own email"
    );
    assert_eq!(
        schedule.edited_by, "test-user-2",
        "Non-admin should have their own username as edited_by"
    );

    // ========================================
    // 13. Script: Without preserve flag, email is NOT preserved
    // ========================================

    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user/script_no_flag",
        Some("original@windmill.dev"),
        false, // preserve_on_behalf_of = false
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2",
        "u/test-user/script_no_flag",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("test@windmill.dev"),
        "Without preserve flag, admin's email should be used"
    );

    Ok(())
}

/// Helper to build a script JSON with custom content (to avoid hash conflicts on same path)
fn script_json(
    path: &str,
    content: &str,
    on_behalf_of_email: Option<&str>,
    preserve: bool,
) -> serde_json::Value {
    let mut script = json!({
        "path": path,
        "summary": "Test script",
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    });
    if let Some(email) = on_behalf_of_email {
        script["on_behalf_of_email"] = json!(email);
    }
    if preserve {
        script["preserve_on_behalf_of"] = json!(true);
    }
    script
}

/// Test script update preserves on_behalf_of_email correctly
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_script_update_preserves_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // ========================================
    // Admin updates with preserve flag
    // ========================================

    // Original-user creates initial version
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&script_json(
        "u/original-user/script_to_update",
        "export async function main() { return 1; }",
        Some("original@windmill.dev"),
        false,
    ))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    let parent_hash: String = resp.text().await?;

    // Admin creates new version with preserve, passing parent_hash
    let mut update = script_json(
        "u/original-user/script_to_update",
        "export async function main() { return 2; }",
        Some("original@windmill.dev"),
        true,
    );
    update["parent_hash"] = json!(parent_hash);
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&update)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should update script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
        "u/original-user/script_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Admin update should preserve script on_behalf_of_email"
    );

    // ========================================
    // Deployer updates with preserve flag
    // ========================================

    // Admin creates initial version under deployer's path with preserve
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&script_json(
        "u/deployer-user/script_deploy_update",
        "export async function main() { return 3; }",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    let parent_hash: String = resp.text().await?;

    // Deployer creates new version with preserve
    let mut update = script_json(
        "u/deployer-user/script_deploy_update",
        "export async function main() { return 4; }",
        Some("original@windmill.dev"),
        true,
    );
    update["parent_hash"] = json!(parent_hash);
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&update)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Deployer should update script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
        "u/deployer-user/script_deploy_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Deployer update should preserve script on_behalf_of_email"
    );

    // ========================================
    // Non-admin cannot preserve on update
    // ========================================

    // Admin creates initial version under non-admin's path with preserve
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&script_json(
        "u/test-user-2/script_nonadmin_update",
        "export async function main() { return 5; }",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    let parent_hash: String = resp.text().await?;

    // Non-admin creates new version with preserve (should be denied)
    let mut update = script_json(
        "u/test-user-2/script_nonadmin_update",
        "export async function main() { return 6; }",
        Some("original@windmill.dev"),
        true,
    );
    update["parent_hash"] = json!(parent_hash);
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN_2",
    )
    .json(&update)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Non-admin should update script: {}",
        resp.text().await?
    );

    let script = sqlx::query!(
        "SELECT on_behalf_of_email FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
        "u/test-user-2/script_nonadmin_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        script.on_behalf_of_email.as_deref(),
        Some("test2@windmill.dev"),
        "Non-admin update should overwrite script on_behalf_of_email with their own"
    );

    Ok(())
}

/// Test flow update preserves on_behalf_of_email correctly
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_flow_update_preserves_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create initial flow with original email
    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&new_flow_with_on_behalf_of(
        "u/original-user/flow_to_update",
        Some("original@windmill.dev"),
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create flow: {}",
        resp.text().await?
    );

    // Admin updates with preserve flag
    let resp = authed(
        client().post(format!(
            "{base}/flows/update/u/original-user/flow_to_update"
        )),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/original-user/flow_to_update",
        "summary": "Updated flow",
        "description": "",
        "value": { "modules": [] },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        },
        "on_behalf_of_email": "original@windmill.dev",
        "preserve_on_behalf_of": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should update flow: {}",
        resp.text().await?
    );

    let flow = sqlx::query!(
        "SELECT on_behalf_of_email FROM flow WHERE path = $1 AND workspace_id = $2",
        "u/original-user/flow_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        flow.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Admin update should preserve flow on_behalf_of_email"
    );

    // ========================================
    // Deployer updates with preserve flag
    // ========================================

    // Admin creates flow under deployer's path with preserve
    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN",
    )
    .json(&new_flow_with_on_behalf_of(
        "u/deployer-user/flow_deploy_update",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create flow: {}",
        resp.text().await?
    );

    // Deployer updates at their own path with preserve
    let resp = authed(
        client().post(format!(
            "{base}/flows/update/u/deployer-user/flow_deploy_update"
        )),
        "DEPLOYER_TOKEN",
    )
    .json(&json!({
        "path": "u/deployer-user/flow_deploy_update",
        "summary": "Deployer updated flow",
        "description": "",
        "value": { "modules": [] },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        },
        "on_behalf_of_email": "original@windmill.dev",
        "preserve_on_behalf_of": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Deployer should update flow: {}",
        resp.text().await?
    );

    let flow = sqlx::query!(
        "SELECT on_behalf_of_email FROM flow WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/flow_deploy_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        flow.on_behalf_of_email.as_deref(),
        Some("original@windmill.dev"),
        "Deployer update should preserve flow on_behalf_of_email"
    );

    // ========================================
    // Non-admin cannot preserve on update
    // ========================================

    // Admin creates flow under non-admin's path with preserve
    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN",
    )
    .json(&new_flow_with_on_behalf_of(
        "u/test-user-2/flow_nonadmin_update",
        Some("original@windmill.dev"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create flow: {}",
        resp.text().await?
    );

    // Non-admin updates at their own path with preserve (should be denied)
    let resp = authed(
        client().post(format!(
            "{base}/flows/update/u/test-user-2/flow_nonadmin_update"
        )),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "path": "u/test-user-2/flow_nonadmin_update",
        "summary": "Non-admin updated flow",
        "description": "",
        "value": { "modules": [] },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        },
        "on_behalf_of_email": "original@windmill.dev",
        "preserve_on_behalf_of": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Non-admin should update flow: {}",
        resp.text().await?
    );

    let flow = sqlx::query!(
        "SELECT on_behalf_of_email FROM flow WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/flow_nonadmin_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        flow.on_behalf_of_email.as_deref(),
        Some("test2@windmill.dev"),
        "Non-admin update should overwrite flow on_behalf_of_email with their own"
    );

    Ok(())
}

/// Test app update preserves on_behalf_of correctly
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_app_update_preserves_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create initial app
    let resp = authed(
        client().post(format!("{base}/apps/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&new_app_with_on_behalf_of(
        "u/original-user/app_to_update",
        Some("u/original-user"),
        Some("original@windmill.dev"),
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create app: {}",
        resp.text().await?
    );

    // Admin updates with preserve flag
    let resp = authed(
        client().post(format!("{base}/apps/update/u/original-user/app_to_update")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "summary": "Updated app",
        "policy": {
            "execution_mode": "anonymous",
            "triggerables": {},
            "on_behalf_of": "u/original-user",
            "on_behalf_of_email": "original@windmill.dev"
        },
        "preserve_on_behalf_of": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should update app: {}",
        resp.text().await?
    );

    let app = sqlx::query!(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2",
        "u/original-user/app_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    let policy = app.policy;
    assert_eq!(
        policy.get("on_behalf_of").and_then(|v| v.as_str()),
        Some("u/original-user"),
        "Admin update should preserve app on_behalf_of"
    );
    assert_eq!(
        policy.get("on_behalf_of_email").and_then(|v| v.as_str()),
        Some("original@windmill.dev"),
        "Admin update should preserve app on_behalf_of_email"
    );

    // ========================================
    // Deployer updates with preserve flag
    // ========================================

    // Admin creates app under deployer's path with preserve
    let resp = authed(client().post(format!("{base}/apps/create")), "SECRET_TOKEN")
        .json(&new_app_with_on_behalf_of(
            "u/deployer-user/app_deploy_update",
            Some("u/original-user"),
            Some("original@windmill.dev"),
            true,
        ))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create app: {}",
        resp.text().await?
    );

    // Deployer updates at their own path with preserve
    let resp = authed(
        client().post(format!(
            "{base}/apps/update/u/deployer-user/app_deploy_update"
        )),
        "DEPLOYER_TOKEN",
    )
    .json(&json!({
        "summary": "Deployer updated app",
        "policy": {
            "execution_mode": "anonymous",
            "triggerables": {},
            "on_behalf_of": "u/original-user",
            "on_behalf_of_email": "original@windmill.dev"
        },
        "preserve_on_behalf_of": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Deployer should update app: {}",
        resp.text().await?
    );

    let app = sqlx::query!(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/app_deploy_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    let policy = app.policy;
    assert_eq!(
        policy.get("on_behalf_of").and_then(|v| v.as_str()),
        Some("u/original-user"),
        "Deployer update should preserve app on_behalf_of"
    );
    assert_eq!(
        policy.get("on_behalf_of_email").and_then(|v| v.as_str()),
        Some("original@windmill.dev"),
        "Deployer update should preserve app on_behalf_of_email"
    );

    // ========================================
    // Non-admin cannot preserve on update
    // ========================================

    // Admin creates app under non-admin's path with preserve
    let resp = authed(client().post(format!("{base}/apps/create")), "SECRET_TOKEN")
        .json(&new_app_with_on_behalf_of(
            "u/test-user-2/app_nonadmin_update",
            Some("u/original-user"),
            Some("original@windmill.dev"),
            true,
        ))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create app: {}",
        resp.text().await?
    );

    // Non-admin updates at their own path with preserve (should be denied)
    let resp = authed(
        client().post(format!(
            "{base}/apps/update/u/test-user-2/app_nonadmin_update"
        )),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "summary": "Non-admin updated app",
        "policy": {
            "execution_mode": "anonymous",
            "triggerables": {},
            "on_behalf_of": "u/original-user",
            "on_behalf_of_email": "original@windmill.dev"
        },
        "preserve_on_behalf_of": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Non-admin should update app: {}",
        resp.text().await?
    );

    let app = sqlx::query!(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/app_nonadmin_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    let policy = app.policy;
    assert_eq!(
        policy.get("on_behalf_of").and_then(|v| v.as_str()),
        Some("u/test-user-2"),
        "Non-admin update should overwrite app on_behalf_of with their own"
    );
    assert_eq!(
        policy.get("on_behalf_of_email").and_then(|v| v.as_str()),
        Some("test2@windmill.dev"),
        "Non-admin update should overwrite app on_behalf_of_email with their own"
    );

    Ok(())
}

/// Test schedule update preserves email/edited_by correctly
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_schedule_update_preserves_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create script for schedule
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/original-user/scheduled_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Create initial schedule
    let resp = authed(
        client().post(format!("{base}/schedules/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&json!({
        "path": "u/original-user/schedule_to_update",
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC",
        "script_path": "u/original-user/scheduled_script",
        "is_flow": false,
        "enabled": false
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Should create schedule: {}",
        resp.text().await?
    );

    // Verify initial state
    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/original-user/schedule_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(schedule.email, "original@windmill.dev");
    assert_eq!(schedule.edited_by, "original-user");

    // Admin updates with preserve flag
    let resp = authed(
        client().post(format!(
            "{base}/schedules/update/u/original-user/schedule_to_update"
        )),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "schedule": "0 0 */12 * * *",
        "timezone": "UTC",
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should update schedule: {}",
        resp.text().await?
    );

    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/original-user/schedule_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        schedule.email, "original@windmill.dev",
        "Admin update should preserve schedule email"
    );
    assert_eq!(
        schedule.edited_by, "original-user",
        "Admin update should preserve schedule edited_by"
    );

    // ========================================
    // Deployer updates with preserve flag
    // ========================================

    // Create script under deployer's path
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/deployer-user/sched_deploy_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Admin creates schedule under deployer's path with preserve
    let resp = authed(
        client().post(format!("{base}/schedules/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/deployer-user/schedule_deploy_update",
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC",
        "script_path": "u/deployer-user/sched_deploy_script",
        "is_flow": false,
        "enabled": false,
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should create schedule: {}",
        resp.text().await?
    );

    // Deployer updates at their own path with preserve
    let resp = authed(
        client().post(format!(
            "{base}/schedules/update/u/deployer-user/schedule_deploy_update"
        )),
        "DEPLOYER_TOKEN",
    )
    .json(&json!({
        "schedule": "0 0 */8 * * *",
        "timezone": "UTC",
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Deployer should update schedule: {}",
        resp.text().await?
    );

    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/schedule_deploy_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        schedule.email, "original@windmill.dev",
        "Deployer update should preserve schedule email"
    );
    assert_eq!(
        schedule.edited_by, "original-user",
        "Deployer update should preserve schedule edited_by"
    );

    // ========================================
    // Non-admin cannot preserve on update
    // ========================================

    // Create script under non-admin's path
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user-2/sched_nonadmin_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Admin creates schedule under non-admin's path with preserve
    let resp = authed(
        client().post(format!("{base}/schedules/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user-2/schedule_nonadmin_update",
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC",
        "script_path": "u/test-user-2/sched_nonadmin_script",
        "is_flow": false,
        "enabled": false,
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should create schedule: {}",
        resp.text().await?
    );

    // Non-admin updates at their own path with preserve (should be denied)
    let resp = authed(
        client().post(format!(
            "{base}/schedules/update/u/test-user-2/schedule_nonadmin_update"
        )),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "schedule": "0 0 */4 * * *",
        "timezone": "UTC",
        "email": "original-user",
        "preserve_email": true
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Non-admin should update schedule: {}",
        resp.text().await?
    );

    let schedule = sqlx::query!(
        "SELECT email, edited_by FROM schedule WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/schedule_nonadmin_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    // When preserve is denied, resolve_email returns the authed user's email
    assert_eq!(
        schedule.email, "test2@windmill.dev",
        "Non-admin update should overwrite schedule email with their own"
    );

    Ok(())
}

// ============================================================================
// HTTP Trigger Tests
// ============================================================================
// All trigger types share the same BaseTriggerData.resolve_email() and
// resolve_edited_by() code path. Testing HTTP triggers validates the
// preservation logic for all trigger types (WebSocket, MQTT, PostgreSQL,
// Kafka, NATS, SQS, GCP, Email).

/// HTTP Trigger: admin preserve_email tests (HTTP triggers require admin)
#[cfg(feature = "http_trigger")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_http_trigger_preserve_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create script for triggers to reference (admin-only for HTTP triggers)
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user/trigger_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // ========================================
    // 1. Admin preserves email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/http_triggers/create")),
        "SECRET_TOKEN",
    )
    .json(&new_http_trigger(
        "u/test-user/http_admin_preserve",
        "u/test-user/trigger_script",
        "admin-preserve",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create http trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM http_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user/http_admin_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "original@windmill.dev",
        "Admin should preserve http trigger email"
    );
    assert_eq!(
        trigger.edited_by, "original-user",
        "Admin should preserve http trigger edited_by"
    );

    // ========================================
    // 2. Without preserve flag, email is NOT preserved
    // ========================================

    let resp = authed(
        client().post(format!("{base}/http_triggers/create")),
        "SECRET_TOKEN",
    )
    .json(&new_http_trigger(
        "u/test-user/http_no_flag",
        "u/test-user/trigger_script",
        "no-flag",
        Some("original-user"),
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create http trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM http_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user/http_no_flag",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "test@windmill.dev",
        "Without preserve flag, admin's own email should be used"
    );
    assert_eq!(
        trigger.edited_by, "test-user",
        "Without preserve flag, admin's own username should be used"
    );

    Ok(())
}

/// HTTP Trigger update: admin preserves email/edited_by (HTTP triggers require admin)
#[cfg(feature = "http_trigger")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_http_trigger_update_preserves_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create script for the trigger (admin creates everything for HTTP triggers)
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user/http_update_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Admin creates initial trigger without preserve (sets admin's own email)
    let resp = authed(
        client().post(format!("{base}/http_triggers/create")),
        "SECRET_TOKEN",
    )
    .json(&new_http_trigger(
        "u/test-user/http_to_update",
        "u/test-user/http_update_script",
        "to-update",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create http trigger: {}",
        resp.text().await?
    );

    // Verify initial state
    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM http_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user/http_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(trigger.email, "original@windmill.dev");
    assert_eq!(trigger.edited_by, "original-user");

    // Admin updates with preserve flag
    let resp = authed(
        client().post(format!(
            "{base}/http_triggers/update/u/test-user/http_to_update"
        )),
        "SECRET_TOKEN",
    )
    .json(&new_http_trigger(
        "u/test-user/http_to_update",
        "u/test-user/http_update_script",
        "to-update",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should update http trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM http_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user/http_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "original@windmill.dev",
        "Admin update should preserve http trigger email"
    );
    assert_eq!(
        trigger.edited_by, "original-user",
        "Admin update should preserve http trigger edited_by"
    );

    Ok(())
}

// ============================================================================
// WebSocket Trigger Tests
// ============================================================================

/// WebSocket Trigger: admin, deployer, and non-admin preserve_email tests
#[cfg(feature = "websocket")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_websocket_trigger_preserve_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create scripts for triggers to reference
    for (token, path) in [
        ("SECRET_TOKEN", "u/test-user/ws_script"),
        ("DEPLOYER_TOKEN", "u/deployer-user/ws_script"),
        ("SECRET_TOKEN_2", "u/test-user-2/ws_script"),
    ] {
        let resp = authed(client().post(format!("{base}/scripts/create")), token)
            .json(&new_script_with_on_behalf_of(path, None, false))
            .send()
            .await?;
        assert_eq!(
            resp.status(),
            201,
            "Should create script {path}: {}",
            resp.text().await?
        );
    }

    // ========================================
    // 1. Admin preserves email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/websocket_triggers/create")),
        "SECRET_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/test-user/ws_admin_preserve",
        "u/test-user/ws_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create websocket trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user/ws_admin_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "original@windmill.dev",
        "Admin should preserve websocket trigger email"
    );
    assert_eq!(
        trigger.edited_by, "original-user",
        "Admin should preserve websocket trigger edited_by"
    );

    // ========================================
    // 2. Deployer preserves email
    // ========================================

    let resp = authed(
        client().post(format!("{base}/websocket_triggers/create")),
        "DEPLOYER_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/deployer-user/ws_deployer_preserve",
        "u/deployer-user/ws_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Deployer should create websocket trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/ws_deployer_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "original@windmill.dev",
        "Deployer should preserve websocket trigger email"
    );
    assert_eq!(
        trigger.edited_by, "original-user",
        "Deployer should preserve websocket trigger edited_by"
    );

    // ========================================
    // 3. Non-admin cannot preserve
    // ========================================

    let resp = authed(
        client().post(format!("{base}/websocket_triggers/create")),
        "SECRET_TOKEN_2",
    )
    .json(&new_websocket_trigger(
        "u/test-user-2/ws_no_preserve",
        "u/test-user-2/ws_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Non-admin should create websocket trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/ws_no_preserve",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "test2@windmill.dev",
        "Non-admin should have their own email"
    );
    assert_eq!(
        trigger.edited_by, "test-user-2",
        "Non-admin should have their own username as edited_by"
    );

    Ok(())
}

/// WebSocket Trigger update: admin preserves email/edited_by
#[cfg(feature = "websocket")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_websocket_trigger_update_preserves_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create script
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/original-user/ws_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Create initial trigger
    let resp = authed(
        client().post(format!("{base}/websocket_triggers/create")),
        "ORIGINAL_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/original-user/ws_to_update",
        "u/original-user/ws_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create websocket trigger: {}",
        resp.text().await?
    );

    // Verify initial state
    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/original-user/ws_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(trigger.email, "original@windmill.dev");
    assert_eq!(trigger.edited_by, "original-user");

    // Admin updates with preserve flag
    let resp = authed(
        client().post(format!(
            "{base}/websocket_triggers/update/u/original-user/ws_to_update"
        )),
        "SECRET_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/original-user/ws_to_update",
        "u/original-user/ws_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Admin should update websocket trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/original-user/ws_to_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "original@windmill.dev",
        "Admin update should preserve websocket trigger email"
    );
    assert_eq!(
        trigger.edited_by, "original-user",
        "Admin update should preserve websocket trigger edited_by"
    );

    // ========================================
    // Deployer updates with preserve flag
    // ========================================

    // Create script under deployer's path
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/deployer-user/ws_deploy_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Admin creates trigger under deployer's path with preserve
    let resp = authed(
        client().post(format!("{base}/websocket_triggers/create")),
        "SECRET_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/deployer-user/ws_deploy_update",
        "u/deployer-user/ws_deploy_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create websocket trigger: {}",
        resp.text().await?
    );

    // Deployer updates at their own path with preserve
    let resp = authed(
        client().post(format!(
            "{base}/websocket_triggers/update/u/deployer-user/ws_deploy_update"
        )),
        "DEPLOYER_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/deployer-user/ws_deploy_update",
        "u/deployer-user/ws_deploy_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Deployer should update websocket trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/deployer-user/ws_deploy_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "original@windmill.dev",
        "Deployer update should preserve websocket trigger email"
    );
    assert_eq!(
        trigger.edited_by, "original-user",
        "Deployer update should preserve websocket trigger edited_by"
    );

    // ========================================
    // Non-admin cannot preserve on update
    // ========================================

    // Create script under non-admin's path
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_on_behalf_of(
        "u/test-user-2/ws_nonadmin_script",
        None,
        false,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Should create script: {}",
        resp.text().await?
    );

    // Admin creates trigger under non-admin's path with preserve
    let resp = authed(
        client().post(format!("{base}/websocket_triggers/create")),
        "SECRET_TOKEN",
    )
    .json(&new_websocket_trigger(
        "u/test-user-2/ws_nonadmin_update",
        "u/test-user-2/ws_nonadmin_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "Admin should create websocket trigger: {}",
        resp.text().await?
    );

    // Non-admin updates at their own path with preserve (should be denied)
    let resp = authed(
        client().post(format!(
            "{base}/websocket_triggers/update/u/test-user-2/ws_nonadmin_update"
        )),
        "SECRET_TOKEN_2",
    )
    .json(&new_websocket_trigger(
        "u/test-user-2/ws_nonadmin_update",
        "u/test-user-2/ws_nonadmin_script",
        Some("original-user"),
        true,
    ))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "Non-admin should update websocket trigger: {}",
        resp.text().await?
    );

    let trigger = sqlx::query!(
        "SELECT email, edited_by FROM websocket_trigger WHERE path = $1 AND workspace_id = $2",
        "u/test-user-2/ws_nonadmin_update",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        trigger.email, "test2@windmill.dev",
        "Non-admin update should overwrite websocket trigger email with their own"
    );
    assert_eq!(
        trigger.edited_by, "test-user-2",
        "Non-admin update should overwrite websocket trigger edited_by with their own"
    );

    Ok(())
}
