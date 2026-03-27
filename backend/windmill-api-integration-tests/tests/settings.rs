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
async fn test_settings_2xx(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/settings");

    let resp = authed(client().get(format!("{base}/envs"))).send().await?;
    assert_2xx(resp.status().as_u16(), &resp.text().await?, "GET /envs");

    let resp = authed(client().get(format!("{base}/global/hub_base_url")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /global/hub_base_url",
    );

    let resp = authed(client().post(format!("{base}/global/test_key")))
        .json(&json!({"value": "test"}))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /global/test_key",
    );

    let resp = authed(client().get(format!("{base}/instance_config")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /instance_config",
    );

    let resp = authed(client().get(format!("{base}/instance_config/yaml")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /instance_config/yaml",
    );

    let resp = authed(client().get(format!("{base}/latest_key_renewal_attempt")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /latest_key_renewal_attempt",
    );

    let resp = authed(client().post(format!("{base}/sync_cached_resource_types")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /sync_cached_resource_types",
    );

    // --- Reachability only (need external services) ---

    let resp = authed(
        client()
            .post(format!("{base}/test_smtp"))
            .json(&json!({"to": "test@test.com", "subject": "test", "content": "test"})),
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for POST /test_smtp"
    );

    let resp = authed(
        client()
            .post(format!("{base}/test_license_key"))
            .json(&json!({"license_key": "fake"})),
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for POST /test_license_key"
    );

    Ok(())
}
