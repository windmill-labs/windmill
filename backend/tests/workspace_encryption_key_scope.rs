//! The workspace encryption key must never reach a scope-restricted token.
//!
//! The route that serves it and the tarball export that embeds it both resolve
//! to `workspaces:read`, while the key decrypts every secret variable of the
//! workspace offline — so a `workspaces:read` token would recover, past its own
//! scopes, what `variables:read` gates on the same handler (GHSA-g3x2-mwm6-jrc3).

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Mint an API token for test-user (a workspace admin) restricted to `scopes`.
async fn mint_scoped_token(port: u16, scopes: Vec<&str>) -> anyhow::Result<String> {
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/users/tokens/create")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "label": "scoped", "scopes": scopes, "workspace_id": "test-workspace" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "mint scoped token");
    Ok(resp.text().await?)
}

#[sqlx::test(fixtures("base"))]
async fn test_workspace_key_denied_to_scoped_token(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let scoped = mint_scoped_token(port, vec!["workspaces:read"]).await?;

    let resp = authed(
        client().get(format!("{ws}/workspaces/encryption_key")),
        &scoped,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "workspaces:read token must not read the encryption key: {}",
        resp.text().await.unwrap_or_default()
    );

    let resp = authed(
        client().get(format!(
            "{ws}/workspaces/tarball?archive_type=tar&include_key=true"
        )),
        &scoped,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "workspaces:read token must not export the encryption key: {}",
        resp.text().await.unwrap_or_default()
    );

    // The same admin's unscoped token keeps both paths working.
    let resp = authed(
        client().get(format!("{ws}/workspaces/encryption_key")),
        ADMIN_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "unscoped admin token reads the key");

    let resp = authed(
        client().get(format!(
            "{ws}/workspaces/tarball?archive_type=tar&include_key=true"
        )),
        ADMIN_TOKEN,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "unscoped admin token exports the key");

    Ok(())
}
