/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{middleware::Next, response::Response, Router};
use http::Request;

pub fn global_service() -> Router {
    crate::scim_ee::global_service()
}

pub async fn ee() -> String {
    crate::scim_ee::ee().await
}

pub async fn has_scim_token<B>(request: Request<B>, next: Next) -> Response {
    crate::scim_ee::has_scim_token(request, next).await
}