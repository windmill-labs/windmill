/*!
 * End-to-end integration tests for Windmill trigger listeners.
 *
 * Each test is `#[ignore]` because it requires a running external service
 * (MQTT broker, NATS server, Kafka broker, etc.). See individual test doc
 * comments for setup instructions.
 *
 * Quick start — use the helper scripts in `tests/fixtures/`:
 * ```bash
 * ./tests/fixtures/start_all_triggers.sh          # start all services
 * ./tests/fixtures/start_all_triggers.sh oss      # start OSS services only
 * ./tests/fixtures/start_all_triggers.sh stop     # tear down everything
 * ```
 *
 * The general pattern:
 * 1. Insert a test script + trigger row + resource into the DB
 * 2. Start the API server with listeners enabled (server_mode=true)
 * 3. Connect to the external service and send a test message
 * 4. Poll `v2_job` for a job matching the trigger path + trigger_kind
 * 5. Verify the args shape/content
 */

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::time::Duration;

use windmill_test_utils::*;

// ============================================================================
// Helpers
// ============================================================================

/// Row shape for polling v2_job.
#[derive(Debug)]
#[allow(dead_code)]
struct TriggerJobRow {
    id: uuid::Uuid,
    runnable_path: Option<String>,
    trigger_kind: Option<String>,
    args: Option<sqlx::types::Json<serde_json::Value>>,
}

/// Poll `v2_job` every 500ms for up to `timeout` for a job whose
/// `runnable_path` and `trigger_kind` match the expected values.
async fn poll_for_trigger_job(
    db: &Pool<Postgres>,
    script_path: &str,
    trigger_kind: &str,
    timeout: Duration,
) -> anyhow::Result<TriggerJobRow> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let row = sqlx::query_as!(
            TriggerJobRow,
            r#"
            SELECT id, runnable_path, trigger_kind AS "trigger_kind: String",
                   args AS "args: sqlx::types::Json<serde_json::Value>"
            FROM v2_job
            WHERE runnable_path = $1
              AND trigger_kind = $2::job_trigger_kind
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            script_path,
            trigger_kind as _,
        )
        .fetch_optional(db)
        .await?;

        if let Some(job) = row {
            return Ok(job);
        }

        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for trigger job (script_path={}, trigger_kind={})",
                script_path,
                trigger_kind
            );
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Insert a minimal test script row that trigger listeners can reference.
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

/// Insert a resource row for triggers that resolve connection details from the
/// `resource` table.
async fn insert_resource(
    db: &Pool<Postgres>,
    path: &str,
    resource_type: &str,
    value: serde_json::Value,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', $1, $2::jsonb, $3, '{}'::jsonb, 'test-user')",
    )
    .bind(path)
    .bind(value)
    .bind(resource_type)
    .execute(db)
    .await?;
    Ok(())
}

// ============================================================================
// MQTT Trigger E2E
// ============================================================================

/// End-to-end test for MQTT trigger.
///
/// Requires a running MQTT broker. Setup:
/// ```bash
/// ./tests/fixtures/start_mqtt.sh
/// ```
///
/// Run:
/// ```bash
/// cargo test --test trigger_e2e test_mqtt_e2e --features mqtt_trigger \
///     -- --ignored --nocapture
/// ```
#[ignore = "requires running MQTT broker on localhost:1883"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mqtt_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let script_path = "f/test/mqtt_e2e_handler";
    insert_test_script(&db, script_path).await?;

    insert_resource(
        &db,
        "u/test-user/mqtt_res",
        "mqtt",
        json!({
            "broker": "localhost",
            "port": 1883
        }),
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO mqtt_trigger (
            path, mqtt_resource_path, subscribe_topics, client_version,
            script_path, is_flow, workspace_id, edited_by, email
        )
        VALUES ($1, $2, ARRAY[$3::jsonb], $4::mqtt_client_version, $5, $6, $7, $8, $9)
        "#,
    )
    .bind("f/test/mqtt_e2e_trigger")
    .bind("u/test-user/mqtt_res")
    .bind(json!({"topic": "windmill/test/e2e", "qos": "qos0"}))
    .bind("v5")
    .bind(script_path)
    .bind(false)
    .bind("test-workspace")
    .bind("test-user")
    .bind("test@windmill.dev")
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Publish a message using rumqttc
    let mut mqtt_opts = rumqttc::MqttOptions::new("windmill-e2e-test", "localhost", 1883);
    mqtt_opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut eventloop) = rumqttc::AsyncClient::new(mqtt_opts, 10);

    // Drive the event loop in the background
    let el_handle = tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(500)).await;
    client
        .publish(
            "windmill/test/e2e",
            rumqttc::QoS::AtLeastOnce,
            false,
            b"hello from e2e test".to_vec(),
        )
        .await?;

    let job = poll_for_trigger_job(&db, script_path, "mqtt", Duration::from_secs(30)).await?;
    assert!(job.args.is_some(), "job should have args");

    client.disconnect().await.ok();
    el_handle.abort();

    Ok(())
}

// ============================================================================
// WebSocket Trigger E2E
// ============================================================================

/// End-to-end test for WebSocket trigger.
///
/// Requires a WebSocket echo server. Setup:
/// ```bash
/// ./tests/fixtures/start_websocket.sh
/// ```
///
/// Run:
/// ```bash
/// cargo test --test trigger_e2e test_websocket_e2e --features websocket \
///     -- --ignored --nocapture
/// ```
#[ignore = "requires running WebSocket echo server on localhost:8765"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_websocket_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let script_path = "f/test/ws_e2e_handler";
    insert_test_script(&db, script_path).await?;

    sqlx::query!(
        r#"
        INSERT INTO websocket_trigger (
            path, url, script_path, is_flow, workspace_id,
            edited_by, email, initial_messages
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        "f/test/ws_e2e_trigger",
        "ws://localhost:8765",
        script_path,
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        &[json!({"type": "RawMessage", "content": "hello from e2e test"})] as &[serde_json::Value],
    )
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;

    // The WebSocket trigger connects to the server and sends initial_messages,
    // and each received message triggers a job.
    let job = poll_for_trigger_job(&db, script_path, "websocket", Duration::from_secs(30)).await?;
    assert!(job.args.is_some(), "job should have args");

    Ok(())
}

// ============================================================================
// Postgres Trigger E2E
// ============================================================================

/// End-to-end test for Postgres trigger (logical replication).
///
/// Requires PostgreSQL with `wal_level=logical`. Setup:
/// ```bash
/// ./tests/fixtures/start_postgres_replication.sh
/// ```
/// (The script checks wal_level and creates the table/publication/slot in the
/// main DB. This test re-creates them in its isolated sqlx::test database.)
///
/// Run:
/// ```bash
/// cargo test --test trigger_e2e test_postgres_e2e --features postgres_trigger \
///     -- --ignored --nocapture
/// ```
#[ignore = "requires PostgreSQL with wal_level=logical"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_postgres_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let script_path = "f/test/pg_e2e_handler";
    insert_test_script(&db, script_path).await?;

    // Create the tracked table + publication + replication slot inside the
    // isolated test database (sqlx::test gives us a fresh DB each run).
    // Replication slots are server-wide so we use a random suffix.
    let suffix: u32 = rand::random();
    let slot_name = format!("test_e2e_slot_{suffix}");
    let pub_name = format!("test_e2e_pub_{suffix}");

    sqlx::query("CREATE TABLE test_trigger_table (id serial PRIMARY KEY, data text)")
        .execute(&db)
        .await?;
    sqlx::query(&format!("CREATE PUBLICATION {pub_name} FOR TABLE test_trigger_table"))
        .execute(&db)
        .await?;
    sqlx::query(&format!(
        "SELECT pg_create_logical_replication_slot('{slot_name}', 'pgoutput')"
    ))
    .execute(&db)
    .await?;

    // Extract the test DB name from the pool so the resource points here,
    // not at the main windmill database.
    let test_db_name: String =
        sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&db)
            .await?;

    insert_resource(
        &db,
        "u/test-user/pg_res",
        "postgresql",
        json!({
            "user": "postgres",
            "password": "changeme",
            "host": "localhost",
            "port": 5432,
            "dbname": test_db_name,
            "sslmode": "disable"
        }),
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO postgres_trigger (
            path, script_path, is_flow, workspace_id, edited_by, email,
            postgres_resource_path, replication_slot_name, publication_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind("f/test/pg_e2e_trigger")
    .bind(script_path)
    .bind(false)
    .bind("test-workspace")
    .bind("test-user")
    .bind("test@windmill.dev")
    .bind("u/test-user/pg_res")
    .bind(&slot_name)
    .bind(&pub_name)
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Insert a row into the tracked table to trigger a change event
    sqlx::query("INSERT INTO test_trigger_table (data) VALUES ('e2e test data')")
        .execute(&db)
        .await?;

    let job = poll_for_trigger_job(&db, script_path, "postgres", Duration::from_secs(30)).await?;
    assert!(job.args.is_some(), "job should have args");

    Ok(())
}

// ============================================================================
// Kafka Trigger E2E (Enterprise)
// ============================================================================

/// End-to-end test for Kafka trigger (Enterprise only).
///
/// Requires a running Kafka broker with the test topic. Setup:
/// ```bash
/// ./tests/fixtures/start_kafka.sh
/// ```
///
/// Run:
/// ```bash
/// cargo test --test trigger_e2e test_kafka_e2e \
///     --features kafka,enterprise,private -- --ignored --nocapture
/// ```
#[cfg(all(feature = "enterprise", feature = "private"))]
#[ignore = "requires running Kafka broker on localhost:9092"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_kafka_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let script_path = "f/test/kafka_e2e_handler";
    insert_test_script(&db, script_path).await?;

    insert_resource(
        &db,
        "u/test-user/kafka_res",
        "kafka",
        json!({
            "brokers": ["localhost:9092"],
            "security": { "label": "PLAINTEXT" }
        }),
    )
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO kafka_trigger (
            path, kafka_resource_path, topics, group_id,
            script_path, is_flow, workspace_id, edited_by, email
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "f/test/kafka_e2e_trigger",
        "u/test-user/kafka_res",
        &["windmill-e2e-test"] as &[&str],
        "windmill-e2e-test-group",
        script_path,
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Produce messages using rdkafka. The consumer starts with auto.offset.reset=latest
    // and needs time for group rebalance, so we send repeatedly until a job appears.
    use rdkafka::config::ClientConfig;
    use rdkafka::producer::{FutureProducer, FutureRecord};

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()?;

    let db2 = db.clone();
    let produce_handle = tokio::spawn(async move {
        for _ in 0..30 {
            let _ = producer
                .send(
                    FutureRecord::to("windmill-e2e-test")
                        .payload("hello from kafka e2e test")
                        .key("test-key"),
                    Duration::from_secs(5),
                )
                .await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    let job = poll_for_trigger_job(&db2, script_path, "kafka", Duration::from_secs(30)).await?;
    produce_handle.abort();
    assert!(job.args.is_some(), "job should have args");

    Ok(())
}

// ============================================================================
// NATS Trigger E2E (Enterprise)
// ============================================================================

/// End-to-end test for NATS trigger (Enterprise only).
///
/// Requires a running NATS server. Setup:
/// ```bash
/// ./tests/fixtures/start_nats.sh
/// ```
///
/// Run:
/// ```bash
/// cargo test --test trigger_e2e test_nats_e2e \
///     --features nats,enterprise,private -- --ignored --nocapture
/// ```
#[cfg(all(feature = "enterprise", feature = "private"))]
#[ignore = "requires running NATS server on localhost:4222"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_nats_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let script_path = "f/test/nats_e2e_handler";
    insert_test_script(&db, script_path).await?;

    insert_resource(
        &db,
        "u/test-user/nats_res",
        "nats",
        json!({
            "servers": ["nats://localhost:4222"],
            "auth": { "label": "NO_AUTH" },
            "require_tls": false
        }),
    )
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO nats_trigger (
            path, nats_resource_path, subjects, script_path,
            is_flow, workspace_id, edited_by, email, use_jetstream
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "f/test/nats_e2e_trigger",
        "u/test-user/nats_res",
        &["windmill.e2e.test"] as &[&str],
        script_path,
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        false,
    )
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Publish a message using async-nats
    let nats_client = async_nats::connect("localhost:4222").await?;
    nats_client
        .publish("windmill.e2e.test", "hello from nats e2e test".into())
        .await?;
    nats_client.flush().await?;

    let job = poll_for_trigger_job(&db, script_path, "nats", Duration::from_secs(30)).await?;
    assert!(job.args.is_some(), "job should have args");

    Ok(())
}

// ============================================================================
// SQS Trigger E2E (Enterprise)
// ============================================================================

/// End-to-end test for SQS trigger (Enterprise only).
///
/// Requires LocalStack with the test queue. Setup:
/// ```bash
/// ./tests/fixtures/start_sqs.sh
/// ```
///
/// Run:
/// ```bash
/// AWS_ENDPOINT_URL=http://localhost:4566 \
/// cargo test --test trigger_e2e test_sqs_e2e \
///     --features sqs_trigger,enterprise,private -- --ignored --nocapture
/// ```
#[cfg(all(feature = "enterprise", feature = "private"))]
#[ignore = "requires LocalStack SQS on localhost:4566"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_sqs_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // The SQS listener uses aws_config which respects AWS_ENDPOINT_URL for LocalStack.
    std::env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");

    let script_path = "f/test/sqs_e2e_handler";
    insert_test_script(&db, script_path).await?;

    insert_resource(
        &db,
        "u/test-user/aws_res",
        "aws",
        json!({
            "awsAccessKeyId": "test",
            "awsSecretAccessKey": "test",
            "region": "us-east-1"
        }),
    )
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO sqs_trigger (
            path, queue_url, aws_resource_path, script_path,
            is_flow, workspace_id, edited_by, email
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        "f/test/sqs_e2e_trigger",
        "http://localhost:4566/000000000000/windmill-e2e-test",
        "u/test-user/aws_res",
        script_path,
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Send a message using aws-sdk-sqs
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url("http://localhost:4566")
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_credential_types::Credentials::new(
            "test", "test", None, None, "test",
        ))
        .load()
        .await;
    let sqs_client = aws_sdk_sqs::Client::new(&config);

    sqs_client
        .send_message()
        .queue_url("http://localhost:4566/000000000000/windmill-e2e-test")
        .message_body("hello from sqs e2e test")
        .send()
        .await?;

    let job = poll_for_trigger_job(&db, script_path, "sqs", Duration::from_secs(30)).await?;
    assert!(job.args.is_some(), "job should have args");

    Ok(())
}

// ============================================================================
// GCP Pub/Sub Trigger E2E (Enterprise)
// ============================================================================

/// End-to-end test for GCP Pub/Sub trigger (Enterprise only).
///
/// Requires the GCP Pub/Sub emulator with test topic/subscription. Setup:
/// ```bash
/// ./tests/fixtures/start_gcp_pubsub.sh
/// ```
///
/// Run:
/// ```bash
/// PUBSUB_EMULATOR_HOST=localhost:8085 \
/// cargo test --test trigger_e2e test_gcp_e2e \
///     --features gcp_trigger,enterprise,private -- --ignored --nocapture
/// ```
#[cfg(all(feature = "enterprise", feature = "private"))]
#[ignore = "requires GCP Pub/Sub emulator on localhost:8085"]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_gcp_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let script_path = "f/test/gcp_e2e_handler";
    insert_test_script(&db, script_path).await?;

    // The GCP emulator doesn't require real credentials, but the resource
    // row must still exist for the listener to resolve it.
    // The private_key must use literal \n (backslash-n) as in real GCP service
    // account JSON files. The trigger code re-parses it through serde_json to
    // convert those escape sequences to actual newlines.
    insert_resource(
        &db,
        "u/test-user/gcp_res",
        "google",
        json!({
            "project_id": "test-project",
            "private_key_id": "test",
            "private_key": "-----BEGIN RSA PRIVATE KEY-----\\nMIIBogIBAAJBALRiMLAH\\n-----END RSA PRIVATE KEY-----\\n",
            "client_email": "test@test-project.iam.gserviceaccount.com",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token",
            "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs"
        }),
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO gcp_trigger (
            path, gcp_resource_path, topic_id, subscription_id,
            delivery_type, subscription_mode, script_path, is_flow,
            workspace_id, edited_by, email
        )
        VALUES ($1, $2, $3, $4, $5::delivery_mode, $6::gcp_subscription_mode, $7, $8, $9, $10, $11)
        "#,
    )
    .bind("f/test/gcp_e2e_trigger")
    .bind("u/test-user/gcp_res")
    .bind("windmill-e2e-test")
    .bind("windmill-e2e-sub")
    .bind("pull")
    .bind("existing")
    .bind(script_path)
    .bind(false)
    .bind("test-workspace")
    .bind("test-user")
    .bind("test@windmill.dev")
    .execute(&db)
    .await?;

    let _server = ApiServer::start_with_listeners(db.clone()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Publish a message to the emulator via HTTP
    let client = reqwest::Client::new();
    let emulator_host =
        std::env::var("PUBSUB_EMULATOR_HOST").unwrap_or_else(|_| "localhost:8085".to_string());
    // The google-cloud-pubsub crate uses "local-project" as the default project ID
    // when PUBSUB_EMULATOR_HOST is set, so we must publish to that project's topic.
    let publish_url = format!(
        "http://{}/v1/projects/local-project/topics/windmill-e2e-test:publish",
        emulator_host
    );

    let message_data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "hello from gcp e2e test",
    );
    client
        .post(&publish_url)
        .json(&json!({
            "messages": [{ "data": message_data }]
        }))
        .send()
        .await?;

    let job = poll_for_trigger_job(&db, script_path, "gcp", Duration::from_secs(30)).await?;
    assert!(job.args.is_some(), "job should have args");

    Ok(())
}
