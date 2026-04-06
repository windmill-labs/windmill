/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

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
    secret_backend::{
        AwsSecretsManagerBackend, AwsSecretsManagerSettings, AzureKeyVaultBackend,
        AzureKeyVaultSettings, SecretBackendConfig, VaultBackend, VaultSettings,
    },
};

#[cfg(all(feature = "private", feature = "enterprise"))]
use tokio::sync::RwLock;

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

#[cfg(all(feature = "private", feature = "enterprise"))]
struct CachedAwsSmBackend {
    backend: Arc<dyn SecretBackend>,
    settings: AwsSecretsManagerSettings,
}

#[cfg(all(feature = "private", feature = "enterprise"))]
lazy_static::lazy_static! {
    static ref AWS_SM_BACKEND_CACHE: RwLock<Option<CachedAwsSmBackend>> = RwLock::new(None);
}

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
        SecretBackendConfig::AzureKeyVault(settings) => {
            get_or_create_azure_kv_backend(db, settings).await
        }
        SecretBackendConfig::AwsSecretsManager(settings) => {
            get_or_create_aws_sm_backend(db, settings).await
        }
    }
}

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_vault_backend(
    _db: &DB,
    settings: VaultSettings,
) -> Result<Arc<dyn SecretBackend>> {
    {
        let cache = VAULT_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings { return Ok(cached.backend.clone()); }
        }
    }
    let mut cache = VAULT_BACKEND_CACHE.write().await;
    if let Some(ref cached) = *cache {
        if cached.settings == settings { return Ok(cached.backend.clone()); }
    }
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
    *cache = Some(CachedVaultBackend { backend: backend.clone(), settings });
    Ok(backend)
}

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_azure_kv_backend(
    _db: &DB,
    settings: AzureKeyVaultSettings,
) -> Result<Arc<dyn SecretBackend>> {
    {
        let cache = AZURE_KV_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings { return Ok(cached.backend.clone()); }
        }
    }
    let mut cache = AZURE_KV_BACKEND_CACHE.write().await;
    if let Some(ref cached) = *cache {
        if cached.settings == settings { return Ok(cached.backend.clone()); }
    }
    let backend: Arc<dyn SecretBackend> = Arc::new(AzureKeyVaultBackend::new(settings.clone()));
    *cache = Some(CachedAzureKvBackend { backend: backend.clone(), settings });
    Ok(backend)
}

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_aws_sm_backend(
    _db: &DB,
    settings: AwsSecretsManagerSettings,
) -> Result<Arc<dyn SecretBackend>> {
    {
        let cache = AWS_SM_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings { return Ok(cached.backend.clone()); }
        }
    }
    let mut cache = AWS_SM_BACKEND_CACHE.write().await;
    if let Some(ref cached) = *cache {
        if cached.settings == settings { return Ok(cached.backend.clone()); }
    }
    let backend: Arc<dyn SecretBackend> =
        Arc::new(AwsSecretsManagerBackend::new_with_client(settings.clone()).await?);
    *cache = Some(CachedAwsSmBackend { backend: backend.clone(), settings });
    Ok(backend)
}

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn is_vault_backend_configured(db: &DB) -> Result<bool> {
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

#[cfg(all(feature = "private", feature = "enterprise"))]
fn is_vault_stored_value(value: &str) -> bool { value.starts_with("$vault:") }

#[cfg(all(feature = "private", feature = "enterprise"))]
fn is_azure_kv_stored_value(value: &str) -> bool { value.starts_with("$azure_kv:") }

#[cfg(all(feature = "private", feature = "enterprise"))]
fn is_aws_sm_stored_value(value: &str) -> bool { value.starts_with("$aws_sm:") }

#[cfg(all(feature = "private", feature = "enterprise"))]
fn is_external_stored_value(value: &str) -> bool {
    is_vault_stored_value(value) || is_azure_kv_stored_value(value) || is_aws_sm_stored_value(value)
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn rename_vault_secrets_with_prefix(
    _db: &DB, _workspace_id: &str, _old_prefix: &str, _new_prefix: &str,
    _variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    Ok(vec![])
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn rename_vault_secrets_with_prefix(
    db: &DB, workspace_id: &str, old_prefix: &str, new_prefix: &str,
    variables: Vec<(String, String)>,
) -> Result<Vec<(String, String)>> {
    if !is_vault_backend_configured(db).await? { return Ok(vec![]); }
    let backend = get_secret_backend(db).await?;
    let mut updates = Vec::new();

    for (old_path, value) in variables {
        if !is_external_stored_value(&value) { continue; }

        let marker_prefix = if value.starts_with("$azure_kv:") {
            "$azure_kv:"
        } else if value.starts_with("$aws_sm:") {
            "$aws_sm:"
        } else {
            "$vault:"
        };

        let new_path = if old_path.starts_with(old_prefix) {
            format!("{}{}", new_prefix, &old_path[old_prefix.len()..])
        } else { continue; };

        let secret_value = match backend.get_secret(workspace_id, &old_path).await {
            Ok(v) => v,
            Err(Error::NotFound(_)) => {
                updates.push((old_path, format!("{}{}", marker_prefix, new_path)));
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to read secret at {} during bulk rename: {}", old_path, e);
                continue;
            }
        };

        if let Err(e) = backend.set_secret(workspace_id, &new_path, &secret_value).await {
            tracing::error!("Failed to write secret to {} during bulk rename: {}", new_path, e);
            continue;
        }

        if let Err(e) = backend.delete_secret(workspace_id, &old_path).await {
            tracing::warn!("Failed to delete old secret at {} after rename: {}", old_path, e);
        }

        updates.push((old_path, format!("{}{}", marker_prefix, new_path)));
    }
    Ok(updates)
}
