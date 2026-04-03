/*
 * Author: Windmill Labs, Inc
 * Copyright (C) Windmill Labs, Inc - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 */

use async_trait::async_trait;

use crate::db::DB;
use crate::error::{Error, Result};

use super::{AzureKeyVaultSettings, SecretBackend, SecretMigrationReport};

pub struct AzureKeyVaultBackend;

impl AzureKeyVaultBackend {
    pub fn new(_settings: AzureKeyVaultSettings) -> Self {
        AzureKeyVaultBackend
    }
}

#[async_trait]
impl SecretBackend for AzureKeyVaultBackend {
    async fn get_secret(&self, _workspace_id: &str, _path: &str) -> Result<String> {
        Err(Error::internal_err(
            "Azure Key Vault integration requires Enterprise Edition".to_string(),
        ))
    }

    async fn set_secret(&self, _workspace_id: &str, _path: &str, _value: &str) -> Result<()> {
        Err(Error::internal_err(
            "Azure Key Vault integration requires Enterprise Edition".to_string(),
        ))
    }

    async fn delete_secret(&self, _workspace_id: &str, _path: &str) -> Result<()> {
        Err(Error::internal_err(
            "Azure Key Vault integration requires Enterprise Edition".to_string(),
        ))
    }

    fn backend_name(&self) -> &'static str {
        "azure_key_vault"
    }
}

pub async fn test_azure_kv_connection(_settings: &AzureKeyVaultSettings) -> Result<()> {
    Err(Error::internal_err(
        "Azure Key Vault integration requires Enterprise Edition".to_string(),
    ))
}

pub async fn migrate_secrets_to_azure_kv(
    _db: &DB,
    _settings: &AzureKeyVaultSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(
        "Azure Key Vault integration requires Enterprise Edition".to_string(),
    ))
}

pub async fn migrate_secrets_from_azure_kv(
    _db: &DB,
    _settings: &AzureKeyVaultSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(
        "Azure Key Vault integration requires Enterprise Edition".to_string(),
    ))
}
