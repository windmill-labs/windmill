/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#![allow(non_snake_case)]

#[cfg(not(feature = "private"))]
use axum::{routing::post, Router};

#[cfg(not(feature = "private"))]
pub struct ServiceProviderExt();

#[cfg(not(feature = "private"))]
pub async fn build_sp_extension() -> anyhow::Result<ServiceProviderExt> {
    return Ok(ServiceProviderExt());
}

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new().route("/acs", post(acs))
}

#[cfg(not(feature = "private"))]
pub async fn acs() -> String {
    // Implementation is not open source as it is a Windmill Enterprise Edition feature
    "SAML available only in enterprise version".to_string()
}
