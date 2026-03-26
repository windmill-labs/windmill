//! Tests that call trigger trait methods directly to verify
//! all dynamic SQL correctly references the permissioned_as column.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::DB;
use windmill_trigger::handler::TriggerCrud;
use windmill_trigger::listener::Listener;
use windmill_trigger::types::TriggerMode;

/// Helper to insert a minimal trigger row.
async fn insert_trigger(db: &DB, table: &str, path: &str, extra_cols: &str, extra_vals: &str) {
    let sql = format!(
        "INSERT INTO {} (path, script_path, is_flow, workspace_id, edited_by, permissioned_as{}) \
         VALUES ($1, 'f/test/handler', false, 'test-workspace', 'test-user', 'u/test-user'{})",
        table, extra_cols, extra_vals
    );
    sqlx::query(&sql).bind(path).execute(db).await.unwrap();
}

#[cfg(feature = "websocket")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_websocket(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_trigger(
        &db,
        "websocket_trigger",
        "f/test/listener_ws",
        ", url",
        ", 'wss://example.com'",
    )
    .await;
    let triggers = windmill_trigger_websocket::WebsocketTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_ws"));
    Ok(())
}

#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_postgres(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_trigger(
        &db,
        "postgres_trigger",
        "f/test/listener_pg",
        ", postgres_resource_path, replication_slot_name, publication_name",
        ", 'u/test/pg', 'slot', 'pub'",
    )
    .await;
    let triggers = windmill_trigger_postgres::PostgresTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_pg"));
    Ok(())
}

#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_mqtt(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO mqtt_trigger (path, mqtt_resource_path, subscribe_topics, client_version, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, ARRAY[$3::jsonb], $4::mqtt_client_version, $5, false, 'test-workspace', 'test-user', 'u/test-user')"
    )
    .bind("f/test/listener_mqtt").bind("u/test/mqtt")
    .bind(json!({"topic": "t", "qos": "qos0"})).bind("v5").bind("f/test/handler")
    .execute(&db).await?;

    let triggers = windmill_trigger_mqtt::MqttTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_mqtt"));
    Ok(())
}

#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_kafka(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO kafka_trigger (path, kafka_resource_path, group_id, topics, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, $5, false, 'test-workspace', 'test-user', 'u/test-user')",
        "f/test/listener_kafka",
        "u/test/kafka",
        "grp",
        &["topic"] as &[&str],
        "f/test/handler"
    )
    .execute(&db)
    .await?;

    let triggers = windmill_trigger_kafka::KafkaTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_kafka"));
    Ok(())
}

#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_nats(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO nats_trigger (path, nats_resource_path, subjects, use_jetstream, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, $5, false, 'test-workspace', 'test-user', 'u/test-user')",
        "f/test/listener_nats",
        "u/test/nats",
        &["subj"] as &[&str],
        false,
        "f/test/handler"
    )
    .execute(&db)
    .await?;

    let triggers = windmill_trigger_nats::NatsTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_nats"));
    Ok(())
}

#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_sqs(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO sqs_trigger (path, queue_url, aws_resource_path, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, false, 'test-workspace', 'test-user', 'u/test-user')",
        "f/test/listener_sqs",
        "https://sqs.example.com/q",
        "u/test/aws",
        "f/test/handler"
    )
    .execute(&db)
    .await?;

    let triggers = windmill_trigger_sqs::SqsTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_sqs"));
    Ok(())
}

#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_listener_query_gcp(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO gcp_trigger (path, gcp_resource_path, topic_id, subscription_id, \
         delivery_type, subscription_mode, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, $5::delivery_mode, $6::gcp_subscription_mode, $7, false, 'test-workspace', 'test-user', 'u/test-user')"
    )
    .bind("f/test/listener_gcp").bind("u/test/gcp").bind("topic").bind("sub")
    .bind("pull").bind("existing").bind("f/test/handler")
    .execute(&db).await?;

    let triggers = windmill_trigger_gcp::GcpTrigger
        .fetch_enabled_unlistened_triggers(&db)
        .await?;
    assert!(triggers.iter().any(|t| t.path == "f/test/listener_gcp"));
    Ok(())
}

// ============================================================================
// Handler trait method tests (get_trigger_by_path, list_triggers, set_trigger_mode)
// ============================================================================

fn make_authed() -> windmill_api_auth::ApiAuthed {
    windmill_api_auth::ApiAuthed {
        email: "test@windmill.dev".to_string(),
        username: "test-user".to_string(),
        is_admin: true,
        is_operator: false,
        groups: vec![],
        folders: vec![],
        scopes: None,
        username_override: None,
        token_prefix: None,
    }
}

/// Tests get_trigger_by_path, list_triggers, set_trigger_mode for websocket (server_state=true).
#[cfg(feature = "websocket")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_websocket(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_trigger(
        &db,
        "websocket_trigger",
        "f/test/handler_ws",
        ", url",
        ", 'wss://example.com'",
    )
    .await;

    let handler = windmill_trigger_websocket::WebsocketTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_ws")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    let triggers = handler
        .list_triggers(&mut *conn, "test-workspace", None)
        .await?;
    assert!(triggers.iter().any(|t| t.base.path == "f/test/handler_ws"));

    let authed = make_authed();
    let updated = handler
        .set_trigger_mode(
            &authed,
            &mut *conn,
            "test-workspace",
            "f/test/handler_ws",
            &TriggerMode::Disabled,
        )
        .await?;
    assert!(updated);

    Ok(())
}

/// Tests get_trigger_by_path, list_triggers, set_trigger_mode for mqtt (server_state=true).
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_mqtt(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO mqtt_trigger (path, mqtt_resource_path, subscribe_topics, client_version, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, ARRAY[$3::jsonb], $4::mqtt_client_version, $5, false, 'test-workspace', 'test-user', 'u/test-user')",
    )
    .bind("f/test/handler_mqtt").bind("u/test/mqtt")
    .bind(json!({"topic": "t", "qos": "qos0"})).bind("v5").bind("f/test/handler")
    .execute(&db).await?;

    let handler = windmill_trigger_mqtt::MqttTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_mqtt")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    let authed = make_authed();
    let updated = handler
        .set_trigger_mode(
            &authed,
            &mut *conn,
            "test-workspace",
            "f/test/handler_mqtt",
            &TriggerMode::Disabled,
        )
        .await?;
    assert!(updated);

    Ok(())
}

/// Tests handler queries for kafka trigger (EE only — OSS stub returns ()).
#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_kafka(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO kafka_trigger (path, kafka_resource_path, group_id, topics, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, $5, false, 'test-workspace', 'test-user', 'u/test-user')",
        "f/test/handler_kafka",
        "u/test/kafka",
        "grp",
        &["topic"] as &[&str],
        "f/test/handler"
    )
    .execute(&db)
    .await?;

    let handler = windmill_trigger_kafka::KafkaTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_kafka")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    Ok(())
}

/// Tests handler queries for postgres trigger.
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_postgres(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_trigger(
        &db,
        "postgres_trigger",
        "f/test/handler_pg",
        ", postgres_resource_path, replication_slot_name, publication_name",
        ", 'u/test/pg', 'slot', 'pub'",
    )
    .await;

    let handler = windmill_trigger_postgres::PostgresTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_pg")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    Ok(())
}

/// Tests handler queries for nats trigger (EE only).
#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_nats(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO nats_trigger (path, nats_resource_path, subjects, use_jetstream, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, $5, false, 'test-workspace', 'test-user', 'u/test-user')",
        "f/test/handler_nats",
        "u/test/nats",
        &["subj"] as &[&str],
        false,
        "f/test/handler"
    )
    .execute(&db)
    .await?;

    let handler = windmill_trigger_nats::NatsTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_nats")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    Ok(())
}

/// Tests handler queries for sqs trigger (EE only).
#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_sqs(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO sqs_trigger (path, queue_url, aws_resource_path, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, false, 'test-workspace', 'test-user', 'u/test-user')",
        "f/test/handler_sqs",
        "https://sqs.example.com/q",
        "u/test/aws",
        "f/test/handler"
    )
    .execute(&db)
    .await?;

    let handler = windmill_trigger_sqs::SqsTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_sqs")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    Ok(())
}

/// Tests handler queries for gcp trigger (EE only).
#[cfg(feature = "private")]
#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_handler_queries_gcp(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO gcp_trigger (path, gcp_resource_path, topic_id, subscription_id, \
         delivery_type, subscription_mode, \
         script_path, is_flow, workspace_id, edited_by, permissioned_as) \
         VALUES ($1, $2, $3, $4, $5::delivery_mode, $6::gcp_subscription_mode, $7, false, 'test-workspace', 'test-user', 'u/test-user')",
    )
    .bind("f/test/handler_gcp").bind("u/test/gcp").bind("topic").bind("sub")
    .bind("pull").bind("existing").bind("f/test/handler")
    .execute(&db).await?;

    let handler = windmill_trigger_gcp::GcpTrigger;
    let mut conn = db.acquire().await?;

    let trigger = handler
        .get_trigger_by_path(&mut *conn, "test-workspace", "f/test/handler_gcp")
        .await?;
    assert_eq!(trigger.base.permissioned_as, "u/test-user");

    Ok(())
}
