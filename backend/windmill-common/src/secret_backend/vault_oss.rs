/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! HashiCorp Vault secret backend stubs (Open Source Edition)
//!
//! This module provides stub implementations for Vault integration.
//! The actual Vault integration requires Enterprise Edition.

use std::sync::Arc;

use crate::db::DB;
use crate::error::{Error, Result};

use super::{
    database::DatabaseBackend, SecretBackend, SecretBackendConfig, SecretMigrationReport,
    VaultSettings,
};

/// Stub VaultBackend for OSS - all operations return EE required error
pub struct VaultBackend;

impl VaultBackend {
    pub fn new(_settings: VaultSettings) -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SecretBackend for VaultBackend {
    async fn get_secret(&self, _workspace_id: &str, _path: &str) -> Result<String> {
        Err(Error::internal_err(
            "HashiCorp Vault integration requires Enterprise Edition".to_string(),
        ))
    }

    async fn set_secret(&self, _workspace_id: &str, _path: &str, _value: &str) -> Result<()> {
        Err(Error::internal_err(
            "HashiCorp Vault integration requires Enterprise Edition".to_string(),
        ))
    }

    async fn delete_secret(&self, _workspace_id: &str, _path: &str) -> Result<()> {
        Err(Error::internal_err(
            "HashiCorp Vault integration requires Enterprise Edition".to_string(),
        ))
    }

    fn backend_name(&self) -> &'static str {
        "hashicorp_vault"
    }
}

/// Create the appropriate secret backend based on configuration
///
/// In OSS, always returns DatabaseBackend regardless of config.
/// Vault configuration is ignored with a warning.
pub async fn create_secret_backend(
    db: DB,
    config: &SecretBackendConfig,
) -> Result<Arc<dyn SecretBackend>> {
    match config {
        SecretBackendConfig::Database => Ok(Arc::new(DatabaseBackend::new(db))),
        SecretBackendConfig::HashiCorpVault(_) => {
            tracing::warn!(
                "HashiCorp Vault is configured but requires Enterprise Edition. \
                 Falling back to database backend."
            );
            Ok(Arc::new(DatabaseBackend::new(db)))
        }
    }
}

/// Test connection to Vault (OSS stub)
pub async fn test_vault_connection(_settings: &VaultSettings, _db: Option<&DB>) -> Result<()> {
    Err(Error::internal_err(
        "HashiCorp Vault integration requires Enterprise Edition".to_string(),
    ))
}

/// Migrate secrets from database to Vault (OSS stub)
pub async fn migrate_secrets_to_vault(
    _db: &DB,
    _settings: &VaultSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(
        "HashiCorp Vault integration requires Enterprise Edition".to_string(),
    ))
}

/// Migrate secrets from Vault back to database (OSS stub)
pub async fn migrate_secrets_to_database(
    _db: &DB,
    _settings: &VaultSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(
        "HashiCorp Vault integration requires Enterprise Edition".to_string(),
    ))
}

/// Generate a JWT for Vault authentication (OSS stub)
pub async fn generate_vault_jwt(_db: &DB, _vault_address: &str) -> Result<String> {
    Err(Error::internal_err(
        "HashiCorp Vault integration requires Enterprise Edition".to_string(),
    ))
}
