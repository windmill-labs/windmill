/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, fmt::Debug};

use axum::body::StreamBody;
use axum::response::IntoResponse;
use axum::{routing::get, Json, Router};
use hmac::Mac;
use hyper::{HeaderMap, StatusCode};

use oauth2::{Client as OClient, *};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use windmill_common::more_serde::maybe_number_opt;

use crate::{HTTP_CLIENT, OAUTH_CLIENTS};
use windmill_common::error::{self, to_anyhow};
use windmill_common::oauth2::*;

use crate::db::DB;
use std::str;

pub fn global_service() -> Router {
    Router::new()
        .route("/list_supabase", get(list_supabase))
        .route("/list_logins", get(list_logins))
        .route("/list_connects", get(list_connects))
}

pub fn workspaced_service() -> Router {
    Router::new()
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum InstanceEvent {
    UserAdded { email: String },
    // UserDeleted { email: String },
    // UserDeletedWorkspace { workspace: String, email: String },
    UserAddedWorkspace { workspace: String, email: String },
    UserInvitedWorkspace { workspace: String, email: String },
    UserJoinedWorkspace { workspace: String, email: String, username: String },
}

#[derive(Debug, Clone)]
pub struct ClientWithScopes {
    _client: OClient,
    scopes: Vec<String>,
    extra_params: Option<HashMap<String, String>>,
    _extra_params_callback: Option<HashMap<String, String>>,
    _allowed_domains: Option<Vec<String>>,
    _userinfo_url: Option<String>,
}

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

#[derive(Debug)]
pub struct AllClients {
    pub logins: BasicClientsMap,
    pub connects: BasicClientsMap,
    pub slack: Option<OClient>,
}

pub fn build_oauth_clients(
    _base_url: &str,
    _oauths_from_config: Option<HashMap<String, OAuthClient>>,
) -> anyhow::Result<AllClients> {
    // Implementation is not open source
    return Ok(AllClients {
        logins: HashMap::default(),
        connects: HashMap::default(),
        slack: None,
    });
}

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
    // Implementation is not open source
    return Ok(Json(Logins { oauth: vec![], saml: None }));
}

#[derive(Serialize)]
struct ScopesAndParams {
    scopes: Vec<String>,
    extra_params: Option<HashMap<String, String>>,
}
async fn list_connects() -> error::JsonResult<HashMap<String, ScopesAndParams>> {
    Ok(Json(
        (&OAUTH_CLIENTS.read().await.connects)
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_owned(),
                    ScopesAndParams {
                        scopes: v.scopes.clone(),
                        extra_params: v.extra_params.clone(),
                    },
                )
            })
            .collect::<HashMap<String, ScopesAndParams>>(),
    ))
}

pub async fn _refresh_token<'c>(
    _tx: Transaction<'c, Postgres>,
    _path: &str,
    _w_id: &str,
    _id: i32,
) -> error::Result<String> {
    // Implementation is not open source
    Err(error::Error::BadRequest(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

async fn list_supabase(headers: HeaderMap) -> impl IntoResponse {
    let token = headers
        .get("X-Supabase-Token")
        .map(|x| x.to_str().unwrap_or(""))
        .unwrap_or("");
    let resp = HTTP_CLIENT
        .get("https://api.supabase.com/v1/projects")
        .bearer_auth(token)
        .send()
        .await
        .map_err(to_anyhow)?;

    let status_code = resp.status();
    let stream = resp.bytes_stream();

    Ok((status_code, StreamBody::new(stream))) as error::Result<(StatusCode, StreamBody<_>)>
}

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
