/*!
 * Integration tests for the native trigger system (Google).
 *
 * Tests cover 4 business-logic areas:
 * 1. Resource path change — cleanup old path, recreate at new path
 * 2. Config loading — workspace-level, instance-level, token update
 * 3. Channel expiration renewal — should_renew_channel pure logic
 * 4. Delete workspace integration — full cascade, cleanup preserves triggers, parse_stop_channel_params
 */

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_api_auth::ApiAuthed;
use windmill_common::variables::{build_crypt, encrypt};
use windmill_native_triggers::{
    decrypt_oauth_data, delete_native_trigger, delete_workspace_integration,
    get_workspace_integration,
    google::{parse_stop_channel_params, should_renew_channel},
    store_native_trigger, store_workspace_integration, NativeTriggerConfig, OAuthConfig,
    ServiceName,
};

// ============================================================================
// Helpers
// ============================================================================

async fn insert_test_script(db: &Pool<Postgres>, path: &str) -> anyhow::Result<i64> {
    let hash: i64 = rand::random::<i64>().unsigned_abs() as i64;
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content,
                  created_by, language, kind, lock)
         VALUES ('test-workspace', $1, $2, '', '', 'def main(): pass',
                  'test-user', 'python3', 'script', '')",
    )
    .bind(hash)
    .bind(path)
    .execute(db)
    .await?;
    Ok(hash)
}

fn test_authed() -> ApiAuthed {
    ApiAuthed {
        email: "test@windmill.dev".to_string(),
        username: "test-user".to_string(),
        is_admin: true,
        is_operator: false,
        groups: vec!["all".to_string()],
        folders: vec![],
        scopes: None,
        username_override: None,
        token_prefix: None,
    }
}

/// Set up a complete workspace integration with account+variable+resource.
/// Returns (resource_path, account_id).
async fn setup_oauth_integration(
    db: &Pool<Postgres>,
    service_name: ServiceName,
    resource_path: &str,
    access_token: &str,
    refresh_token: &str,
    oauth_data_override: Option<serde_json::Value>,
) -> anyhow::Result<i32> {
    // 1. Create account with is_workspace_integration=true
    let account_id: i32 = sqlx::query_scalar!(
        "INSERT INTO account (workspace_id, client, expires_at, refresh_token, is_workspace_integration)
         VALUES ('test-workspace', $1, now() + interval '1 hour', $2, true)
         RETURNING id",
        service_name.as_str(),
        refresh_token,
    )
    .fetch_one(db)
    .await?;

    // 2. Encrypt and create variable
    let mc = build_crypt(db, "test-workspace").await?;
    let encrypted = encrypt(&mc, access_token);

    sqlx::query!(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, account, is_oauth)
         VALUES ('test-workspace', $1, $2, true, 'test oauth token', $3, true)",
        resource_path,
        encrypted,
        account_id,
    )
    .execute(db)
    .await?;

    // 3. Create resource
    let resource_value = json!({ "token": format!("$var:{}", resource_path) });
    sqlx::query!(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', $1, $2, $3, '{}'::jsonb, 'test-user')",
        resource_path,
        resource_value,
        service_name.resource_type(),
    )
    .execute(db)
    .await?;

    // 4. Store workspace integration with resource_path
    let oauth_data = oauth_data_override.unwrap_or_else(|| {
        json!({
            "client_id": "test-client-id",
            "client_secret": "test-client-secret",
            "base_url": "https://example.com",
            "resource_path": resource_path,
        })
    });

    let authed = test_authed();
    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        service_name,
        oauth_data,
        Some(resource_path),
    )
    .await?;
    tx.commit().await?;

    Ok(account_id)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

// ============================================================================
// 1. Resource Path Change
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_resource_path_change(db: Pool<Postgres>) -> anyhow::Result<()> {
    let path_a = "u/test-user/native_gworkspace";
    setup_oauth_integration(
        &db,
        ServiceName::Google,
        path_a,
        "token-a",
        "refresh-a",
        None,
    )
    .await?;

    // Verify decrypt works at path A
    let config: OAuthConfig =
        decrypt_oauth_data(&db, "test-workspace", ServiceName::Google).await?;
    assert_eq!(config.access_token, "token-a");

    // Cleanup old path
    let mut tx = db.begin().await?;
    windmill_native_triggers::workspace_integrations::cleanup_oauth_resource(
        &mut *tx,
        "test-workspace",
        ServiceName::Google,
    )
    .await;
    tx.commit().await?;

    // Recreate at path B
    let path_b = "u/test-user/native_gworkspace_v2";
    setup_oauth_integration(
        &db,
        ServiceName::Google,
        path_b,
        "token-b",
        "refresh-b",
        None,
    )
    .await?;

    // Path A resources should be gone
    let var_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM variable WHERE workspace_id = 'test-workspace' AND path = $1",
        path_a,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(var_count, 0, "variable at old path should be deleted");

    let res_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM resource WHERE workspace_id = 'test-workspace' AND path = $1",
        path_a,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(res_count, 0, "resource at old path should be deleted");

    // Path B should work
    let config: OAuthConfig =
        decrypt_oauth_data(&db, "test-workspace", ServiceName::Google).await?;
    assert_eq!(config.access_token, "token-b");
    assert_eq!(config.refresh_token.as_deref(), Some("refresh-b"));

    Ok(())
}

// ============================================================================
// 2. Config Loading — workspace vs instance + token update
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_decrypt_workspace_level(db: Pool<Postgres>) -> anyhow::Result<()> {
    let resource_path = "u/test-user/native_gworkspace";
    setup_oauth_integration(
        &db,
        ServiceName::Google,
        resource_path,
        "ws-access-token",
        "ws-refresh-token",
        None,
    )
    .await?;

    let config: OAuthConfig =
        decrypt_oauth_data(&db, "test-workspace", ServiceName::Google).await?;

    assert_eq!(config.access_token, "ws-access-token");
    assert_eq!(config.refresh_token.as_deref(), Some("ws-refresh-token"));
    assert_eq!(config.client_id, "test-client-id");
    assert_eq!(config.client_secret, "test-client-secret");
    assert_eq!(config.base_url, "https://example.com");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_decrypt_instance_level(db: Pool<Postgres>) -> anyhow::Result<()> {
    // Insert instance-level credentials into global_settings
    sqlx::query!(
        "INSERT INTO global_settings (name, value) VALUES ('oauths', $1)
         ON CONFLICT (name) DO UPDATE SET value = $1",
        json!({
            "gworkspace": {
                "id": "instance-client-id",
                "secret": "instance-client-secret"
            }
        }),
    )
    .execute(&db)
    .await?;

    let resource_path = "u/test-user/native_gworkspace";
    let oauth_data = json!({
        "instance_shared": true,
        "base_url": "https://accounts.google.com",
        "resource_path": resource_path,
    });

    setup_oauth_integration(
        &db,
        ServiceName::Google,
        resource_path,
        "inst-access-token",
        "inst-refresh-token",
        Some(oauth_data),
    )
    .await?;

    let config: OAuthConfig =
        decrypt_oauth_data(&db, "test-workspace", ServiceName::Google).await?;

    assert_eq!(config.client_id, "instance-client-id");
    assert_eq!(config.client_secret, "instance-client-secret");
    assert_eq!(config.access_token, "inst-access-token");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_token_update_persists(db: Pool<Postgres>) -> anyhow::Result<()> {
    let resource_path = "u/test-user/native_gworkspace";
    let account_id = setup_oauth_integration(
        &db,
        ServiceName::Google,
        resource_path,
        "old-access-token",
        "old-refresh-token",
        None,
    )
    .await?;

    // Verify old tokens
    let config: OAuthConfig =
        decrypt_oauth_data(&db, "test-workspace", ServiceName::Google).await?;
    assert_eq!(config.access_token, "old-access-token");

    // Simulate token refresh: update variable + account
    let mc = build_crypt(&db, "test-workspace").await?;
    let new_encrypted = encrypt(&mc, "new-access-token");
    sqlx::query!(
        "UPDATE variable SET value = $1 WHERE workspace_id = 'test-workspace' AND path = $2",
        new_encrypted,
        resource_path,
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "UPDATE account SET refresh_token = $1 WHERE workspace_id = 'test-workspace' AND id = $2",
        "new-refresh-token",
        account_id,
    )
    .execute(&db)
    .await?;

    // Verify new tokens
    let config: OAuthConfig =
        decrypt_oauth_data(&db, "test-workspace", ServiceName::Google).await?;
    assert_eq!(config.access_token, "new-access-token");
    assert_eq!(config.refresh_token.as_deref(), Some("new-refresh-token"));

    Ok(())
}

// ============================================================================
// 3. Channel Expiration Renewal — should_renew_channel
// ============================================================================

#[test]
fn test_should_renew_drive_channel_expired() {
    let config = json!({
        "triggerType": "drive",
        "expiration": (now_ms() - 1000).to_string(),
    });
    assert!(should_renew_channel(&config));
}

#[test]
fn test_should_renew_drive_channel_within_window() {
    // 30 minutes remaining — within the 1-hour Drive renewal window
    let config = json!({
        "triggerType": "drive",
        "expiration": (now_ms() + 30 * 60 * 1000).to_string(),
    });
    assert!(should_renew_channel(&config));
}

#[test]
fn test_should_renew_drive_channel_not_yet() {
    // 2 hours remaining — outside the 1-hour Drive renewal window
    let config = json!({
        "triggerType": "drive",
        "expiration": (now_ms() + 2 * 60 * 60 * 1000).to_string(),
    });
    assert!(!should_renew_channel(&config));
}

#[test]
fn test_should_renew_calendar_channel_within_window() {
    // 12 hours remaining — within the 1-day Calendar renewal window
    let config = json!({
        "triggerType": "calendar",
        "expiration": (now_ms() + 12 * 60 * 60 * 1000).to_string(),
    });
    assert!(should_renew_channel(&config));
}

#[test]
fn test_should_renew_calendar_channel_not_yet() {
    // 2 days remaining — outside the 1-day Calendar renewal window
    let config = json!({
        "triggerType": "calendar",
        "expiration": (now_ms() + 2 * 24 * 60 * 60 * 1000).to_string(),
    });
    assert!(!should_renew_channel(&config));
}

#[test]
fn test_should_renew_channel_zero_expiration() {
    let config = json!({
        "triggerType": "drive",
        "expiration": "0",
    });
    assert!(!should_renew_channel(&config));
}

#[test]
fn test_should_renew_channel_missing_fields() {
    assert!(!should_renew_channel(&json!({})));
}

// ============================================================================
// 4. Delete Workspace Integration
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_delete_integration_full_cascade(db: Pool<Postgres>) -> anyhow::Result<()> {
    let resource_path = "u/test-user/native_gworkspace";
    let account_id = setup_oauth_integration(
        &db,
        ServiceName::Google,
        resource_path,
        "token",
        "refresh",
        None,
    )
    .await?;

    // Add a native trigger linked to this integration
    insert_test_script(&db, "f/test/handler").await?;
    let trigger_config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &trigger_config,
        json!({"triggerType": "drive"}),
    )
    .await?;

    // Step 1: Delete triggers
    let deleted =
        delete_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1").await?;
    assert!(deleted);

    // Step 2: Cleanup OAuth resources
    let mut tx = db.begin().await?;
    windmill_native_triggers::workspace_integrations::cleanup_oauth_resource(
        &mut *tx,
        "test-workspace",
        ServiceName::Google,
    )
    .await;
    tx.commit().await?;

    // Step 3: Delete workspace integration
    let mut tx = db.begin().await?;
    let deleted =
        delete_workspace_integration(&mut *tx, "test-workspace", ServiceName::Google).await?;
    tx.commit().await?;
    assert!(deleted);

    // Verify everything is gone
    let var_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM variable WHERE workspace_id = 'test-workspace' AND path = $1",
        resource_path,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(var_count, 0);

    let acc_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM account WHERE workspace_id = 'test-workspace' AND id = $1",
        account_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(acc_count, 0);

    let res_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM resource WHERE workspace_id = 'test-workspace' AND path = $1",
        resource_path,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(res_count, 0);

    assert!(
        get_workspace_integration(&db, "test-workspace", ServiceName::Google)
            .await
            .is_err()
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_cleanup_preserves_triggers(db: Pool<Postgres>) -> anyhow::Result<()> {
    let resource_path = "u/test-user/native_gworkspace";
    setup_oauth_integration(
        &db,
        ServiceName::Google,
        resource_path,
        "token",
        "refresh",
        None,
    )
    .await?;

    // Create a trigger
    insert_test_script(&db, "f/test/handler").await?;
    let trigger_config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &trigger_config,
        json!({"triggerType": "drive"}),
    )
    .await?;

    // Cleanup OAuth only — should NOT remove the trigger
    let mut tx = db.begin().await?;
    windmill_native_triggers::workspace_integrations::cleanup_oauth_resource(
        &mut *tx,
        "test-workspace",
        ServiceName::Google,
    )
    .await;
    tx.commit().await?;

    // OAuth resources gone
    let var_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM variable WHERE workspace_id = 'test-workspace' AND path = $1",
        resource_path,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(var_count, 0);

    // Trigger still exists
    let trigger_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM native_trigger WHERE workspace_id = 'test-workspace' AND service_name = 'google'"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(trigger_count, 1, "trigger should survive OAuth cleanup");

    Ok(())
}

// --- parse_stop_channel_params ---

#[test]
fn test_parse_stop_channel_params_drive() {
    let config = json!({
        "triggerType": "drive",
        "googleResourceId": "res-123",
    });
    let (resource_id, url) = parse_stop_channel_params(&config);
    assert_eq!(resource_id, "res-123");
    assert!(
        url.contains("googleapis.com/drive/v3/channels/stop"),
        "url={}",
        url
    );
}

#[test]
fn test_parse_stop_channel_params_calendar() {
    let config = json!({
        "triggerType": "calendar",
        "googleResourceId": "res-456",
    });
    let (resource_id, url) = parse_stop_channel_params(&config);
    assert_eq!(resource_id, "res-456");
    assert!(
        url.contains("googleapis.com/calendar/v3/channels/stop"),
        "url={}",
        url
    );
}

#[test]
fn test_parse_stop_channel_params_default() {
    // Missing triggerType defaults to Drive
    let config = json!({ "googleResourceId": "res-789" });
    let (resource_id, url) = parse_stop_channel_params(&config);
    assert_eq!(resource_id, "res-789");
    assert!(url.contains("drive/v3/channels/stop"), "url={}", url);
}

#[test]
fn test_parse_stop_channel_params_missing_resource_id() {
    let config = json!({ "triggerType": "drive" });
    let (resource_id, _url) = parse_stop_channel_params(&config);
    assert_eq!(resource_id, "");
}
