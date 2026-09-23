//! Request headers reaching a runnable's preprocessor over MCP.
//!
//! The property this pins is structural rather than a filter: the model writes
//! the tool's arguments, which become `event.body`, while the server writes
//! `event.headers`. A model that guesses a header's name can only ever land in
//! `body`, so an identity read from `headers` is one prompt injection cannot
//! forge. Nothing else in the suite exercises MCP argument shaping end to end.
//!
//! Requires: bun runtime, live database (migrations applied by sqlx::test).
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const SCRIPT_PATH: &str = "u/test-user/mcp_hdr_probe";

/// A bun lock the executor accepts without installing anything: no dependencies
/// in the `package.json` half, `<empty>` for the `bun.lock` half. The empty
/// string is not a substitute: a lock carrying no `//bun.lock` separator is
/// rejected at run time.
const EMPTY_BUN_LOCK: &str = "{}\n//bun.lock\n<empty>";

/// Echoes the two halves of the event separately, so the assertions can tell
/// which one a value arrived in.
const PREPROCESSOR_SCRIPT: &str = r#"
export async function preprocessor(event: any) {
  return {
    kind: event.kind,
    from_headers: event.headers?.["x-user-id"] ?? "",
    from_body: event.body?.x_user_id ?? "",
    header_names: Object.keys(event.headers ?? {}).sort(),
  };
}

export async function main(kind: string, from_headers: string, from_body: string, header_names: string[]) {
  return { kind, from_headers, from_body, header_names };
}
"#;

async fn insert_mcp_token(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes)
         VALUES (encode(sha256('MCP_TOKEN'::bytea), 'hex'), 'MCP_TOK', 'MCP_TOKEN', 'test@windmill.dev', 'mcp token', true, ARRAY['mcp:all'])",
    )
    .execute(db)
    .await?;
    Ok(())
}

/// POST one JSON-RPC message. The endpoint answers either `application/json` or
/// a single-event SSE stream, so strip the `data: ` framing before parsing.
async fn mcp_post(port: u16, headers: &[(&str, &str)], body: Value) -> anyhow::Result<Value> {
    let mut req = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/mcp/w/test-workspace/mcp"
        ))
        .header("Authorization", "Bearer MCP_TOKEN")
        .header("Accept", "application/json, text/event-stream")
        .json(&body);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let text = req.send().await?.text().await?;
    let payload = text
        .lines()
        .find_map(|l| l.strip_prefix("data: "))
        .unwrap_or(text.trim());
    serde_json::from_str(payload).map_err(|e| anyhow::anyhow!("unparseable MCP body {text:?}: {e}"))
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_preprocessor_receives_the_callers_headers(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    insert_mcp_token(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "path": SCRIPT_PATH,
            "summary": "mcp header probe",
            "description": "",
            "content": PREPROCESSOR_SCRIPT,
            "language": "bun",
            "lock": EMPTY_BUN_LOCK,
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { "x_user_id": { "type": "string" } },
                "required": []
            }
        }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "create script: {}",
        resp.text().await.unwrap_or_default()
    );

    // A supplied lock queues no dependency job, so the version is deployed (hence
    // listable and runnable) as soon as the create returns.
    let queued: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM v2_job_queue WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(queued, 0, "the supplied lock must queue no dependency job");

    let tools = mcp_post(
        port,
        &[],
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
    )
    .await?;
    let tool_name = tools["result"]["tools"]
        .as_array()
        .and_then(|list| {
            list.iter()
                .filter_map(|t| t["name"].as_str())
                .find(|n| n.contains("mcp__hdr__probe"))
        })
        .ok_or_else(|| anyhow::anyhow!("the deployed script was not listed as a tool: {tools}"))?
        .to_string();

    let result = in_test_worker(
        db.clone(),
        async {
            mcp_post(
                port,
                // Every name the withheld list covers has to be on the wire, or
                // asserting its absence proves nothing. `Authorization` is already
                // set by `mcp_post`, and `extract_token` reads it before the
                // cookie, so sending one does not disturb auth.
                &[
                    ("X-User-Id", "alice@corp.example"),
                    ("Cookie", "session=secret"),
                    ("Proxy-Authorization", "Basic Zm9v"),
                ],
                json!({
                    "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                    // The model names the header it wants to spoof. Its value is an
                    // argument, so it can only ever reach `event.body`.
                    "params": { "name": tool_name, "arguments": { "x_user_id": "attacker@evil.test" } }
                }),
            )
            .await
        },
        port,
    )
    .await?;

    let text = result["result"]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("tool call returned no text content: {result}"))?;
    let out: Value = serde_json::from_str(text)?;

    assert_eq!(out["kind"], "mcp", "preprocessor event kind: {out}");
    assert_eq!(
        out["from_headers"], "alice@corp.example",
        "the caller's header must reach event.headers: {out}"
    );
    assert_eq!(
        out["from_body"], "attacker@evil.test",
        "the model's argument must land in event.body, not overwrite the header: {out}"
    );

    let names: Vec<&str> = out["header_names"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    assert!(
        names.contains(&"x-user-id"),
        "event.headers must carry the request's own headers: {names:?}"
    );
    for withheld in ["authorization", "cookie", "proxy-authorization"] {
        assert!(
            !names.contains(&withheld),
            "{withheld} is withheld from a preprocessor: {names:?}"
        );
    }

    Ok(())
}
