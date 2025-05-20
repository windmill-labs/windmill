/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::saml_ee;

use axum::{routing::post, Router};

pub struct ServiceProviderExt(); // This struct remains as is.

pub async fn build_sp_extension() -> anyhow::Result<ServiceProviderExt> {
    #[cfg(feature = "private")]
    {
        return saml_ee::build_sp_extension().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return Ok(ServiceProviderExt());
    }
}

pub fn global_service() -> Router {
    #[cfg(feature = "private")]
    {
        // Assuming the ee version also configures the acs route internally or returns a configured Router
        return saml_ee::global_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new().route("/acs", post(acs))
    }
}

pub async fn acs() -> String {
    #[cfg(feature = "private")]
    {
        return saml_ee::acs().await;
    }
    #[cfg(not(feature = "private"))]
    {
        // Implementation is not open source as it is a Windmill Enterprise Edition feature
        "SAML available only in enterprise version".to_string()
    }
}
