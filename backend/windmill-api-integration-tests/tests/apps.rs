use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn app_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/apps/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(app_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

fn new_app(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "value": {
            "type": "rawapp",
            "inline_script": null
        },
        "policy": {
            "execution_mode": "anonymous",
            "triggerables": {},
            "on_behalf_of": null,
            "on_behalf_of_email": null
        }
    })
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_app_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/apps");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app("u/test-user/test_app", "Test app"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // create second app
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app("u/test-user/another_app", "Another app"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get by path ---
    let resp = authed_get(port, "get/p", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_app");
    assert_eq!(body["summary"], "Test app");

    // get not found
    let resp = authed_get(port, "get/p", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get draft ---
    let resp = authed_get(port, "get/draft", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_app");

    // --- get lite ---
    let resp = authed_get(port, "get/lite", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 apps, got {}",
        list.len()
    );
    assert!(list.iter().any(|a| a["path"] == "u/test-user/test_app"));

    // --- list_search ---
    let resp = authed(client().get(format!("{base}/list_search")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- history ---
    let resp = authed_get(port, "history/p", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!history.is_empty());

    // --- get by version ---
    let version = &history[0]["version"];
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/apps/get/v/{version}"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // --- custom_path_exists ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/apps/custom_path_exists/nonexistent"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- secret_of ---
    let resp = authed_get(port, "secret_of", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let secret_id = resp.text().await?;
    assert!(!secret_id.is_empty());

    // --- get_latest_version ---
    let resp = authed_get(port, "get_latest_version", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);

    // --- public_app (unauthed, by secret) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/apps_u/public_app/{secret_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "public_app: {}",
        resp.text().await?
    );

    // --- secret_of_latest_version ---
    let resp = authed_get(port, "secret_of_latest_version", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let secret = resp.text().await?;
    assert!(!secret.is_empty());

    // --- list_paths_from_workspace_runnable ---
    let resp = authed(client().get(format!(
        "{base}/list_paths_from_workspace_runnable/script/u/test-user/test_app"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "list_paths_from_workspace_runnable: {}",
        resp.text().await?
    );


    // --- history_update ---
    let app_body = authed_get(port, "get/p", "u/test-user/test_app").await;
    let app = app_body.json::<serde_json::Value>().await?;
    let app_id = &app["id"];
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/apps/history_update/a/{app_id}/v/{version}"
    )))
    .json(&json!({"deployment_msg": "deployed v1"}))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "history_update: {}",
        resp.text().await?
    );

    // --- update ---
    let resp = authed(client().post(app_url(port, "update", "u/test-user/test_app")))
        .json(&json!({
            "summary": "Updated app",
            "policy": {
                "execution_mode": "anonymous",
                "triggerables": {},
                "on_behalf_of": null,
                "on_behalf_of_email": null
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // verify update
    let resp = authed_get(port, "get/p", "u/test-user/test_app").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated app");

    // --- delete ---
    let resp = authed(client().delete(app_url(port, "delete", "u/test-user/another_app")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/another_app").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // ===== Hub endpoints (require external network, expect 500 or 200) =====

    // --- hub/list ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/apps/hub/list"
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
        "http://localhost:{port}/api/apps/hub/get/1"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get: unexpected status {}",
        resp.status()
    );

    // --- hub/get_raw ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/apps/hub/get_raw/1"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get_raw: unexpected status {}",
        resp.status()
    );

    Ok(())
}
