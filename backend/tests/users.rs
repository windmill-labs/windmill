use sqlx::{Pool, Postgres};

mod common;
use common::*;

fn user_url(port: u16, endpoint: &str, name: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/users/{endpoint}/{name}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(fixtures("base"))]
async fn test_user_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/users");

    // --- whoami ---
    let resp = authed(client().get(format!("{base}/whoami")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");
    assert_eq!(body["email"], "test@windmill.dev");
    assert_eq!(body["is_admin"], true);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());
    assert!(list.iter().any(|u| u["username"] == "test-user"));

    // --- list_usernames ---
    let resp = authed(client().get(format!("{base}/list_usernames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let usernames = resp.json::<Vec<String>>().await?;
    assert!(usernames.contains(&"test-user".to_string()));

    // --- get ---
    let resp = authed(client().get(user_url(port, "get", "test-user")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");

    // --- whois ---
    let resp = authed(client().get(user_url(port, "whois", "test-user")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");

    // --- username_to_email ---
    let resp = authed(client().get(user_url(port, "username_to_email", "test-user")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let email = resp.text().await?;
    assert_eq!(email, "test@windmill.dev");

    // --- is_owner ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/users/is_owner/u/test-user/test"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    Ok(())
}
