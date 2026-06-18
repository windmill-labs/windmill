/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Database backend for secret storage
//!
//! This is the default backend that stores secrets encrypted in the PostgreSQL database
//! using the existing magic_crypt encryption with workspace-specific keys.

use async_trait::async_trait;

use crate::db::DB;
use crate::error::{Error, Result};
use crate::variables::{build_crypt, decrypt, encrypt};

use super::SecretBackend;

/// Database-backed secret storage
///
/// This backend stores secrets encrypted in the `variable` table using
/// the workspace's encryption key. This is the default and original
/// behavior of Windmill.
pub struct DatabaseBackend {
    db: DB,
}

impl DatabaseBackend {
    /// Create a new database backend
    pub fn new(db: DB) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SecretBackend for DatabaseBackend {
    async fn get_secret(&self, workspace_id: &str, path: &str) -> Result<String> {
        let variable = sqlx::query!(
            "SELECT value FROM variable WHERE path = $1 AND workspace_id = $2 AND is_secret = true",
            path,
            workspace_id
        )
        .fetch_optional(&self.db)
        .await?;

        let variable = variable.ok_or_else(|| {
            Error::NotFound(format!(
                "Secret variable {} not found in workspace {}",
                path, workspace_id
            ))
        })?;

        let value = variable.value;
        if value.is_empty() {
            return Ok(String::new());
        }

        let mc = build_crypt(&self.db, workspace_id).await?;
        decrypt(&mc, value)
            .map_err(|e| Error::internal_err(format!("Error decrypting variable {}: {}", path, e)))
    }

    async fn set_secret(&self, workspace_id: &str, path: &str, value: &str) -> Result<()> {
        let mc = build_crypt(&self.db, workspace_id).await?;
        let encrypted_value = encrypt(&mc, value);

        // Update the value in the database
        // Note: This assumes the variable row already exists (created via the normal API)
        let result = sqlx::query!(
            "UPDATE variable SET value = $1 WHERE path = $2 AND workspace_id = $3 AND is_secret = true",
            encrypted_value,
            path,
            workspace_id
        )
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Secret variable {} not found in workspace {}",
                path, workspace_id
            )));
        }

        Ok(())
    }

    async fn delete_secret(&self, _workspace_id: &str, _path: &str) -> Result<()> {
        // For database backend, deletion is handled by the normal variable deletion flow
        // The encrypted value is just deleted along with the row
        // This method is a no-op for database backend since the caller handles the DELETE
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "database"
    }
}

/// Encrypt a value for storage in the database
///
/// This is a convenience function for use when creating new secrets.
pub async fn encrypt_for_database(db: &DB, workspace_id: &str, value: &str) -> Result<String> {
    let mc = build_crypt(db, workspace_id).await?;
    Ok(encrypt(&mc, value))
}

/// Decrypt a value from the database
///
/// This is a convenience function for reading secrets.
pub async fn decrypt_from_database(
    db: &DB,
    workspace_id: &str,
    encrypted_value: String,
) -> Result<String> {
    if encrypted_value.is_empty() {
        return Ok(String::new());
    }
    let mc = build_crypt(db, workspace_id).await?;
    decrypt(&mc, encrypted_value)
        .map_err(|e| Error::internal_err(format!("Error decrypting value: {}", e)))
}
