/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::oidc_ee;

use axum::Router;

pub fn global_service() -> Router {
    #[cfg(feature = "private")]
    {
        return oidc_ee::global_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return oidc_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}
