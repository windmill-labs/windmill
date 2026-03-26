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

const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_inputs_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    let resp = authed(client().get(format!(
        "{base}/history?runnable_id=fake&runnable_type=ScriptHash"
    )))
    .send()
    .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /history");

    let resp = authed(client().get(format!(
        "{base}/list?runnable_id=fake&runnable_type=ScriptHash"
    )))
    .send()
    .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /list");

    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({"name": "test", "args": {}}))
        .send()
        .await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "POST /create");

    let resp = authed(client().post(format!("{base}/delete/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /delete/{id}",
    );

    let resp = authed(client().get(format!("{base}/{FAKE_UUID}/args")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /{id}/args",
    );

    Ok(())
}
