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

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_latest_event_id_returns_valid_id(db: Pool<Postgres>) {
    // Get current latest id
    let latest_id = get_latest_event_id(&db).await.expect("Should get latest event id");
    assert!(latest_id >= 0, "Latest id should be non-negative");

    // Insert a new event and verify latest_id increases
    let new_id = insert_test_event(&db, "test_latest_id", "payload").await;
    let new_latest_id = get_latest_event_id(&db).await.expect("Should get latest event id");
    assert!(new_latest_id >= new_id, "Latest id should be >= new event id");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_latest_event_id_with_events(db: Pool<Postgres>) {
    let _id1 = insert_test_event(&db, "test_channel_1", "payload1").await;
    let _id2 = insert_test_event(&db, "test_channel_2", "payload2").await;
    let id3 = insert_test_event(&db, "test_channel_3", "payload3").await;

    let latest_id = get_latest_event_id(&db).await.expect("Should get latest event id");
    assert!(latest_id >= id3, "Latest id should be >= last inserted id");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_poll_notify_events_no_new_events(db: Pool<Postgres>) {
    // Get latest id first
    let latest_id = get_latest_event_id(&db).await.unwrap();

    // Poll from the latest id - should return empty since no new events
    let events = poll_notify_events(&db, latest_id).await.expect("Should poll events");
    assert!(events.is_empty(), "Should return empty vec when polling from latest id");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_poll_notify_events_returns_new_events(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_poll_notify_events_respects_last_event_id(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_cleanup_old_events(db: Pool<Postgres>) {
    // Use unique channel names to avoid interference from other tests
    let old_channel = format!("test_cleanup_old_{}", uuid::Uuid::new_v4());
    let recent_channel = format!("test_cleanup_recent_{}", uuid::Uuid::new_v4());

    // Insert an event with old timestamp
    sqlx::query(
        "INSERT INTO notify_event (channel, payload, created_at) VALUES ($1, $2, now() - interval '15 minutes')",
    )
    .bind(&old_channel)
    .bind("old_payload")
    .execute(&db)
    .await
    .expect("Failed to insert old event");

    // Insert a recent event
    sqlx::query(
        "INSERT INTO notify_event (channel, payload) VALUES ($1, $2)",
    )
    .bind(&recent_channel)
    .bind("recent_payload")
    .execute(&db)
    .await
    .expect("Failed to insert recent event");

    // Count before cleanup
    let old_count_before = count_events_for_channel(&db, &old_channel).await;
    assert_eq!(old_count_before, 1, "Should have 1 old event before cleanup");

    // Cleanup events older than 10 minutes
    let deleted = cleanup_old_events(&db, 10).await.expect("Should cleanup events");
    assert!(deleted >= 1, "Should delete at least 1 old event");

    // Verify old event is gone
    let old_count = count_events_for_channel(&db, &old_channel).await;
    assert_eq!(old_count, 0, "Old event should be deleted");

    // Verify recent event is still there
    let recent_count = count_events_for_channel(&db, &recent_channel).await;
    assert_eq!(recent_count, 1, "Recent event should still exist");
}

// ============================================================================
// Database Trigger Tests - Verify triggers insert events correctly
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_config_change(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_global_setting_change_insert(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_global_setting_change_update(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_global_setting_change_delete(db: Pool<Postgres>) {
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

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_workspace_envs_change(db: Pool<Postgres>) {
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a workspace env (test-workspace exists from fixture)
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_workspace_key_change(db: Pool<Postgres>) {
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert a workspace key (base fixture already has one, so this will conflict and update)
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_token_invalidation(db: Pool<Postgres>) {
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

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_webhook_change(db: Pool<Postgres>) {
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Update webhook setting (workspace_settings exists from base fixture)
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_workspace_premium_change(db: Pool<Postgres>) {
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
}

// ============================================================================
// HTTP Trigger Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_http_trigger_change(db: Pool<Postgres>) {
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
}

// ============================================================================
// Script/Flow Version Change Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_runnable_version_change_script(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_notify_runnable_version_change_flow(db: Pool<Postgres>) {
    // First create a flow with empty versions array
    let flow_path = format!("f/test/flow_{}", uuid::Uuid::new_v4());

    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema, versions)
         VALUES ('test-workspace', $1, 'test', 'test', '{}'::jsonb, 'test-user', '{}'::json, ARRAY[]::bigint[])",
    )
    .bind(&flow_path)
    .execute(&db)
    .await
    .expect("Failed to insert flow");

    let before_id = get_latest_event_id(&db).await.unwrap();

    // Update the flow's versions array (this triggers flow_versions_append_trigger)
    sqlx::query(
        "UPDATE flow SET versions = array_append(versions, 1::bigint) WHERE workspace_id = 'test-workspace' AND path = $1",
    )
    .bind(&flow_path)
    .execute(&db)
    .await
    .expect("Failed to update flow versions");

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
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_concurrent_event_insertion(db: Pool<Postgres>) {
    // Use a unique channel name for this test run
    let channel = format!("test_concurrent_{}", uuid::Uuid::new_v4());
    let before_id = get_latest_event_id(&db).await.unwrap();

    // Insert multiple events concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let db = db.clone();
            let ch = channel.clone();
            tokio::spawn(async move {
                sqlx::query_scalar::<_, i64>(
                    "INSERT INTO notify_event (channel, payload) VALUES ($1, $2) RETURNING id",
                )
                .bind(&ch)
                .bind(format!("payload_{}", i))
                .fetch_one(&db)
                .await
                .expect("Failed to insert event")
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
        .filter(|e| e.channel == channel)
        .collect();

    assert_eq!(concurrent_events.len(), 10, "Should have all 10 concurrent events");

    // Verify all events have unique IDs
    let ids: std::collections::HashSet<i64> = concurrent_events.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 10, "All events should have unique IDs");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_polling_isolation(db: Pool<Postgres>) {
    // Use a unique channel name for this test
    let channel = format!("test_isolation_{}", uuid::Uuid::new_v4());

    // Get baseline before inserting
    let baseline_id = get_latest_event_id(&db).await.unwrap();

    // Insert some events
    let id1 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO notify_event (channel, payload) VALUES ($1, $2) RETURNING id",
    )
    .bind(&channel)
    .bind("payload1")
    .fetch_one(&db)
    .await
    .expect("Failed to insert event");

    let id2 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO notify_event (channel, payload) VALUES ($1, $2) RETURNING id",
    )
    .bind(&channel)
    .bind("payload2")
    .fetch_one(&db)
    .await
    .expect("Failed to insert event");

    let _id3 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO notify_event (channel, payload) VALUES ($1, $2) RETURNING id",
    )
    .bind(&channel)
    .bind("payload3")
    .fetch_one(&db)
    .await
    .expect("Failed to insert event");

    // Two different "consumers" polling from different points
    let events_from_baseline = poll_notify_events(&db, baseline_id).await.expect("Should poll events");
    let events_from_id1 = poll_notify_events(&db, id1).await.expect("Should poll events");
    let events_from_id2 = poll_notify_events(&db, id2).await.expect("Should poll events");

    // Filter to our test events
    let from_baseline: Vec<_> = events_from_baseline.iter().filter(|e| e.channel == channel).collect();
    let from_id1: Vec<_> = events_from_id1.iter().filter(|e| e.channel == channel).collect();
    let from_id2: Vec<_> = events_from_id2.iter().filter(|e| e.channel == channel).collect();

    assert_eq!(from_baseline.len(), 3, "Polling from baseline should include all 3 events");
    assert_eq!(from_id1.len(), 2, "Polling from id1 should include id2 and id3");
    assert_eq!(from_id2.len(), 1, "Polling from id2 should include only id3");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_empty_payload(db: Pool<Postgres>) {
    let before_id = get_latest_event_id(&db).await.unwrap();

    insert_test_event(&db, "test_empty_payload", "").await;

    let events = poll_notify_events(&db, before_id).await.expect("Should poll events");
    let empty_events: Vec<_> = events
        .iter()
        .filter(|e| e.channel == "test_empty_payload")
        .collect();

    assert_eq!(empty_events.len(), 1, "Should have event with empty payload");
    assert_eq!(empty_events[0].payload, "", "Payload should be empty string");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_large_payload(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_special_characters_in_payload(db: Pool<Postgres>) {
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
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_cleanup_with_no_old_events(db: Pool<Postgres>) {
    // Use a unique channel name for this test
    let channel = format!("test_no_old_{}", uuid::Uuid::new_v4());

    // Insert only recent events
    sqlx::query("INSERT INTO notify_event (channel, payload) VALUES ($1, $2)")
        .bind(&channel)
        .bind("recent1")
        .execute(&db)
        .await
        .expect("Failed to insert event");

    sqlx::query("INSERT INTO notify_event (channel, payload) VALUES ($1, $2)")
        .bind(&channel)
        .bind("recent2")
        .execute(&db)
        .await
        .expect("Failed to insert event");

    let before_count = count_events_for_channel(&db, &channel).await;
    assert_eq!(before_count, 2, "Should have 2 recent events");

    // Cleanup old events (none of our events should be deleted since they're recent)
    let _deleted = cleanup_old_events(&db, 10).await.expect("Should cleanup events");

    let after_count = count_events_for_channel(&db, &channel).await;
    assert_eq!(after_count, 2, "Recent events should not be deleted");
}

// ============================================================================
// Multi-Server Integration Tests
// ============================================================================
// These tests start two actual windmill server processes on different ports
// with LISTEN_NEW_EVENTS_INTERVAL_SEC=1, trigger DB changes, and verify
// both servers process the events via their log output.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

struct ServerProcess {
    child: Child,
    log_lines: Arc<Mutex<Vec<String>>>,
    _stdout_handle: std::thread::JoinHandle<()>,
    _stderr_handle: std::thread::JoinHandle<()>,
}

impl ServerProcess {
    fn start(port: u16, db_url: &str) -> Self {
        let binary = std::env::var("WINDMILL_BINARY")
            .unwrap_or_else(|_| format!("{}/target/debug/windmill", env!("CARGO_MANIFEST_DIR")));

        let mut child = Command::new(&binary)
            .env("DATABASE_URL", db_url)
            .env("MODE", "server")
            .env("PORT", port.to_string())
            .env("LISTEN_NEW_EVENTS_INTERVAL_SEC", "1")
            .env("RUST_LOG", "info")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to start windmill on port {port}: {e}"));

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let log_lines = Arc::new(Mutex::new(Vec::new()));
        let log_lines_stdout = log_lines.clone();
        let log_lines_stderr = log_lines.clone();

        // Read both stdout and stderr into the same log buffer
        let _reader_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    log_lines_stdout.lock().unwrap().push(line);
                }
            }
        });
        let _stderr_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    log_lines_stderr.lock().unwrap().push(line);
                }
            }
        });

        ServerProcess { child, log_lines, _stdout_handle: _reader_handle, _stderr_handle }
    }

    fn logs_contain(&self, needle: &str) -> bool {
        self.log_lines.lock().unwrap().iter().any(|l| l.contains(needle))
    }

    fn dump_logs(&self) -> String {
        self.log_lines.lock().unwrap().join("\n")
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Wait for server to be ready by polling its HTTP endpoint.
async fn wait_for_server(port: u16, timeout_secs: u64) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/version", port);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if client.get(&url).send().await.is_ok() {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    false
}

/// Helper to get a database connection (only used by the e2e multi-server test)
async fn get_db() -> Pool<Postgres> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:changeme@localhost:5432/windmill".to_string());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

#[tokio::test]
#[ignore = "slow - starts two server processes with 1s poll interval"]
async fn test_two_server_processes_both_receive_event() {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:changeme@localhost:5432/windmill".to_string());

    // Start two server processes on different ports with 1s poll interval
    let mut server_a = ServerProcess::start(19100, &db_url);
    let mut server_b = ServerProcess::start(19200, &db_url);

    // Wait for both servers to be ready
    let (ready_a, ready_b) = tokio::join!(
        wait_for_server(19100, 30),
        wait_for_server(19200, 30),
    );
    assert!(ready_a, "Server A (port 19100) failed to start. Logs:\n{}", server_a.dump_logs());
    assert!(ready_b, "Server B (port 19200) failed to start. Logs:\n{}", server_b.dump_logs());

    // Give servers a moment to complete their first poll cycle
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Trigger a global setting change via direct DB insert
    let db = get_db().await;
    let setting_name = format!("test_e2e_{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ($1, '\"e2e_test\"'::jsonb)
         ON CONFLICT (name) DO UPDATE SET value = '\"e2e_test\"'::jsonb",
    )
    .bind(&setting_name)
    .execute(&db)
    .await
    .expect("Failed to insert global setting");

    // Wait for at least 2 poll cycles (interval is 1s)
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let needle = format!("Global setting change detected: {}", setting_name);
    assert!(
        server_a.logs_contain(&needle),
        "Server A should have processed the global setting event.\nSearching for: {}\nServer A logs:\n{}",
        needle, server_a.dump_logs()
    );
    assert!(
        server_b.logs_contain(&needle),
        "Server B should have processed the global setting event.\nSearching for: {}\nServer B logs:\n{}",
        needle, server_b.dump_logs()
    );

    // Cleanup
    sqlx::query("DELETE FROM global_settings WHERE name = $1")
        .bind(&setting_name)
        .execute(&db)
        .await
        .ok();

    // Explicitly kill before drop to avoid port conflicts with other tests
    let _ = server_a.child.kill();
    let _ = server_b.child.kill();
}
