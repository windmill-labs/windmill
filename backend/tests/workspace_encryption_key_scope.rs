//! The workspace encryption key must never reach a scope-restricted token.
//!
//! The route that serves it and the tarball export that embeds it both resolve
//! to `workspaces:read`, while the key decrypts every secret variable of the
//! workspace offline — so a `workspaces:read` token would recover, past its own
//! scopes, what `variables:read` gates on the same handler (GHSA-g3x2-mwm6-jrc3).
//! Replacing the key reaches the same secrets, so it is held to the same bar.

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

    // 403 has other producers on these routes (`require_admin`, a route-scope denial),
    // so every case pins the guard's own message rather than the status alone.
    let resp = authed(
        client().get(format!("{ws}/workspaces/encryption_key")),
        &scoped,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 403, "workspaces:read must not read the key");
    assert!(
        resp.text().await?.contains("without scopes"),
        "the 403 must come from the encryption-key guard"
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
        "workspaces:read must not export the key"
    );
    assert!(
        resp.text().await?.contains("without scopes"),
        "the 403 must come from the encryption-key guard"
    );

    // Replacing the key is the same capability: the server re-encrypts every secret
    // under a key the caller chose. `workspaces:write` reaches the route, so a 403 here
    // is the guard rather than the route's own scope check.
    let scoped_write = mint_scoped_token(port, vec!["workspaces:write"]).await?;
    let resp = authed(
        client().post(format!("{ws}/workspaces/encryption_key")),
        &scoped_write,
    )
    .json(&json!({ "new_key": "a".repeat(64) }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        403,
        "workspaces:write token must not replace the encryption key"
    );
    assert!(
        resp.text().await?.contains("without scopes"),
        "the 403 must come from the encryption-key guard"
    );

    // The same admin's unscoped token keeps both read paths working.
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
