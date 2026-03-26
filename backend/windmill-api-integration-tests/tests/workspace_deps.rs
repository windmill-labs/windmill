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
async fn test_workspace_deps_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspace_dependencies");

    let resp = authed(client().get(format!("{base}/list"))).send().await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /list");

    let resp = authed(client().get(format!("{base}/get_latest/python3")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_latest/python3",
    );

    let resp = authed(client().post(format!("{base}/archive/python3")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /archive/python3",
    );

    let resp = authed(client().post(format!("{base}/delete/python3")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /delete/python3",
    );

    Ok(())
}
