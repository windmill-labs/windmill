#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::oidc_ee::*;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "private"))]
use tokio::sync::RwLock;
#[cfg(all(
    feature = "enterprise",
    feature = "openidconnect",
    not(feature = "private")
))]
use {
    crate::db::DB,
    crate::{
        auth::IdToken as WindmillIdToken,
        error::{Error, Result},
    },
    anyhow,
};

#[cfg(all(feature = "openidconnect", not(feature = "private")))]
use openidconnect::AdditionalClaims;

#[cfg(all(feature = "openidconnect", not(feature = "private")))]
impl AdditionalClaims for JobClaim {}

#[cfg(all(feature = "openidconnect", not(feature = "private")))]
impl AdditionalClaims for WorkspaceClaim {}

#[cfg(all(feature = "openidconnect", not(feature = "private")))]
impl AdditionalClaims for InstanceClaim {}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg(not(feature = "private"))]
pub struct WorkspaceClaim {
    pub workspace: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg(not(feature = "private"))]
pub struct InstanceClaim {}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg(not(feature = "private"))]
pub struct JobClaim {
    pub job_id: String,
    pub path: Option<String>,
    pub flow_path: Option<String>,
    pub groups: Vec<String>,
    pub username: String,
    pub email: String,
    pub workspace: String,
}

#[cfg(not(feature = "private"))]
lazy_static::lazy_static! {
    static ref PRIVATE_KEY: RwLock<Option<String>> = RwLock::new(None);
}

#[cfg(not(feature = "private"))]
pub async fn generate_id_token<T: AdditionalClaims>(
    _db: Option<&DB>,
    _claim: T,
    _audience: &str,
    _identifier: String,
    _email: Option<String>,
) -> Result<WindmillIdToken> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(all(
    feature = "enterprise",
    feature = "openidconnect",
    not(feature = "private")
))]
pub async fn get_private_key(_db: Option<&DB>) -> anyhow::Result<String> {
    Err(anyhow::anyhow!(
        "Not implemented in Windmill's Open Source repository"
    ))
}
