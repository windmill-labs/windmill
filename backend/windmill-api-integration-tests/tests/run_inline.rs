use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[cfg(feature = "run_inline")]
async fn init_inline_utils(port: u16) -> anyhow::Result<()> {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let (killpill_tx, killpill_rx) = windmill_common::KillpillSender::new(1);
        let base_internal_url = format!("http://localhost:{}", port);
        windmill_worker::init_worker_internal_server_inline_utils(killpill_rx, base_internal_url)
            .expect("Failed to initialize inline utils");
        // Keep killpill_tx alive for the test duration
        std::mem::forget(killpill_tx);
    });

    Ok(())
}

fn run_inline_url(port: u16, endpoint: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/jobs/run_inline/{endpoint}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn new_script(
    path: &str,
    summary: &str,
    content: &str,
    language: &str,
    schema_properties: serde_json::Value,
) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "content": content,
        "language": language,
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": schema_properties,
            "required": []
        }
    })
}

#[cfg(feature = "run_inline")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Initialize inline utils for script execution
    init_inline_utils(port).await?;

    // Create a DuckDB script (one of the languages that supports inline execution)
    // DuckDB requires parameter declarations in comments: -- $param_name (type)
    let script_path = "u/test-user/inline_test";
    let script_content = "-- $x (integer)
-- $y (integer)
SELECT $x + $y as result";

    let resp = authed(client().post(format!("{base}/scripts/create")))
        .json(&new_script(
            script_path,
            "Inline test script",
            script_content,
            "duckdb",
            json!({
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            }),
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create script: {}", resp.text().await?);

    // Test run_inline by path with args
    let resp = authed(client().post(run_inline_url(port, &format!("p/{script_path}"))))
        .json(&json!({
            "args": {
                "x": 5,
                "y": 15
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "run_inline by path with args failed");
    let result = resp.json::<serde_json::Value>().await?;
    // DuckDB query should return array with one row containing result field
    // The result structure is [{"result": 20}]
    assert!(result.is_array(), "expected array result, got: {}", result);
    let rows = result.as_array().unwrap();
    assert_eq!(rows.len(), 1, "expected 1 row, got: {}", rows.len());
    assert_eq!(
        rows[0]["result"],
        json!(20),
        "expected result 20, got: {}",
        rows[0]
    );

    Ok(())
}

#[cfg(feature = "run_inline")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_hash(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Initialize inline utils for script execution
    init_inline_utils(port).await?;

    // Create a DuckDB script and get its hash
    let script_path = "u/test-user/inline_hash_test";
    let script_content = "-- $a (integer)
-- $b (integer)
SELECT $a * $b as product";

    let resp = authed(client().post(format!("{base}/scripts/create")))
        .json(&new_script(
            script_path,
            "Inline hash test script",
            script_content,
            "duckdb",
            json!({
                "a": {"type": "integer"},
                "b": {"type": "integer"}
            }),
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create script: {}", resp.text().await?);

    // Get the script to retrieve its hash
    let resp = authed(client().get(format!("{base}/scripts/get/p/{script_path}")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let script_data = resp.json::<serde_json::Value>().await?;
    let hash = script_data["hash"]
        .as_str()
        .expect("hash should be present");

    // Test run_inline by hash with args
    let resp = authed(client().post(run_inline_url(port, &format!("h/{hash}"))))
        .json(&json!({
            "args": {
                "a": 7,
                "b": 3
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "run_inline by hash with args failed");
    let result = resp.json::<serde_json::Value>().await?;
    // Should return array with one row: [{"product": 21}]
    assert!(result.is_array(), "expected array result");
    let rows = result.as_array().unwrap();
    assert_eq!(rows.len(), 1, "expected 1 row");
    assert_eq!(
        rows[0]["product"],
        json!(21),
        "expected product 21, got: {}",
        rows[0]
    );

    Ok(())
}

#[cfg(feature = "run_inline")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_preview(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Initialize inline utils for script execution
    init_inline_utils(port).await?;

    // Test run_inline preview with direct DuckDB content
    let resp = authed(client().post(run_inline_url(port, "preview")))
        .json(&json!({
            "content": "-- $msg (text)\nSELECT 'Hello, ' || $msg || '!' as greeting",
            "language": "duckdb",
            "args": {
                "msg": "World"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "run_inline preview failed");
    let result = resp.json::<serde_json::Value>().await?;
    // Should return array with one row: [{"greeting": "Hello, World!"}]
    assert!(result.is_array(), "expected array result");
    let rows = result.as_array().unwrap();
    assert_eq!(rows.len(), 1, "expected 1 row");
    assert_eq!(
        rows[0]["greeting"],
        json!("Hello, World!"),
        "expected 'Hello, World!', got: {}",
        rows[0]
    );

    Ok(())
}

#[cfg(feature = "run_inline")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_nonexistent_script(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Initialize inline utils
    init_inline_utils(port).await?;

    // Test run_inline by path with non-existent script - should return an error
    let resp = authed(client().post(run_inline_url(port, "p/u/test-user/nonexistent_script")))
        .json(&json!({
            "args": null
        }))
        .send()
        .await
        .unwrap();

    // Should return an error (script not found)
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "expected error status for nonexistent script, got: {}",
        resp.status()
    );

    Ok(())
}
