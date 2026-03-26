use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn assert_route_matched(status: u16, body: &str, endpoint: &str) {
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 (empty body) for {} -- route pattern not matched",
        endpoint,
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_capture_unauthed_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/capture_u/webhook/ScriptHash/u/test-user/fake_script");

    let resp = client().head(&base).send().await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "HEAD /webhook");

    let resp = client().post(&base).json(&json!({})).send().await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "POST /webhook");

    Ok(())
}
