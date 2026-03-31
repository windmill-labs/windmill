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
async fn test_folder_history_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Create a folder first
    let resp = authed(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/folders/create"
            ))
            .json(&json!({"name": "test_hist_folder", "owners": ["u/test-user"]})),
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /folders/create");

    // GET /folders_history/get/{folder} → 200 (empty array)
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/folders_history/get/test_hist_folder"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /folders_history/get/test_hist_folder");

    Ok(())
}
