/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new()
}

#[cfg(not(feature = "private"))]
pub fn workspaced_service() -> Router {
    Router::new()
}
