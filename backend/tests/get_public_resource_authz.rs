//! Regression test for GHSA-qfg7-x243-5hg4 (component A).
//!
//! `GET /w/{workspace}/apps_u/public_resource/{path}` is intentionally
//! unauthenticated: anonymous viewers of public apps must be able to fetch the
//! app theme and the json_schema resources backing app forms.
//!
//! The bug was that the `f/app_themes/` branch ran an unconstrained
//! `SELECT value FROM resource WHERE path = $1 AND workspace_id = $2` with no
//! `resource_type` filter, so any resource of any type placed under the
//! `f/app_themes/` prefix (e.g. a credential resource with inline plaintext)
//! was disclosed to an unauthenticated caller.
//!
//! This test pins down:
//!   - a real `app_theme` resource is still served unauthenticated (the
//!     legitimate feature must not regress), and
//!   - a non-`app_theme` resource placed under `f/app_themes/` is NOT
//!     disclosed anymore (the fix).

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

#[sqlx::test(fixtures("base"))]
async fn test_public_resource_app_themes_type_constrained(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // A legitimate app theme.
    let resp = authed(
        client().post(format!("{ws}/resources/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "f/app_themes/theme_0",
        "resource_type": "app_theme",
        "value": { "name": "My Theme", "value": ".app{color:red}" }
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "creating the app_theme resource should succeed: {}",
        resp.text().await?
    );

    // A non-theme resource maliciously planted under the f/app_themes/ prefix,
    // carrying an inline plaintext secret.
    let resp = authed(
        client().post(format!("{ws}/resources/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "f/app_themes/evil",
        "resource_type": "postgresql",
        "value": { "host": "db.internal", "password": "PLAINTEXT_LEAK" }
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "creating the planted resource should succeed: {}",
        resp.text().await?
    );

    // The theme is still served to an UNAUTHENTICATED caller (no token).
    let resp = client()
        .get(format!("{ws}/apps_u/public_resource/f/app_themes/theme_0"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: Option<serde_json::Value> = resp.json().await?;
    let body = body.expect("legitimate app_theme must still be served unauthenticated");
    assert_eq!(
        body["value"], ".app{color:red}",
        "the real theme value must be returned"
    );

    // The planted non-theme resource must NOT be disclosed: the response is
    // `null` because the query is now constrained to resource_type='app_theme'.
    let resp = client()
        .get(format!("{ws}/apps_u/public_resource/f/app_themes/evil"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: Option<serde_json::Value> = resp.json().await?;
    assert!(
        body.is_none(),
        "a non-app_theme resource under f/app_themes/ must not be disclosed to an unauthenticated caller, got: {body:?}"
    );

    Ok(())
}
