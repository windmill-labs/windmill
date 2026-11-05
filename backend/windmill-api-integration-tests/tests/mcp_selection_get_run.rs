//! `get_run` as a token scoped to a selection of runnables is offered it: listed only beside a
//! runnable the user can run, it reads a run of a selected flow, and refuses one of a flow
//! outside the selection, although the token's owner can read both. The session tool it reuses reads through a JWT without route scopes, so this
//! check is the only thing holding a selection to its own runs.
//!
//! Requires: live database (migrations applied by sqlx::test). No worker: a queued run is
//! readable.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const SELECTED: &str = "u/test-user/selected_flow";
const OTHER: &str = "u/test-user/other_flow";

async fn mcp(port: u16, token: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    let text = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/mcp/w/test-workspace/mcp"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
        .send()
        .await?
        .text()
        .await?;
    let payload = text
        .lines()
        .find_map(|l| l.strip_prefix("data: "))
        .unwrap_or(text.trim());
    serde_json::from_str(payload).map_err(|e| anyhow::anyhow!("unparseable MCP body {text:?}: {e}"))
}

async fn mcp_call(port: u16, name: &str, arguments: Value) -> anyhow::Result<Value> {
    mcp(
        port,
        "SELECTION_TOKEN",
        "tools/call",
        json!({ "name": name, "arguments": arguments }),
    )
    .await
}

async fn lists_get_run(port: u16, token: &str) -> anyhow::Result<bool> {
    let list = mcp(port, token, "tools/list", json!({})).await?;
    Ok(list["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools.iter().any(|t| t["name"] == "get_run")))
}

async fn create_and_queue(port: u16, path: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let api = format!("http://localhost:{port}/api/w/test-workspace");
    let resp = client
        .post(format!("{api}/flows/create"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "path": path,
            "summary": "",
            "value": { "modules": [{ "id": "a", "value": { "type": "identity" } }] },
            "schema": { "type": "object", "properties": {}, "required": [] }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create flow: {}", resp.text().await?);
    let resp = client
        .post(format!("{api}/jobs/run/f/{path}"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "run flow: {}", resp.text().await?);
    Ok(resp.text().await?)
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_get_run_reads_only_runs_of_the_selection(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes, workspace_id)
         VALUES (encode(sha256('SELECTION_TOKEN'::bytea), 'hex'), 'SELECTION', 'SELECTION_TOKEN',
                 'test@windmill.dev', 'selection', false, ARRAY['mcp:flows:u/test-user/selected_flow'], 'test-workspace'),
                (encode(sha256('NOTHING_TOKEN'::bytea), 'hex'), 'NOTHING', 'NOTHING_TOKEN',
                 'test@windmill.dev', 'nothing', false, ARRAY['mcp:flows:u/test-user/no_such_flow'], 'test-workspace')",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    let selected = create_and_queue(port, SELECTED).await?;
    let other = create_and_queue(port, OTHER).await?;

    assert!(lists_get_run(port, "SELECTION_TOKEN").await?);
    assert!(
        !lists_get_run(port, "NOTHING_TOKEN").await?,
        "no get_run beside no runnable the user can run"
    );

    let read = mcp_call(port, "get_run", json!({ "id": selected })).await?;
    let text = read["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains(SELECTED),
        "a run of the selected flow is read: {read}"
    );

    let refused = mcp_call(port, "get_run", json!({ "id": other })).await?;
    assert!(
        refused["error"].is_object() && !refused.to_string().contains(OTHER),
        "a run outside the selection is refused without reading it: {refused}"
    );

    Ok(())
}
