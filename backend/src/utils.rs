/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Response;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};

use crate::error::{to_anyhow, Error, Result};

pub const MAX_PER_PAGE: usize = 1000;
pub const DEFAULT_PER_PAGE: usize = 100;

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}
#[derive(Deserialize)]
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

pub async fn require_super_admin<'c>(
    db: &mut Transaction<'c, Postgres>,
    email: Option<String>,
) -> Result<()> {
    let is_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        email.as_ref()
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching super admin: {e}")))?;
    if !is_admin {
        Err(Error::NotAuthorized(
            "This endpoint require caller to be a super admin".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub fn require_admin(is_admin: bool, username: &str) -> Result<()> {
    if !is_admin {
        Err(Error::NotAuthorized(format!(
            "This endpoint require caller {} to be an admin",
            username
        )))
    } else {
        Ok(())
    }
}

pub fn rd_string(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
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

pub async fn now_from_db<'c>(
    db: &mut Transaction<'c, Postgres>,
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

pub fn get_owner_from_path(path: &str) -> String {
    path.split('/').take(2).collect::<Vec<_>>().join("/")
}

pub async fn list_elems_from_hub(
    http_client: reqwest::Client,
    url: &str,
    email: Option<String>,
    username: String,
    host: String,
) -> Result<serde_json::Value> {
    let rows = http_get_from_hub(http_client, url, email, username, host, false)
        .await?
        .json::<serde_json::Value>()
        .await
        .map_err(to_anyhow)?;
    Ok(rows)
}

pub async fn http_get_from_hub(
    http_client: reqwest::Client,
    url: &str,
    email: Option<String>,
    username: String,
    host: String,
    plain: bool,
) -> Result<Response> {
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
        .header("X-email", email.unwrap_or_else(|| "".to_string()))
        .header("X-username", username)
        .header("X-hostname", host)
        .send()
        .await
        .map_err(to_anyhow)?;

    Ok(response)
}
