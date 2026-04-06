/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use windmill_common::{
    db::DB,
    error::{Error, Result},
    secret_backend::{database::DatabaseBackend, SecretBackend},
    variables::{build_crypt, decrypt, encrypt},
};

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

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_or_create_vault_backend(
    _db: &DB,
    settings: VaultSettings,
) -> Result<Arc<dyn SecretBackend>> {
    {
        let cache = VAULT_BACKEND_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.settings == settings {
                return Ok(cached.backend.clone());
            }
        }
    }
    let mut cache = VAULT_BACKEND_CACHE.write().await;
    if let Some(ref cached) = *cache {
        if cached.settings == settings {
            return Ok(cached.backend.clone());
        }
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
            if cached.settings == settings {
                return Ok(cached.backend.clone());
            }
        }
    }
    let mut cache = AZURE_KV_BACKEND_CACHE.write().await;
    if let Some(ref cached) = *cache {
        if cached.settings == settings {
            return Ok(cached.backend.clone());
        }
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

pub async fn get_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    encrypted_value: &str,
) -> Result<String> {
    let backend = get_secret_backend(db).await?;
    match backend.backend_name() {
        "database" => {
            let mc = build_crypt(db, workspace_id).await?;
            decrypt(&mc, encrypted_value.to_string()).map_err(|e| {
                Error::internal_err(format!("Error decrypting variable {}: {}", path, e))
            })
        }
        "hashicorp_vault" | "azure_key_vault" | "aws_secrets_manager" => {
            backend.get_secret(workspace_id, path).await
        }
        _ => Err(Error::internal_err(format!(
            "Unknown backend: {}",
            backend.backend_name()
        ))),
    }
}

pub async fn store_secret_value(
    db: &DB,
    workspace_id: &str,
    path: &str,
    plain_value: &str,
) -> Result<String> {
    let backend = get_secret_backend(db).await?;
    match backend.backend_name() {
        "database" => {
            let mc = build_crypt(db, workspace_id).await?;
            Ok(encrypt(&mc, plain_value))
        }
        "hashicorp_vault" => {
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

pub async fn delete_secret_from_backend(db: &DB, workspace_id: &str, path: &str) -> Result<()> {
    if is_vault_backend_configured(db).await? {
        let backend = get_secret_backend(db).await?;
        match backend.delete_secret(workspace_id, path).await {
            Ok(()) => Ok(()),
            Err(Error::NotFound(_)) => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        Ok(())
    }
}

pub fn is_vault_stored_value(value: &str) -> bool {
    value.starts_with("$vault:")
}

pub fn is_azure_kv_stored_value(value: &str) -> bool {
    value.starts_with("$azure_kv:")
}

pub fn is_aws_sm_stored_value(value: &str) -> bool {
    value.starts_with("$aws_sm:")
}

pub fn is_external_stored_value(value: &str) -> bool {
    is_vault_stored_value(value) || is_azure_kv_stored_value(value) || is_aws_sm_stored_value(value)
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn rename_vault_secret(
    _db: &DB,
    _workspace_id: &str,
    _old_path: &str,
    new_path: &str,
    current_value: &str,
) -> Result<Option<String>> {
    if is_vault_stored_value(current_value) {
        return Ok(Some(format!("$vault:{}", new_path)));
    }
    if is_azure_kv_stored_value(current_value) {
        return Ok(Some(format!("$azure_kv:{}", new_path)));
    }
    if is_aws_sm_stored_value(current_value) {
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
            marker_prefix, old_path, new_path
        );
        return Ok(Some(format!("{}{}", marker_prefix, new_path)));
    }

    let backend = get_secret_backend(db).await?;

    let secret_value = match backend.get_secret(workspace_id, old_path).await {
        Ok(value) => value,
        Err(Error::NotFound(_)) => {
            tracing::warn!(
                "Secret not found in backend at path {} during rename to {}",
                old_path, new_path
            );
            return Ok(Some(format!("{}{}", marker_prefix, new_path)));
        }
        Err(e) => return Err(e),
    };

    backend.set_secret(workspace_id, new_path, &secret_value).await?;

    if let Err(e) = backend.delete_secret(workspace_id, old_path).await {
        tracing::warn!(
            "Failed to delete old secret at {} after rename to {}: {}",
            old_path, new_path, e
        );
    }

    Ok(Some(format!("{}{}", marker_prefix, new_path)))
}

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
