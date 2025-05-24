/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{middleware::Next, response::Response, routing::get, Router};
use hyper::Request;

pub fn global_service() -> Router {
    Router::new().route("/ee", get(ee))
}

pub async fn ee() -> String {
    return "Enterprise Edition".to_string();
}

pub async fn has_scim_token<B>(_request: Request<B>, _next: Next) -> Response {
    //Not implemented in open-source version
    todo!()
}
