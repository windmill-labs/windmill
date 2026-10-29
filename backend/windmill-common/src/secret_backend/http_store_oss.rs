/*
 * Author: Windmill Labs, Inc
 * Copyright (C) Windmill Labs, Inc - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 */

use async_trait::async_trait;

use crate::db::DB;
use crate::error::{Error, Result};

use super::{HttpStoreSettings, SecretBackend, SecretMigrationReport};

const EE_REQUIRED: &str = "The HTTP secret store backend requires Enterprise Edition";

pub struct HttpStoreBackend;

impl HttpStoreBackend {
    pub async fn new(_settings: HttpStoreSettings) -> Result<Self> {
        Ok(HttpStoreBackend)
    }
}

#[async_trait]
impl SecretBackend for HttpStoreBackend {
    async fn get_secret(&self, _workspace_id: &str, _path: &str) -> Result<String> {
        Err(Error::internal_err(EE_REQUIRED.to_string()))
    }

    async fn set_secret(&self, _workspace_id: &str, _path: &str, _value: &str) -> Result<()> {
        Err(Error::internal_err(EE_REQUIRED.to_string()))
    }

    async fn delete_secret(&self, _workspace_id: &str, _path: &str) -> Result<()> {
        Err(Error::internal_err(EE_REQUIRED.to_string()))
    }

    fn backend_name(&self) -> &'static str {
        "http_store"
    }
}

pub async fn test_http_store_connection(
    _settings: &HttpStoreSettings,
    _workspace_id: &str,
    _path: &str,
) -> Result<()> {
    Err(Error::internal_err(EE_REQUIRED.to_string()))
}

pub async fn migrate_secrets_from_http_store(
    _db: &DB,
    _settings: &HttpStoreSettings,
) -> Result<SecretMigrationReport> {
    Err(Error::internal_err(EE_REQUIRED.to_string()))
}
