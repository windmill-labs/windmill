use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn flow_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/flows/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(flow_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

fn new_flow(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "value": {
            "modules": []
        },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_flow_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/flows");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_flow("u/test-user/test_flow", "Test flow"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // create second flow
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_flow("u/test-user/another_flow", "Another flow"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get by path ---
    let resp = authed_get(port, "get", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");
    assert_eq!(body["summary"], "Test flow");

    // get not found
    let resp = authed_get(port, "get", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get draft ---
    let resp = authed_get(port, "get/draft", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 flows, got {}",
        list.len()
    );
    assert!(list.iter().any(|f| f["path"] == "u/test-user/test_flow"));

    // --- list_search ---
    let resp = authed(client().get(format!("{base}/list_search")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- list_paths ---
    let resp = authed(client().get(format!("{base}/list_paths")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let paths = resp.json::<Vec<String>>().await?;
    assert!(paths.contains(&"u/test-user/test_flow".to_string()));

    // --- history ---
    let resp = authed_get(port, "history/p", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!history.is_empty());

    // --- get_latest_version ---
    let resp = authed_get(port, "get_latest_version", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);

    // --- deployment_status ---
    let resp = authed_get(port, "deployment_status/p", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);

    // --- list_tokens ---
    let resp = authed_get(port, "list_tokens", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- list_paths_from_workspace_runnable ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/list_paths_from_workspace_runnable/flow/u/test-user/test_flow"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- get by version ---
    let version = &history[0]["id"];
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/get/v/{version}"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");

    // --- update ---
    let resp = authed(client().post(flow_url(port, "update", "u/test-user/test_flow")))
        .json(&new_flow("u/test-user/test_flow", "Updated flow"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // verify update
    let resp = authed_get(port, "get", "u/test-user/test_flow").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated flow");

    // history should have 2 entries
    let resp = authed_get(port, "history/p", "u/test-user/test_flow").await;
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        history.len() >= 2,
        "expected at least 2 history entries, got {}",
        history.len()
    );
    let latest_version = &history[0]["id"];

    // --- get by version + path ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/get/v/{latest_version}/p/u/test-user/test_flow"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");

    // --- history_update ---
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/history_update/v/{latest_version}"
    )))
    .json(&json!({"deployment_msg": "deployed v2"}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "history_update: {}", resp.text().await?);

    // --- get_triggers_count ---
    let resp = authed(client().get(flow_url(
        port,
        "get_triggers_count",
        "u/test-user/test_flow",
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // --- toggle_workspace_error_handler (EE-gated, expect 400 in OSS) ---
    let resp = authed(client().post(flow_url(
        port,
        "toggle_workspace_error_handler",
        "u/test-user/test_flow",
    )))
    .json(&json!({}))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "toggle_workspace_error_handler: unexpected status {}",
        resp.status()
    );

    // --- archive ---
    let resp = authed(client().post(flow_url(port, "archive", "u/test-user/another_flow")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // archived flow should still be gettable
    let resp = authed_get(port, "get", "u/test-user/another_flow").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["archived"], true);

    // --- delete ---
    let resp = authed(client().delete(flow_url(port, "delete", "u/test-user/another_flow")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/another_flow").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // ===== Hub endpoints (require external network, expect 500 or 200) =====

    // --- hub/list ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/flows/hub/list"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/list: unexpected status {}",
        resp.status()
    );

    // --- hub/get ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/flows/hub/get/1"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get: unexpected status {}",
        resp.status()
    );

    Ok(())
}
