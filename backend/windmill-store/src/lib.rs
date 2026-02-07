/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(all(feature = "oauth2", feature = "private"))]
mod oauth_refresh_ee;
#[cfg(feature = "oauth2")]
pub mod oauth_refresh_oss;
pub mod resources;
pub mod secret_backend_ext;
pub mod var_resource_cache;
pub mod variables;
