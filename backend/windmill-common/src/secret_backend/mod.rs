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

#[cfg(test)]
mod tests;

#[cfg(feature = "private")]
pub use vault_ee::*;

#[cfg(not(feature = "private"))]
pub use vault_oss::*;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Trait for secret storage backends
///
/// Implementations of this trait handle the storage and retrieval of secrets.
/// The default implementation stores secrets encrypted in the database.
/// Enterprise Edition supports HashiCorp Vault as an alternative backend.
#[async_trait]
pub trait SecretBackend: Send + Sync {
    /// Retrieve a secret value
    ///
    /// # Arguments
    /// * `workspace_id` - The workspace identifier
    /// * `path` - The path/name of the secret variable
    ///
    /// # Returns
    /// The decrypted secret value
    async fn get_secret(&self, workspace_id: &str, path: &str) -> Result<String>;

    /// Store a secret value
    ///
    /// # Arguments
    /// * `workspace_id` - The workspace identifier
    /// * `path` - The path/name of the secret variable
    /// * `value` - The plaintext secret value to store
    async fn set_secret(&self, workspace_id: &str, path: &str, value: &str) -> Result<()>;

    /// Delete a secret
    ///
    /// # Arguments
    /// * `workspace_id` - The workspace identifier
    /// * `path` - The path/name of the secret variable
    async fn delete_secret(&self, workspace_id: &str, path: &str) -> Result<()>;

    /// Get the name of this backend for logging/debugging
    fn backend_name(&self) -> &'static str;
}

/// Configuration for secret storage backend
///
/// This enum is stored in global_settings and determines which backend
/// is used for secret storage at the instance level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SecretBackendConfig {
    /// Store secrets encrypted in the database (default behavior)
    Database,
    /// Store secrets in HashiCorp Vault (Enterprise Edition only)
    HashiCorpVault(VaultSettings),
}

impl Default for SecretBackendConfig {
    fn default() -> Self {
        SecretBackendConfig::Database
    }
}

/// Settings for HashiCorp Vault integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultSettings {
    /// Vault server address (e.g., "https://vault.company.com:8200")
    pub address: String,
    /// KV v2 mount path (e.g., "windmill")
    pub mount_path: String,
    /// JWT auth role name configured in Vault (used for JWT/OIDC auth)
    /// Optional - if not provided, token auth is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_role: Option<String>,
    /// Vault Enterprise namespace (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Static Vault token for testing/development (optional)
    /// If provided, this is used instead of JWT authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Result of a secret migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMigrationReport {
    /// Total number of secrets found
    pub total_secrets: usize,
    /// Number of secrets successfully migrated
    pub migrated_count: usize,
    /// Number of secrets that failed to migrate
    pub failed_count: usize,
    /// Details of any failures
    pub failures: Vec<SecretMigrationFailure>,
}

/// Details of a failed secret migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMigrationFailure {
    /// Workspace ID where the secret is located
    pub workspace_id: String,
    /// Path of the secret that failed to migrate
    pub path: String,
    /// Error message
    pub error: String,
}
