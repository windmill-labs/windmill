/*!
 * Tests for the polling-based notify_event system that replaces PostgreSQL LISTEN/NOTIFY.
 *
 * These tests verify:
 * 1. Database triggers correctly insert events into notify_event table
 * 2. Polling functions retrieve events correctly
 * 3. Cleanup functions delete old events
 * 4. All notification channels work as expected
 */

use sqlx::{Pool, Postgres};
use windmill_common::notify_events::{cleanup_old_events, get_latest_event_id, poll_notify_events};

mod common;

/// Helper to get a database connection for tests
async fn get_db() -> Pool<Postgres> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:changeme@localhost:5432/windmill".to_string());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

/// Helper to insert a test event directly
async fn insert_test_event(db: &Pool<Postgres>, channel: &str, payload: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO notify_event (channel, payload) VALUES ($1, $2) RETURNING id",
    )
    .bind(channel)
    .bind(payload)
    .fetch_one(db)
    .await
    .expect("Failed to insert test event")
}

/// Helper to count events for a channel
async fn count_events_for_channel(db: &Pool<Postgres>, channel: &str) -> i64 {
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notify_event WHERE channel = $1")
        .bind(channel)
        .fetch_one(db)
        .await
        .expect("Failed to count events");
    result.0
}

/// Helper to delete all test events
async fn cleanup_test_events(db: &Pool<Postgres>) {
    sqlx::query("DELETE FROM notify_event WHERE channel LIKE 'test_%'")
        .execute(db)
        .await
        .ok();
}

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[tokio::test]
async fn test_get_latest_event_id_empty_table() {
    let db = get_db().await;

    // Clear any existing events first for a clean test
    let _ = sqlx::query("DELETE FROM notify_event").execute(&db).await;

    let latest_id = get_latest_event_id(&db).await.expect("Should get latest event id");
    assert_eq!(latest_id, 0, "Empty table should return 0");
}

#[tokio::test]
async fn test_get_latest_event_id_with_events() {
    let db = get_db().await;
    cleanup_test_events(&db).await;

    let _id1 = insert_test_event(&db, "test_channel_1", "payload1").await;
    let _id2 = insert_test_event(&db, "test_channel_2", "payload2").await;
    let id3 = insert_test_event(&db, "test_channel_3", "payload3").await;

    let latest_id = get_latest_event_id(&db).await.expect("Should get latest event id");
    assert!(latest_id >= id3, "Latest id should be >= last inserted id");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_poll_notify_events_empty() {
    let db = get_db().await;

    let latest_id = get_latest_event_id(&db).await.unwrap();
    let events = poll_notify_events(&db, latest_id).await.expect("Should poll events");
    assert!(events.is_empty(), "Should return empty vec when no new events");
}

#[tokio::test]
async fn test_poll_notify_events_returns_new_events() {
    let db = get_db().await;
    cleanup_test_events(&db).await;

    let before_id = get_latest_event_id(&db).await.unwrap();

    let _id1 = insert_test_event(&db, "test_poll_channel", "payload1").await;
    let _id2 = insert_test_event(&db, "test_poll_channel", "payload2").await;

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    assert!(events.len() >= 2, "Should return at least 2 new events");

    // Verify the events we inserted are present
    let our_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_poll_channel")
        .collect();
    assert_eq!(our_events.len(), 2, "Should have exactly our 2 test events");

    // Verify ordering (ascending by id)
    assert!(our_events[0].id < our_events[1].id, "Events should be ordered by id ascending");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_poll_notify_events_respects_last_event_id() {
    let db = get_db().await;
    cleanup_test_events(&db).await;

    let id1 = insert_test_event(&db, "test_respect_id", "payload1").await;
    let _id2 = insert_test_event(&db, "test_respect_id", "payload2").await;
    let _id3 = insert_test_event(&db, "test_respect_id", "payload3").await;

    // Poll from id1 should only return id2 and id3
    let events = poll_notify_events(&db, id1).await.expect("Should poll events");
    let our_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_respect_id")
        .collect();

    assert_eq!(our_events.len(), 2, "Should only return events after id1");
    assert!(our_events.iter().all(|e| e.id > id1), "All events should have id > id1");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_cleanup_old_events() {
    let db = get_db().await;
    cleanup_test_events(&db).await;

    // Insert an event with old timestamp
    sqlx::query(
        "INSERT INTO notify_event (channel, payload, created_at) VALUES ($1, $2, now() - interval '15 minutes')",
    )
    .bind("test_cleanup_old")
    .bind("old_payload")
    .execute(&db)
    .await
    .expect("Failed to insert old event");

    // Insert a recent event
    insert_test_event(&db, "test_cleanup_recent", "recent_payload").await;

    // Cleanup events older than 10 minutes
    let deleted = cleanup_old_events(&db, 10).await.expect("Should cleanup events");
    assert!(deleted >= 1, "Should delete at least 1 old event");

    // Verify old event is gone
    let old_count = count_events_for_channel(&db, "test_cleanup_old").await;
    assert_eq!(old_count, 0, "Old event should be deleted");

    // Verify recent event is still there
    let recent_count = count_events_for_channel(&db, "test_cleanup_recent").await;
    assert_eq!(recent_count, 1, "Recent event should still exist");

    cleanup_test_events(&db).await;
}

// ============================================================================
// Database Trigger Tests - Verify triggers insert events correctly
// ============================================================================

#[tokio::test]
async fn test_trigger_notify_config_change() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert or update a config entry
    sqlx::query(
        "INSERT INTO config (name, config) VALUES ('test_config_trigger', '{}'::jsonb)
         ON CONFLICT (name) DO UPDATE SET config = '{}'::jsonb",
    )
    .execute(&db)
    .await
    .expect("Failed to insert config");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let config_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_config_change" && e.payload == "test_config_trigger")
        .collect();

    assert!(!config_events.is_empty(), "Should have notify_config_change event");

    // Cleanup
    sqlx::query("DELETE FROM config WHERE name = 'test_config_trigger'")
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_global_setting_change_insert() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Use a unique setting name for testing
    let setting_name = format!("test_setting_{}", uuid::Uuid::new_v4());

    // Insert a global setting
    sqlx::query("INSERT INTO global_settings (name, value) VALUES ($1, '{}'::jsonb)")
        .bind(&setting_name)
        .execute(&db)
        .await
        .expect("Failed to insert global setting");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let setting_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_global_setting_change" && e.payload == setting_name)
        .collect();

    assert!(!setting_events.is_empty(), "Should have notify_global_setting_change event on insert");

    // Cleanup
    sqlx::query("DELETE FROM global_settings WHERE name = $1")
        .bind(&setting_name)
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_global_setting_change_update() {
    let db = get_db().await;

    // Use a unique setting name for testing
    let setting_name = format!("test_setting_update_{}", uuid::Uuid::new_v4());

    // First insert
    sqlx::query("INSERT INTO global_settings (name, value) VALUES ($1, '{}'::jsonb)")
        .bind(&setting_name)
        .execute(&db)
        .await
        .expect("Failed to insert global setting");

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Update the setting
    sqlx::query("UPDATE global_settings SET value = '{\"updated\": true}'::jsonb WHERE name = $1")
        .bind(&setting_name)
        .execute(&db)
        .await
        .expect("Failed to update global setting");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let setting_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_global_setting_change" && e.payload == setting_name)
        .collect();

    assert!(!setting_events.is_empty(), "Should have notify_global_setting_change event on update");

    // Cleanup
    sqlx::query("DELETE FROM global_settings WHERE name = $1")
        .bind(&setting_name)
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_global_setting_change_delete() {
    let db = get_db().await;

    // Use a unique setting name for testing
    let setting_name = format!("test_setting_delete_{}", uuid::Uuid::new_v4());

    // First insert
    sqlx::query("INSERT INTO global_settings (name, value) VALUES ($1, '{}'::jsonb)")
        .bind(&setting_name)
        .execute(&db)
        .await
        .expect("Failed to insert global setting");

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Delete the setting
    sqlx::query("DELETE FROM global_settings WHERE name = $1")
        .bind(&setting_name)
        .execute(&db)
        .await
        .expect("Failed to delete global setting");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let setting_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_global_setting_change" && e.payload == setting_name)
        .collect();

    assert!(!setting_events.is_empty(), "Should have notify_global_setting_change event on delete");
}

#[tokio::test]
async fn test_trigger_notify_workspace_envs_change() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a workspace env (test-workspace should exist from fixtures)
    sqlx::query(
        "INSERT INTO workspace_env (workspace_id, name, value) VALUES ('test-workspace', 'TEST_ENV_VAR', 'test_value')
         ON CONFLICT (workspace_id, name) DO UPDATE SET value = 'test_value_updated'",
    )
    .execute(&db)
    .await
    .expect("Failed to insert workspace env");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let env_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_workspace_envs_change" && e.payload == "test-workspace")
        .collect();

    assert!(!env_events.is_empty(), "Should have notify_workspace_envs_change event");

    // Cleanup
    sqlx::query("DELETE FROM workspace_env WHERE workspace_id = 'test-workspace' AND name = 'TEST_ENV_VAR'")
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_workspace_key_change() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a workspace key
    sqlx::query(
        "INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('test-workspace', 'cloud', 'test_key_value')
         ON CONFLICT (workspace_id, kind) DO UPDATE SET key = 'test_key_value_updated'",
    )
    .execute(&db)
    .await
    .expect("Failed to insert workspace key");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let key_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_workspace_key_change" && e.payload == "test-workspace")
        .collect();

    assert!(!key_events.is_empty(), "Should have notify_workspace_key_change event");

    // Cleanup
    sqlx::query("DELETE FROM workspace_key WHERE workspace_id = 'test-workspace' AND kind = 'cloud'")
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_token_invalidation() {
    let db = get_db().await;

    // First insert a session token
    let token = format!("test_token_{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO token (token, label, email, workspace_id, owner, expiration)
         VALUES ($1, 'session', 'test@test.com', 'test-workspace', 'test-user', now() + interval '1 hour')",
    )
    .bind(&token)
    .execute(&db)
    .await
    .expect("Failed to insert token");

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Delete the token (should trigger notification)
    sqlx::query("DELETE FROM token WHERE token = $1")
        .bind(&token)
        .execute(&db)
        .await
        .expect("Failed to delete token");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let token_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_token_invalidation" && e.payload == token)
        .collect();

    assert!(!token_events.is_empty(), "Should have notify_token_invalidation event");
}

#[tokio::test]
async fn test_trigger_var_cache_invalidation() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a variable
    let var_path = format!("test_var_{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description)
         VALUES ('test-workspace', $1, 'test_value', false, 'test variable')",
    )
    .bind(&var_path)
    .execute(&db)
    .await
    .expect("Failed to insert variable");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let var_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "var_cache_invalidation")
        .filter(|e| e.payload.contains("test-workspace") && e.payload.contains(&var_path))
        .collect();

    assert!(!var_events.is_empty(), "Should have var_cache_invalidation event");

    // Verify JSON payload structure
    let payload: serde_json::Value = serde_json::from_str(&var_events[0].payload)
        .expect("Payload should be valid JSON");
    assert_eq!(payload.get("workspace_id").and_then(|v| v.as_str()), Some("test-workspace"));
    assert_eq!(payload.get("path").and_then(|v| v.as_str()), Some(var_path.as_str()));
    assert!(payload.get("operation").is_some(), "Should have operation field");

    // Cleanup
    sqlx::query("DELETE FROM variable WHERE workspace_id = 'test-workspace' AND path = $1")
        .bind(&var_path)
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_resource_cache_invalidation() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a resource
    let resource_path = format!("test_resource_{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, description)
         VALUES ('test-workspace', $1, '{}'::jsonb, 'test_type', 'test resource')",
    )
    .bind(&resource_path)
    .execute(&db)
    .await
    .expect("Failed to insert resource");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let resource_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "resource_cache_invalidation")
        .filter(|e| e.payload.contains("test-workspace") && e.payload.contains(&resource_path))
        .collect();

    assert!(!resource_events.is_empty(), "Should have resource_cache_invalidation event");

    // Verify JSON payload structure
    let payload: serde_json::Value = serde_json::from_str(&resource_events[0].payload)
        .expect("Payload should be valid JSON");
    assert_eq!(payload.get("workspace_id").and_then(|v| v.as_str()), Some("test-workspace"));
    assert_eq!(payload.get("path").and_then(|v| v.as_str()), Some(resource_path.as_str()));
    assert!(payload.get("operation").is_some(), "Should have operation field");

    // Cleanup
    sqlx::query("DELETE FROM resource WHERE workspace_id = 'test-workspace' AND path = $1")
        .bind(&resource_path)
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_webhook_change() {
    let db = get_db().await;

    // Ensure workspace_settings exists for test-workspace
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id) VALUES ('test-workspace')
         ON CONFLICT (workspace_id) DO NOTHING",
    )
    .execute(&db)
    .await
    .ok();

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Update webhook setting
    sqlx::query("UPDATE workspace_settings SET webhook = 'https://test.webhook.com' WHERE workspace_id = 'test-workspace'")
        .execute(&db)
        .await
        .expect("Failed to update webhook");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let webhook_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_webhook_change" && e.payload == "test-workspace")
        .collect();

    assert!(!webhook_events.is_empty(), "Should have notify_webhook_change event");

    // Cleanup - reset webhook
    sqlx::query("UPDATE workspace_settings SET webhook = NULL WHERE workspace_id = 'test-workspace'")
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_workspace_premium_change() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Toggle premium status
    sqlx::query("UPDATE workspace SET premium = NOT premium WHERE id = 'test-workspace'")
        .execute(&db)
        .await
        .expect("Failed to update workspace premium");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let premium_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_workspace_premium_change" && e.payload == "test-workspace")
        .collect();

    assert!(!premium_events.is_empty(), "Should have notify_workspace_premium_change event");

    // Toggle back
    sqlx::query("UPDATE workspace SET premium = NOT premium WHERE id = 'test-workspace'")
        .execute(&db)
        .await
        .ok();
}

// ============================================================================
// HTTP Trigger Tests (conditional on feature)
// ============================================================================

#[tokio::test]
async fn test_trigger_notify_http_trigger_change() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    let trigger_path = format!("test_http_trigger_{}", uuid::Uuid::new_v4());

    // Insert an HTTP trigger
    sqlx::query(
        "INSERT INTO http_trigger (path, route_path, route_path_key, script_path, is_flow, workspace_id, edited_by, email, http_method, authentication_method)
         VALUES ($1, '/test/route', '/test/route', 'test/script', false, 'test-workspace', 'test-user', 'test@test.com', 'get', 'none')",
    )
    .bind(&trigger_path)
    .execute(&db)
    .await
    .expect("Failed to insert HTTP trigger");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let http_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_http_trigger_change")
        .filter(|e| e.payload.contains("test-workspace") && e.payload.contains(&trigger_path))
        .collect();

    assert!(!http_events.is_empty(), "Should have notify_http_trigger_change event");

    // Cleanup
    sqlx::query("DELETE FROM http_trigger WHERE path = $1")
        .bind(&trigger_path)
        .execute(&db)
        .await
        .ok();
}

// ============================================================================
// Script/Flow Version Change Tests
// ============================================================================

#[tokio::test]
async fn test_trigger_notify_runnable_version_change_script() {
    let db = get_db().await;

    // First create a script without lock
    let script_path = format!("f/test/script_{}", uuid::Uuid::new_v4());
    let script_hash: i64 = rand::random::<i64>().abs();

    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, kind)
         VALUES ('test-workspace', $1, $2, 'test', 'test', 'def main(): pass', 'test-user', 'python3', 'script')",
    )
    .bind(script_hash)
    .bind(&script_path)
    .execute(&db)
    .await
    .expect("Failed to insert script");

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Update the lock field (this should trigger the notification)
    sqlx::query("UPDATE script SET lock = 'test_lock_content' WHERE hash = $1")
        .bind(script_hash)
        .execute(&db)
        .await
        .expect("Failed to update script lock");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let script_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_runnable_version_change")
        .filter(|e| e.payload.contains("test-workspace") && e.payload.contains("script"))
        .collect();

    assert!(!script_events.is_empty(), "Should have notify_runnable_version_change event for script");

    // Verify payload format: workspace_id:source_type:path:kind
    let parts: Vec<&str> = script_events[0].payload.split(':').collect();
    assert!(parts.len() >= 4, "Payload should have at least 4 parts");
    assert_eq!(parts[0], "test-workspace", "First part should be workspace_id");
    assert_eq!(parts[1], "script", "Second part should be 'script'");

    // Cleanup
    sqlx::query("DELETE FROM script WHERE hash = $1")
        .bind(script_hash)
        .execute(&db)
        .await
        .ok();
}

#[tokio::test]
async fn test_trigger_notify_runnable_version_change_flow() {
    let db = get_db().await;

    // First create a flow
    let flow_path = format!("f/test/flow_{}", uuid::Uuid::new_v4());

    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema)
         VALUES ('test-workspace', $1, 'test', 'test', '{}'::jsonb, 'test-user', '{}'::json)",
    )
    .bind(&flow_path)
    .execute(&db)
    .await
    .expect("Failed to insert flow");

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a flow version (this should trigger the notification)
    sqlx::query(
        "INSERT INTO flow_version (workspace_id, path, value, schema, created_by)
         VALUES ('test-workspace', $1, '{}'::jsonb, '{}'::json, 'test-user')",
    )
    .bind(&flow_path)
    .execute(&db)
    .await
    .expect("Failed to insert flow version");

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let flow_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "notify_runnable_version_change")
        .filter(|e| e.payload.contains("test-workspace") && e.payload.contains("flow"))
        .collect();

    assert!(!flow_events.is_empty(), "Should have notify_runnable_version_change event for flow");

    // Verify payload format
    let parts: Vec<&str> = flow_events[0].payload.split(':').collect();
    assert!(parts.len() >= 4, "Payload should have at least 4 parts");
    assert_eq!(parts[0], "test-workspace", "First part should be workspace_id");
    assert_eq!(parts[1], "flow", "Second part should be 'flow'");

    // Cleanup
    sqlx::query("DELETE FROM flow_version WHERE workspace_id = 'test-workspace' AND path = $1")
        .bind(&flow_path)
        .execute(&db)
        .await
        .ok();
    sqlx::query("DELETE FROM flow WHERE workspace_id = 'test-workspace' AND path = $1")
        .bind(&flow_path)
        .execute(&db)
        .await
        .ok();
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_event_insertion() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert multiple events concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let db = db.clone();
            tokio::spawn(async move {
                insert_test_event(&db, "test_concurrent", &format!("payload_{}", i)).await
            })
        })
        .collect();

    // Wait for all insertions
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let concurrent_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_concurrent")
        .collect();

    assert_eq!(concurrent_events.len(), 10, "Should have all 10 concurrent events");

    // Verify all events have unique IDs
    let ids: std::collections::HashSet<i64> = concurrent_events.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 10, "All events should have unique IDs");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_polling_isolation() {
    let db = get_db().await;

    // Insert some events
    let id1 = insert_test_event(&db, "test_isolation", "payload1").await;
    let id2 = insert_test_event(&db, "test_isolation", "payload2").await;
    let _id3 = insert_test_event(&db, "test_isolation", "payload3").await;

    // Two different "consumers" polling from different points
    let events_from_0 = poll_notify_events(&db, 0).await.expect("Should poll events");
    let events_from_id1 = poll_notify_events(&db, id1).await.expect("Should poll events");
    let events_from_id2 = poll_notify_events(&db, id2).await.expect("Should poll events");

    // Filter to our test events
    let from_0: Vec<_> = events_from_0.iter().filter(|e| e.channel == "test_isolation").collect();
    let from_id1: Vec<_> = events_from_id1.iter().filter(|e| e.channel == "test_isolation").collect();
    let from_id2: Vec<_> = events_from_id2.iter().filter(|e| e.channel == "test_isolation").collect();

    assert!(from_0.len() >= 3, "Polling from 0 should include all events");
    assert_eq!(from_id1.len(), 2, "Polling from id1 should include id2 and id3");
    assert_eq!(from_id2.len(), 1, "Polling from id2 should include only id3");

    cleanup_test_events(&db).await;
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_empty_payload() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    insert_test_event(&db, "test_empty_payload", "").await;

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let empty_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_empty_payload")
        .collect();

    assert_eq!(empty_events.len(), 1, "Should have event with empty payload");
    assert_eq!(empty_events[0].payload, "", "Payload should be empty string");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_large_payload() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Create a large payload (1KB)
    let large_payload = "x".repeat(1024);
    insert_test_event(&db, "test_large_payload", &large_payload).await;

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let large_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_large_payload")
        .collect();

    assert_eq!(large_events.len(), 1, "Should have event with large payload");
    assert_eq!(large_events[0].payload.len(), 1024, "Payload should be preserved");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_special_characters_in_payload() {
    let db = get_db().await;
    let before_id = get_latest_event_id(&db).await.unwrap();

    let special_payload = r#"{"key": "value with \"quotes\" and 'apostrophes'", "unicode": "日本語", "newline": "line1\nline2"}"#;
    insert_test_event(&db, "test_special_chars", special_payload).await;

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let special_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_special_chars")
        .collect();

    assert_eq!(special_events.len(), 1, "Should have event with special characters");
    assert_eq!(special_events[0].payload, special_payload, "Special characters should be preserved");

    cleanup_test_events(&db).await;
}

#[tokio::test]
async fn test_cleanup_with_no_old_events() {
    let db = get_db().await;
    cleanup_test_events(&db).await;

    // Insert only recent events
    insert_test_event(&db, "test_no_old", "recent1").await;
    insert_test_event(&db, "test_no_old", "recent2").await;

    let before_count = count_events_for_channel(&db, "test_no_old").await;
    assert_eq!(before_count, 2, "Should have 2 recent events");

    // Cleanup old events (none should be deleted)
    let _deleted = cleanup_old_events(&db, 10).await.expect("Should cleanup events");
    // Note: deleted might be 0 or might include other test artifacts

    let after_count = count_events_for_channel(&db, "test_no_old").await;
    assert_eq!(after_count, 2, "Recent events should not be deleted");

    cleanup_test_events(&db).await;
}
