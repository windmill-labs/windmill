//! `/api/r/*` resolves its token outside the `ApiAuthed` extractor, so it must refuse a
//! read-only token itself: calling a route runs its runnable as the route's owner.

#![cfg(feature = "http_trigger")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base", "hello"))]
async fn read_only_token_cannot_call_http_route(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, workspace_id, read_only)
         VALUES (encode(sha256('RO_TOKEN'::bytea), 'hex'), 'RO_TOKEN', 'RO_TOKEN', 'test@windmill.dev',
                 'ro', false, 'test-workspace', true)",
    )
    .execute(&db)
    .await?;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/http_triggers/create"
        ))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({
            "path": "u/test-user/route",
            "script_path": "f/system/hello",
            "is_flow": false,
            "route_path": "ro",
            "request_type": "async",
            "authentication_method": "windmill",
            "http_method": "post",
            "is_static_website": false,
            "workspaced_route": false,
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let call = |token: &'static str| {
        client
            .post(format!("http://localhost:{port}/api/r/ro"))
            .bearer_auth(token)
            .json(&json!({}))
            .send()
    };
    let resp = call("RO_TOKEN").await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);
    let resp = call("SECRET_TOKEN").await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    Ok(())
}
