use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_granular_acls_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/acls");

    // GET /acls/get/group_/all → 200
    let resp = authed(client().get(format!("{base}/get/group_/all")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /acls/get/group_/all");

    // POST /acls/add/group_/all → 200
    let resp = authed(client().post(format!("{base}/add/group_/all")))
        .json(&json!({"owner": "u/test-user-2", "write": true}))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /acls/add/group_/all");

    // POST /acls/remove/group_/all → 200
    let resp = authed(client().post(format!("{base}/remove/group_/all")))
        .json(&json!({"owner": "u/test-user-2"}))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /acls/remove/group_/all");

    Ok(())
}
