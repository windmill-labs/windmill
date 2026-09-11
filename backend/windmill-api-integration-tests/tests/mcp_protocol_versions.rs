//! Protocol-version negotiation for the MCP endpoint.
//!
//! The endpoint is dual-era: legacy revisions keep the `initialize` handshake,
//! while `2026-07-28` carries its version as per-request metadata and is served
//! statelessly. Both are answered on the same URL, so a bump of the rmcp SDK
//! must not silently drop either side.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Every revision the server advertises, oldest first.
const SUPPORTED: [&str; 5] = [
    "2024-11-05",
    "2025-03-26",
    "2025-06-18",
    "2025-11-25",
    "2026-07-28",
];

const MODERN: &str = "2026-07-28";

async fn insert_mcp_token(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes)
         VALUES (encode(sha256('MCP_TOKEN'::bytea), 'hex'), 'MCP_TOK', 'MCP_TOKEN', 'test@windmill.dev', 'mcp token', true, ARRAY['mcp:all'])",
    )
    .execute(db)
    .await?;
    Ok(())
}

/// POST one JSON-RPC message and return the HTTP status plus the decoded body.
/// The endpoint answers either `application/json` or a single-event SSE stream,
/// so strip the `data: ` framing before parsing.
async fn post(
    port: u16,
    headers: &[(&str, &str)],
    body: Value,
) -> anyhow::Result<(reqwest::StatusCode, Value)> {
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
    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    let payload = text
        .lines()
        .find_map(|l| l.strip_prefix("data: "))
        .unwrap_or(text.trim());
    let parsed = serde_json::from_str(payload)
        .map_err(|e| anyhow::anyhow!("status {status}, unparseable body {text:?}: {e}"))?;
    Ok((status, parsed))
}

fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "0.0.1" },
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_legacy_initialize_negotiates_requested_version(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    insert_mcp_token(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    // A legacy client must be answered with the revision it asked for, not with
    // whatever the SDK happens to call `LATEST`.
    for version in SUPPORTED.iter().filter(|v| **v != MODERN) {
        let (status, body) = post(
            port,
            &[("MCP-Protocol-Version", version)],
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {
                    "protocolVersion": version,
                    "capabilities": {},
                    "clientInfo": { "name": "test-client", "version": "0.0.1" },
                }
            }),
        )
        .await?;

        assert_eq!(status, 200, "initialize {version} failed: {body}");
        assert_eq!(
            body["result"]["protocolVersion"], *version,
            "initialize {version} negotiated the wrong revision: {body}"
        );
        // `Implementation::from_build_env()` expands its `env!` inside rmcp, so
        // the obvious constructor makes the server introduce itself as the SDK.
        assert_eq!(
            body["result"]["serverInfo"]["name"], "windmill",
            "server must identify itself, not the SDK: {body}"
        );
    }

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_modern_requests_are_served_without_initialize(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    insert_mcp_token(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    // `server/discover` is the modern replacement for the handshake: it must
    // exist and advertise exactly the revisions the server implements.
    let (status, body) = post(
        port,
        &[
            ("MCP-Protocol-Version", MODERN),
            ("Mcp-Method", "server/discover"),
        ],
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { "_meta": modern_meta() }
        }),
    )
    .await?;
    assert_eq!(status, 200, "server/discover failed: {body}");
    assert_eq!(
        body["result"]["supportedVersions"],
        json!(SUPPORTED),
        "server/discover advertised the wrong revisions: {body}"
    );

    // A modern call carries its version in `_meta` and needs no prior session.
    let (status, body) = post(
        port,
        &[
            ("MCP-Protocol-Version", MODERN),
            ("Mcp-Method", "tools/list"),
        ],
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/list",
            "params": { "_meta": modern_meta() }
        }),
    )
    .await?;
    assert_eq!(status, 200, "modern tools/list failed: {body}");
    assert!(
        body["result"]["tools"]
            .as_array()
            .is_some_and(|t| !t.is_empty()),
        "modern tools/list returned no tools: {body}"
    );

    // SEP-2549 cache hints are required at 2026-07-28 and rmcp omits them unless
    // set, which makes strict clients (e.g. the Python SDK) reject the whole
    // response rather than degrade.
    assert!(
        body["result"]["ttlMs"].is_number(),
        "modern tools/list is missing ttlMs: {body}"
    );
    assert_eq!(
        body["result"]["cacheScope"], "private",
        "tools/list must not be cached across callers: {body}"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_unsupported_version_lists_supported_ones(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    insert_mcp_token(&db).await?;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    // The client's only way forward is the `supported` list, so an unknown
    // version must fail with it rather than with a generic error.
    let (status, body) = post(
        port,
        &[
            ("MCP-Protocol-Version", "1900-01-01"),
            ("Mcp-Method", "tools/list"),
        ],
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list",
            "params": { "_meta": {
                "io.modelcontextprotocol/protocolVersion": "1900-01-01",
                "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "0.0.1" },
                "io.modelcontextprotocol/clientCapabilities": {},
            }}
        }),
    )
    .await?;

    assert_eq!(status, 400, "expected 400 for unknown version: {body}");
    assert_eq!(body["error"]["code"], -32022, "wrong error code: {body}");
    assert_eq!(
        body["error"]["data"]["supported"],
        json!(SUPPORTED),
        "error did not advertise the supported revisions: {body}"
    );

    Ok(())
}
