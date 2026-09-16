/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the external instance cluster comes from: the enterprise implementation, or a refusal.
//! `private` alone is not that edition: community builds carry it.

use crate::error::Error;

pub fn external_instance_pg_unavailable() -> Error {
    Error::BadRequest(
        "External instance databases are a Windmill Enterprise Edition feature".to_string(),
    )
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) use crate::external_instance_pg_ee::{
    create_external_instance_database_unchecked, drop_external_instance_database_unchecked,
    external_instance_connection_unchecked, setup_external_instance_pg_unchecked,
    validate_external_instance_pg_setting,
};

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) fn ensure_external_instance_available() -> crate::error::Result<()> {
    Ok(())
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) use ce::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod ce {
    use super::external_instance_pg_unavailable as unavailable;
    use crate::{
        error::Result, external_instance_pg::ExternalInstancePgSetupReport, PgDatabase, DB,
    };

    pub(crate) fn validate_external_instance_pg_setting(_value: &serde_json::Value) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) fn ensure_external_instance_available() -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn setup_external_instance_pg_unchecked(
        _db: &DB,
        _rotate_passwords: bool,
    ) -> Result<ExternalInstancePgSetupReport> {
        Err(unavailable())
    }

    pub(crate) async fn external_instance_connection_unchecked(
        _db: &DB,
        _dbname: &str,
        _replication: bool,
    ) -> Result<PgDatabase> {
        Err(unavailable())
    }

    pub(crate) async fn create_external_instance_database_unchecked(
        _db: &DB,
        _dbname: &str,
        _tag: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn drop_external_instance_database_unchecked(
        _db: &DB,
        _dbname: &str,
    ) -> Result<()> {
        Err(unavailable())
    }
}
