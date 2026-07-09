/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Resolution of the configured secret backend.
//!
//! Lives in `windmill-common` (rather than the API/store crates) so that
//! lower-level helpers such as [`crate::variables::get_variable_or_self`] can
//! route secret reads through the configured backend. With an external backend
//! (Vault / Azure Key Vault / AWS Secrets Manager), the `variable.value` column
//! holds a `$vault:`/`$azure_kv:`/`$aws_sm:` marker rather than base64
//! ciphertext, so decrypting it directly fails — reads must go through the
//! backend instead.
//!
//! Note: external backends require Enterprise Edition. The OSS version only
//! supports the database backend.

use std::sync::Arc;

use crate::{
    db::DB,
    error::{Error, Result},
    secret_backend::{database::DatabaseBackend, SecretBackend},
    variables::{build_crypt, decrypt},
};

#[cfg(all(feature = "private", feature = "enterprise"))]
use crate::{
    global_settings::{load_value_from_global_settings, SECRET_BACKEND_SETTING},
    secret_backend::{
        AwsSecretsManagerBackend, AwsSecretsManagerSettings, AzureKeyVaultBackend,
        AzureKeyVaultSettings, SecretBackendConfig, VaultBackend, VaultSettings,
    },
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

#[cfg(all(feature = "private", feature = "enterprise"))]
struct CachedAzureKvBackend {
    backend: Arc<dyn SecretBackend>,
    settings: AzureKeyVaultSettings,
}

#[cfg(all(feature = "private", feature = "enterprise"))]
lazy_static::lazy_static! {
    static ref AZURE_KV_BACKEND_CACHE: RwLock<Option<CachedAzureKvBackend>> = RwLock::new(None);
}

// Cached AWS Secrets Manager backend
#[cfg(all(feature = "private", feature = "enterprise"))]
struct CachedAwsSmBackend {
    backend: Arc<dyn SecretBackend>,
    settings: AwsSecretsManagerSettings,
}

#[cfg(all(feature = "private", feature = "enterprise"))]
lazy_static::lazy_static! {
    static ref AWS_SM_BACKEND_CACHE: RwLock<Option<CachedAwsSmBackend>> = RwLock::new(None);
}

/// Get the current secret backend based on global settings
///
/// OSS: Always returns DatabaseBackend
/// EE: Returns configured backend (Database or Vault)
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn get_secret_backend(db: &DB) -> Result<Arc<dyn SecretBackend>> {
    Ok(Arc::new(DatabaseBackend::new(db.clone())))
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn get_secret_backend(db: &DB) -> Result<Arc<dyn SecretBackend>> {
    let config = match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        Some(value) => serde_json::from_value::<SecretBackendConfig>(value).unwrap_or_default(),
        None => SecretBackendConfig::default(),
    };

    match config {
        SecretBackendConfig::Database => Ok(Arc::new(DatabaseBackend::new(db.clone()))),
        SecretBackendConfig::HashiCorpVault(settings) => {
            get_or_create_vault_backend(db, settings).await
        }
        SecretBackendConfig::AzureKeyVault(settings) => {
            get_or_create_azure_kv_backend(db, settings).await
        }
        SecretBackendConfig::AwsSecretsManager(settings) => {
            get_or_create_aws_sm_backend(db, settings).await
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

/// Get a cached Azure Key Vault backend or create a new one if settings changed
#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_azure_kv_backend(
    _db: &DB,
    settings: AzureKeyVaultSettings,
) -> Result<Arc<dyn SecretBackend>> {
    // Check if we have a cached backend with matching settings (read lock)
    {
        let cache = AZURE_KV_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings {
                return Ok(cached.backend.clone());
            }
        }
    }

    // Need to create a new backend - acquire write lock
    let mut cache = AZURE_KV_BACKEND_CACHE.write().await;

    // Double-check (another task may have created it while we waited)
    if let Some(ref cached) = *cache {
        if cached.settings == settings {
            return Ok(cached.backend.clone());
        }
    }

    // Create new backend
    let backend: Arc<dyn SecretBackend> = Arc::new(AzureKeyVaultBackend::new(settings.clone()));

    // Cache it
    *cache = Some(CachedAzureKvBackend { backend: backend.clone(), settings });

    Ok(backend)
}

/// Get a cached AWS SM backend or create a new one if settings changed
#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_aws_sm_backend(
    _db: &DB,
    settings: AwsSecretsManagerSettings,
) -> Result<Arc<dyn SecretBackend>> {
    {
        let cache = AWS_SM_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings {
                return Ok(cached.backend.clone());
            }
        }
    }

    let mut cache = AWS_SM_BACKEND_CACHE.write().await;

    if let Some(ref cached) = *cache {
        if cached.settings == settings {
            return Ok(cached.backend.clone());
        }
    }

    let backend: Arc<dyn SecretBackend> =
        Arc::new(AwsSecretsManagerBackend::new_with_client(settings.clone()).await?);

    *cache = Some(CachedAwsSmBackend { backend: backend.clone(), settings });

    Ok(backend)
}

/// Check if a Vault backend is currently configured
///
/// OSS: Always returns false
/// EE: Checks global settings
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn is_vault_backend_configured(_db: &DB) -> Result<bool> {
    Ok(false)
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn is_vault_backend_configured(db: &DB) -> Result<bool> {
    let config = match load_value_from_global_settings(db, SECRET_BACKEND_SETTING).await? {
        Some(value) => serde_json::from_value::<SecretBackendConfig>(value).unwrap_or_default(),
        None => SecretBackendConfig::default(),
    };

    Ok(matches!(
        config,
        SecretBackendConfig::HashiCorpVault(_)
            | SecretBackendConfig::AzureKeyVault(_)
            | SecretBackendConfig::AwsSecretsManager(_)
    ))
}

/// Get a secret value using the configured backend
///
/// For database backend: decrypts `encrypted_value` using the workspace key
/// For external backends (EE only): fetches from the backend at `path`,
/// ignoring `encrypted_value` (which holds only a `$...:` marker)
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
        "azure_key_vault" => backend.get_secret(workspace_id, path).await,
        "aws_secrets_manager" => backend.get_secret(workspace_id, path).await,
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

/// Check if a value is stored in Vault (indicated by the $vault: prefix)
pub fn is_vault_stored_value(value: &str) -> bool {
    value.starts_with("$vault:")
}

/// Check if a value is stored in Azure Key Vault (indicated by the $azure_kv: prefix)
pub fn is_azure_kv_stored_value(value: &str) -> bool {
    value.starts_with("$azure_kv:")
}

/// Check if a value is stored in AWS Secrets Manager (indicated by the $aws_sm: prefix)
pub fn is_aws_sm_stored_value(value: &str) -> bool {
    value.starts_with("$aws_sm:")
}

/// Check if a value is stored in any external secret backend
pub fn is_external_stored_value(value: &str) -> bool {
    is_vault_stored_value(value) || is_azure_kv_stored_value(value) || is_aws_sm_stored_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_markers_are_detected() {
        assert!(is_external_stored_value("$vault:u/admin/secret"));
        assert!(is_external_stored_value("$azure_kv:u/admin/secret"));
        assert!(is_external_stored_value("$aws_sm:u/admin/secret"));
    }

    #[test]
    fn base64_ciphertext_is_not_treated_as_external() {
        // A base64 magic_crypt blob must route through `decrypt`, never the
        // external backend. The leading `$` is what distinguishes a marker from
        // ciphertext; decrypting a marker fails with "Invalid byte 36" (`$`),
        // which is the bug this gate prevents.
        for v in [
            "bm90LWEtbWFya2Vy",
            "AAAA1234+/abcd==",
            "",
            "$something_else",
        ] {
            assert!(!is_external_stored_value(v), "unexpected external: {v:?}");
        }
    }
}
