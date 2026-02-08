/*!
 * Integration tests for the trigger system (captures, HTTP triggers, trigger configs).
 *
 * These tests verify:
 * 1. Capture config CRUD (create/ping/list/delete via API)
 * 2. Capture payload insertion and retrieval
 * 3. HTTP trigger CRUD and route matching
 * 4. All trigger types DB schema validation
 */

use serde::Deserialize;
use serde_json::json;
use sqlx::{Pool, Postgres};

mod common;
use common::*;

// ============================================================================
// Capture Config Tests (direct DB)
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_capture_config_insert_and_query(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO capture_config (workspace_id, path, is_flow, trigger_kind, owner, email)
        VALUES ($1, $2, $3, $4::trigger_kind, $5, $6)
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let config = sqlx::query!(
        r#"
        SELECT path, owner, email, trigger_kind AS "trigger_kind: String"
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/script",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(config.path, "f/test/script");
    assert_eq!(config.owner, "test-user");
    assert_eq!(config.email, "test@windmill.dev");
    assert_eq!(config.trigger_kind, "webhook");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_config_upsert(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO capture_config (workspace_id, path, is_flow, trigger_kind, owner, email)
        VALUES ($1, $2, $3, $4::trigger_kind, $5, $6)
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO capture_config (workspace_id, path, is_flow, trigger_kind, owner, email)
        VALUES ($1, $2, $3, $4::trigger_kind, $5, $6)
        ON CONFLICT (workspace_id, path, is_flow, trigger_kind)
        DO UPDATE SET owner = $5, email = $6, server_id = NULL, error = NULL
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        "new-owner",
        "new@windmill.dev",
    )
    .execute(&db)
    .await?;

    let config = sqlx::query!(
        "SELECT owner, email FROM capture_config WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/script",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(config.owner, "new-owner");
    assert_eq!(config.email, "new@windmill.dev");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_config_ping_updates_timestamp(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO capture_config (workspace_id, path, is_flow, trigger_kind, owner, email)
        VALUES ($1, $2, $3, $4::trigger_kind, $5, $6)
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let before = sqlx::query!(
        "SELECT last_client_ping FROM capture_config WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/script",
    )
    .fetch_one(&db)
    .await?;
    assert!(before.last_client_ping.is_none());

    sqlx::query!(
        r#"
        UPDATE capture_config SET last_client_ping = NOW()
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4::trigger_kind
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
    )
    .execute(&db)
    .await?;

    let after = sqlx::query!(
        "SELECT last_client_ping FROM capture_config WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/script",
    )
    .fetch_one(&db)
    .await?;
    assert!(after.last_client_ping.is_some());

    Ok(())
}

// ============================================================================
// Capture Payload Tests (direct DB)
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_capture_insert_and_list(db: Pool<Postgres>) -> anyhow::Result<()> {
    for i in 0..2 {
        sqlx::query!(
            r#"
            INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
            VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
            "#,
            "test-workspace",
            "f/test/script",
            false,
            "webhook" as _,
            json!({"key": format!("value{}", i)}),
            json!({"pre": format!("args{}", i)}),
            "test-user",
        )
        .execute(&db)
        .await?;
    }

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM capture WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/script",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(count, Some(2));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_delete(db: Pool<Postgres>) -> anyhow::Result<()> {
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
        VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
        RETURNING id
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        json!({"key": "value"}),
        json!({"pre": "args"}),
        "test-user",
    )
    .fetch_one(&db)
    .await?;

    sqlx::query!("DELETE FROM capture WHERE id = $1", id)
        .execute(&db)
        .await?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM capture WHERE id = $1",
        id,
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(count, Some(0));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_filter_by_trigger_kind(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
        VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        json!({"source": "webhook"}),
        json!({}),
        "test-user",
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
        VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "email" as _,
        json!({"source": "email"}),
        json!({}),
        "test-user",
    )
    .execute(&db)
    .await?;

    let webhook_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM capture
        WHERE workspace_id = $1 AND path = $2 AND trigger_kind = $3::trigger_kind
        "#,
        "test-workspace",
        "f/test/script",
        "webhook" as _,
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(webhook_count, Some(1));

    let email_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM capture
        WHERE workspace_id = $1 AND path = $2 AND trigger_kind = $3::trigger_kind
        "#,
        "test-workspace",
        "f/test/script",
        "email" as _,
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(email_count, Some(1));

    Ok(())
}

// ============================================================================
// Capture API Tests (via HTTP)
// ============================================================================

#[derive(Debug, Deserialize)]
struct CaptureResponse {
    id: i64,
    #[allow(dead_code)]
    trigger_kind: String,
    main_args: serde_json::Value,
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_api_set_config_and_list(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let response = client
        .client()
        .post(format!(
            "{}/w/test-workspace/capture/set_config",
            client.baseurl()
        ))
        .json(&json!({
            "path": "f/test/my_script",
            "is_flow": false,
            "trigger_kind": "webhook",
        }))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "set_config should succeed, got: {}",
        response.status()
    );

    let config = sqlx::query!(
        "SELECT owner FROM capture_config WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/my_script",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(config.owner, "test-user");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_api_list_captures(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    for i in 0..3 {
        sqlx::query!(
            r#"
            INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
            VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
            "#,
            "test-workspace",
            "f/test/script",
            false,
            "webhook" as _,
            json!({"index": i}),
            json!({}),
            "test-user",
        )
        .execute(&db)
        .await?;
    }

    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/capture/list/script/f/test/script",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(response.status().is_success(), "list captures should succeed");

    let captures: Vec<CaptureResponse> = response.json().await?;
    assert_eq!(captures.len(), 3);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_api_get_single(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
        VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
        RETURNING id
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        json!({"hello": "world"}),
        json!({}),
        "test-user",
    )
    .fetch_one(&db)
    .await?;

    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/capture/{}",
            client.baseurl(),
            id
        ))
        .send()
        .await?;

    assert!(response.status().is_success(), "get capture should succeed");

    let capture: CaptureResponse = response.json().await?;
    assert_eq!(capture.id, id);
    assert_eq!(capture.main_args["hello"], "world");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_api_delete(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
        VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
        RETURNING id
        "#,
        "test-workspace",
        "f/test/script",
        false,
        "webhook" as _,
        json!({"data": "to_delete"}),
        json!({}),
        "test-user",
    )
    .fetch_one(&db)
    .await?;

    let response = client
        .client()
        .delete(format!(
            "{}/w/test-workspace/capture/{}",
            client.baseurl(),
            id
        ))
        .send()
        .await?;

    assert!(response.status().is_success(), "delete should succeed");

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM capture WHERE id = $1",
        id,
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(count, Some(0));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_capture_api_pagination(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    for i in 0..5 {
        sqlx::query!(
            r#"
            INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by)
            VALUES ($1, $2, $3, $4::trigger_kind, $5::jsonb, $6::jsonb, $7)
            "#,
            "test-workspace",
            "f/test/script",
            false,
            "webhook" as _,
            json!({"index": i}),
            json!({}),
            "test-user",
        )
        .execute(&db)
        .await?;
    }

    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/capture/list/script/f/test/script?per_page=2",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(response.status().is_success());
    let limited: Vec<CaptureResponse> = response.json().await?;
    assert_eq!(limited.len(), 2, "per_page=2 should limit to 2 results");

    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/capture/list/script/f/test/script",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(response.status().is_success());
    let all: Vec<CaptureResponse> = response.json().await?;
    assert_eq!(all.len(), 5, "without limit should return all 5");

    Ok(())
}

// ============================================================================
// HTTP Trigger Tests (direct DB)
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_http_trigger_insert_and_query(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO http_trigger (
            path, route_path, route_path_key, script_path, is_flow,
            workspace_id, edited_by, email, http_method,
            authentication_method, is_static_website, workspaced_route,
            wrap_body, raw_string
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::http_method,
                $10::authentication_method, $11, $12, $13, $14)
        "#,
        "f/test/http_trigger",
        "api/v1/users/:id",
        "api/v1/users",
        "f/test/handler_script",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        "post" as _,
        "none" as _,
        false,
        false,
        false,
        false,
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        r#"
        SELECT
            path, route_path, script_path,
            http_method AS "http_method: String",
            authentication_method AS "authentication_method: String"
        FROM http_trigger
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/http_trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.path, "f/test/http_trigger");
    assert_eq!(trigger.route_path, "api/v1/users/:id");
    assert_eq!(trigger.script_path, "f/test/handler_script");
    assert_eq!(trigger.http_method, "post");
    assert_eq!(trigger.authentication_method, "none");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_http_trigger_multiple_methods(db: Pool<Postgres>) -> anyhow::Result<()> {
    let methods = ["get", "post", "put", "delete", "patch"];

    for method in &methods {
        sqlx::query!(
            r#"
            INSERT INTO http_trigger (
                path, route_path, route_path_key, script_path, is_flow,
                workspace_id, edited_by, email, http_method,
                authentication_method, is_static_website, workspaced_route,
                wrap_body, raw_string
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::http_method,
                    $10::authentication_method, $11, $12, $13, $14)
            "#,
            format!("f/test/trigger_{}", method),
            format!("api/{}", method),
            format!("api/{}", method),
            "f/test/handler",
            false,
            "test-workspace",
            "test-user",
            "test@windmill.dev",
            *method as _,
            "none" as _,
            false,
            false,
            false,
            false,
        )
        .execute(&db)
        .await?;
    }

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM http_trigger WHERE workspace_id = $1",
        "test-workspace",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(count, Some(methods.len() as i64));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_http_trigger_authentication_methods(db: Pool<Postgres>) -> anyhow::Result<()> {
    let auth_methods = ["none", "windmill", "api_key", "basic_http", "signature"];

    for (i, auth) in auth_methods.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO http_trigger (
                path, route_path, route_path_key, script_path, is_flow,
                workspace_id, edited_by, email, http_method,
                authentication_method, is_static_website, workspaced_route,
                wrap_body, raw_string
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::http_method,
                    $10::authentication_method, $11, $12, $13, $14)
            "#,
            format!("f/test/trigger_{}", i),
            format!("api/{}", i),
            format!("api/{}", i),
            "f/test/handler",
            false,
            "test-workspace",
            "test-user",
            "test@windmill.dev",
            "get" as _,
            *auth as _,
            false,
            false,
            false,
            auth == &"signature",
        )
        .execute(&db)
        .await?;
    }

    let sig_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM http_trigger
        WHERE workspace_id = $1 AND authentication_method = $2::authentication_method
        "#,
        "test-workspace",
        "signature" as _,
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(sig_count, Some(1));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_http_trigger_update(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO http_trigger (
            path, route_path, route_path_key, script_path, is_flow,
            workspace_id, edited_by, email, http_method,
            authentication_method, is_static_website, workspaced_route,
            wrap_body, raw_string
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::http_method,
                $10::authentication_method, $11, $12, $13, $14)
        "#,
        "f/test/trigger",
        "api/v1/old",
        "api/v1/old",
        "f/test/old_handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        "get" as _,
        "none" as _,
        false,
        false,
        false,
        false,
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "UPDATE http_trigger SET script_path = $1 WHERE workspace_id = $2 AND path = $3",
        "f/test/new_handler",
        "test-workspace",
        "f/test/trigger",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        "SELECT script_path FROM http_trigger WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.script_path, "f/test/new_handler");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_http_trigger_delete(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO http_trigger (
            path, route_path, route_path_key, script_path, is_flow,
            workspace_id, edited_by, email, http_method,
            authentication_method, is_static_website, workspaced_route,
            wrap_body, raw_string
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::http_method,
                $10::authentication_method, $11, $12, $13, $14)
        "#,
        "f/test/to_delete",
        "api/delete_me",
        "api/delete_me",
        "f/test/handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        "get" as _,
        "none" as _,
        false,
        false,
        false,
        false,
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "DELETE FROM http_trigger WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/to_delete",
    )
    .execute(&db)
    .await?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM http_trigger WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/to_delete",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(count, Some(0));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_http_trigger_mode_filtering(db: Pool<Postgres>) -> anyhow::Result<()> {
    let modes = ["enabled", "disabled", "suspended"];

    for (i, mode) in modes.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO http_trigger (
                path, route_path, route_path_key, script_path, is_flow,
                workspace_id, edited_by, email, http_method,
                authentication_method, is_static_website, workspaced_route,
                wrap_body, raw_string, mode
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::http_method,
                    $10::authentication_method, $11, $12, $13, $14, $15::trigger_mode)
            "#,
            format!("f/test/trigger_{}", i),
            format!("api/{}", i),
            format!("api/{}", i),
            "f/test/handler",
            false,
            "test-workspace",
            "test-user",
            "test@windmill.dev",
            "get" as _,
            "none" as _,
            false,
            false,
            false,
            false,
            *mode as _,
        )
        .execute(&db)
        .await?;
    }

    // Query for active triggers (enabled or suspended, matching refresh_routers logic)
    let active_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM http_trigger
        WHERE workspace_id = $1
          AND (mode = 'enabled'::trigger_mode OR mode = 'suspended'::trigger_mode)
        "#,
        "test-workspace",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(active_count, Some(2));

    Ok(())
}

// ============================================================================
// Other Trigger Types Tests (DB schema validation)
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_websocket_trigger_insert(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO websocket_trigger (
            path, url, script_path, is_flow, workspace_id,
            edited_by, email
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        "f/test/ws_trigger",
        "wss://example.com/feed",
        "f/test/ws_handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        r#"
        SELECT url, script_path, mode AS "mode: String"
        FROM websocket_trigger WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/ws_trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.url, "wss://example.com/feed");
    assert_eq!(trigger.script_path, "f/test/ws_handler");
    assert_eq!(trigger.mode, "enabled");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_kafka_trigger_insert(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO kafka_trigger (
            path, kafka_resource_path, topics, group_id, script_path,
            is_flow, workspace_id, edited_by, email
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "f/test/kafka_trigger",
        "u/admin/kafka_resource",
        &["topic-a", "topic-b"] as &[&str],
        "my-consumer-group",
        "f/test/kafka_handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        r#"
        SELECT kafka_resource_path, topics, group_id, mode AS "mode: String"
        FROM kafka_trigger
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/kafka_trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.kafka_resource_path, "u/admin/kafka_resource");
    assert_eq!(trigger.topics, vec!["topic-a", "topic-b"]);
    assert_eq!(trigger.group_id, "my-consumer-group");
    assert_eq!(trigger.mode, "enabled");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_postgres_trigger_insert(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO postgres_trigger (
            path, script_path, is_flow, workspace_id, edited_by, email,
            postgres_resource_path, replication_slot_name, publication_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "f/test/pg_trigger",
        "f/test/pg_handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        "u/admin/pg_resource",
        "test_slot",
        "test_publication",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        r#"
        SELECT postgres_resource_path, replication_slot_name, publication_name, mode AS "mode: String"
        FROM postgres_trigger
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/pg_trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.postgres_resource_path, "u/admin/pg_resource");
    assert_eq!(trigger.replication_slot_name, "test_slot");
    assert_eq!(trigger.publication_name, "test_publication");
    assert_eq!(trigger.mode, "enabled");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_nats_trigger_insert(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO nats_trigger (
            path, nats_resource_path, subjects, script_path,
            is_flow, workspace_id, edited_by, email, use_jetstream
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "f/test/nats_trigger",
        "u/admin/nats_resource",
        &["orders.>", "payments.*"] as &[&str],
        "f/test/nats_handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        false,
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        r#"
        SELECT nats_resource_path, subjects, use_jetstream, mode AS "mode: String"
        FROM nats_trigger
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/nats_trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.nats_resource_path, "u/admin/nats_resource");
    assert_eq!(trigger.subjects, vec!["orders.>", "payments.*"]);
    assert_eq!(trigger.use_jetstream, false);
    assert_eq!(trigger.mode, "enabled");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_sqs_trigger_insert(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO sqs_trigger (
            path, queue_url, aws_resource_path, script_path,
            is_flow, workspace_id, edited_by, email
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        "f/test/sqs_trigger",
        "https://sqs.us-east-1.amazonaws.com/123456789/my-queue",
        "u/admin/aws_resource",
        "f/test/sqs_handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        r#"
        SELECT queue_url, aws_resource_path,
               aws_auth_resource_type AS "aws_auth_resource_type: String",
               mode AS "mode: String"
        FROM sqs_trigger
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/sqs_trigger",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(
        trigger.queue_url,
        "https://sqs.us-east-1.amazonaws.com/123456789/my-queue"
    );
    assert_eq!(trigger.aws_resource_path, "u/admin/aws_resource");
    assert_eq!(trigger.aws_auth_resource_type, "credentials");
    assert_eq!(trigger.mode, "enabled");

    Ok(())
}

// ============================================================================
// Cross-trigger tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_trigger_server_state_tracking(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO websocket_trigger (
            path, url, script_path, is_flow, workspace_id,
            edited_by, email, server_id, error
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "f/test/ws_with_error",
        "wss://example.com/feed",
        "f/test/handler",
        false,
        "test-workspace",
        "test-user",
        "test@windmill.dev",
        "server-abc-123",
        "connection refused",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        "SELECT server_id, error FROM websocket_trigger WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/ws_with_error",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.server_id, Some("server-abc-123".to_string()));
    assert_eq!(trigger.error, Some("connection refused".to_string()));

    sqlx::query!(
        "UPDATE websocket_trigger SET error = NULL WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/ws_with_error",
    )
    .execute(&db)
    .await?;

    let trigger = sqlx::query!(
        "SELECT error FROM websocket_trigger WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/ws_with_error",
    )
    .fetch_one(&db)
    .await?;

    assert!(trigger.error.is_none());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_trigger_mode_filtering(db: Pool<Postgres>) -> anyhow::Result<()> {
    let modes = ["enabled", "disabled", "enabled"];

    for (i, mode) in modes.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO websocket_trigger (
                path, url, script_path, is_flow, workspace_id,
                edited_by, email, mode
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::trigger_mode)
            "#,
            format!("f/test/ws_trigger_{}", i),
            "wss://example.com",
            "f/test/handler",
            false,
            "test-workspace",
            "test-user",
            "test@windmill.dev",
            *mode as _,
        )
        .execute(&db)
        .await?;
    }

    let enabled_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM websocket_trigger WHERE workspace_id = $1 AND mode = 'enabled'::trigger_mode",
        "test-workspace",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(enabled_count, Some(2));

    let disabled_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM websocket_trigger WHERE workspace_id = $1 AND mode = 'disabled'::trigger_mode",
        "test-workspace",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(disabled_count, Some(1));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_multiple_capture_configs_per_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    let trigger_kinds = ["webhook", "email", "kafka"];

    for kind in &trigger_kinds {
        sqlx::query!(
            r#"
            INSERT INTO capture_config (workspace_id, path, is_flow, trigger_kind, owner, email)
            VALUES ($1, $2, $3, $4::trigger_kind, $5, $6)
            "#,
            "test-workspace",
            "f/test/multi_trigger_script",
            false,
            *kind as _,
            "test-user",
            "test@windmill.dev",
        )
        .execute(&db)
        .await?;
    }

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM capture_config WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        "f/test/multi_trigger_script",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(count, Some(3));

    Ok(())
}

// ============================================================================
// Schedule Tests (DB-level)
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_schedule_insert_and_query(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO schedule (
            workspace_id, path, edited_by, schedule, enabled,
            script_path, is_flow, email, timezone
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        "test-workspace",
        "f/test/my_schedule",
        "test-user",
        "0 */5 * * *",
        true,
        "f/test/scheduled_script",
        false,
        "test@windmill.dev",
        "UTC",
    )
    .execute(&db)
    .await?;

    let schedule = sqlx::query!(
        r#"
        SELECT path, schedule, enabled, script_path, timezone
        FROM schedule
        WHERE workspace_id = $1 AND path = $2
        "#,
        "test-workspace",
        "f/test/my_schedule",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(schedule.path, "f/test/my_schedule");
    assert_eq!(schedule.schedule, "0 */5 * * *");
    assert_eq!(schedule.enabled, true);
    assert_eq!(schedule.script_path, "f/test/scheduled_script");
    assert_eq!(schedule.timezone, "UTC");

    Ok(())
}
