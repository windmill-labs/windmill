/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "oauth2")]
pub mod oauth_refresh_oss;
#[cfg(feature = "private")]
mod oauth_refresh_ee;
pub mod resources;
pub mod secret_backend_ext;
pub mod var_resource_cache;
pub mod variables;
