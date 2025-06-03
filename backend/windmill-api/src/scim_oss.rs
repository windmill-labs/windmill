#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::scim_ee::*;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use axum::{middleware::Next, response::Response, routing::get, Router};
#[cfg(not(feature = "private"))]
use hyper::Request;

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new().route("/ee", get(ee))
}

#[cfg(not(feature = "private"))]
pub async fn ee() -> String {
    return "Enterprise Edition".to_string();
}

#[cfg(not(feature = "private"))]
pub async fn has_scim_token<B>(_request: Request<B>, _next: Next) -> Response {
    //Not implemented in open-source version
    todo!()
}
