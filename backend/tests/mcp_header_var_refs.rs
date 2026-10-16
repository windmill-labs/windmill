//! An MCP resource header whose value is a `$var:` reference reaches the MCP
//! server as the variable's value. A scoped token needs only the resource's
//! scope for that, not one on the variable.
#![cfg(feature = "mcp")]

use sqlx::{Pool, Postgres};
use tokio::io::AsyncReadExt;
use windmill_common::client::AuthedClient;
use windmill_test_utils::*;
use windmill_worker::{load_mcp_tools, McpResourceConfig};

const SECRET_VALUE: &str = "S3CRET-MCP-HEADER-VALUE";

/// A secret variable and an MCP resource at `url` sending it as a header.
async fn seed(db: &Pool<Postgres>, url: &str) -> anyhow::Result<()> {
    let mc = windmill_common::variables::build_crypt(db, "test-workspace").await?;
    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
         VALUES ('test-workspace', 'u/test-user/mcp_header', $1, true, '', '{}')",
    )
    .bind(windmill_common::variables::encrypt(&mc, SECRET_VALUE))
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', 'u/test-user/mcp', $1, 'mcp', '{}', 'test-user')",
    )
    .bind(serde_json::json!({
        "name": "mcp",
        "url": url,
        "token": "$var:u/test-user/mcp_header",
        "headers": { "X-Api-Key": "$var:u/test-user/mcp_header", "X-Plain": "plain" }
    }))
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_mcp_header_var_ref_is_resolved(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    std::env::set_var("ALLOW_PRIVATE_MCP_SERVER_URLS", "true");

    // Not an MCP server: it captures the first request and hangs up, so loading
    // fails at the connect step after the headers were sent.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let mcp_port = listener.local_addr()?.port();
    let first_request = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        let mut buf = vec![0u8; 16 * 1024];
        let n = socket.read(&mut buf).await?;
        anyhow::Ok(String::from_utf8_lossy(&buf[..n]).to_lowercase())
    });
    seed(&db, &format!("http://127.0.0.1:{mcp_port}/mcp")).await?;

    let server = ApiServer::start(db.clone()).await?;
    let client = AuthedClient::new(
        format!("http://localhost:{}", server.addr.port()),
        "test-workspace".to_string(),
        "SECRET_TOKEN".to_string(),
        None,
    );
    let config = vec![McpResourceConfig {
        resource_path: "$res:u/test-user/mcp".to_string(),
        include_tools: None,
        exclude_tools: None,
    }];
    let _ = load_mcp_tools(&db, "test-workspace", config, &client).await;

    let request = first_request.await??;
    assert!(
        request.contains(&format!("x-api-key: {}", SECRET_VALUE.to_lowercase())),
        "the header must carry the variable's value, not the reference: {request}"
    );
    assert!(request.contains("x-plain: plain"), "{request}");

    Ok(())
}

/// A token scoped to the resource alone resolves the variables its token and
/// headers reference: the resource's scope covers them.
#[sqlx::test(fixtures("base"))]
async fn test_mcp_var_refs_need_only_the_resource_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed(&db, "https://mcp.invalid.windmill.test").await?;
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes)
         VALUES (encode(sha256('RES_ONLY_TOKEN'::bytea), 'hex'), 'RES_ONLY_T', 'RES_ONLY_TOKEN',
                 'test@windmill.dev', 'resources only', false, '{resources:read}')",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{}/api/w/test-workspace/resources/mcp_tools/u/test-user/mcp",
            server.addr.port()
        ))
        .header("Authorization", "Bearer RES_ONLY_TOKEN")
        .send()
        .await?;
    let body = resp.text().await?;
    assert!(
        body.contains("Failed to connect to MCP server"),
        "the variables must resolve and only the connection fail: {body}"
    );

    Ok(())
}
