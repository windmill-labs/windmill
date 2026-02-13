/*!
 * Integration tests for the native trigger system (Google + Nextcloud).
 *
 * Tests cover:
 * 1. Pure logic functions (resolve_endpoint, ServiceName, generate_webhook_service_url)
 * 2. Native trigger DB CRUD (store/get/update/delete/list)
 * 3. Workspace integration CRUD
 * 4. OAuth resource lifecycle (decrypt_oauth_data, cleanup)
 * 5. Token helpers (get/delete by prefix)
 */

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_api_auth::ApiAuthed;
use windmill_common::variables::{build_crypt, encrypt};
use windmill_native_triggers::{
    decrypt_oauth_data, delete_native_trigger, delete_token_by_prefix,
    delete_workspace_integration, generate_webhook_service_url, get_native_trigger,
    get_native_trigger_by_script, get_token_by_prefix, get_workspace_integration,
    list_native_triggers, resolve_endpoint, store_native_trigger, store_workspace_integration,
    update_native_trigger_error, update_native_trigger_service_config, NativeTriggerConfig,
    OAuthConfig, ServiceName,
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
    access_token: &str,
    refresh_token: &str,
) -> anyhow::Result<(String, i32)> {
    let resource_path = format!("u/test-user/native_{}", service_name.resource_type());

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
    let oauth_data = json!({
        "client_id": "test-client-id",
        "client_secret": "test-client-secret",
        "base_url": "https://example.com",
        "resource_path": resource_path,
    });

    let authed = test_authed();
    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        service_name,
        oauth_data,
    )
    .await?;
    tx.commit().await?;

    Ok((resource_path, account_id))
}

// ============================================================================
// 1. Pure Logic Tests
// ============================================================================

#[test]
fn test_resolve_endpoint_absolute_url() {
    assert_eq!(
        resolve_endpoint(
            "https://nc.example.com",
            "https://oauth2.googleapis.com/token"
        ),
        "https://oauth2.googleapis.com/token"
    );
    assert_eq!(
        resolve_endpoint("https://nc.example.com", "http://localhost:8080/path"),
        "http://localhost:8080/path"
    );
}

#[test]
fn test_resolve_endpoint_relative_path() {
    assert_eq!(
        resolve_endpoint("https://nc.example.com", "/apps/oauth2/api/v1/token"),
        "https://nc.example.com/apps/oauth2/api/v1/token"
    );
    assert_eq!(
        resolve_endpoint("https://nc.example.com", "/path"),
        "https://nc.example.com/path"
    );
}

#[test]
fn test_service_name_roundtrip() {
    for name in ["nextcloud", "google"] {
        let service = ServiceName::try_from(name.to_string()).unwrap();
        assert_eq!(service.as_str(), name);
    }
}

#[test]
fn test_service_name_try_from_unknown() {
    assert!(ServiceName::try_from("unknown_service".to_string()).is_err());
}

#[test]
fn test_service_name_methods() {
    let services = [ServiceName::Nextcloud, ServiceName::Google];
    for service in services {
        assert!(!service.as_str().is_empty());
        assert!(!service.token_endpoint().is_empty());
        assert!(!service.auth_endpoint().is_empty());
        assert!(!service.oauth_scopes().is_empty());
        assert!(!service.resource_type().is_empty());
        assert!(!service.to_string().is_empty());
    }

    // Google has extra auth params, Nextcloud doesn't
    assert!(!ServiceName::Google.extra_auth_params().is_empty());
    assert!(ServiceName::Nextcloud.extra_auth_params().is_empty());
}

#[test]
fn test_generate_webhook_url_with_and_without_external_id() {
    let url = generate_webhook_service_url(
        "https://app.windmill.dev",
        "my-workspace",
        "f/test/script",
        false,
        None,
        ServiceName::Google,
        "tok123",
    );
    assert!(url.contains("https://app.windmill.dev"));
    assert!(url.contains("my-workspace"));
    assert!(url.contains("f/test/script"));
    assert!(url.contains("token=tok123"));
    assert!(url.contains("service_name=google"));
    assert!(!url.contains("trigger_external_id"));

    let url_with_id = generate_webhook_service_url(
        "https://app.windmill.dev",
        "my-workspace",
        "f/test/script",
        true,
        Some("ext-123"),
        ServiceName::Nextcloud,
        "tok456",
    );
    assert!(url_with_id.contains("trigger_external_id=ext-123"));
    assert!(url_with_id.contains("service_name=nextcloud"));
    // is_flow=true → "f" prefix
    assert!(url_with_id.contains("/run/f/"));
}

// ============================================================================
// 2. Native Trigger DB CRUD Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_store_and_get_native_trigger(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/handler").await?;

    let config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };

    // Store for Google
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "google-ext-1",
        &config,
        json!({"channel_id": "ch1"}),
    )
    .await?;

    // Store for Nextcloud
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Nextcloud,
        "nc-ext-1",
        &config,
        json!({"webhook_id": "wh1"}),
    )
    .await?;

    // Get Google trigger
    let trigger = get_native_trigger(&db, "test-workspace", ServiceName::Google, "google-ext-1")
        .await?
        .expect("should find Google trigger");
    assert_eq!(trigger.external_id, "google-ext-1");
    assert_eq!(trigger.script_path, "f/test/handler");
    assert!(!trigger.is_flow);
    assert_eq!(trigger.webhook_token_prefix, "abcdefghij");
    assert!(trigger.error.is_none());

    // Get Nextcloud trigger
    let trigger = get_native_trigger(&db, "test-workspace", ServiceName::Nextcloud, "nc-ext-1")
        .await?
        .expect("should find Nextcloud trigger");
    assert_eq!(trigger.external_id, "nc-ext-1");

    // Get with wrong external_id
    let missing =
        get_native_trigger(&db, "test-workspace", ServiceName::Google, "nonexistent").await?;
    assert!(missing.is_none());

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_store_native_trigger_upsert(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/handler").await?;
    insert_test_script(&db, "f/test/handler2").await?;

    let config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &config,
        json!({"v": 1}),
    )
    .await?;

    // Upsert with new script_path
    let config2 = NativeTriggerConfig {
        script_path: "f/test/handler2".to_string(),
        is_flow: false,
        webhook_token: "zzzzzzzzzz1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &config2,
        json!({"v": 2}),
    )
    .await?;

    let trigger = get_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1")
        .await?
        .unwrap();
    assert_eq!(trigger.script_path, "f/test/handler2");
    assert_eq!(trigger.webhook_token_prefix, "zzzzzzzzzz");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_delete_native_trigger(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/handler").await?;

    let config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &config,
        json!({}),
    )
    .await?;

    let deleted =
        delete_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1").await?;
    assert!(deleted);

    let missing = get_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1").await?;
    assert!(missing.is_none());

    let deleted_again =
        delete_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1").await?;
    assert!(!deleted_again);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_native_triggers(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/script_a").await?;
    insert_test_script(&db, "f/test/script_b").await?;

    let config_a = NativeTriggerConfig {
        script_path: "f/test/script_a".to_string(),
        is_flow: false,
        webhook_token: "aaaaaaaaaa1234567890".to_string(),
    };
    let config_b = NativeTriggerConfig {
        script_path: "f/test/script_b".to_string(),
        is_flow: false,
        webhook_token: "bbbbbbbbbb1234567890".to_string(),
    };

    // 2 Google triggers, 1 Nextcloud
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "g1",
        &config_a,
        json!({}),
    )
    .await?;
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "g2",
        &config_b,
        json!({}),
    )
    .await?;
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Nextcloud,
        "nc1",
        &config_a,
        json!({}),
    )
    .await?;

    let google_triggers = list_native_triggers(
        &db,
        "test-workspace",
        ServiceName::Google,
        None,
        None,
        None,
        None,
    )
    .await?;
    assert_eq!(google_triggers.len(), 2);

    let nc_triggers = list_native_triggers(
        &db,
        "test-workspace",
        ServiceName::Nextcloud,
        None,
        None,
        None,
        None,
    )
    .await?;
    assert_eq!(nc_triggers.len(), 1);

    // Filter by path
    let filtered = list_native_triggers(
        &db,
        "test-workspace",
        ServiceName::Google,
        None,
        None,
        Some("f/test/script_a"),
        None,
    )
    .await?;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].script_path, "f/test/script_a");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_native_trigger_error(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/handler").await?;

    let config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &config,
        json!({}),
    )
    .await?;

    // Set error
    update_native_trigger_error(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        Some("connection failed"),
    )
    .await?;
    let trigger = get_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1")
        .await?
        .unwrap();
    assert_eq!(trigger.error.as_deref(), Some("connection failed"));

    // Clear error
    update_native_trigger_error(&db, "test-workspace", ServiceName::Google, "ext-1", None).await?;
    let trigger = get_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1")
        .await?
        .unwrap();
    assert!(trigger.error.is_none());

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_native_trigger_service_config(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/handler").await?;

    let config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &config,
        json!({"channel_id": "old"}),
    )
    .await?;

    let new_config = json!({"channel_id": "new", "resource_id": "abc"});
    update_native_trigger_service_config(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &new_config,
    )
    .await?;

    let trigger = get_native_trigger(&db, "test-workspace", ServiceName::Google, "ext-1")
        .await?
        .unwrap();
    assert_eq!(trigger.service_config.unwrap(), new_config);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_native_trigger_by_script(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_test_script(&db, "f/test/handler").await?;

    let config = NativeTriggerConfig {
        script_path: "f/test/handler".to_string(),
        is_flow: false,
        webhook_token: "abcdefghij1234567890".to_string(),
    };
    store_native_trigger(
        &db,
        "test-workspace",
        ServiceName::Google,
        "ext-1",
        &config,
        json!({}),
    )
    .await?;

    let found = get_native_trigger_by_script(
        &db,
        "test-workspace",
        ServiceName::Google,
        "f/test/handler",
        false,
    )
    .await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().external_id, "ext-1");

    // Wrong script path
    let missing = get_native_trigger_by_script(
        &db,
        "test-workspace",
        ServiceName::Google,
        "f/test/nonexistent",
        false,
    )
    .await?;
    assert!(missing.is_none());

    // Wrong is_flow
    let missing = get_native_trigger_by_script(
        &db,
        "test-workspace",
        ServiceName::Google,
        "f/test/handler",
        true,
    )
    .await?;
    assert!(missing.is_none());

    Ok(())
}

// ============================================================================
// 3. Workspace Integration CRUD Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_store_and_get_workspace_integration(db: Pool<Postgres>) -> anyhow::Result<()> {
    let authed = test_authed();
    let oauth_data = json!({
        "client_id": "my-client",
        "client_secret": "my-secret",
        "base_url": "https://nc.example.com",
    });

    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        ServiceName::Nextcloud,
        oauth_data.clone(),
    )
    .await?;
    tx.commit().await?;

    let integration =
        get_workspace_integration(&db, "test-workspace", ServiceName::Nextcloud).await?;
    assert_eq!(integration.oauth_data["client_id"], "my-client");
    assert_eq!(integration.created_by, "test-user");

    // Upsert with new config
    let new_data = json!({
        "client_id": "updated-client",
        "client_secret": "updated-secret",
        "base_url": "https://new.example.com",
    });
    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        ServiceName::Nextcloud,
        new_data,
    )
    .await?;
    tx.commit().await?;

    let integration =
        get_workspace_integration(&db, "test-workspace", ServiceName::Nextcloud).await?;
    assert_eq!(integration.oauth_data["client_id"], "updated-client");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_delete_workspace_integration(db: Pool<Postgres>) -> anyhow::Result<()> {
    let authed = test_authed();
    let oauth_data = json!({
        "client_id": "my-client",
        "client_secret": "my-secret",
        "base_url": "https://nc.example.com",
    });

    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        ServiceName::Nextcloud,
        oauth_data,
    )
    .await?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    let deleted =
        delete_workspace_integration(&mut *tx, "test-workspace", ServiceName::Nextcloud).await?;
    tx.commit().await?;
    assert!(deleted);

    let result = get_workspace_integration(&db, "test-workspace", ServiceName::Nextcloud).await;
    assert!(result.is_err());

    let mut tx = db.begin().await?;
    let deleted_again =
        delete_workspace_integration(&mut *tx, "test-workspace", ServiceName::Nextcloud).await?;
    tx.commit().await?;
    assert!(!deleted_again);

    Ok(())
}

// ============================================================================
// 4. OAuth Resource Lifecycle Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_decrypt_oauth_data(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (resource_path, _account_id) = setup_oauth_integration(
        &db,
        ServiceName::Google,
        "my-access-token",
        "my-refresh-token",
    )
    .await?;
    assert_eq!(resource_path, "u/test-user/native_gworkspace");

    let config: OAuthConfig =
        decrypt_oauth_data(&db, &db, "test-workspace", ServiceName::Google).await?;

    assert_eq!(config.access_token, "my-access-token");
    assert_eq!(config.refresh_token.as_deref(), Some("my-refresh-token"));
    assert_eq!(config.client_id, "test-client-id");
    assert_eq!(config.client_secret, "test-client-secret");
    assert_eq!(config.base_url, "https://example.com");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_decrypt_oauth_data_nextcloud(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (resource_path, _) =
        setup_oauth_integration(&db, ServiceName::Nextcloud, "nc-token", "nc-refresh").await?;
    assert_eq!(resource_path, "u/test-user/native_nextcloud");

    let config: OAuthConfig =
        decrypt_oauth_data(&db, &db, "test-workspace", ServiceName::Nextcloud).await?;

    assert_eq!(config.access_token, "nc-token");
    assert_eq!(config.refresh_token.as_deref(), Some("nc-refresh"));

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_decrypt_oauth_data_missing_resource_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    // Store integration without resource_path
    let authed = test_authed();
    let oauth_data = json!({
        "client_id": "test-id",
        "client_secret": "test-secret",
        "base_url": "https://example.com",
    });
    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        ServiceName::Google,
        oauth_data,
    )
    .await?;
    tx.commit().await?;

    let result: Result<OAuthConfig, _> =
        decrypt_oauth_data(&db, &db, "test-workspace", ServiceName::Google).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("resource_path"),
        "error should mention resource_path: {}",
        err
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_decrypt_oauth_data_missing_variable(db: Pool<Postgres>) -> anyhow::Result<()> {
    // Store integration with resource_path but no variable
    let authed = test_authed();
    let oauth_data = json!({
        "client_id": "test-id",
        "client_secret": "test-secret",
        "base_url": "https://example.com",
        "resource_path": "u/test-user/native_gworkspace",
    });
    let mut tx = db.begin().await?;
    store_workspace_integration(
        &mut *tx,
        &authed,
        "test-workspace",
        ServiceName::Google,
        oauth_data,
    )
    .await?;
    tx.commit().await?;

    let result: Result<OAuthConfig, _> =
        decrypt_oauth_data(&db, &db, "test-workspace", ServiceName::Google).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Variable"),
        "error should mention variable: {}",
        err
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_cleanup_oauth_resource(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (resource_path, account_id) =
        setup_oauth_integration(&db, ServiceName::Google, "token", "refresh").await?;

    // Verify everything exists
    let var_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM variable WHERE workspace_id = 'test-workspace' AND path = $1",
        resource_path,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(var_count, 1);

    let acc_count: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM account WHERE workspace_id = 'test-workspace' AND id = $1",
        account_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(acc_count, 1);

    // Cleanup
    let mut tx = db.begin().await?;
    windmill_native_triggers::workspace_integrations::cleanup_oauth_resource(
        &mut *tx,
        "test-workspace",
        ServiceName::Google,
    )
    .await;
    tx.commit().await?;

    // Verify everything is deleted
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

    // Cleanup again — should be idempotent
    let mut tx = db.begin().await?;
    windmill_native_triggers::workspace_integrations::cleanup_oauth_resource(
        &mut *tx,
        "test-workspace",
        ServiceName::Google,
    )
    .await;
    tx.commit().await?;

    Ok(())
}

// ============================================================================
// 5. Token Helpers
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_token_by_prefix(db: Pool<Postgres>) -> anyhow::Result<()> {
    // base fixture creates SECRET_TOKEN
    let token = get_token_by_prefix(&db, "SECRET_TO").await?;
    assert_eq!(token.as_deref(), Some("SECRET_TOKEN"));

    let missing = get_token_by_prefix(&db, "NONEXISTENT").await?;
    assert!(missing.is_none());

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_delete_token_by_prefix(db: Pool<Postgres>) -> anyhow::Result<()> {
    let deleted = delete_token_by_prefix(&db, "SECRET_TO").await?;
    assert!(deleted);

    let token = get_token_by_prefix(&db, "SECRET_TO").await?;
    assert!(token.is_none());

    let deleted_again = delete_token_by_prefix(&db, "SECRET_TO").await?;
    assert!(!deleted_again);

    Ok(())
}
