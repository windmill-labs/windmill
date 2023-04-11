/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::error::{Error, Result};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_PER_PAGE: usize = 10000;
pub const DEFAULT_PER_PAGE: usize = 1000;

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripPath(pub String);

impl StripPath {
    pub fn to_path(&self) -> &str {
        if self.0.starts_with('/') {
            self.0.strip_prefix('/').unwrap()
        } else {
            &self.0
        }
    }
}

pub fn require_admin(is_admin: bool, username: &str) -> Result<()> {
    if !is_admin {
        Err(Error::RequireAdmin(username.to_string()))
    } else {
        Ok(())
    }
}

pub fn paginate(pagination: Pagination) -> (usize, usize) {
    let per_page = pagination
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .max(1)
        .min(MAX_PER_PAGE);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;
    (per_page, offset)
}

#[cfg(feature = "sqlx")]
pub async fn now_from_db<'c>(
    db: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<chrono::DateTime<chrono::Utc>> {
    Ok(sqlx::query_scalar!("SELECT now()")
        .fetch_one(db)
        .await?
        .unwrap())
}

pub fn not_found_if_none<T, U: AsRef<str>>(opt: Option<T>, kind: &str, name: U) -> Result<T> {
    if let Some(o) = opt {
        Ok(o)
    } else {
        Err(Error::NotFound(format!(
            "{} not found at name {}",
            kind,
            name.as_ref()
        )))
    }
}

#[cfg(feature = "reqwest")]
pub async fn list_elems_from_hub(
    http_client: &reqwest::Client,
    url: &str,
    email: &str,
) -> Result<serde_json::Value> {
    let rows = http_get_from_hub(http_client, url, email, false)
        .await?
        .json::<serde_json::Value>()
        .await
        .map_err(crate::error::to_anyhow)?;
    Ok(rows)
}

#[cfg(feature = "reqwest")]
pub async fn http_get_from_hub(
    http_client: &reqwest::Client,
    url: &str,
    email: &str,
    plain: bool,
) -> Result<reqwest::Response> {
    let response = http_client
        .get(url)
        .header(
            "Accept",
            if plain {
                "text/plain"
            } else {
                "application/json"
            },
        )
        .header("X-email", email)
        .send()
        .await
        .map_err(crate::error::to_anyhow)?;

    Ok(response)
}

pub fn rd_string(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn calculate_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s);
    format!("{:x}", hasher.finalize())
}
