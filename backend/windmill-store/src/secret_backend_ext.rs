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
    get_secret_backend, get_secret_value, is_aws_sm_stored_value, is_azure_kv_stored_value,
    is_external_stored_value, is_vault_backend_configured, is_vault_stored_value,
};

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
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Persist a freshly minted OAuth access token to the secret variable backing
/// a resource, routing through the configured secret backend.
///
/// This is the write counterpart of the lazy on-fetch OAuth refresh: it stores
/// the token via [`store_secret_value`] (which writes to the external backend —
/// AWS Secrets Manager / Azure Key Vault / Vault — when one is configured, or
/// encrypts for the database backend) and updates `variable.value` with the
/// returned value (the encrypted blob for the DB backend, or a `$...:` marker
/// for an external backend). Using a raw `UPDATE variable SET value = <encrypted>`
/// here instead would leave the external store frozen at its connect-time token
/// while reads (which resolve through the backend) keep serving the stale value.
///
/// The caller has already committed the `account` row as fresh (advanced
/// `expires_at`) by the time we get here. If persisting the token fails — most
/// likely a transient error talking to an external backend — that would leave
/// the account marked fresh while the served secret is stale, so the on-fetch
/// refresh gate (`now() > expires_at`) would skip refresh and keep serving the
/// stale token for the whole token lifetime. To avoid that we reset `expires_at`
/// to the past (and record `refresh_error`) on failure — looking the account up
/// via `variable.account` — so the very next fetch retries the refresh instead.
///
/// Authorization contract: this performs NO access control. It writes the
/// caller-supplied token into the secret variable at `path` and may mutate the
/// linked `account` row, so callers MUST have already authorized the operation
/// against `workspace_id`/`path` (the OAuth refresh adapters only run after the
/// read path has resolved and gated the variable). It is therefore kept
/// `pub(crate)` and intended solely for the in-crate refresh adapters.
#[cfg(feature = "oauth2")]
pub(crate) async fn store_oauth_token_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    token: &str,
) -> Result<()> {
    let persist = async {
        let value = store_secret_value(db, workspace_id, path, token).await?;
        sqlx::query("UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3")
            .bind(value)
            .bind(workspace_id)
            .bind(path)
            .execute(db)
            .await?;
        Ok::<(), Error>(())
    }
    .await;

    if let Err(e) = persist {
        // Mark the account expired again so the next fetch re-runs the refresh
        // instead of serving the now-stale token until it naturally expires. The
        // account id is the one linked from the variable being refreshed.
        let account_id: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT account FROM variable WHERE workspace_id = $1 AND path = $2",
        )
        .bind(workspace_id)
        .bind(path)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .flatten();

        if let Some(account_id) = account_id {
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
        }
        return Err(e);
    }

    Ok(())
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
    _old_path: &str,
    new_path: &str,
    current_value: &str,
) -> Result<Option<String>> {
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
