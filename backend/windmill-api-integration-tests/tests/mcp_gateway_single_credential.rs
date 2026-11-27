//! The gateway checks scopes on and acts as one and the same token.
//!
//! A multi-workspace session checks a call against the MCP scopes of `ApiAuthed`, then runs
//! it as the owner of the token the gateway middleware picked. `ApiAuthed` prefers the
//! `token` cookie over `?token=` and the gateway never reads the cookie, so a request
//! carrying both must still be held to the URL token's scopes.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

/// Both workspace-less, which puts the gateway in multi-workspace mode.
async fn insert_mcp_tokens(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES
         (encode(sha256('OWNER_ONE_SCRIPT'::bytea), 'hex'), 'OWNER_ONE_', 'OWNER_ONE_SCRIPT', 'test@windmill.dev', 'agent', true, ARRAY['mcp:scripts:u/test-user/weekly_report']),
         (encode(sha256('OTHER_MCP_ALL'::bytea), 'hex'), 'OTHER_MCP_', 'OTHER_MCP_ALL', 'test2@windmill.dev', 'mine', false, ARRAY['mcp:all'])",
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The endpoint answers `application/json` or a single-event SSE stream.
async fn gateway_post(port: u16, cookie: Option<&str>, body: Value) -> anyhow::Result<Value> {
    let mut req = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/mcp/gateway?token=OWNER_ONE_SCRIPT"
        ))
        .header("Accept", "application/json, text/event-stream")
        .json(&body);
    if let Some(cookie) = cookie {
        req = req.header("Cookie", format!("token={cookie}"));
    }
    let text = req.send().await?.text().await?;
    let payload = text
        .lines()
        .find_map(|l| l.strip_prefix("data: "))
        .unwrap_or(text.trim());
    serde_json::from_str(payload).map_err(|e| anyhow::anyhow!("unparseable MCP body {text:?}: {e}"))
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_gateway_cookie_does_not_widen_url_token(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    insert_mcp_tokens(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
        "name": "getVariable",
        "arguments": { "workspace_id": "test-workspace", "path": "u/test-user/db_password" },
    }});
    let list = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} });

    for cookie in [None, Some("OTHER_MCP_ALL")] {
        let resp = gateway_post(port, cookie, call.clone()).await?;
        assert!(
            resp["error"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("endpoint 'getVariable' not in token scope")),
            "cookie {cookie:?}: a one-script token reached getVariable: {resp}"
        );

        let resp = gateway_post(port, cookie, list.clone()).await?;
        let tools = resp["result"]["tools"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("no tool list in {resp}"))?;
        assert!(
            !tools.iter().any(|t| t["name"] == "getVariable"),
            "cookie {cookie:?}: a one-script token was offered getVariable"
        );
    }

    Ok(())
}
