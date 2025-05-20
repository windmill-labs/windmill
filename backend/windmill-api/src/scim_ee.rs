/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::scim_ee;

use axum::{middleware::Next, response::Response, routing::get, Router};
use hyper::Request;

pub fn global_service() -> Router {
    #[cfg(feature = "private")]
    {
        return scim_ee::global_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new().route("/ee", get(ee)) // ee function itself will be conditional
    }
}

pub async fn ee() -> String {
    #[cfg(feature = "private")]
    {
        return scim_ee::ee().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return "Enterprise Edition".to_string();
    }
}

pub async fn has_scim_token<B>(request: Request<B>, next: Next) -> Response {
    #[cfg(feature = "private")]
    {
        return scim_ee::has_scim_token(request, next).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (request, next);
        //Not implemented in open-source version
        todo!()
    }
}
