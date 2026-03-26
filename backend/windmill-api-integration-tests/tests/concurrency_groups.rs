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

const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_concurrency_groups_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/concurrency_groups/list"
    )))
    .send()
    .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /api/concurrency_groups/list",
    );

    let resp = authed(client().delete(format!(
        "http://localhost:{port}/api/concurrency_groups/prune/fake/key"
    )))
    .send()
    .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "DELETE /api/concurrency_groups/prune/fake/key",
    );

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/concurrency_groups/{FAKE_UUID}/key"
    )))
    .send()
    .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /api/concurrency_groups/{id}/key",
    );

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/concurrency_groups/list_jobs"
    )))
    .send()
    .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /api/w/test-workspace/concurrency_groups/list_jobs",
    );

    Ok(())
}
