/*
 * Re-export from windmill-store, with extra handlers that depend on windmill-api internals.
 */

pub use windmill_store::resources::*;

use axum::{
    extract::{Extension, Path, Query},
    routing::get,
    Json, Router,
};
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_common::{
    db::{DbWithOptAuthed, UserDB, DB},
    error::JsonResult,
    utils::StripPath,
};

use crate::auth::Tokened;

/// Extends the windmill-store workspaced_service with routes that require windmill-api types.
pub fn workspaced_service() -> Router {
    let router = windmill_store::resources::workspaced_service().route(
        "/get_value_interpolated/*path",
        get(get_resource_value_interpolated),
    );

    #[cfg(feature = "mcp")]
    let router = router.route("/mcp_tools/*path", get(get_mcp_tools));

    router
}

async fn get_resource_value_interpolated(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(job_info): Query<JobInfo>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;

    let db_with_opt_authed =
        DbWithOptAuthed::from_authed(&authed, db.clone(), Some(user_db.clone()));
    return get_resource_value_interpolated_internal(
        &db_with_opt_authed,
        w_id.as_str(),
        path,
        job_info.job_id,
        Some(token.as_str()),
        job_info.allow_cache.unwrap_or(false),
    )
    .await
    .map(|success| Json(success));
}

#[cfg(feature = "mcp")]
async fn get_mcp_tools(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<serde_json::Value>> {
    use serde_json::value::RawValue;
    use windmill_common::error::{Error, Result};
    use windmill_common::utils::not_found_if_none;

    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;

    let mut tx = user_db.clone().begin(&authed).await?;

    // Fetch the MCP resource from database
    let resource_value_o = sqlx::query_scalar!(
        "SELECT value as \"value: sqlx::types::Json<Box<RawValue>>\" FROM resource WHERE path = $1 AND workspace_id = $2",
        &path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    if resource_value_o.is_none() {
        explain_resource_perm_error(&path, &w_id, &db, &authed).await?;
    }

    let resource_value = not_found_if_none(resource_value_o, "Resource", path)?
        .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", path)))?;

    // Parse MCP resource
    let mcp_resource = serde_json::from_str::<windmill_mcp::McpResource>(resource_value.0.get())
        .map_err(|e| Error::BadRequest(format!("Failed to parse MCP resource: {}", e)))?;

    // Check if token needs refresh before creating MCP client
    #[cfg(feature = "oauth2")]
    {
        tracing::info!("Checking if token needs refresh before creating MCP client");
        if let Some(ref token_path) = mcp_resource.token {
            let token_var_path = token_path.trim_start_matches("$var:");

            // Query to check if token is expired
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
                &w_id
            )
            .fetch_optional(&db)
            .await?;

            if let Some(info) = token_info {
                if let (Some(account_id), Some(true)) = (info.account_id, info.is_expired) {
                    let refresh_tx = user_db.begin(&authed).await?;
                    if let Err(e) = crate::oauth2_oss::_refresh_token(
                        refresh_tx,
                        token_var_path,
                        &w_id,
                        account_id,
                        &db,
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

    // Create MCP client connection
    let client = windmill_mcp::McpClient::from_resource(mcp_resource, &db, &w_id)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to connect to MCP server: {}", e)))?;

    // Get raw MCP tools and convert to JSON
    let tools: Vec<serde_json::Value> = client
        .available_tools()
        .iter()
        .map(|tool| {
            serde_json::to_value(tool)
                .map_err(|e| Error::ExecutionErr(format!("Failed to serialize MCP tool: {}", e)))
        })
        .collect::<Result<Vec<_>>>()?;

    // Gracefully shutdown the client
    if let Err(e) = client.shutdown().await {
        tracing::warn!("Failed to shutdown MCP client: {}", e);
    }

    Ok(Json(tools))
}
