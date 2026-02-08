use serde_json::json;
use sqlx::{Pool, Postgres};

mod common;
use common::*;

fn group_url(port: u16, endpoint: &str, name: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/groups/{endpoint}/{name}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(fixtures("base"))]
async fn test_group_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/groups");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_group",
            "summary": "A test group"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);

    // create second group
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "another_group",
            "summary": "Another group"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create another: {}", resp.text().await?);

    // create duplicate -> error
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_group",
            "summary": "Duplicate"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // --- get ---
    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_group");
    assert_eq!(body["summary"], "A test group");

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|g| g["name"] == "test_group"));

    // --- listnames ---
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let names = resp.json::<Vec<String>>().await?;
    assert!(names.contains(&"test_group".to_string()));

    // --- update ---
    let resp = authed(client().post(group_url(port, "update", "test_group")))
        .json(&json!({"summary": "Updated summary"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated summary");

    // --- adduser ---
    let resp = authed(client().post(group_url(port, "adduser", "test_group")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "adduser: {}", resp.text().await?);

    // verify membership
    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    let members = body["members"].as_array().unwrap();
    assert!(
        members.iter().any(|m| m.as_str() == Some("test-user")),
        "expected test-user in members, got: {:?}",
        members
    );

    // --- removeuser ---
    let resp = authed(client().post(group_url(port, "removeuser", "test_group")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- is_owner ---
    let resp = authed(client().get(group_url(port, "is_owner", "test_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    // --- delete ---
    let resp = authed(client().delete(group_url(port, "delete", "another_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // verify deleted - get should 404 or the group shouldn't appear in list
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    let names = resp.json::<Vec<String>>().await?;
    assert!(!names.contains(&"another_group".to_string()));

    Ok(())
}
