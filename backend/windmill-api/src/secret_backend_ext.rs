/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret backend extension for the API layer
//!
//! Backend resolution and read helpers live in
//! `windmill_common::secret_backend`; this module keeps the API-specific bulk
//! rename helper used when renaming users.
//!
//! Note: HashiCorp Vault integration requires Enterprise Edition.
//! The OSS version only supports the database backend.

use windmill_common::{db::DB, error::Result};

#[cfg(all(feature = "private", feature = "enterprise"))]
use windmill_common::{
    error::Error,
    secret_backend::{
        get_secret_backend, is_aws_sm_stored_value, is_azure_kv_stored_value,
        is_external_stored_value, is_vault_backend_configured,
    },
};

/// Bulk rename secrets in Vault when a path prefix changes (e.g., user rename)
/// EE only feature.
///
/// This is used when renaming users where many secrets need their paths updated.
/// Returns a list of (old_path, new_value) pairs for updating the database.
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn rename_vault_secrets_with_prefix(
    _db: &DB,
    _workspace_id: &str,
    _old_prefix: &str,
    _new_prefix: &str,
    _variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    // OSS: No Vault support, return empty
    Ok(vec![])
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn rename_vault_secrets_with_prefix(
    db: &DB,
    workspace_id: &str,
    old_prefix: &str,
    new_prefix: &str,
    variables: Vec<(String, String)>, // (path, value) pairs
) -> Result<Vec<(String, String)>> {
    // Only process if an external secret backend is configured
    if !is_vault_backend_configured(db).await? {
        return Ok(vec![]);
    }

    let backend = get_secret_backend(db).await?;
    let mut updates = Vec::new();

    for (old_path, value) in variables {
        // Only handle externally-stored values
        if !is_external_stored_value(&value) {
            continue;
        }

        // Determine the marker prefix from the stored value
        let marker_prefix = if is_azure_kv_stored_value(&value) {
            "$azure_kv:"
        } else if is_aws_sm_stored_value(&value) {
            "$aws_sm:"
        } else {
            "$vault:"
        };

        // Calculate new path by replacing prefix
        let new_path = if old_path.starts_with(old_prefix) {
            format!("{}{}", new_prefix, &old_path[old_prefix.len()..])
        } else {
            continue; // Path doesn't match prefix, skip
        };

        // Read from old path
        let secret_value = match backend.get_secret(workspace_id, &old_path).await {
            Ok(v) => v,
            Err(Error::NotFound(_)) => {
                // Just update DB reference
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

        // Write to new path
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

        // Delete from old path
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
