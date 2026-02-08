/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret backend extension for the API layer
//!
//! This module provides helper functions for integrating the SecretBackend
//! trait with variable operations in the API.
//!
//! Note: HashiCorp Vault integration requires Enterprise Edition.
//! The OSS version only supports the database backend.

#[cfg(all(feature = "private", feature = "enterprise"))]
use std::sync::Arc;

use windmill_common::{db::DB, error::Result};

#[cfg(all(feature = "private", feature = "enterprise"))]
use windmill_common::error::Error;

#[cfg(all(feature = "private", feature = "enterprise"))]
use windmill_common::secret_backend::{database::DatabaseBackend, SecretBackend};

#[cfg(all(feature = "private", feature = "enterprise"))]
use windmill_common::{
    global_settings::{load_value_from_global_settings, SECRET_BACKEND_SETTING},
    secret_backend::{SecretBackendConfig, VaultBackend, VaultSettings},
};

#[cfg(all(feature = "private", feature = "enterprise"))]
use tokio::sync::RwLock;

// Cached Vault backend to avoid recreating it for every request
// This enables connection pooling and avoids repeated setup overhead
#[cfg(all(feature = "private", feature = "enterprise"))]
struct CachedVaultBackend {
    backend: Arc<dyn SecretBackend>,
    settings: VaultSettings,
}

#[cfg(all(feature = "private", feature = "enterprise"))]
lazy_static::lazy_static! {
    static ref VAULT_BACKEND_CACHE: RwLock<Option<CachedVaultBackend>> = RwLock::new(None);
}

/// Get the current secret backend based on global settings (EE only)
#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_secret_backend(db: &DB) -> Result<Arc<dyn SecretBackend>> {
    let config = match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        Some(value) => serde_json::from_value::<SecretBackendConfig>(value).unwrap_or_default(),
        None => SecretBackendConfig::default(),
    };

    match config {
        SecretBackendConfig::Database => Ok(Arc::new(DatabaseBackend::new(db.clone()))),
        SecretBackendConfig::HashiCorpVault(settings) => {
            get_or_create_vault_backend(db, settings).await
        }
    }
}

/// Get a cached Vault backend or create a new one if settings changed
#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_vault_backend(
    _db: &DB,
    settings: VaultSettings,
) -> Result<Arc<dyn SecretBackend>> {
    // Check if we have a cached backend with matching settings (read lock)
    {
        let cache = VAULT_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings {
                return Ok(cached.backend.clone());
            }
        }
    }

    // Need to create a new backend - acquire write lock
    let mut cache = VAULT_BACKEND_CACHE.write().await;

    // Double-check (another task may have created it while we waited)
    if let Some(ref cached) = *cache {
        if cached.settings == settings {
            return Ok(cached.backend.clone());
        }
    }

    // Create new backend
    let backend: Arc<dyn SecretBackend> = {
        #[cfg(feature = "openidconnect")]
        if settings.token.is_none() {
            Arc::new(VaultBackend::new_with_db(settings.clone(), _db.clone()))
        } else {
            Arc::new(VaultBackend::new(settings.clone()))
        }

        #[cfg(not(feature = "openidconnect"))]
        Arc::new(VaultBackend::new(settings.clone()))
    };

    // Cache it
    *cache = Some(CachedVaultBackend { backend: backend.clone(), settings });

    Ok(backend)
}

/// Check if a Vault backend is currently configured (EE only)
#[cfg(all(feature = "private", feature = "enterprise"))]
async fn is_vault_backend_configured(db: &DB) -> Result<bool> {
    let config = match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        Some(value) => serde_json::from_value::<SecretBackendConfig>(value).unwrap_or_default(),
        None => SecretBackendConfig::default(),
    };

    Ok(matches!(config, SecretBackendConfig::HashiCorpVault(_)))
}

/// Check if a value is stored in Vault (indicated by the $vault: prefix)
#[cfg(all(feature = "private", feature = "enterprise"))]
fn is_vault_stored_value(value: &str) -> bool {
    value.starts_with("$vault:")
}

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
    // Only process if Vault is configured
    if !is_vault_backend_configured(db).await? {
        return Ok(vec![]);
    }

    let backend = get_secret_backend(db).await?;
    let mut updates = Vec::new();

    for (old_path, value) in variables {
        // Only handle Vault-stored values
        if !is_vault_stored_value(&value) {
            continue;
        }

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
                updates.push((old_path, format!("$vault:{}", new_path)));
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

        updates.push((old_path, format!("$vault:{}", new_path)));
    }

    Ok(updates)
}
