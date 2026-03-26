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
async fn test_capture_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // POST /capture/set_config → 200 (authed)
    let resp = authed(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/capture/set_config"
            ))
            .json(&json!({
                "trigger_kind": "webhook",
                "path": "u/test-user/test_capture",
                "is_flow": false
            })),
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /capture/set_config");

    // GET /capture/list/{...} → 200 (authed)
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/capture/list/script/u/test-user/test_capture"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(
        status,
        &body,
        "GET /capture/list/script/u/test-user/test_capture",
    );

    Ok(())
}
