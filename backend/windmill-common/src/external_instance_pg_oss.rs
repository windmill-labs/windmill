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
    setup_external_instance_pg_unchecked, validate_external_instance_pg_setting,
};

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) use ce::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod ce {
    use super::external_instance_pg_unavailable as unavailable;
    use crate::{error::Result, external_instance_pg::ExternalInstancePgSetupReport, DB};

    pub(crate) fn validate_external_instance_pg_setting(_value: &serde_json::Value) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn setup_external_instance_pg_unchecked(
        _db: &DB,
        _rotate_passwords: bool,
    ) -> Result<ExternalInstancePgSetupReport> {
        Err(unavailable())
    }
}
