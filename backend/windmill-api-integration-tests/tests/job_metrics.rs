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
async fn test_job_metrics_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/job_metrics");

    let resp = authed(client().post(format!("{base}/get/{FAKE_UUID}")))
        .json(&json!({}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /get/{id}",
    );

    let resp = authed(client().post(format!("{base}/set_progress/{FAKE_UUID}")))
        .json(&json!({"percent": 50}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /set_progress/{id}",
    );

    let resp = authed(client().get(format!("{base}/get_progress/{FAKE_UUID}")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_progress/{id}",
    );

    Ok(())
}
