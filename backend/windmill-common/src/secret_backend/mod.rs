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
pub mod resolver;

pub use resolver::*;

#[cfg(feature = "private")]
pub mod vault_ee;

#[cfg(feature = "private")]
pub mod azure_kv_ee;
pub mod azure_kv_oss;
pub mod vault_oss;

#[cfg(feature = "private")]
pub mod aws_sm_ee;
pub mod aws_sm_oss;

#[cfg(feature = "private")]
pub mod http_store_ee;
pub mod http_store_oss;

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

#[cfg(feature = "private")]
pub use http_store_ee::*;

#[cfg(not(feature = "private"))]
pub use http_store_oss::*;

use std::collections::BTreeMap;

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

    /// Check that a variable path can be stored by this backend, before a reference to it is saved
    fn check_path(&self, _path: &str) -> Result<()> {
        Ok(())
    }

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
    /// Store secrets in Azure Key Vault (Enterprise Edition only)
    AzureKeyVault(AzureKeyVaultSettings),
    /// Store secrets in AWS Secrets Manager (Enterprise Edition only)
    AwsSecretsManager(AwsSecretsManagerSettings),
    /// Read secrets from any HTTPS secret store (Enterprise Edition only, read-only)
    HttpStore(HttpStoreSettings),
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
    /// Optional path prefix inserted between the KV `data`/`metadata` segment
    /// and the workspace id, e.g. "apps/windmill". When set, secrets live at
    /// `<mount>/data/<prefix>/<workspace>/<secret>`, so a Vault policy can be
    /// scoped to exactly `<mount>/data/<prefix>/*`. Surrounding slashes are
    /// trimmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_secret_path_prefix: Option<String>,
    /// JWT auth role name configured in Vault (used for JWT/OIDC auth)
    /// Optional - if not provided, token auth is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_role: Option<String>,
    /// Mount path for the JWT auth method in Vault (defaults to "jwt").
    /// Set this when the JWT auth method is mounted at a non-default path,
    /// e.g. via `vault auth enable -path=my-mount jwt`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_mount_path: Option<String>,
    /// Vault Enterprise namespace (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Static Vault token for testing/development (optional)
    /// If provided, this is used instead of JWT authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Skip TLS certificate verification when connecting to Vault
    /// Only use for self-signed certificates in development environments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_ssl_verify: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AzureKeyVaultSettings {
    /// Azure Key Vault URL (e.g., "https://myvault.vault.azure.net")
    pub vault_url: String,
    /// Azure AD tenant ID
    pub tenant_id: String,
    /// Azure AD application (client) ID
    pub client_id: String,
    /// Azure AD client secret. Optional — when omitted, the integration falls back to
    /// Azure Workload Identity Federation: the Kubernetes-projected service-account JWT at
    /// `AZURE_FEDERATED_TOKEN_FILE` is exchanged with Entra ID for an access token (no
    /// long-lived secret stored on the Windmill instance).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Static Bearer token for testing/development (optional)
    /// If provided, this is used instead of OAuth2 client credentials authentication
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

/// Settings for the generic HTTP secret store backend.
///
/// The backend is read-only: a secret variable is a reference whose store key is derived from the
/// workspace id and the variable path through the `read` template, and values are managed in the
/// store itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpStoreSettings {
    /// Scheme, host and optional path prefix of the store (e.g. "https://secrets.example.internal")
    pub base_url: String,
    pub read: HttpStoreRead,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_encoding: Option<HttpStorePathEncoding>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_headers: BTreeMap<String, String>,
    pub auth: HttpStoreAuth,
    /// PEM bundle of additional CA certificates trusted for the store's server certificate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_bundle_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_ssl_verify: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpStoreRead {
    /// Path template appended to `base_url`, with `{workspace}` and `{path}` placeholders
    pub path: String,
    /// Query parameters; values may use the same placeholders
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query: BTreeMap<String, String>,
    /// RFC 6901 JSON pointer to the value in the response body; empty or absent means the whole
    /// body is the value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_pointer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_encoding: Option<HttpStoreValueEncoding>,
}

/// How a variable path becomes a store key
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HttpStorePathEncoding {
    /// The whole path is one percent-encoded segment (`u/alice/db` -> `u%2Falice%2Fdb`)
    #[default]
    Url,
    /// Segments joined with `--` (`u/alice/db` -> `u--alice--db`)
    DoubleDash,
    /// The path is used as-is, as several URL segments
    Raw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HttpStoreValueEncoding {
    #[default]
    Plain,
    Base64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum HttpStoreAuth {
    None,
    StaticHeader {
        header_name: String,
        header_value: String,
    },
    /// A token read from a file (e.g. a Kubernetes projected service-account token), re-read
    /// when it changes on disk
    BearerTokenFile {
        token_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        header_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reload_interval_secs: Option<u64>,
    },
    /// A TLS client certificate and key read from PEM files, re-read when they change on disk.
    /// `cert_path` may hold the full chain.
    ClientCertificate {
        cert_path: String,
        key_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reload_interval_secs: Option<u64>,
    },
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
