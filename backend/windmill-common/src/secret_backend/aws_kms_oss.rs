/*
 * Author: Windmill Labs, Inc
 * Copyright (C) Windmill Labs, Inc - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 */

use async_trait::async_trait;

use crate::db::DB;
use crate::error::{Error, Result};

use super::{AwsKmsSettings, SecretBackend, SecretMigrationReport};

pub struct AwsKmsBackend;

impl AwsKmsBackend {
    pub fn new(_settings: AwsKmsSettings) -> Self {
        AwsKmsBackend
    }

    pub async fn encrypt_value(&self, _plaintext: &str) -> Result<String> {
        Err(Error::internal_err(
            "AWS KMS integration requires Enterprise Edition".to_string(),
        ))
    }

    pub async fn decrypt_value(&self, _ciphertext_b64: &str) -> Result<String> {
        Err(Error::internal_err(
            "AWS KMS integration requires Enterprise Edition".to_string(),
        ))
    }
}

#[async_trait]
impl SecretBackend for AwsKmsBackend {
    async fn get_secret(&self, _workspace_id: &str, _path: &str) -> Result<String> {
        Err(Error::internal_err(
            "AWS KMS integration requires Enterprise Edition".to_string(),
        ))
    }

    async fn set_secret(&self, _workspace_id: &str, _path: &str, _value: &str) -> Result<()> {
        Err(Error::internal_err(
            "AWS KMS integration requires Enterprise Edition".to_string(),
        ))
    }

    async fn delete_secret(&self, _workspace_id: &str, _path: &str) -> Result<()> {
        // No-op for KMS: secrets are stored in the database, not externally
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "aws_kms"
    }
}

pub async fn test_aws_kms_connection(_settings: &AwsKmsSettings) -> Result<()> {
    Err(Error::internal_err(
        "AWS KMS integration requires Enterprise Edition".to_string(),
    ))
}

pub async fn migrate_secrets_to_aws_kms(
    _db: &DB,
    _settings: &AwsKmsSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(
        "AWS KMS integration requires Enterprise Edition".to_string(),
    ))
}

pub async fn migrate_secrets_from_aws_kms(
    _db: &DB,
    _settings: &AwsKmsSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(
        "AWS KMS integration requires Enterprise Edition".to_string(),
    ))
}

pub async fn decrypt_aws_kms_value(_db: &DB, _value: &str) -> Result<String> {
    Err(Error::internal_err(
        "AWS KMS integration requires Enterprise Edition".to_string(),
    ))
}
