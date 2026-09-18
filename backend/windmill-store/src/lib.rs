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

#[cfg(all(test, feature = "oauth2", feature = "private", feature = "enterprise"))]
mod oauth_refresh_secret_backend_tests;

use windmill_common::{db::Authable, error::Error};

/// Denial message for an item the caller can see the existence of but not read. It must never
/// describe the grants themselves: `extra_perms` says which users and groups hold read/write,
/// which the folder ACL model restricts to folder owners, and the caller reaching here is by
/// definition not one. The acting identity is named because a job started on behalf of another
/// user is authorized as that user, so a denial that omits it reads as a grant bug.
pub(crate) fn perm_denied_error(
    kind: &str,
    path: &str,
    authed: Option<&(impl Authable + Sync)>,
) -> Error {
    let requester = match authed {
        Some(authed) => format!("{} ({})", authed.username(), authed.email()),
        None => "an unauthenticated caller".to_string(),
    };
    let folder = path
        .strip_prefix("f/")
        .and_then(|rest| rest.split('/').next())
        .filter(|folder| !folder.is_empty());
    match folder {
        Some(folder) => Error::NotAuthorized(format!(
            "{kind} {path} exists but you don't have access to it, requested as {requester}. \
             Ask an owner of folder {folder} to grant you access."
        )),
        None => Error::NotAuthorized(format!(
            "{kind} {path} exists but you don't have access to it, requested as {requester}."
        )),
    }
}
