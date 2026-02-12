use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn script_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/scripts/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(script_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

fn new_script(path: &str, summary: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_script_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/test_script",
            "Test script",
            "export async function main() { return 42; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // create second script
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/another_script",
            "Another script",
            "export async function main() { return 'hello'; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists/p", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists/p", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get by path ---
    let resp = authed_get(port, "get/p", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_script");
    assert_eq!(body["summary"], "Test script");
    assert_eq!(body["language"], "deno");
    assert!(body["hash"].is_string(), "expected hash to be a hex string");
    let hash = body["hash"].as_str().unwrap().to_string();

    // get not found
    let resp = authed_get(port, "get/p", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get by hash ---
    let resp = authed_get(port, "get/h", &hash).await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_script");

    // --- get draft ---
    let resp = authed_get(port, "get/draft", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_script");

    // --- raw by path (requires language extension) ---
    let resp = authed_get(port, "raw/p", "u/test-user/test_script.ts").await;
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.contains("return 42"), "expected script content, got: {body}");

    // --- raw by hash (requires .ts suffix) ---
    let resp = authed_get(port, "raw/h", &format!("{hash}.ts")).await;
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.contains("return 42"));

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 scripts, got {}",
        list.len()
    );
    assert!(list.iter().any(|s| s["path"] == "u/test-user/test_script"));

    // list with path_start filter
    let resp = authed(client().get(format!(
        "{base}/list?path_start=u/test-user/another"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["path"], "u/test-user/another_script");

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
    assert!(paths.contains(&"u/test-user/test_script".to_string()));

    // --- history ---
    let resp = authed_get(port, "history/p", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!history.is_empty());

    // --- get_latest_version ---
    let resp = authed_get(port, "get_latest_version", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);

    // --- deployment_status ---
    let resp = authed_get(port, "deployment_status/h", &hash).await;
    assert_eq!(resp.status(), 200);

    // --- raw_unpinned by path ---
    let resp = authed_get(port, "raw_unpinned/p", "u/test-user/test_script.ts").await;
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.contains("return 42"));

    // --- list_tokens ---
    let resp = authed_get(port, "list_tokens", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- list_paths_from_workspace_runnable ---
    let resp = authed_get(
        port,
        "list_paths_from_workspace_runnable",
        "u/test-user/test_script",
    )
    .await;
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- update script (create new version) ---
    let mut updated = new_script(
        "u/test-user/test_script",
        "Updated test script",
        "export async function main() { return 99; }",
    );
    updated["parent_hash"] = json!(&hash);
    let resp = authed(client().post(format!("{base}/create")))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "update: {}", resp.text().await?);

    // verify new version
    let resp = authed_get(port, "get/p", "u/test-user/test_script").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated test script");
    let new_hash = body["hash"].as_str().unwrap();
    assert_ne!(new_hash, hash, "hash should change on update");

    // history should have 2 entries now
    let resp = authed_get(port, "history/p", "u/test-user/test_script").await;
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        history.len() >= 2,
        "expected at least 2 history entries, got {}",
        history.len()
    );

    // --- history_update ---
    let resp = authed(client().post(format!(
        "{base}/history_update/h/{new_hash}/p/u/test-user/test_script"
    )))
    .json(&json!({"deployment_msg": "deployed v2"}))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "history_update: {}",
        resp.text().await?
    );

    // --- toggle_workspace_error_handler (EE-gated, expect 400 in OSS) ---
    let resp = authed(client().post(script_url(
        port,
        "toggle_workspace_error_handler/p",
        "u/test-user/test_script",
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

    // --- get_triggers_count ---
    let resp = authed_get(port, "get_triggers_count", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);

    // --- tokened_raw (global unauthed, token in URL) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/scripts_u/tokened_raw/test-workspace/SECRET_TOKEN/u/test-user/test_script.ts"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "tokened_raw: {}",
        resp.text().await?
    );

    // --- archive by path ---
    let resp = authed(client().post(script_url(
        port,
        "archive/p",
        "u/test-user/another_script",
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // archived script should still be gettable
    let resp = authed_get(port, "get/p", "u/test-user/another_script").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["archived"], true);
    let another_hash = body["hash"].as_str().unwrap().to_string();

    // --- archive by hash ---
    let resp = authed(client().post(script_url(port, "archive/h", &another_hash)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete by hash ---
    let resp = authed(client().post(script_url(port, "delete/h", &another_hash)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete_bulk ---
    let resp = authed(client().delete(format!("{base}/delete_bulk")))
        .json(&json!({"paths": ["u/test-user/test_script"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete_bulk: {}", resp.text().await?);

    let resp = authed_get(port, "exists/p", "u/test-user/test_script").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // --- empty_ts (global unauthed) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/scripts_u/empty_ts/u/test-user/any_script"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.is_empty(), "expected empty string, got: {body}");

    // ===== Hub endpoints (require external network, expect 500 or 200) =====

    // --- hub/top ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/scripts/hub/top"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/top: unexpected status {}",
        resp.status()
    );

    // --- hub/get (raw script by path, needs hub/ prefix in path) ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/scripts/hub/get/hub/1/hello"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get: unexpected status {}",
        resp.status()
    );

    // --- hub/get_full (full script by path, needs hub/ prefix in path) ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/scripts/hub/get_full/hub/1/hello"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get_full: unexpected status {}",
        resp.status()
    );

    // --- integrations hub/list ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/integrations/hub/list"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "integrations hub/list: unexpected status {}",
        resp.status()
    );

    Ok(())
}
