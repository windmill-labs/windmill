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

/// Start a mock npm registry that returns valid JSON for any GET request.
async fn start_mock_registry() -> u16 {
    use axum::{routing::get, Json, Router};

    let app = Router::new().fallback(get(|| async {
        Json(json!({
            "name": "test-package",
            "versions": {"1.0.0": {"name": "test-package", "version": "1.0.0"}},
            "dist-tags": {"latest": "1.0.0"}
        }))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    port
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_npm_proxy_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/npm_proxy");

    // Start mock npm registry
    let mock_port = start_mock_registry().await;
    let mock_url = format!("http://127.0.0.1:{mock_port}");

    // Configure the npm registry to point to our mock
    let resp = authed(
        client()
            .post(format!(
                "http://localhost:{port}/api/settings/global/npm_config_registry"
            ))
            .json(&json!({"value": mock_url})),
    )
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /settings/global/npm_config_registry",
    );

    // GET /metadata/{package}
    let resp = authed(client().get(format!("{base}/metadata/lodash")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /npm_proxy/metadata/lodash",
    );

    // GET /resolve/{package}
    let resp = authed(client().get(format!("{base}/resolve/lodash")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /npm_proxy/resolve/lodash",
    );

    Ok(())
}
