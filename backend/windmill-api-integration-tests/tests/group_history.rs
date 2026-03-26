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
async fn test_group_history_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/groups_history");

    let resp = authed(client().get(format!("{base}/get/all")))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /get/all");

    Ok(())
}
