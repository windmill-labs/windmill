/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
// This file is `oauth2_ee.rs` and provides the Enterprise Edition implementations
// for oauth2 functionalities, used when the "private" feature is enabled.

use std::{collections::HashMap, fmt::Debug};

use axum::{routing::get, Json, Router};
use hmac::Mac;

#[cfg(feature = "oauth2")]
use itertools::Itertools;
#[cfg(feature = "oauth2")]
use oauth2::{Client as OClient, *};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
#[cfg(feature = "oauth2")]
use windmill_common::more_serde::maybe_number_opt;

#[cfg(feature = "oauth2")]
use crate::OAUTH_CLIENTS;
use windmill_common::error;
use windmill_common::oauth2::*;

use crate::db::DB;
use std::str;

pub fn global_service() -> Router {
    Router::new()
        .route("/list_logins", get(list_logins)) // list_logins itself will be conditional
        .route("/list_connects", get(list_connects)) // list_connects itself will be conditional
}

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return oauth2_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

#[cfg(feature = "oauth2")]
#[derive(Debug, Clone)]
pub struct ClientWithScopes {
    _client: OClient,
    _scopes: Vec<String>,
    _extra_params: Option<HashMap<String, String>>,
    _extra_params_callback: Option<HashMap<String, String>>,
    _allowed_domains: Option<Vec<String>>,
    _userinfo_url: Option<String>,
}
#[cfg(feature = "oauth2")]
pub type BasicClientsMap = HashMap<String, ClientWithScopes>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    auth_url: String,
    token_url: String,
    userinfo_url: Option<String>,
    scopes: Option<Vec<String>>,
    extra_params: Option<HashMap<String, String>>,
    extra_params_callback: Option<HashMap<String, String>>,
    req_body_auth: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthClient {
    id: String,
    secret: String,
    allowed_domains: Option<Vec<String>>,
    connect_config: Option<OAuthConfig>,
    login_config: Option<OAuthConfig>,
}

#[cfg(feature = "oauth2")]
#[derive(Debug)]
pub struct AllClients {
    pub logins: BasicClientsMap,
    pub connects: BasicClientsMap,
    pub slack: Option<OClient>,
}

#[cfg(feature = "oauth2")]
pub async fn build_oauth_clients(
    base_url: &str,
    oauths_from_config: Option<HashMap<String, OAuthClient>>,
    _base_url: &str,
    _oauths_from_config: Option<HashMap<String, OAuthClient>>,
    _db: &DB,
) -> anyhow::Result<AllClients> {
    // TODO: Implement the actual Enterprise Edition logic for build_oauth_clients.
    // This function is called from `oauth2_oss.rs` when the "private" feature is enabled.
    panic!("oauth2_ee::build_oauth_clients (Enterprise Edition) not implemented.");
}

#[cfg(feature = "oauth2")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    access_token: AccessToken,
    #[serde(deserialize_with = "maybe_number_opt")]
    #[serde(default)]
    expires_in: Option<u64>,
    refresh_token: Option<RefreshToken>,
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(default)]
    scope: Option<Vec<Scope>>,
}

#[derive(Serialize)]
struct Logins {
    oauth: Vec<String>,
    saml: Option<String>,
}
async fn list_logins() -> error::JsonResult<Logins> {
    // TODO: Implement the actual Enterprise Edition logic for list_logins.
    panic!("oauth2_ee::list_logins (Enterprise Edition) not implemented.");
}

#[cfg(feature = "oauth2")]
// This is the EE version of list_connects when feature "oauth2" is enabled.
// It's called as `list_connects_oauth2` from oauth2_oss.rs.
async fn list_connects_oauth2() -> error::JsonResult<Vec<String>> {
    // TODO: Implement the actual Enterprise Edition logic for list_connects_oauth2.
    panic!("oauth2_ee::list_connects_oauth2 (Enterprise Edition) not implemented.");
}

#[cfg(not(feature = "oauth2"))]
// This is the EE version of list_connects when feature "oauth2" is NOT enabled.
// It's called as `list_connects_no_oauth2` from oauth2_oss.rs.
async fn list_connects_no_oauth2() -> error::JsonResult<Vec<String>> {
    // TODO: Implement the actual Enterprise Edition logic for list_connects_no_oauth2.
    panic!("oauth2_ee::list_connects_no_oauth2 (Enterprise Edition) not implemented.");
}

pub async fn _refresh_token<'c>(
    tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    _tx: Transaction<'c, Postgres>,
    _path: &str,
    _w_id: &str,
    _id: i32,
    _db: &DB,
) -> error::Result<String> {
    // TODO: Implement the actual Enterprise Edition logic for _refresh_token.
    panic!("oauth2_ee::_refresh_token (Enterprise Edition) not implemented.");
}

pub async fn check_nb_of_user(_db: &DB) -> error::Result<()> {
    // TODO: Implement the actual Enterprise Edition logic for check_nb_of_user.
    // This might involve different user limits or licensing checks.
    // For example, EE version might bypass these checks or have different limits.
    Ok(()) // Placeholder: Assume EE version has different logic or no limits here.
}

#[derive(Clone, Debug)]
pub struct SlackVerifier {
    _mac: HmacSha256,
}

impl SlackVerifier {
    pub fn new<S: AsRef<[u8]>>(secret: S) -> anyhow::Result<SlackVerifier> {
        HmacSha256::new_from_slice(secret.as_ref())
            .map(|mac| SlackVerifier { _mac: mac })
            .map_err(|_| anyhow::anyhow!("invalid secret"))
    }
}
