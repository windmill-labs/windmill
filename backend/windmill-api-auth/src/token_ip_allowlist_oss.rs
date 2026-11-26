/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Token IP allowlists are an Enterprise Edition feature. Without it a stored list is ignored
//! (never enforced, never inherited by a minted token) and asking for one is refused. `private`
//! alone is not that edition: community builds carry it.

use windmill_common::error::Error;

pub fn token_ip_allowlists_unavailable() -> Error {
    Error::BadRequest("Token IP allowlists are a Windmill Enterprise Edition feature".to_string())
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub use crate::token_ip_allowlist_ee::{client_ip_allowed, load_allowed_ips, token_ip_allowlist};

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub use ce::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod ce {
    use windmill_common::{client_ip::IpNet, error};

    pub fn load_allowed_ips(_stored: Option<Vec<String>>) -> Option<Vec<IpNet>> {
        None
    }

    pub fn client_ip_allowed(
        _allowed: Option<&[IpNet]>,
        _client_ip: Option<std::net::IpAddr>,
    ) -> bool {
        true
    }

    pub fn token_ip_allowlist(
        _caller: Option<&[IpNet]>,
        requested: Option<&[String]>,
    ) -> error::Result<Option<Vec<String>>> {
        match requested {
            Some(_) => Err(super::token_ip_allowlists_unavailable()),
            None => Ok(None),
        }
    }
}
