/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! OAuth2 client and token management for Windmill.
//!
//! This crate provides OAuth2 functionality including:
//! - OAuth2 client configuration and building
//! - Token exchange and refresh
//! - Slack OAuth integration
//! - Client credentials flow support

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::anyhow;
use base64::Engine;
use hmac::Mac;
use itertools::Itertools;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use tokio::sync::RwLock;
use tower_cookies::{Cookie, Cookies};
use windmill_common::error::{self, to_anyhow, Error};
use windmill_common::more_serde::maybe_number_opt;
use windmill_common::oauth2::*;
use windmill_common::utils::now_from_db;
use windmill_common::variables::{build_crypt, encrypt};
use windmill_common::BASE_URL;

pub type DB = sqlx::Pool<sqlx::Postgres>;

// Re-export oauth2 types that consumers need (also used internally)
pub use oauth2::{
    helpers, AccessToken, AuthType, Client as OClient, RefreshToken, Scope, State, Url,
};

// Re-export reqwest Client (version 0.12 compatible with async-oauth2)
pub use reqwest::Client as HttpClient;

pub use windmill_common::utils::{COOKIE_DOMAIN, IS_SECURE};

lazy_static::lazy_static! {
    /// HTTP client for OAuth operations (reqwest 0.12, compatible with async-oauth2)
    pub static ref OAUTH_HTTP_CLIENT: reqwest::Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/oauth")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create OAuth HTTP client");

    pub static ref OAUTH_CLIENTS: Arc<RwLock<AllClients>> = Arc::new(RwLock::new(AllClients {
        logins: HashMap::new(),
        connects: HashMap::new(),
        slack: None
    }));
}

/// OAuth client with associated scopes and configuration
#[derive(Debug, Clone)]
pub struct ClientWithScopes {
    pub display_name: Option<String>,
    pub client: OClient,
    pub scopes: Vec<String>,
    pub extra_params: Option<HashMap<String, String>>,
    pub extra_params_callback: Option<HashMap<String, String>>,
    pub allowed_domains: Option<Vec<String>>,
    pub userinfo_url: Option<String>,
    pub grant_types: Vec<String>,
}

/// Map of OAuth client names to their configurations
pub type BasicClientsMap = HashMap<String, ClientWithScopes>;

/// OAuth provider configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    #[serde(default = "empty_auth")]
    pub auth_url: String,
    #[serde(default = "empty_string")]
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub extra_params: Option<HashMap<String, String>>,
    pub extra_params_callback: Option<HashMap<String, String>>,
    pub req_body_auth: Option<bool>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
}

/// OAuth client credentials
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthClient {
    #[serde(default = "empty_string")]
    pub id: String,
    #[serde(default = "empty_string")]
    pub secret: String,
    #[serde(default, deserialize_with = "windmill_common::utils::empty_as_none")]
    pub display_name: Option<String>,
    pub allowed_domains: Option<Vec<String>>,
    pub connect_config: Option<OAuthConfig>,
    pub login_config: Option<OAuthConfig>,
    pub tenant: Option<String>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
}

fn empty_string() -> String {
    "".to_string()
}

fn empty_auth() -> String {
    "https://missing-auth-url".to_string()
}

fn default_grant_types() -> Vec<String> {
    vec!["authorization_code".to_string()]
}

/// Container for all OAuth clients (login, connect, and slack)
#[derive(Debug)]
pub struct AllClients {
    pub logins: BasicClientsMap,
    pub connects: BasicClientsMap,
    pub slack: Option<OClient>,
}

/// Slack token response from OAuth flow
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SlackTokenResponse {
    pub access_token: AccessToken,
    pub team_id: String,
    pub team_name: String,
    #[serde(rename = "scope")]
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub scopes: Option<Vec<Scope>>,
    pub bot: SlackBotToken,
}

/// Standard OAuth token response
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: AccessToken,
    #[serde(deserialize_with = "maybe_number_opt")]
    #[serde(default)]
    pub expires_in: Option<u64>,
    pub refresh_token: Option<RefreshToken>,
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(default)]
    pub scope: Option<Vec<Scope>>,
}

/// Slack bot token from OAuth response
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SlackBotToken {
    pub bot_access_token: String,
}

/// OAuth callback parameters
#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

/// Build all OAuth clients from configuration
pub async fn build_oauth_clients(
    base_url: &str,
    oauths_from_config: Option<HashMap<String, OAuthClient>>,
    connect_configs_json: &str,
    login_configs_json: &str,
) -> anyhow::Result<AllClients> {
    let connect_configs =
        serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json)?;
    let login_configs = serde_json::from_str::<HashMap<String, OAuthConfig>>(login_configs_json)?;

    let oauths = if let Some(oauths) = oauths_from_config {
        tracing::info!("Using OAuth clients from config: {oauths:?}");
        oauths
    } else {
        let path = "./oauth.json";
        let content: String = if let Ok(e) = std::env::var("OAUTH_JSON_AS_BASE64") {
            std::str::from_utf8(
                &base64::engine::general_purpose::STANDARD
                    .decode(e)
                    .map_err(to_anyhow)?,
            )?
            .to_string()
        } else if std::path::Path::new(path).exists() {
            std::fs::read_to_string(path).map_err(to_anyhow)?
        } else {
            tracing::warn!("oauth.json not found, no OAuth clients loaded");
            return Ok(AllClients {
                logins: HashMap::new(),
                connects: HashMap::new(),
                slack: None,
            });
        };

        if content.is_empty() {
            tracing::warn!("oauth.json is empty, no OAuth clients loaded");
            return Ok(AllClients {
                logins: HashMap::new(),
                connects: HashMap::new(),
                slack: None,
            });
        };
        match serde_json::from_str::<HashMap<String, OAuthClient>>(&content) {
            Ok(clients) => clients,
            Err(e) => {
                tracing::error!("deserializing oauth.json: {e}");
                HashMap::new()
            }
        }
        .into_iter()
        .collect()
    };

    tracing::info!("OAuth loaded clients: {}", oauths.keys().join(", "));

    let logins = login_configs
        .into_iter()
        .filter_map(|x| oauths.get(&x.0).map(|c| (x.0, (c, x.1))))
        .chain(oauths.iter().filter_map(|x| {
            x.1.login_config
                .as_ref()
                .map(|c| (x.0.clone(), (x.1, c.clone())))
        }))
        .filter_map(|(k, (client_params, config))| {
            let named_client = build_basic_client(
                k.clone(),
                config.clone(),
                client_params.clone(),
                true,
                base_url,
                None,
            );
            named_client
                .map(|named_client| {
                    (
                        named_client.0,
                        ClientWithScopes {
                            client: named_client.1,
                            scopes: config.scopes.unwrap_or(vec![]),
                            extra_params: config.extra_params,
                            extra_params_callback: config.extra_params_callback,
                            allowed_domains: client_params.allowed_domains.clone(),
                            userinfo_url: config.userinfo_url,
                            display_name: client_params.display_name.clone(),
                            grant_types: client_params.grant_types.clone(),
                        },
                    )
                })
                .map_err(|e| {
                    tracing::error!("Error building oauth client {k}: {e}");
                    e
                })
                .ok()
        })
        .collect();

    let connects = connect_configs
        .into_iter()
        .filter_map(|x| oauths.get(&x.0).map(|c| (x.0, (c, x.1))))
        .chain(oauths.iter().filter_map(|x| {
            x.1.connect_config
                .as_ref()
                .map(|c| (x.0.clone(), (x.1, c.clone())))
        }))
        .filter_map(|(k, (client_params, config))| {
            let named_client = build_basic_client(
                k.clone(),
                config.clone(),
                client_params.clone(),
                false,
                base_url,
                if k == "supabase_wizard" {
                    Some(format!("{base_url}/oauth/callback_supabase"))
                } else {
                    None
                },
            );
            named_client
                .map(|named_client| {
                    (
                        named_client.0,
                        ClientWithScopes {
                            client: named_client.1,
                            scopes: config.scopes.unwrap_or(vec![]),
                            extra_params: config.extra_params,
                            extra_params_callback: config.extra_params_callback,
                            allowed_domains: None,
                            userinfo_url: None,
                            display_name: client_params.display_name.clone(),
                            grant_types: client_params.grant_types.clone(),
                        },
                    )
                })
                .map_err(|e| {
                    tracing::error!("Error building oauth client {k}: {e}");
                    e
                })
                .ok()
        })
        .collect();

    let slack = oauths
        .get("slack")
        .map(|v| {
            build_basic_client(
                "slack".to_string(),
                OAuthConfig {
                    auth_url: "https://slack.com/oauth/authorize".to_string(),
                    token_url: "https://slack.com/api/oauth.access".to_string(),
                    userinfo_url: None,
                    scopes: None,
                    extra_params: None,
                    extra_params_callback: None,
                    req_body_auth: None,
                    grant_types: vec!["authorization_code".to_string()],
                },
                v.clone(),
                false,
                base_url,
                Some(format!("{base_url}/oauth/callback_slack")),
            )
            .map(|x| x.1)
            .map_err(|e| {
                tracing::error!("Error building oauth slack client: {e}");
                e
            })
            .ok()
        })
        .flatten();

    let all_clients = AllClients { logins, connects, slack };
    tracing::debug!("Final oauth config: {all_clients:#?}");
    Ok(all_clients)
}

/// Build a basic OAuth client from configuration
pub fn build_basic_client(
    name: String,
    config: OAuthConfig,
    client_params: OAuthClient,
    login: bool,
    base_url: &str,
    override_callback: Option<String>,
) -> error::Result<(String, OClient)> {
    let auth_url = Url::parse(&config.auth_url)
        .map_err(|e| anyhow!("Invalid authorization endpoint URL: {e}"))?;
    let token_url =
        Url::parse(&config.token_url).map_err(|e| anyhow!("Invalid token endpoint URL: {e}"))?;

    let redirect_url = if login {
        format!("{base_url}/user/login_callback/{name}")
    } else if let Some(callback) = override_callback {
        callback
    } else {
        format!("{base_url}/oauth/callback/{name}")
    };

    let mut client = OClient::new(client_params.id, auth_url, token_url);
    if config.req_body_auth.unwrap_or(false) {
        client.set_auth_type(AuthType::RequestBody);
    }
    client.set_client_secret(client_params.secret.clone());
    client.set_redirect_url(
        Url::parse(&redirect_url).map_err(|e| anyhow!("Invalid redirect URL: {e}"))?,
    );

    Ok((name.to_string(), client))
}

/// Build a Slack OAuth client with custom credentials
pub async fn build_slack_client(
    client_id: &str,
    client_secret: &str,
    _workspace_id: &str,
) -> error::Result<OClient> {
    let auth_url = Url::parse("https://slack.com/oauth/authorize")
        .map_err(|e| anyhow!("Invalid Slack authorization URL: {e}"))?;
    let token_url = Url::parse("https://slack.com/api/oauth.access")
        .map_err(|e| anyhow!("Invalid Slack token URL: {e}"))?;

    let base_url = BASE_URL.read().await.clone();
    let redirect_url = format!("{}/oauth/callback_slack", base_url);

    let mut client = OClient::new(client_id.to_string(), auth_url, token_url);
    client.set_client_secret(client_secret.to_string());
    client.set_redirect_url(
        Url::parse(&redirect_url).map_err(|e| anyhow!("Invalid redirect URL: {e}"))?,
    );

    Ok(client)
}

/// Build OAuth client for client credentials flow with resource-level credentials
pub async fn build_client_credentials_oauth_client(
    db: &DB,
    client_name: &str,
    client_id: &str,
    client_secret: &str,
    cc_token_url_override: Option<&str>,
    connect_configs_json: &str,
) -> error::Result<(OClient, OAuthClient)> {
    use windmill_common::global_settings::{load_value_from_global_settings, OAUTH_SETTING};

    let oauths = load_value_from_global_settings(db, OAUTH_SETTING).await?;
    let oauths = oauths.unwrap_or_default();
    let oauth_config = oauths
        .get(client_name)
        .ok_or_else(|| error::Error::BadRequest("OAuth configuration not found".to_string()))?;

    let oauth_client_config: OAuthClient = serde_json::from_value(oauth_config.clone())
        .map_err(|e| error::Error::BadRequest(format!("Invalid OAuth config: {}", e)))?;

    let mut connect_config = if let Some(ref config) = oauth_client_config.connect_config {
        if !config.auth_url.is_empty() && !config.token_url.is_empty() {
            config.clone()
        } else {
            let static_configs =
                serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json)
                    .map_err(|e| {
                        error::Error::InternalErr(format!(
                            "Failed to parse oauth_connect.json: {}",
                            e
                        ))
                    })?;

            static_configs.get(client_name).cloned().ok_or_else(|| {
                error::Error::BadRequest(format!(
                    "OAuth configuration not found for '{}' in either global settings or static config",
                    client_name
                ))
            })?
        }
    } else {
        let static_configs =
            serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json).map_err(
                |e| error::Error::InternalErr(format!("Failed to parse oauth_connect.json: {}", e)),
            )?;

        static_configs.get(client_name).cloned().ok_or_else(|| {
            error::Error::BadRequest(format!(
                "OAuth configuration not found for '{}' in either global settings or static config",
                client_name
            ))
        })?
    };

    if let Some(override_url) = cc_token_url_override {
        connect_config.token_url = override_url.to_string();
    }

    let resource_oauth_client = OAuthClient {
        id: client_id.to_string(),
        secret: client_secret.to_string(),
        allowed_domains: oauth_client_config.allowed_domains.clone(),
        connect_config: Some(connect_config.clone()),
        login_config: oauth_client_config.login_config.clone(),
        display_name: oauth_client_config.display_name.clone(),
        grant_types: oauth_client_config.grant_types.clone(),
        tenant: oauth_client_config.tenant.clone(),
    };

    let base_url = BASE_URL.read().await.clone();
    let (_, client) = build_basic_client(
        client_name.to_string(),
        connect_config,
        resource_oauth_client,
        false,
        &base_url,
        None,
    )?;

    Ok((client, oauth_client_config))
}

/// Exchange authorization code for tokens
pub async fn exchange_code<T: DeserializeOwned>(
    callback: OAuthCallback,
    cookies: &Cookies,
    client: OClient,
    extra_params: Option<HashMap<String, String>>,
    http_client: &reqwest::Client,
) -> error::Result<T> {
    let name = if COOKIE_DOMAIN.is_some() {
        "csrf_domain"
    } else {
        "csrf"
    };
    let csrf_state = cookies
        .get(name)
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());
    if callback.state != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let mut token_url = client.exchange_code(callback.code);

    if let Some(extra_params) = extra_params {
        for (key, value) in extra_params {
            token_url = token_url.param(key, value)
        }
    }

    token_url
        .with_client(http_client)
        .execute::<T>()
        .await
        .map_err(|e| error::Error::InternalErr(format!("{:?}", e)))
}

/// Internal token exchange implementation
pub async fn exchange_token(
    client: OClient,
    refresh_token: &str,
    grant_type: &str,
    oauth_client_info: Option<&ClientWithScopes>,
    http_client: &reqwest::Client,
) -> Result<TokenResponse, Error> {
    let token_json = match grant_type {
        "authorization_code" => client
            .exchange_refresh_token(&RefreshToken::from(refresh_token))
            .with_client(http_client)
            .execute::<serde_json::Value>()
            .await
            .map_err(to_anyhow)?,
        "client_credentials" => {
            let mut token_request = client.exchange_client_credentials();

            if let Some(oauth_info) = oauth_client_info {
                if let Some(extra_params) = oauth_info.extra_params_callback.as_ref() {
                    for (key, value) in extra_params.iter() {
                        token_request = token_request.param(key.clone(), value.clone());
                    }
                }
            }

            token_request
                .with_client(http_client)
                .execute::<serde_json::Value>()
                .await
                .map_err(to_anyhow)?
        }
        "" | _ if grant_type.is_empty() => client
            .exchange_refresh_token(&RefreshToken::from(refresh_token))
            .with_client(http_client)
            .execute::<serde_json::Value>()
            .await
            .map_err(to_anyhow)?,
        _ => {
            return Err(Error::BadRequest(format!(
                "Unsupported grant type: {}",
                grant_type
            )))
        }
    };

    let token = serde_json::from_value::<TokenResponse>(token_json.clone()).map_err(|e| {
        Error::BadConfig(format!(
            "Error deserializing response as a new token: {e}\nresponse:{token_json}"
        ))
    })?;
    Ok(token)
}

/// Pre-fetched account fields needed for token refresh.
pub struct OAuthAccountInfo {
    pub client: String,
    pub refresh_token: String,
    pub grant_type: String,
    pub cc_client_id: Option<String>,
    pub cc_client_secret: Option<String>,
    pub cc_token_url: Option<String>,
}

/// Refresh an OAuth token and update the database.
/// Fetches the account from DB, then delegates to `refresh_token_for_account`.
pub async fn refresh_token<'c>(
    mut tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
    oauth_clients: &AllClients,
    http_client: &reqwest::Client,
    connect_configs_json: &str,
) -> error::Result<String> {
    let account = sqlx::query_as!(
        OAuthAccountInfo,
        "SELECT client, refresh_token, grant_type, cc_client_id, cc_client_secret, cc_token_url FROM account WHERE workspace_id = $1 AND id = $2",
        w_id,
        id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let account = windmill_common::utils::not_found_if_none(account, "Account", &id.to_string())?;

    refresh_token_for_account(tx, path, w_id, id, db, account, oauth_clients, http_client, connect_configs_json).await
}

/// Refresh an OAuth token given pre-fetched account info (no additional SELECT).
pub async fn refresh_token_for_account<'c>(
    mut tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
    account: OAuthAccountInfo,
    oauth_clients: &AllClients,
    http_client: &reqwest::Client,
    connect_configs_json: &str,
) -> error::Result<String> {
    let oauth_client_info = oauth_clients
        .connects
        .get(&account.client)
        .ok_or_else(|| error::Error::BadRequest("invalid client".to_string()))?
        .clone();

    let mut client = if account.grant_type == "client_credentials" {
        match (&account.cc_client_id, &account.cc_client_secret) {
            (Some(client_id), Some(client_secret)) => {
                let (client, _) = build_client_credentials_oauth_client(
                    db,
                    &account.client,
                    client_id,
                    client_secret,
                    account.cc_token_url.as_deref(),
                    connect_configs_json,
                )
                .await?;
                client
            }
            _ => {
                return Err(error::Error::BadRequest(
                    "client_credentials flow requires cc_client_id and cc_client_secret to be stored in account".to_string()
                ));
            }
        }
    } else {
        oauth_client_info.client.to_owned()
    };

    if account.grant_type == "client_credentials" {
        for scope in oauth_client_info.scopes.iter() {
            client.add_scope(scope);
        }
    }

    tracing::info!(
        grant_type = %account.grant_type,
        client = %account.client,
        workspace_id = %w_id,
        account_id = %id,
        "Refreshing OAuth token"
    );

    let token = exchange_token(
        client,
        &account.refresh_token,
        &account.grant_type,
        Some(&oauth_client_info),
        http_client,
    )
    .await;

    if let Err(token_err) = token {
        sqlx::query!(
            "UPDATE account SET refresh_error = $1 WHERE workspace_id = $2 AND id = $3",
            token_err.alt(),
            w_id,
            id,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(error::Error::BadRequest(format!(
            "Error refreshing token: {}",
            token_err.alt()
        )));
    };

    let token = token.unwrap();

    let expires_at = now_from_db(&mut *tx).await?
        + chrono::Duration::try_seconds(
            token
                .expires_in
                .ok_or_else(|| Error::InternalErr("expires_in expected and not found".to_string()))?
                .try_into()
                .unwrap(),
        )
        .unwrap_or_default();
    sqlx::query!(
        "UPDATE account SET refresh_token = $1, expires_at = $2, refresh_error = NULL WHERE workspace_id = $3 AND id = $4",
        token
            .refresh_token
            .map(|x| x.to_string())
            .unwrap_or(account.refresh_token),
        expires_at,
        w_id,
        id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let token_str = token.access_token.to_string();
    let mc = build_crypt(db, w_id).await?;
    let encrypted_token = encrypt(&mc, token_str.as_str());

    sqlx::query!(
        "UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3",
        encrypted_token,
        w_id,
        path
    )
    .execute(db)
    .await?;

    tracing::info!(
        grant_type = %account.grant_type,
        client = %account.client,
        workspace_id = %w_id,
        account_id = %id,
        "OAuth token refreshed successfully"
    );

    Ok(token_str)
}

/// Generate OAuth redirect URL with CSRF protection
pub fn oauth_redirect(
    clients: &HashMap<String, ClientWithScopes>,
    client_name: String,
    cookies: Cookies,
    scopes: Option<Vec<String>>,
    extra_params: Option<HashMap<String, String>>,
    is_secure: bool,
) -> error::Result<axum::response::Redirect> {
    let client_w_scopes = clients
        .get(&client_name)
        .ok_or_else(|| error::Error::BadRequest("client not found".to_string()))?;
    let state = State::new_random();
    let mut client = client_w_scopes.client.clone();
    let scopes_iter = if let Some(scopes) = scopes {
        scopes
    } else {
        client_w_scopes.scopes.clone()
    };

    for scope in scopes_iter.iter() {
        client.add_scope(scope);
    }

    let mut auth_url = client.authorize_url(&state);

    if let Some(extra_params) = extra_params {
        let mut query_string = auth_url.query_pairs_mut();
        for (key, value) in extra_params {
            query_string.append_pair(&key, &value);
        }
    }

    set_csrf_cookie(&state, cookies, is_secure);
    Ok(axum::response::Redirect::to(auth_url.as_str()))
}

/// Set CSRF cookie for OAuth state verification
pub fn set_csrf_cookie(state: &State, cookies: Cookies, is_secure: bool) {
    let csrf = state.to_base64();
    let name = if COOKIE_DOMAIN.is_some() {
        "csrf_domain".to_string()
    } else {
        "csrf".to_string()
    };
    let mut cookie = Cookie::new(name, csrf);
    cookie.set_secure(is_secure);
    cookie.set_same_site(Some(tower_cookies::cookie::SameSite::Lax));
    cookie.set_http_only(true);
    cookie.set_path("/");
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }
    cookies.add(cookie);
}

/// Slack signature verifier for webhook authentication
#[derive(Clone, Debug)]
pub struct SlackVerifier {
    mac: HmacSha256,
}

impl SlackVerifier {
    pub fn new<S: AsRef<[u8]>>(secret: S) -> anyhow::Result<SlackVerifier> {
        HmacSha256::new_from_slice(secret.as_ref())
            .map(|mac| SlackVerifier { mac })
            .map_err(|_| anyhow::anyhow!("invalid secret"))
    }

    pub fn verify(&self, ts: &str, body: &str, exp_sig: &str) -> anyhow::Result<()> {
        let basestring = format!("v0:{}:{}", ts, body);
        let mut mac = self.mac.clone();

        mac.update(basestring.as_bytes());
        let sig = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
        if sig != exp_sig {
            Err(anyhow::anyhow!("signature mismatch"))?;
        }
        Ok(())
    }
}

/// Fetch user info from OAuth provider
pub async fn http_get_user_info<T: DeserializeOwned>(
    http_client: &reqwest::Client,
    url: &str,
    token: &str,
) -> error::Result<T> {
    let res = http_client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(to_anyhow)
        .map_err(|e| error::Error::InternalErr(format!("failed to fetch user info: {}", e)))?;
    if !res.status().is_success() {
        tracing::debug!(
            "The bearer token of the failed oauth user info exchange is: {}",
            token
        );
        return Err(error::Error::BadConfig(format!(
            "The user info endpoint responded with non 200: {}\n{}\n{}",
            res.status(),
            res.headers()
                .iter()
                .map(|x| format!("{}: {}", x.0.as_str(), x.1.to_str().unwrap_or_default()))
                .collect::<Vec<_>>()
                .join("\n"),
            res.text().await.unwrap_or_default(),
        )));
    }
    Ok(res.json::<T>().await.map_err(to_anyhow).map_err(|e| {
        error::Error::InternalErr(format!("failed to decode json from user info: {}", e))
    })?)
}

/// GitHub email info response
#[derive(Deserialize)]
pub struct GHEmailInfo {
    pub email: String,
    pub verified: bool,
    pub primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_verifier() {
        let verifier = SlackVerifier::new("test_secret").unwrap();
        assert!(verifier.verify("123", "body", "wrong_sig").is_err());
    }
}
