#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::oauth2_ee::*;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use std::{collections::HashMap, fmt::Debug};

#[cfg(not(feature = "private"))]
use axum::{routing::get, Json, Router};
#[cfg(not(feature = "private"))]
use hmac::Mac;

#[cfg(all(feature = "oauth2", not(feature = "private")))]
use itertools::Itertools;
#[cfg(not(feature = "private"))]
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "private"))]
use sqlx::{Postgres, Transaction};
#[cfg(all(feature = "oauth2", not(feature = "private")))]
use windmill_common::more_serde::maybe_number_opt;
#[cfg(all(feature = "oauth2", not(feature = "private")))]
use windmill_oauth::{helpers, AccessToken, RefreshToken, Scope};

#[cfg(all(feature = "oauth2", not(feature = "private")))]
use crate::OAUTH_CLIENTS;
#[cfg(not(feature = "private"))]
use windmill_common::error;
#[cfg(not(feature = "private"))]
use windmill_common::oauth2::*;

#[cfg(not(feature = "private"))]
use crate::db::DB;
#[cfg(not(feature = "private"))]
use std::str;

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new()
        .route("/list_logins", get(list_logins))
        .route("/list_connects", get(list_connects))
}

#[cfg(not(feature = "private"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(all(feature = "oauth2", not(feature = "private")))]
pub use windmill_oauth::{AllClients, BasicClientsMap, ClientWithScopes};

#[cfg(not(feature = "private"))]
pub use windmill_oauth::{OAuthClient, OAuthConfig};

#[cfg(all(feature = "oauth2", not(feature = "private")))]
pub async fn build_oauth_clients(
    _base_url: &str,
    _oauths_from_config: Option<HashMap<String, OAuthClient>>,
    _db: &DB,
) -> anyhow::Result<AllClients> {
    // Implementation is not open source
    return Ok(AllClients {
        logins: HashMap::default(),
        connects: HashMap::default(),
        slack: None,
    });
}

#[cfg(all(feature = "oauth2", not(feature = "private")))]
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

#[cfg(not(feature = "private"))]
#[derive(Serialize)]
struct Logins {
    oauth: Vec<String>,
    saml: Option<String>,
}
#[cfg(not(feature = "private"))]
async fn list_logins() -> error::JsonResult<Logins> {
    // Implementation is not open source
    return Ok(Json(Logins { oauth: vec![], saml: None }));
}

#[allow(unused)]
#[cfg(all(feature = "oauth2", not(feature = "private")))]
async fn list_connects() -> error::JsonResult<Vec<String>> {
    Ok(Json(
        (&OAUTH_CLIENTS.read().await.connects)
            .keys()
            .map(|x| x.to_owned())
            .collect_vec(),
    ))
}

#[allow(unused)]
#[cfg(not(all(feature = "oauth2", not(feature = "private"))))]
async fn list_connects() -> windmill_common::error::JsonResult<Vec<String>> {
    // Implementation is not open source
    return Ok(axum::Json(vec![]));
}

#[cfg(not(feature = "private"))]
pub async fn _refresh_token<'c>(
    _tx: Transaction<'c, Postgres>,
    _path: &str,
    _w_id: &str,
    _id: i32,
    _db: &DB,
) -> error::Result<String> {
    // Implementation is not open source
    Err(error::Error::BadRequest(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub async fn check_nb_of_user(db: &DB) -> error::Result<()> {
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
#[cfg(not(feature = "private"))]
pub struct SlackVerifier {
    _mac: HmacSha256,
}
#[cfg(not(feature = "private"))]
impl SlackVerifier {
    pub fn new<S: AsRef<[u8]>>(secret: S) -> anyhow::Result<SlackVerifier> {
        HmacSha256::new_from_slice(secret.as_ref())
            .map(|mac| SlackVerifier { _mac: mac })
            .map_err(|_| anyhow::anyhow!("invalid secret"))
    }
}
