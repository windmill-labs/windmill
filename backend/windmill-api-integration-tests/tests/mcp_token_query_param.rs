//! The `mcp_disable_token_query_param` switch closes the URL-borne credential path.
//!
//! The rejection is a middleware layered between the `WWW-Authenticate` decorator and
//! everything that reads a token, on both the workspaced and the gateway router. Reordering
//! that stack, or adding a third MCP mount without it, leaves the switch inert while every
//! other MCP test still passes, so the two mounts are pinned here together.
#![cfg(feature = "mcp")]

use std::sync::atomic::Ordering;

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_common::global_settings::MCP_DISABLE_TOKEN_QUERY_PARAM;
use windmill_test_utils::*;

/// Workspace-less with an `mcp:` scope, which is what the gateway mount requires; the
/// workspaced mount takes its workspace from the path, so one token reaches both.
async fn insert_mcp_token(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes)
         VALUES (encode(sha256('MCP_TOKEN'::bytea), 'hex'), 'MCP_TOK', 'MCP_TOKEN', 'test@windmill.dev', 'mcp token', true, ARRAY['mcp:all'])",
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn tools_list(url: &str) -> anyhow::Result<reqwest::Response> {
    Ok(reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} }))
        .send()
        .await?)
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_token_query_param_switch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    insert_mcp_token(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();
    let workspaced =
        format!("http://localhost:{port}/api/mcp/w/test-workspace/mcp?token=MCP_TOKEN");
    let gateway = format!("http://localhost:{port}/api/mcp/gateway?token=MCP_TOKEN");

    assert_eq!(
        tools_list(&workspaced).await?.status(),
        200,
        "a URL-borne token is the documented default and must keep working while the switch is off"
    );
    assert_eq!(tools_list(&gateway).await?.status(), 200);

    MCP_DISABLE_TOKEN_QUERY_PARAM.store(true, Ordering::Relaxed);

    for url in [&workspaced, &gateway] {
        let resp = tools_list(url).await?;
        assert_eq!(
            resp.status(),
            401,
            "{url} still admitted a token in the URL"
        );
        // What sends the client into the OAuth flow rather than leaving it stuck on a 401.
        assert!(
            resp.headers().contains_key("www-authenticate"),
            "{url} rejected without pointing at the authorization server"
        );
    }

    // The header stays open: it is the channel the OAuth flow itself hands tokens over on.
    let resp = reqwest::Client::new()
        .post(format!("http://localhost:{port}/api/mcp/gateway"))
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer MCP_TOKEN")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    Ok(())
}
