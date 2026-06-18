use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn authed_with(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

/// Start a mock AI API that echoes back a valid chat completion response.
async fn start_mock_ai_api() -> u16 {
    use axum::{routing::post, Json, Router};

    let app = Router::new().fallback(post(|| async {
        Json(json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "hello"}}]
        }))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    port
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ai_proxy_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // Allow the mock AI API on 127.0.0.1 to pass the SSRF check
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Start mock AI API
    let mock_port = start_mock_ai_api().await;
    let mock_url = format!("http://127.0.0.1:{mock_port}/v1");

    // Create an openai resource pointing to the mock
    let resp = authed(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/resources/create"
            ))
            .json(&json!({
                "path": "f/ai/openai_config",
                "resource_type": "openai",
                "value": {
                    "api_key": "test-key",
                    "base_url": mock_url
                }
            })),
    )
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "create openai resource",
    );

    // Set ai_config on workspace_settings directly via SQL
    sqlx::query(
        "UPDATE workspace_settings SET ai_config = $1::jsonb WHERE workspace_id = 'test-workspace'",
    )
    .bind(json!({
        "providers": {
            "openai": {
                "resource_path": "f/ai/openai_config",
                "models": ["gpt-4"]
            }
        }
    }))
    .execute(&db)
    .await?;

    // POST /w/{ws}/ai/proxy/chat/completions
    let resp = authed(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/ai/proxy/chat/completions"
            ))
            .header("X-Provider", "openai")
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hi"}]
            })),
    )
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /ai/proxy/chat/completions",
    );

    Ok(())
}

/// Regression test for WIN-1971: the AI proxy's X-Resource-Path header must
/// honour resource RLS so that a low-privilege user cannot point the proxy
/// at a resource they are not allowed to read.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ai_proxy_x_resource_path_enforces_rls(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let mock_port = start_mock_ai_api().await;
    let mock_url = format!("http://127.0.0.1:{mock_port}/v1");

    // Resource owned by test-user (admin). With default extra_perms {} the
    // RLS `see_own` policy restricts SELECT to user `test-user`.
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by) \
         VALUES ('test-workspace', 'u/test-user/restricted_openai', $1::jsonb, 'openai', '{}', 'test-user')",
    )
    .bind(json!({
        "api_key": "sk-secret-restricted",
        "base_url": mock_url,
    }))
    .execute(&db)
    .await?;

    // Sanity-check: normal resource API rejects test-user-3 (non-admin) for the restricted path.
    let resp = authed_with(
        client().get(format!(
            "http://localhost:{port}/api/w/test-workspace/resources/get/u/test-user/restricted_openai"
        )),
        "SECRET_TOKEN_3",
    )
    .send()
    .await?;
    assert!(
        resp.status().as_u16() >= 400,
        "normal resource API should deny test-user-3 reading restricted resource, got {}",
        resp.status()
    );

    // The vulnerability: as a non-admin user, point X-Resource-Path at the
    // restricted resource. Must be rejected before the proxy fetches/uses it.
    let resp = authed_with(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/ai/proxy/chat/completions"
            ))
            .header("X-Provider", "openai")
            .header("X-Resource-Path", "u/test-user/restricted_openai")
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hi"}]
            })),
        "SECRET_TOKEN_3",
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert!(
        status >= 400,
        "non-admin user should be rejected when X-Resource-Path points at a resource they cannot read, got {status}: {body}",
    );

    // A resource the non-admin owns must still work through X-Resource-Path.
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by) \
         VALUES ('test-workspace', 'u/test-user-3/own_openai', $1::jsonb, 'openai', '{}', 'test-user-3')",
    )
    .bind(json!({
        "api_key": "sk-self",
        "base_url": mock_url,
    }))
    .execute(&db)
    .await?;

    let resp = authed_with(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/ai/proxy/chat/completions"
            ))
            .header("X-Provider", "openai")
            .header("X-Resource-Path", "u/test-user-3/own_openai")
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hi"}]
            })),
        "SECRET_TOKEN_3",
    )
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "non-admin with X-Resource-Path on owned resource",
    );

    // Admin must still be able to use X-Resource-Path on any resource.
    let resp = authed(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/ai/proxy/chat/completions"
            ))
            .header("X-Provider", "openai")
            .header("X-Resource-Path", "u/test-user/restricted_openai")
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hi"}]
            })),
    )
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "admin with X-Resource-Path on restricted resource",
    );

    Ok(())
}
