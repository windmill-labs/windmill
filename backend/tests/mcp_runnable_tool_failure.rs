//! A script tool on a selection-scoped MCP token reports a run that failed as a tool execution
//! error (`isError`) naming the run, and a run that succeeded as its result.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn seed(db: &Pool<Postgres>) -> anyhow::Result<()> {
    for (hash, path, content) in [
        (
            7000001_i64,
            "u/test-user/fails",
            "echo about to fail\nexit 3",
        ),
        (7000002_i64, "u/test-user/works", "echo all good"),
    ] {
        sqlx::query(
            "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
                                 schema, summary, description, lock)
             VALUES ('test-workspace', $1, $2, $3, 'bash', 'script', 'test-user',
                     '{\"type\": \"object\", \"properties\": {}}', '', '', '')",
        )
        .bind(hash)
        .bind(path)
        .bind(content)
        .execute(db)
        .await?;
    }
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, workspace_id, scopes)
         VALUES (encode(sha256('MCP_SELECTION'::bytea), 'hex'), 'MCP_SELECT', 'MCP_SELECTION',
                 'test@windmill.dev', 'mcp', 'test-workspace',
                 ARRAY['mcp:scripts:u/test-user/fails,u/test-user/works'])",
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn rpc(base: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    let body = reqwest::Client::new()
        .post(format!("{base}/api/mcp/w/test-workspace/mcp"))
        .bearer_auth("MCP_SELECTION")
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
        .send()
        .await?
        .text()
        .await?;
    let data = body
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .find(|l| !l.trim().is_empty())
        .unwrap_or(&body);
    Ok(serde_json::from_str(data.trim())?)
}

async fn call(base: &str, tools: &[Value], path_suffix: &str) -> anyhow::Result<Value> {
    let name = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .find(|n| n.ends_with(path_suffix))
        .unwrap_or_else(|| panic!("no tool for {path_suffix} in {tools:?}"));
    let res = rpc(base, "tools/call", json!({ "name": name, "arguments": {} })).await?;
    Ok(res["result"].clone())
}

#[sqlx::test(fixtures("base"))]
async fn test_failed_run_is_a_tool_error(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}");

    in_test_worker(
        &db,
        async {
            let tools = rpc(&base, "tools/list", json!({})).await?;
            let tools = tools["result"]["tools"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            let failed = call(&base, &tools, "fails").await?;
            assert_eq!(failed["isError"], json!(true), "{failed}");
            let text = failed["content"][0]["text"].as_str().unwrap_or_default();
            assert!(text.contains("exit code"), "{text}");
            let job_id: String = sqlx::query_scalar(
                "SELECT id::text FROM v2_job WHERE runnable_path = 'u/test-user/fails'",
            )
            .fetch_one(&db)
            .await?;
            assert!(text.contains(&job_id), "{text}");

            let ok = call(&base, &tools, "works").await?;
            assert_ne!(ok["isError"], json!(true), "{ok}");
            let text = ok["content"][0]["text"].as_str().unwrap_or_default();
            assert_eq!(text, "\"all good\"");
            anyhow::Ok(())
        },
        port,
    )
    .await?;
    Ok(())
}
