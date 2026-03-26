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
async fn test_settings_route_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/settings");

    let resp = authed(client().get(format!("{base}/envs"))).send().await?;
    assert_route_matched(resp.status().as_u16(), &resp.text().await?, "GET /envs");

    let resp = authed(client().get(format!("{base}/global/hub_base_url")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /global/hub_base_url",
    );

    let resp = authed(client().post(format!("{base}/global/test_key")))
        .json(&json!({"value": "test"}))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /global/test_key",
    );

    let resp = authed(client().get(format!("{base}/list_global")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /list_global",
    );

    let resp = authed(client().get(format!("{base}/instance_config")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /instance_config",
    );

    let resp = authed(client().get(format!("{base}/instance_config/yaml")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /instance_config/yaml",
    );

    let resp = authed(client().get(format!("{base}/get_stats")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /get_stats",
    );

    let resp = authed(client().get(format!("{base}/latest_key_renewal_attempt")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /latest_key_renewal_attempt",
    );

    let resp = authed(client().get(format!("{base}/critical_alerts")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /critical_alerts",
    );

    let resp = authed(client().post(format!("{base}/critical_alerts/0/acknowledge")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /critical_alerts/0/acknowledge",
    );

    let resp = authed(client().post(format!("{base}/critical_alerts/acknowledge_all")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /critical_alerts/acknowledge_all",
    );

    let resp = authed(client().post(format!("{base}/sync_cached_resource_types")))
        .send()
        .await?;
    assert_route_matched(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /sync_cached_resource_types",
    );

    Ok(())
}
