/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::oauth2_ee;

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
    db: &DB,
) -> anyhow::Result<AllClients> {
    #[cfg(feature = "private")]
    {
        return oauth2_ee::build_oauth_clients(base_url, oauths_from_config, db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (base_url, oauths_from_config, db);
        // Implementation is not open source
        return Ok(AllClients {
            logins: HashMap::default(),
            connects: HashMap::default(),
            slack: None,
        });
    }
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
    #[cfg(feature = "private")]
    {
        return oauth2_ee::list_logins().await;
    }
    #[cfg(not(feature = "private"))]
    {
        // Implementation is not open source
        return Ok(Json(Logins { oauth: vec![], saml: None }));
    }
}

#[cfg(feature = "oauth2")]
async fn list_connects() -> error::JsonResult<Vec<String>> {
    #[cfg(feature = "private")]
    {
        // This function is already under #[cfg(feature = "oauth2")]
        return oauth2_ee::list_connects_oauth2().await; // Renamed to avoid conflict if ee has a plain list_connects
    }
    #[cfg(not(feature = "private"))]
    {
        Ok(Json(
            (&OAUTH_CLIENTS.read().await.connects)
                .keys()
                .map(|x| x.to_owned())
                .collect_vec(),
        ))
    }
}

#[cfg(not(feature = "oauth2"))]
async fn list_connects() -> error::JsonResult<Vec<String>> {
    #[cfg(feature = "private")]
    {
         // This function is already under #[cfg(not(feature = "oauth2"))]
        return oauth2_ee::list_connects_no_oauth2().await; // Renamed for clarity
    }
    #[cfg(not(feature = "private"))]
    {
        // Implementation is not open source
        return Ok(Json(vec![]));
    }
}

pub async fn _refresh_token<'c>(
    tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
) -> error::Result<String> {
    #[cfg(feature = "private")]
    {
        return oauth2_ee::_refresh_token(tx, path, w_id, id, db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (tx, path, w_id, id, db);
        // Implementation is not open source
        Err(error::Error::BadRequest(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

pub async fn check_nb_of_user(db: &DB) -> error::Result<()> {
    #[cfg(feature = "private")]
    {
        // Assuming EE might have different logic or bypass this check
        return oauth2_ee::check_nb_of_user(db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let nb_users_sso =
            sqlx::query_scalar!("SELECT COUNT(*) FROM password WHERE login_type != 'password'",)
            .fetch_one(db)
            .await?;
    if nb_users_sso.unwrap_or(0) >= 10 {
        return Err(error::Error::BadRequest(
            "You have reached the maximum number of oauth users accounts (10) without an enterprise license"
                .to_string(),
        ));
    }

    let nb_users = sqlx::query_scalar!("SELECT COUNT(*) FROM password",)
        .fetch_one(db)
        .await?;
    if nb_users.unwrap_or(0) >= 50 {
        return Err(error::Error::BadRequest(
            "You have reached the maximum number of accounts (50) without an enterprise license"
                .to_string(),
        ));
    }
    return Ok(());
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
