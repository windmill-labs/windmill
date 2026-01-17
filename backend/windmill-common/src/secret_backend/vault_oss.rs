/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! HashiCorp Vault secret backend implementation
//!
//! This module provides integration with HashiCorp Vault for storing secrets.
//! It supports both token-based authentication (for testing) and JWT/OIDC
//! authentication (for production use with EE).

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::DB;
use crate::error::{Error, Result};
use crate::variables::{build_crypt, decrypt, encrypt};

use super::{
    database::DatabaseBackend, SecretBackend, SecretBackendConfig, SecretMigrationFailure,
    SecretMigrationReport, VaultSettings,
};

/// HashiCorp Vault backend
pub struct VaultBackend {
    settings: VaultSettings,
    client: reqwest::Client,
}

impl VaultBackend {
    /// Create a new Vault backend
    pub fn new(settings: VaultSettings) -> Self {
        Self {
            settings,
            client: reqwest::Client::new(),
        }
    }

    /// Get headers for Vault API requests
    fn get_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Use token if provided, otherwise JWT auth would be needed
        if let Some(ref token) = self.settings.token {
            headers.insert(
                "X-Vault-Token",
                HeaderValue::from_str(token)
                    .map_err(|e| Error::internal_err(format!("Invalid Vault token: {}", e)))?,
            );
        } else {
            return Err(Error::internal_err(
                "Vault token is required (JWT auth not implemented in OSS)".to_string(),
            ));
        }

        // Add namespace header if specified (Vault Enterprise)
        if let Some(ref namespace) = self.settings.namespace {
            headers.insert(
                "X-Vault-Namespace",
                HeaderValue::from_str(namespace)
                    .map_err(|e| Error::internal_err(format!("Invalid namespace: {}", e)))?,
            );
        }

        Ok(headers)
    }

    /// Build the path for a secret in Vault KV v2
    fn secret_path(&self, workspace_id: &str, path: &str) -> String {
        format!(
            "{}/v1/{}/data/{}/{}",
            self.settings.address.trim_end_matches('/'),
            self.settings.mount_path,
            workspace_id,
            path
        )
    }

    /// Build the path for deleting a secret in Vault KV v2
    fn delete_path(&self, workspace_id: &str, path: &str) -> String {
        format!(
            "{}/v1/{}/metadata/{}/{}",
            self.settings.address.trim_end_matches('/'),
            self.settings.mount_path,
            workspace_id,
            path
        )
    }

    /// Build the path for listing secrets in Vault KV v2
    fn list_path(&self, workspace_id: &str) -> String {
        format!(
            "{}/v1/{}/metadata/{}",
            self.settings.address.trim_end_matches('/'),
            self.settings.mount_path,
            workspace_id
        )
    }
}

/// Vault KV v2 read response
#[derive(Debug, Deserialize)]
struct VaultReadResponse {
    data: VaultDataWrapper,
}

#[derive(Debug, Deserialize)]
struct VaultDataWrapper {
    data: VaultSecretData,
}

#[derive(Debug, Deserialize, Serialize)]
struct VaultSecretData {
    value: String,
}

/// Vault KV v2 write request
#[derive(Debug, Serialize)]
struct VaultWriteRequest {
    data: VaultSecretData,
}

/// Vault list response
#[derive(Debug, Deserialize)]
struct VaultListResponse {
    data: VaultListData,
}

#[derive(Debug, Deserialize)]
struct VaultListData {
    keys: Vec<String>,
}

#[async_trait]
impl SecretBackend for VaultBackend {
    async fn get_secret(&self, workspace_id: &str, path: &str) -> Result<String> {
        let url = self.secret_path(workspace_id, path);
        let headers = self.get_headers()?;

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Vault request failed: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound(format!(
                "Secret {} not found in Vault for workspace {}",
                path, workspace_id
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::internal_err(format!(
                "Vault returned error {}: {}",
                status, body
            )));
        }

        let vault_response: VaultReadResponse = response
            .json()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse Vault response: {}", e)))?;

        Ok(vault_response.data.data.value)
    }

    async fn set_secret(&self, workspace_id: &str, path: &str, value: &str) -> Result<()> {
        let url = self.secret_path(workspace_id, path);
        let headers = self.get_headers()?;

        let request_body = VaultWriteRequest {
            data: VaultSecretData {
                value: value.to_string(),
            },
        };

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Vault request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::internal_err(format!(
                "Vault returned error {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    async fn delete_secret(&self, workspace_id: &str, path: &str) -> Result<()> {
        let url = self.delete_path(workspace_id, path);
        let headers = self.get_headers()?;

        let response = self
            .client
            .delete(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Vault request failed: {}", e)))?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::internal_err(format!(
                "Vault returned error {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "hashicorp_vault"
    }
}

/// Create the appropriate secret backend based on configuration
pub async fn create_secret_backend(
    db: DB,
    config: &SecretBackendConfig,
) -> Result<Arc<dyn SecretBackend>> {
    match config {
        SecretBackendConfig::Database => Ok(Arc::new(DatabaseBackend::new(db))),
        SecretBackendConfig::HashiCorpVault(settings) => {
            Ok(Arc::new(VaultBackend::new(settings.clone())))
        }
    }
}

/// Test connection to Vault
pub async fn test_vault_connection(settings: &VaultSettings) -> Result<()> {
    let backend = VaultBackend::new(settings.clone());

    // Try to access the sys/health endpoint to verify connectivity
    let url = format!(
        "{}/v1/sys/health",
        settings.address.trim_end_matches('/')
    );

    let mut headers = HeaderMap::new();
    if let Some(ref token) = settings.token {
        headers.insert(
            "X-Vault-Token",
            HeaderValue::from_str(token)
                .map_err(|e| Error::internal_err(format!("Invalid Vault token: {}", e)))?,
        );
    }
    if let Some(ref namespace) = settings.namespace {
        headers.insert(
            "X-Vault-Namespace",
            HeaderValue::from_str(namespace)
                .map_err(|e| Error::internal_err(format!("Invalid namespace: {}", e)))?,
        );
    }

    let response = backend
        .client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to connect to Vault: {}", e)))?;

    // Vault health endpoint returns 200 for healthy, 429/472/473/501/503 for various states
    // Any response means Vault is reachable
    if response.status().is_server_error() {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::internal_err(format!(
            "Vault health check failed: {}",
            body
        )));
    }

    // Try to verify we can access the mount point
    let mount_url = format!(
        "{}/v1/sys/mounts/{}/tune",
        settings.address.trim_end_matches('/'),
        settings.mount_path
    );

    let mut mount_headers = HeaderMap::new();
    if let Some(ref token) = settings.token {
        mount_headers.insert(
            "X-Vault-Token",
            HeaderValue::from_str(token)
                .map_err(|e| Error::internal_err(format!("Invalid Vault token: {}", e)))?,
        );
    }
    if let Some(ref namespace) = settings.namespace {
        mount_headers.insert(
            "X-Vault-Namespace",
            HeaderValue::from_str(namespace)
                .map_err(|e| Error::internal_err(format!("Invalid namespace: {}", e)))?,
        );
    }

    let mount_response = backend
        .client
        .get(&mount_url)
        .headers(mount_headers)
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to verify mount point: {}", e)))?;

    if !mount_response.status().is_success() {
        return Err(Error::internal_err(format!(
            "Mount point '{}' not accessible or not found",
            settings.mount_path
        )));
    }

    Ok(())
}

/// Migrate secrets from database to Vault
pub async fn migrate_secrets_to_vault(
    db: &DB,
    settings: &VaultSettings,
) -> Result<SecretMigrationReport> {
    let vault = VaultBackend::new(settings.clone());

    // Get all secret variables from the database
    let secrets = sqlx::query!(
        "SELECT workspace_id, path, value FROM variable WHERE is_secret = true AND value IS NOT NULL AND value != ''"
    )
    .fetch_all(db)
    .await?;

    let total_secrets = secrets.len();
    let mut migrated_count = 0;
    let mut failures = Vec::new();

    for secret in secrets {
        let workspace_id = secret.workspace_id;
        let path = secret.path;
        let encrypted_value = secret.value;

        // Decrypt the value from database
        let decrypted = match async {
            let mc = build_crypt(db, &workspace_id).await?;
            decrypt(&mc, encrypted_value)
                .map_err(|e| Error::internal_err(format!("Decryption error: {}", e)))
        }
        .await
        {
            Ok(v) => v,
            Err(e) => {
                failures.push(SecretMigrationFailure {
                    workspace_id: workspace_id.clone(),
                    path: path.clone(),
                    error: format!("Failed to decrypt: {}", e),
                });
                continue;
            }
        };

        // Write to Vault
        match vault.set_secret(&workspace_id, &path, &decrypted).await {
            Ok(()) => {
                migrated_count += 1;
                tracing::info!(
                    "Migrated secret {}/{} to Vault",
                    workspace_id,
                    path
                );
            }
            Err(e) => {
                failures.push(SecretMigrationFailure {
                    workspace_id,
                    path,
                    error: format!("Failed to write to Vault: {}", e),
                });
            }
        }
    }

    Ok(SecretMigrationReport {
        total_secrets,
        migrated_count,
        failed_count: failures.len(),
        failures,
    })
}

/// Migrate secrets from Vault back to database
///
/// This function reads all secrets from the database variable table (where is_secret = true),
/// fetches their values from Vault, encrypts them with the workspace key, and updates
/// the database values. This approach ensures we only migrate secrets that exist in the
/// database (and thus have proper metadata like descriptions and permissions).
pub async fn migrate_secrets_to_database(
    db: &DB,
    settings: &VaultSettings,
) -> Result<SecretMigrationReport> {
    let vault = VaultBackend::new(settings.clone());

    // Get all secret variables from the database
    // This ensures we only migrate secrets that have corresponding DB entries
    let secrets = sqlx::query!(
        "SELECT workspace_id, path FROM variable WHERE is_secret = true"
    )
    .fetch_all(db)
    .await?;

    let total_secrets = secrets.len();
    let mut migrated_count = 0;
    let mut failures = Vec::new();

    for secret in secrets {
        let workspace_id = secret.workspace_id;
        let path = secret.path;

        // Read the plain text value from Vault
        let plain_value = match vault.get_secret(&workspace_id, &path).await {
            Ok(v) => v,
            Err(Error::NotFound(_)) => {
                // Secret exists in DB but not in Vault - skip it (it might not have been migrated yet)
                tracing::warn!(
                    "Secret {}/{} exists in database but not in Vault, skipping",
                    workspace_id,
                    path
                );
                continue;
            }
            Err(e) => {
                failures.push(SecretMigrationFailure {
                    workspace_id: workspace_id.clone(),
                    path: path.clone(),
                    error: format!("Failed to read from Vault: {}", e),
                });
                continue;
            }
        };

        // Encrypt and store in database
        match async {
            let mc = build_crypt(db, &workspace_id).await?;
            let encrypted = encrypt(&mc, &plain_value);

            sqlx::query!(
                "UPDATE variable SET value = $1 WHERE path = $2 AND workspace_id = $3 AND is_secret = true",
                encrypted,
                path,
                workspace_id
            )
            .execute(db)
            .await?;

            Ok::<(), Error>(())
        }
        .await
        {
            Ok(()) => {
                migrated_count += 1;
                tracing::info!(
                    "Migrated secret {}/{} to database",
                    workspace_id,
                    path
                );
            }
            Err(e) => {
                failures.push(SecretMigrationFailure {
                    workspace_id: workspace_id.clone(),
                    path,
                    error: format!("Failed to write to database: {}", e),
                });
            }
        }
    }

    Ok(SecretMigrationReport {
        total_secrets,
        migrated_count,
        failed_count: failures.len(),
        failures,
    })
}

/// Generate a JWT for Vault authentication (placeholder for EE)
pub async fn generate_vault_jwt(
    _db: &DB,
    _workspace_id: &str,
    _email: &str,
    _vault_address: &str,
) -> Result<String> {
    Err(Error::internal_err(
        "JWT generation for Vault requires Enterprise Edition".to_string(),
    ))
}
