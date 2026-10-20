/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::future::Future;
use std::sync::Arc;

use windmill_common::db::DB;
use windmill_common::error::{self, Error};
use windmill_oauth::{OAuthAccountInfo, RefreshError, RefreshedToken};

#[cfg(feature = "private")]
pub use crate::oauth_refresh_ee::_refresh_token;

/// The `account` row a refresh starts from.
pub struct RefreshAccount {
    pub mcp_server_url: Option<String>,
    pub is_workspace_integration: bool,
    pub info: OAuthAccountInfo,
}

lazy_static::lazy_static! {
    static ref REFRESH_LOCKS: dashmap::DashMap<(String, i32), Arc<tokio::sync::Mutex<()>>> =
        dashmap::DashMap::new();
}

/// Refresh the token of account `id`, which backs the secret variable at `path`.
///
/// No connection is held across `exchange`: it does its own config reads and the provider
/// call, and every write here is a single autocommit statement. Concurrent refreshers of one
/// account are serialized within a process, and across processes the account is only
/// committed if its refresh token is still the one exchanged, so a rotated refresh token is
/// never overwritten by a stale one. An account refreshed by someone else while we waited for
/// the lock is not refreshed again: without `force` any fresh account counts, with it only one
/// whose expiry moved since we asked. Callers must have authorized `w_id`/`path`/`id`.
pub(crate) async fn refresh_account_token<F, Fut>(
    db: &DB,
    w_id: &str,
    path: &str,
    id: i32,
    force: bool,
    exchange: F,
) -> error::Result<String>
where
    F: FnOnce(RefreshAccount) -> Fut,
    Fut: Future<Output = Result<RefreshedToken, RefreshError>>,
{
    let seen_expiry = if force {
        sqlx::query_scalar!(
            "SELECT expires_at FROM account WHERE workspace_id = $1 AND id = $2",
            w_id,
            id
        )
        .fetch_optional(db)
        .await?
    } else {
        None
    };

    let lock = REFRESH_LOCKS
        .entry((w_id.to_string(), id))
        .or_default()
        .clone();
    let _guard = lock.lock().await;

    let row = sqlx::query!(
        "SELECT mcp_server_url, is_workspace_integration, client, refresh_token, grant_type,
            cc_client_id, cc_client_secret, cc_token_url, scopes, expires_at,
            now() > expires_at AS \"expired!\"
         FROM account WHERE workspace_id = $1 AND id = $2",
        w_id,
        id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Account {id} not found")))?;

    let refreshed_meanwhile = if force {
        Some(row.expires_at) != seen_expiry
    } else {
        !row.expired
    };
    if refreshed_meanwhile {
        return current_token(db, w_id, path).await;
    }

    let used = row.refresh_token.clone();
    let account = RefreshAccount {
        mcp_server_url: row.mcp_server_url,
        is_workspace_integration: row.is_workspace_integration,
        info: OAuthAccountInfo {
            client: row.client,
            refresh_token: row.refresh_token,
            grant_type: row.grant_type,
            cc_client_id: row.cc_client_id,
            cc_client_secret: row.cc_client_secret,
            cc_token_url: row.cc_token_url,
            scopes: row.scopes,
        },
    };

    let token = match exchange(account).await {
        Ok(token) => token,
        Err(RefreshError::Other(e)) => return Err(e),
        Err(RefreshError::Provider(msg)) => {
            let recorded = sqlx::query!(
                "UPDATE account SET refresh_error = $1
                 WHERE workspace_id = $2 AND id = $3 AND refresh_token = $4",
                msg,
                w_id,
                id,
                used,
            )
            .execute(db)
            .await?
            .rows_affected();
            // Another process rotated the refresh token first, so the one we sent was spent.
            if recorded == 0 {
                return current_token(db, w_id, path).await;
            }
            return Err(Error::BadRequest(format!("Error refreshing token: {msg}")));
        }
    };

    // Store before marking the account fresh, so no reader sees it fresh with the old token.
    // A failed store already marks the account for a retry; the rotated refresh token must
    // still be saved, or the next refresh sends one the provider has revoked.
    let stored =
        crate::secret_backend_ext::store_oauth_token_value(db, w_id, path, &token.access_token)
            .await;
    sqlx::query!(
        "UPDATE account SET refresh_token = COALESCE($1, refresh_token),
            expires_at = CASE WHEN $2 THEN now() + make_interval(secs => $3) ELSE expires_at END,
            refresh_error = CASE WHEN $2 THEN NULL ELSE refresh_error END
         WHERE workspace_id = $4 AND id = $5 AND refresh_token = $6",
        token.refresh_token,
        stored.is_ok(),
        token.expires_in as f64,
        w_id,
        id,
        used,
    )
    .execute(db)
    .await?;
    stored?;

    Ok(token.access_token)
}

/// Whether account `id` exists in `w_id`, read through the caller's user transaction. `account`
/// carries no RLS, so this gates nothing beyond the workspace the caller is already authorized
/// for: a refresh asked for by account id is scoped by that membership, not by ownership.
pub async fn account_visible(
    user_conn: &mut sqlx::PgConnection,
    w_id: &str,
    id: i32,
) -> error::Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM account WHERE workspace_id = $1 AND id = $2) AS \"exists!\"",
        w_id,
        id
    )
    .fetch_one(user_conn)
    .await?)
}

async fn current_token(db: &DB, w_id: &str, path: &str) -> error::Result<String> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM variable WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Variable {path} not found")))?;
    crate::secret_backend_ext::get_secret_value(db, w_id, path, &value).await
}

/// The standard OAuth / client-credentials exchange.
pub(crate) async fn exchange_standard(
    db: &DB,
    w_id: &str,
    id: i32,
    account: &OAuthAccountInfo,
) -> Result<RefreshedToken, RefreshError> {
    windmill_oauth::exchange_account_token(
        db,
        w_id,
        id,
        account,
        &*windmill_oauth::OAUTH_CLIENTS.load(),
        &windmill_oauth::OAUTH_HTTP_CLIENT,
        include_str!("../../oauth_connect.json"),
    )
    .await
}

#[cfg(not(feature = "private"))]
pub async fn _refresh_token(
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
    force: bool,
) -> error::Result<String> {
    refresh_account_token(db, w_id, path, id, force, |account| async move {
        exchange_standard(db, w_id, id, &account.info).await
    })
    .await
}
