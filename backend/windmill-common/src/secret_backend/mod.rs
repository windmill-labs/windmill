/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Secret Backend abstraction for storing secrets in external vaults
//!
//! This module provides a trait-based abstraction for secret storage,
//! allowing secrets to be stored in the database (default) or in external
//! vaults like HashiCorp Vault (Enterprise Edition).

pub mod database;

#[cfg(feature = "private")]
pub mod vault_ee;

pub mod vault_oss;
#[cfg(feature = "private")]
pub mod azure_kv_ee;
pub mod azure_kv_oss;

#[cfg(feature = "private")]
pub mod aws_sm_ee;
pub mod aws_sm_oss;

#[cfg(test)]
mod tests;

#[cfg(feature = "private")]
pub use vault_ee::*;

#[cfg(not(feature = "private"))]
pub use vault_oss::*;

#[cfg(feature = "private")]
pub use azure_kv_ee::*;

#[cfg(not(feature = "private"))]
pub use azure_kv_oss::*;

#[cfg(feature = "private")]
pub use aws_sm_ee::*;

#[cfg(not(feature = "private"))]
pub use aws_sm_oss::*;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Trait for secret storage backends
#[async_trait]
pub trait SecretBackend: Send + Sync {
    async fn get_secret(&self, workspace_id: &str, path: &str) -> Result<String>;
    async fn set_secret(&self, workspace_id: &str, path: &str, value: &str) -> Result<()>;
    async fn delete_secret(&self, workspace_id: &str, path: &str) -> Result<()>;
    fn backend_name(&self) -> &'static str;
}

/// Configuration for secret storage backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SecretBackendConfig {
    Database,
    HashiCorpVault(VaultSettings),
    AzureKeyVault(AzureKeyVaultSettings),
    AwsSecretsManager(AwsSecretsManagerSettings),
}

impl Default for SecretBackendConfig {
    fn default() -> Self {
        SecretBackendConfig::Database
    }
}

/// Settings for HashiCorp Vault integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultSettings {
    pub address: String,
    pub mount_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AzureKeyVaultSettings {
    pub vault_url: String,
    pub tenant_id: String,
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Settings for AWS Secrets Manager integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AwsSecretsManagerSettings {
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    /// Static AWS access key ID (optional - uses default credential chain if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    /// Static AWS secret access key (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    /// Custom endpoint URL for LocalStack/testing (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
    /// Prefix for secret names in AWS Secrets Manager (e.g., "windmill/")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

/// Result of a secret migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMigrationReport {
    pub total_secrets: usize,
    pub migrated_count: usize,
    pub failed_count: usize,
    pub failures: Vec<SecretMigrationFailure>,
}

/// Details of a failed secret migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMigrationFailure {
    pub workspace_id: String,
    pub path: String,
    pub error: String,
}
