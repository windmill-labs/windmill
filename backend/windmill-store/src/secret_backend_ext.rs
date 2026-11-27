/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret backend extension for the store layer
//!
//! Write-side helpers for integrating the SecretBackend trait with variable
//! operations. Backend resolution and read helpers live in
//! `windmill_common::secret_backend` (so lower-level crates can resolve secrets
//! too) and are re-exported here for existing callers.
//!
//! Note: HashiCorp Vault integration requires Enterprise Edition.
//! The OSS version only supports the database backend.

use windmill_common::{
    db::DB,
    error::{Error, Result},
    variables::{build_crypt, encrypt},
};

pub use windmill_common::secret_backend::{
    check_external_ref, get_secret_backend, get_secret_value, is_aws_sm_stored_value,
    is_azure_kv_stored_value, is_external_ref_value, is_external_stored_value,
    is_http_store_stored_value, is_vault_backend_configured, is_vault_stored_value,
    parse_external_ref, resolves_externally,
};

/// The HTTP store's key is derived from the variable path, so moving a reference would point it
/// at a different key of a store Windmill cannot write to.
pub fn http_store_rename_error(path: &str) -> Error {
    Error::BadRequest(format!(
        "{path} references a secret in the external HTTP secret store by its path and cannot be \
         renamed. Delete it and create a new reference at the new path instead."
    ))
}

/// Store a secret value using the configured backend
///
/// For database backend: encrypts using workspace key and returns encrypted value
/// For vault backend (EE only): stores in Vault and returns a placeholder for DB storage
pub async fn store_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    plain_value: &str,
) -> Result<String> {
    let backend = get_secret_backend(db).await?;

    match backend.backend_name() {
        "database" => {
            // Use existing database encryption
            let mc = build_crypt(db, workspace_id).await?;
            Ok(encrypt(&mc, plain_value))
        }
        "hashicorp_vault" => {
            // Store in Vault and return a marker for DB
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(format!("$vault:{}", path))
        }
        "azure_key_vault" => {
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(format!("$azure_kv:{}", path))
        }
        "aws_secrets_manager" => {
            backend.set_secret(workspace_id, path, plain_value).await?;
            Ok(format!("$aws_sm:{}", path))
        }
        // Read-only store: an empty value creates a reference to the key the path maps to, and
        // the value itself is managed in the store.
        "http_store" if plain_value.is_empty() => {
            backend.check_path(path)?;
            Ok(format!("$http_store:{}", path))
        }
        "http_store" => Err(Error::BadRequest(format!(
            "Secrets are read from an external HTTP secret store, which Windmill cannot write to. \
             Set the value of {path} in the store and save this secret with an empty value to \
             reference it."
        ))),
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Store a secret Windmill itself writes, such as the OAuth access token it refreshes. The
/// read-only HTTP store cannot hold it, so it is encrypted in the database instead, where reads
/// find it: they only go to that store for `$http_store:` references.
pub async fn store_managed_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    plain_value: &str,
) -> Result<String> {
    if get_secret_backend(db).await?.backend_name() == "http_store" {
        let mc = build_crypt(db, workspace_id).await?;
        return Ok(encrypt(&mc, plain_value));
    }
    store_secret_value(db, workspace_id, path, plain_value).await
}

/// Persist a freshly minted OAuth access token to the secret variable backing
/// a resource, routing through the configured secret backend.
///
/// This is the write counterpart of the lazy on-fetch OAuth refresh: it stores
/// the token via [`store_managed_secret_value`] (which writes to the external backend —
/// AWS Secrets Manager / Azure Key Vault / Vault — when one is configured, or
/// encrypts for the database backend and the read-only HTTP store) and updates `variable.value` with the
/// returned value (the encrypted blob for the DB backend, or a `$...:` marker
/// for an external backend). Using a raw `UPDATE variable SET value = <encrypted>`
/// here instead would leave the external store frozen at its connect-time token
/// while reads (which resolve through the backend) keep serving the stale value.
///
/// If persisting the token fails — most likely a transient error talking to an
/// external backend — the account must not end up marked fresh while the served
/// secret is stale, or the on-fetch refresh gate (`now() > expires_at`) would keep
/// serving the stale token for the whole token lifetime. So on failure we reset
/// `expires_at` to the past and record `refresh_error`, and the very next fetch retries the
/// refresh. This also
/// covers another refresher of the account having marked it fresh meanwhile.
///
/// Authorization contract: this performs NO access control. It writes the
/// caller-supplied token into the secret variable at `path` and may mutate the
/// linked `account` row, so callers MUST have already authorized the operation
/// against `workspace_id`/`path` (the OAuth refresh adapters only run after the
/// read path has resolved and gated the variable). It is therefore kept
/// `pub(crate)` and intended solely for the in-crate refresh adapters.
#[cfg(feature = "oauth2")]
///
/// Returns false, storing nothing, when the variable no longer refreshes through `account_id`:
/// a reconnect moved it to another account while this token was being fetched, and writing it
/// would put the previous identity back. The row lock orders this against that reconnect.
pub(crate) async fn store_oauth_token_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    account_id: i32,
    token: &str,
) -> Result<bool> {
    let persist = async {
        let mut tx = db.begin().await?;
        let linked = sqlx::query_scalar::<_, i32>(
            "SELECT 1 FROM variable WHERE workspace_id = $1 AND path = $2 AND account = $3 \
             FOR UPDATE",
        )
        .bind(workspace_id)
        .bind(path)
        .bind(account_id)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
        if !linked {
            return Ok::<bool, Error>(false);
        }
        let value = store_managed_secret_value(db, workspace_id, path, token).await?;
        sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
            .bind(value)
            .bind(workspace_id)
            .bind(path)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
    .await;

    let e = match persist {
        Ok(written) => return Ok(written),
        Err(e) => e,
    };
    // Mark the account expired again so the next fetch re-runs the refresh instead of serving
    // the now-stale token until it naturally expires.
    if let Err(reset_err) = sqlx::query(
        "UPDATE account SET expires_at = now() - interval '1 minute', refresh_error = $1 \
         WHERE workspace_id = $2 AND id = $3",
    )
    .bind(format!(
        "OAuth token was refreshed but persisting it to the secret backend failed: {e}"
    ))
    .bind(workspace_id)
    .bind(account_id)
    .execute(db)
    .await
    {
        tracing::error!(
            workspace_id = %workspace_id,
            account_id = %account_id,
            "failed to reset account expiry after token persistence error: {reset_err}"
        );
    }
    Err(e)
}

/// Delete a secret from the configured backend (if using Vault)
///
/// For database backend: no-op (DB delete is handled separately)
/// For vault backend (EE only): deletes from Vault
pub async fn delete_secret_from_backend(db: &DB, workspace_id: &str, path: &str) -> Result<()> {
    if is_vault_backend_configured(db).await? {
        let backend = get_secret_backend(db).await?;
        // Ignore NotFound errors during deletion (secret might not exist in Vault)
        match backend.delete_secret(workspace_id, path).await {
            Ok(()) => Ok(()),
            Err(Error::NotFound(_)) => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        Ok(())
    }
}

/// Rename a secret in Vault when a variable path changes (EE only)
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn rename_vault_secret(
    _db: &DB,
    _workspace_id: &str,
    old_path: &str,
    new_path: &str,
    current_value: &str,
) -> Result<Option<String>> {
    if is_http_store_stored_value(current_value) {
        return Err(http_store_rename_error(old_path));
    }
    if is_vault_stored_value(current_value) {
        tracing::warn!(
            "Variable has $vault: prefix but Vault requires Enterprise Edition. \
             Updating DB reference to {}",
            new_path
        );
        return Ok(Some(format!("$vault:{}", new_path)));
    }
    if is_azure_kv_stored_value(current_value) {
        tracing::warn!(
            "Variable has $azure_kv: prefix but Azure Key Vault requires Enterprise Edition. \
             Updating DB reference to {}",
            new_path
        );
        return Ok(Some(format!("$azure_kv:{}", new_path)));
    }
    if is_aws_sm_stored_value(current_value) {
        tracing::warn!(
            "Variable has $aws_sm: prefix but AWS Secrets Manager requires Enterprise Edition. \
             Updating DB reference to {}",
            new_path
        );
        return Ok(Some(format!("$aws_sm:{}", new_path)));
    }
    Ok(None)
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn rename_vault_secret(
    db: &DB,
    workspace_id: &str,
    old_path: &str,
    new_path: &str,
    current_value: &str,
) -> Result<Option<String>> {
    if !is_external_stored_value(current_value) {
        return Ok(None);
    }
    if is_http_store_stored_value(current_value) {
        return Err(http_store_rename_error(old_path));
    }

    let marker_prefix = if current_value.starts_with("$azure_kv:") {
        "$azure_kv:"
    } else if current_value.starts_with("$aws_sm:") {
        "$aws_sm:"
    } else {
        "$vault:"
    };

    if !is_vault_backend_configured(db).await? {
        tracing::warn!(
            "Variable value has {} prefix but external secret backend is not configured. \
             Updating DB reference from {} to {}",
            marker_prefix,
            old_path,
            new_path
        );
        return Ok(Some(format!("{}{}", marker_prefix, new_path)));
    }

    let backend = get_secret_backend(db).await?;

    let secret_value = match backend.get_secret(workspace_id, old_path).await {
        Ok(value) => value,
        Err(Error::NotFound(_)) => {
            tracing::warn!(
                "Secret not found in backend at path {} during rename to {}",
                old_path,
                new_path
            );
            return Ok(Some(format!("{}{}", marker_prefix, new_path)));
        }
        Err(e) => return Err(e),
    };

    backend
        .set_secret(workspace_id, new_path, &secret_value)
        .await?;

    if let Err(e) = backend.delete_secret(workspace_id, old_path).await {
        tracing::warn!(
            "Failed to delete old secret at {} after rename to {}: {}",
            old_path,
            new_path,
            e
        );
    }

    Ok(Some(format!("{}{}", marker_prefix, new_path)))
}

/// Bulk rename secrets in Vault when a path prefix changes (e.g., user rename)
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn rename_vault_secrets_with_prefix(
    _db: &DB,
    _workspace_id: &str,
    _old_prefix: &str,
    _new_prefix: &str,
    _variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    Ok(vec![])
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn rename_vault_secrets_with_prefix(
    db: &DB,
    workspace_id: &str,
    old_prefix: &str,
    new_prefix: &str,
    variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    if !is_vault_backend_configured(db).await? {
        return Ok(vec![]);
    }

    let backend = get_secret_backend(db).await?;
    let mut updates = Vec::new();

    for (old_path, value) in variables {
        if !is_external_stored_value(&value) {
            continue;
        }

        let marker_prefix = if value.starts_with("$azure_kv:") {
            "$azure_kv:"
        } else if value.starts_with("$aws_sm:") {
            "$aws_sm:"
        } else {
            "$vault:"
        };

        let new_path = if old_path.starts_with(old_prefix) {
            format!("{}{}", new_prefix, &old_path[old_prefix.len()..])
        } else {
            continue;
        };

        let secret_value = match backend.get_secret(workspace_id, &old_path).await {
            Ok(v) => v,
            Err(Error::NotFound(_)) => {
                updates.push((old_path, format!("{}{}", marker_prefix, new_path)));
                continue;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to read secret at {} during bulk rename: {}",
                    old_path,
                    e
                );
                continue;
            }
        };

        if let Err(e) = backend
            .set_secret(workspace_id, &new_path, &secret_value)
            .await
        {
            tracing::error!(
                "Failed to write secret to {} during bulk rename: {}",
                new_path,
                e
            );
            continue;
        }

        if let Err(e) = backend.delete_secret(workspace_id, &old_path).await {
            tracing::warn!(
                "Failed to delete old secret at {} after rename: {}",
                old_path,
                e
            );
        }

        updates.push((old_path, format!("{}{}", marker_prefix, new_path)));
    }

    Ok(updates)
}

#[cfg(all(test, feature = "oauth2"))]
mod tests {
    use super::*;

    /// A refresh that fetched its token before a reconnect moved the variable to another account
    /// must not write that token over the reconnected one, even while the reconnect is still
    /// uncommitted when the refresh comes to store.
    #[sqlx::test(migrations = "../migrations")]
    async fn refresh_does_not_overwrite_a_reconnect(db: DB) {
        let w = "w";
        let path = "u/test/conn";
        sqlx::query(
            "INSERT INTO workspace (id, name, owner) VALUES ('w', 'w', 'test@windmill.dev')",
        )
        .execute(&db)
        .await
        .unwrap();
        let account = |client: &'static str| {
            let db = db.clone();
            async move {
                sqlx::query_scalar::<_, i32>(
                    "INSERT INTO account (workspace_id, client, expires_at, refresh_token)
                     VALUES ('w', $1, now(), 'r') RETURNING id",
                )
                .bind(client)
                .fetch_one(&db)
                .await
                .unwrap()
            }
        };
        let (old, new) = (account("old").await, account("new").await);
        sqlx::query(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description, account, is_oauth)
             VALUES ('w', $1, 'reconnected', true, '', $2, true)",
        )
        .bind(path)
        .bind(old)
        .execute(&db)
        .await
        .unwrap();

        let mut reconnect = db.begin().await.unwrap();
        sqlx::query("UPDATE variable SET account = $1 WHERE workspace_id = 'w' AND path = $2")
            .bind(new)
            .bind(path)
            .execute(&mut *reconnect)
            .await
            .unwrap();
        let refresh = tokio::spawn({
            let db = db.clone();
            async move { store_oauth_token_value(&db, w, path, old, "stale").await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        reconnect.commit().await.unwrap();

        assert!(!refresh.await.unwrap().unwrap());
        let value: String =
            sqlx::query_scalar("SELECT value FROM variable WHERE workspace_id = 'w' AND path = $1")
                .bind(path)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(value, "reconnected");
    }
}
