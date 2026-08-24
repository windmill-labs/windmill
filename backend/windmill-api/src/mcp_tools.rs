use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::Deserialize;
use serde_json::value::RawValue;
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_common::{
    db::{DbWithOptAuthed, UserDB, DB},
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};
use windmill_store::{resources::explain_resource_perm_error, variables::get_value_internal};

/// A connected MCP server is a third party the user chose, reached over a
/// connection this request holds open: without a deadline one that never answers
/// pins an API worker and the chat turn behind it for as long as it likes.
const MCP_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);
/// Best-effort courtesy to the server, so it cannot extend the deadline above.
const MCP_SHUTDOWN_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

async fn with_deadline<T>(
    what: &str,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::time::timeout(MCP_DEADLINE, fut)
        .await
        .map_err(|_| {
            Error::ExecutionErr(format!(
                "MCP server did not answer within {}s ({what})",
                MCP_DEADLINE.as_secs()
            ))
        })?
}

/// Connect to the MCP server described by the `mcp` resource at `path`.
///
/// The caller is responsible for the scope check; everything else (resource
/// visibility, token resolution) goes through the caller's permissioned path so
/// the endpoint can never act as a confused deputy for a resource or secret the
/// caller cannot read.
async fn connect_mcp_client(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<windmill_mcp::McpClient> {
    let mut tx = user_db.clone().begin(authed).await?;

    let resource_value_o = sqlx::query_scalar!(
        "SELECT value as \"value: sqlx::types::Json<Box<RawValue>>\" FROM resource WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    if resource_value_o.is_none() {
        explain_resource_perm_error(path, w_id, db, authed).await?;
    }

    let resource_value = not_found_if_none(resource_value_o, "Resource", path)?
        .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", path)))?;

    let mcp_resource = serde_json::from_str::<windmill_mcp::McpResource>(resource_value.0.get())
        .map_err(|e| Error::BadRequest(format!("Failed to parse MCP resource: {}", e)))?;

    #[cfg(feature = "oauth2")]
    {
        tracing::info!("Checking if token needs refresh before creating MCP client");
        if let Some(ref token_path) = mcp_resource.token {
            let token_var_path = token_path.trim_start_matches("$var:");

            let token_info = sqlx::query!(
                r#"
            SELECT
                variable.account as account_id,
                (now() > account.expires_at) as "is_expired: bool"
            FROM variable
            LEFT JOIN account ON variable.account = account.id AND account.workspace_id = $2
            WHERE variable.path = $1 AND variable.workspace_id = $2
            "#,
                token_var_path,
                w_id
            )
            .fetch_optional(db)
            .await?;

            if let Some(info) = token_info {
                if let (Some(account_id), Some(true)) = (info.account_id, info.is_expired) {
                    let refresh_tx = user_db.clone().begin(authed).await?;
                    if let Err(e) = crate::oauth2_oss::_refresh_token(
                        refresh_tx,
                        token_var_path,
                        w_id,
                        account_id,
                        db,
                    )
                    .await
                    {
                        tracing::warn!(
                        "Failed to refresh token for MCP resource: {}. Proceeding with possibly expired token.",
                        e
                    );
                    }
                }
            }
        }
    }

    // Resolve the token through the caller's permissioned (RLS + audit) path so
    // a developer cannot exfiltrate a secret they are not allowed to read by
    // pointing an MCP resource's token at it.
    let token = if let Some(token_path) = &mcp_resource.token {
        let token_var_path = token_path.trim_start_matches("$var:");
        if token_var_path.trim().is_empty() {
            None
        } else {
            let db_authed = DbWithOptAuthed::from_authed(authed, db.clone(), Some(user_db.clone()));
            Some(get_value_internal(&db_authed, w_id, token_var_path, false).await?)
        }
    } else {
        None
    };

    windmill_mcp::McpClient::from_resource(mcp_resource, token)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to connect to MCP server: {}", e)))
}

async fn shutdown_mcp_client(client: windmill_mcp::McpClient) {
    match tokio::time::timeout(MCP_SHUTDOWN_DEADLINE, client.shutdown()).await {
        Ok(Err(e)) => tracing::warn!("Failed to shutdown MCP client: {}", e),
        Err(_) => tracing::warn!("MCP client shutdown timed out"),
        Ok(Ok(())) => {}
    }
}

pub(crate) async fn get_mcp_tools(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<serde_json::Value>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;

    let client = with_deadline(
        "listing tools",
        connect_mcp_client(&authed, &db, &user_db, &w_id, path),
    )
    .await?;

    let tools: Vec<serde_json::Value> = client
        .available_tools()
        .iter()
        .map(|tool| {
            serde_json::to_value(tool)
                .map_err(|e| Error::ExecutionErr(format!("Failed to serialize MCP tool: {}", e)))
        })
        .collect::<Result<Vec<_>>>()?;

    shutdown_mcp_client(client).await;

    Ok(Json(tools))
}

#[derive(Deserialize)]
pub(crate) struct CallMcpToolRequest {
    tool: String,
    arguments: Option<Box<RawValue>>,
    /// Set by a caller that skipped the user's confirmation because it had
    /// listed the tool as read-only. Verified below against the live listing.
    read_only: Option<bool>,
}

/// `readOnlyHint` is the server's own claim, so this cannot tell a hostile
/// server from an honest one; what it guarantees is that the claim comes from
/// the server about to be called, not from a listing of whatever the resource
/// pointed at when the caller cached it.
fn tool_is_read_only(client: &windmill_mcp::McpClient, tool: &str) -> bool {
    client
        .available_tools()
        .iter()
        .find(|t| t.name.as_ref() == tool)
        .and_then(|t| t.annotations.as_ref())
        .and_then(|a| a.read_only_hint)
        .unwrap_or(false)
}

pub(crate) async fn call_mcp_tool(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(req): Json<CallMcpToolRequest>,
) -> JsonResult<serde_json::Value> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:write:{}", path))?;

    let arguments = req.arguments.as_ref().map(|a| a.get()).unwrap_or("{}");
    // One deadline over the whole exchange (connect, then call), so a server that
    // stalls after answering the handshake is bounded too.
    let (client, result) = with_deadline(&format!("calling {}", req.tool), async {
        let client = connect_mcp_client(&authed, &db, &user_db, &w_id, path).await?;
        if req.read_only == Some(true) && !tool_is_read_only(&client, &req.tool) {
            return Ok((client, None));
        }
        let result = client.call_tool(&req.tool, arguments).await;
        Ok((client, Some(result)))
    })
    .await?;

    shutdown_mcp_client(client).await;

    let Some(result) = result else {
        return Err(Error::BadRequest(format!(
            "MCP tool {} is not marked read-only by the server, it must be called as a tool that modifies data",
            req.tool
        )));
    };

    // A tool that ran but reported failure comes back as `Ok` with `isError:
    // true` in the payload; forwarding it verbatim lets the caller show the
    // server's own error text instead of a generic 500.
    let result = result
        .map_err(|e| Error::ExecutionErr(format!("Failed to call MCP tool {}: {}", req.tool, e)))?;

    Ok(Json(result))
}
