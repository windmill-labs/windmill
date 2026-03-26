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
async fn test_npm_proxy_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/npm_proxy");

    let resp = authed(client().get(format!("{base}/metadata/lodash")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /metadata/lodash",
    );

    let resp = authed(client().get(format!("{base}/resolve/lodash")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /resolve/lodash",
    );

    let resp = authed(client().get(format!("{base}/filetree/lodash@4.17.21")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /filetree/lodash@4.17.21",
    );

    let resp = authed(client().get(format!("{base}/file/lodash@4.17.21/lodash.js")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /file/lodash@4.17.21/lodash.js",
    );

    Ok(())
}
