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

use std::sync::Arc;

use windmill_common::{
    db::DB,
    error::{Error, Result},
    global_settings::{load_value_from_global_settings, SECRET_BACKEND_SETTING},
    secret_backend::{
        database::DatabaseBackend, vault_oss::VaultBackend, SecretBackend, SecretBackendConfig,
        VaultSettings,
    },
    variables::{build_crypt, decrypt, encrypt},
};

/// Get the current secret backend based on global settings
///
/// This function reads the `secret_backend` setting from global_settings
/// and returns the appropriate backend instance. If no setting is found
/// or the setting is invalid, it defaults to the database backend.
pub async fn get_secret_backend(db: &DB) -> Result<Arc<dyn SecretBackend>> {
    let config = match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        Some(value) => serde_json::from_value::<SecretBackendConfig>(value).unwrap_or_default(),
        None => SecretBackendConfig::default(),
    };

    match config {
        SecretBackendConfig::Database => Ok(Arc::new(DatabaseBackend::new(db.clone()))),
        SecretBackendConfig::HashiCorpVault(settings) => {
            Ok(Arc::new(VaultBackend::new(settings)))
        }
    }
}

/// Check if a Vault backend is currently configured
pub async fn is_vault_backend_configured(db: &DB) -> Result<bool> {
    let config = match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        Some(value) => serde_json::from_value::<SecretBackendConfig>(value).unwrap_or_default(),
        None => SecretBackendConfig::default(),
    };

    Ok(matches!(config, SecretBackendConfig::HashiCorpVault(_)))
}

/// Get a secret value using the configured backend
///
/// For database backend: decrypts using workspace key
/// For vault backend: fetches from Vault directly
pub async fn get_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    encrypted_value: &str,
) -> Result<String> {
    let backend = get_secret_backend(db).await?;

    match backend.backend_name() {
        "database" => {
            // Use existing database decryption
            let mc = build_crypt(db, workspace_id).await?;
            decrypt(&mc, encrypted_value.to_string()).map_err(|e| {
                Error::internal_err(format!("Error decrypting variable {}: {}", path, e))
            })
        }
        "hashicorp_vault" => {
            // Fetch from Vault directly
            backend.get_secret(workspace_id, path).await
        }
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Store a secret value using the configured backend
///
/// For database backend: encrypts using workspace key and returns encrypted value
/// For vault backend: stores in Vault and returns a placeholder for DB storage
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
            // Return a marker indicating the value is stored in Vault
            // The actual value in the DB will be this marker
            Ok(format!("$vault:{}", path))
        }
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Delete a secret from the configured backend (if using Vault)
///
/// For database backend: no-op (DB delete is handled separately)
/// For vault backend: deletes from Vault
pub async fn delete_secret_from_backend(
    db: &DB,
    workspace_id: &str,
    path: &str,
) -> Result<()> {
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

/// Check if a value is stored in Vault (indicated by the $vault: prefix)
pub fn is_vault_stored_value(value: &str) -> bool {
    value.starts_with("$vault:")
}
