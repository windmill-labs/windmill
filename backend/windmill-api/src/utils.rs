/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{body::Body, response::Response};
use serde::{Deserialize, Deserializer};

pub use windmill_api_auth::{check_scopes, require_devops_role, require_super_admin};

#[cfg(feature = "private")]
pub use windmill_common::usernames::generate_instance_wide_unique_username;
pub use windmill_common::utils::WithStarredInfoQuery;

#[cfg(feature = "enterprise")]
pub use windmill_alerting::{
    acknowledge_all_critical_alerts, acknowledge_critical_alert, get_critical_alerts,
    AlertQueryParams,
};

pub fn content_plain(body: Body) -> Response {
    use axum::http::header;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain")
        .body(body)
        .unwrap()
}

#[allow(unused)]
pub fn non_empty_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let o: Option<String> = Option::deserialize(deserializer)?;
    Ok(o.filter(|s| !s.trim().is_empty()))
}

#[cfg(feature = "http_trigger")]
pub use windmill_common::utils::ExpiringCacheEntry;

lazy_static::lazy_static! {
    static ref DUCKLAKE_INSTANCE_PG_PASSWORD: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);
}
