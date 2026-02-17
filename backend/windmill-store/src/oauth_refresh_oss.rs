/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "private")]
pub use crate::oauth_refresh_ee::_refresh_token;
#[cfg(feature = "private")]
pub use crate::oauth_refresh_ee::_refresh_workspace_integration_token;

#[cfg(not(feature = "private"))]
use sqlx::{Postgres, Transaction};
#[cfg(not(feature = "private"))]
use windmill_common::db::DB;
#[cfg(not(feature = "private"))]
use windmill_common::error;

#[cfg(not(feature = "private"))]
pub async fn _refresh_token<'c>(
    tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
) -> error::Result<String> {
    windmill_oauth::refresh_token(
        tx,
        path,
        w_id,
        id,
        db,
        &*windmill_oauth::OAUTH_CLIENTS.read().await,
        &windmill_oauth::OAUTH_HTTP_CLIENT,
        include_str!("../../oauth_connect.json"),
    )
    .await
}

#[cfg(not(feature = "private"))]
pub async fn _refresh_workspace_integration_token<'c>(
    mut tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    account_id: i32,
    db: &DB,
    client_name: &str,
    refresh_token: &str,
) -> error::Result<String> {
    use windmill_common::global_settings::{
        get_instance_oauth_credentials, workspace_integration_auth_endpoint,
        workspace_integration_oauth_key, workspace_integration_token_endpoint,
    };
    use windmill_common::utils::now_from_db;
    use windmill_common::variables::{build_crypt, encrypt};
    use windmill_oauth::{OClient, RefreshToken, Url, OAUTH_HTTP_CLIENT};

    tracing::info!(
        client = %client_name,
        workspace_id = %w_id,
        account_id = %account_id,
        "Refreshing workspace integration OAuth token"
    );

    let oauth_data: serde_json::Value = sqlx::query_scalar(
        "SELECT oauth_data FROM workspace_integrations \
         WHERE workspace_id = $1 AND service_name::text = $2",
    )
    .bind(w_id)
    .bind(client_name)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        error::Error::NotFound(format!(
            "Workspace integration for {} not found or not configured",
            client_name
        ))
    })?;

    let is_instance_shared = oauth_data
        .get("instance_shared")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let (client_id, client_secret, base_url);
    if is_instance_shared {
        let oauth_key = workspace_integration_oauth_key(client_name);
        let (id, secret) = get_instance_oauth_credentials(db, oauth_key).await?;
        client_id = id;
        client_secret = secret;
        base_url = String::new();
    } else {
        client_id = oauth_data["client_id"]
            .as_str()
            .ok_or_else(|| {
                error::Error::InternalErr("Missing client_id in workspace integration".into())
            })?
            .to_string();
        client_secret = oauth_data["client_secret"]
            .as_str()
            .ok_or_else(|| {
                error::Error::InternalErr(
                    "Missing client_secret in workspace integration".into(),
                )
            })?
            .to_string();
        base_url = oauth_data["base_url"].as_str().unwrap_or("").to_string();
    }

    let token_endpoint = workspace_integration_token_endpoint(client_name, &base_url);
    let auth_endpoint = workspace_integration_auth_endpoint(client_name, &base_url);

    let auth_url = Url::parse(&auth_endpoint)
        .map_err(|e| error::Error::InternalErr(format!("Invalid auth URL: {}", e)))?;
    let token_url = Url::parse(&token_endpoint)
        .map_err(|e| error::Error::InternalErr(format!("Invalid token URL: {}", e)))?;

    let mut client = OClient::new(client_id, auth_url, token_url);
    client.set_client_secret(client_secret);

    let token = client
        .exchange_refresh_token(&RefreshToken::from(refresh_token))
        .with_client(&*OAUTH_HTTP_CLIENT)
        .execute::<serde_json::Value>()
        .await
        .map_err(|e| {
            error::Error::InternalErr(format!(
                "Failed to refresh workspace integration token: {:?}",
                e
            ))
        })?;

    #[derive(serde::Deserialize)]
    struct WsTokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }

    let token_result: WsTokenResponse = serde_json::from_value(token)
        .map_err(|e| error::Error::InternalErr(format!("Failed to parse token response: {}", e)))?;

    let expires_at = now_from_db(&mut *tx).await?
        + chrono::Duration::try_seconds(
            token_result
                .expires_in
                .ok_or_else(|| {
                    error::Error::InternalErr("expires_in expected and not found".into())
                })?
                .try_into()
                .unwrap(),
        )
        .unwrap_or_default();

    sqlx::query(
        "UPDATE account SET refresh_token = $1, expires_at = $2, refresh_error = NULL \
         WHERE workspace_id = $3 AND id = $4",
    )
    .bind(
        token_result
            .refresh_token
            .as_deref()
            .unwrap_or(refresh_token),
    )
    .bind(expires_at)
    .bind(w_id)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let token_str = &token_result.access_token;
    let mc = build_crypt(db, w_id).await?;
    let encrypted_token = encrypt(&mc, token_str);

    sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
        .bind(encrypted_token)
        .bind(w_id)
        .bind(path)
        .execute(db)
        .await?;

    tracing::info!(
        client = %client_name,
        workspace_id = %w_id,
        account_id = %account_id,
        "Workspace integration OAuth token refreshed successfully"
    );

    Ok(token_result.access_token)
}
