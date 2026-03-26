use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_route_matched(status: u16, body: &str, endpoint: &str) {
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 (empty body) for {} -- route pattern not matched",
        endpoint,
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ai_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/ai/proxy/chat/completions"
    )))
    .json(&json!({}))
    .send()
    .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /w/:workspace/ai/proxy/chat/completions",
    );

    let resp = client()
        .post(format!(
            "http://localhost:{port}/api/ai/proxy/chat/completions"
        ))
        .json(&json!({}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /ai/proxy/chat/completions",
    );

    Ok(())
}
