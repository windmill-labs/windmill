/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{middleware::Next, response::Response, Router};
use hyper::Request;

pub fn global_service() -> Router {
    Router::new()
}

pub async fn has_scim_token<B>(request: Request<B>, next: Next<B>) -> Response {
    return next.run(request).await;
}
