/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::Router;

pub async fn build_sp_extension() -> anyhow::Result<ServiceProviderExt> {
    crate::saml_ee::build_sp_extension().await
}

pub fn global_service() -> Router {
    crate::saml_ee::global_service()
}

pub async fn acs() -> String {
    crate::saml_ee::acs().await
}

pub use crate::saml_ee::ServiceProviderExt;