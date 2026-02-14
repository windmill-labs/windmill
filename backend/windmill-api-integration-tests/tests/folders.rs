use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn folder_url(port: u16, endpoint: &str, name: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/folders/{endpoint}/{name}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_folder_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/folders");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_folder",
            "summary": "A test folder",
            "display_name": "Test Folder"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);

    // create second folder
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "another_folder",
            "summary": "Another folder"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed(client().get(folder_url(port, "exists", "test_folder")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed(client().get(folder_url(port, "exists", "nonexistent")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get ---
    let resp = authed(client().get(folder_url(port, "get", "test_folder")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_folder");
    assert_eq!(body["summary"], "A test folder");

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|f| f["name"] == "test_folder"));

    // --- listnames ---
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let names = resp.json::<Vec<String>>().await?;
    assert!(names.contains(&"test_folder".to_string()));

    // --- update ---
    let resp = authed(client().post(folder_url(port, "update", "test_folder")))
        .json(&json!({"summary": "Updated summary"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().get(folder_url(port, "get", "test_folder")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated summary");

    // --- addowner ---
    let resp = authed(client().post(folder_url(port, "addowner", "test_folder")))
        .json(&json!({"owner": "u/test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "addowner: {}", resp.text().await?);

    // verify ownership
    let resp = authed(client().get(folder_url(port, "get", "test_folder")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    let owners = body["owners"].as_array().unwrap();
    assert!(
        owners.iter().any(|o| o.as_str() == Some("u/test-user")),
        "expected u/test-user in owners, got: {:?}",
        owners
    );

    // --- removeowner ---
    let resp = authed(client().post(folder_url(port, "removeowner", "test_folder")))
        .json(&json!({"owner": "u/test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- getusage ---
    let resp = authed(client().get(folder_url(port, "getusage", "test_folder")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- is_owner ---
    let resp = authed(client().get(format!(
        "{base}/is_owner/f/test_folder"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<bool>().await?;

    // --- delete ---
    let resp = authed(client().delete(folder_url(port, "delete", "another_folder")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().get(folder_url(port, "exists", "another_folder")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.json::<bool>().await?, false);

    Ok(())
}
