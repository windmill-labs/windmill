use axum::{
    extract::{Extension, Path},
    Json,
};
use serde_json::value::RawValue;
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_common::{
    db::{UserDB, DB},
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};
use windmill_store::resources::explain_resource_perm_error;

pub(crate) async fn get_mcp_tools(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<serde_json::Value>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;

    let mut tx = user_db.clone().begin(&authed).await?;

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

    let client = windmill_mcp::McpClient::from_resource(mcp_resource, &db, &w_id)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to connect to MCP server: {}", e)))?;

    let tools: Vec<serde_json::Value> = client
        .available_tools()
        .iter()
        .map(|tool| {
            serde_json::to_value(tool)
                .map_err(|e| Error::ExecutionErr(format!("Failed to serialize MCP tool: {}", e)))
        })
        .collect::<Result<Vec<_>>>()?;

    if let Err(e) = client.shutdown().await {
        tracing::warn!("Failed to shutdown MCP client: {}", e);
    }

    Ok(Json(tools))
}
