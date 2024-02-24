/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::Router;

pub fn global_service() -> Router {
    Router::new()
}

pub fn workspaced_service() -> Router {
    Router::new()
}
