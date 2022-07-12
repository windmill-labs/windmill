use std::collections::HashMap;
use std::fmt::Debug;

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, FromRequest, Path, Query, RequestParts};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum::{async_trait, Json, Router};
use chrono::Duration;
use hyper::StatusCode;
use itertools::Itertools;

use oauth2::{Client as OClient, *};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use slack_http_verifier::SlackVerifier;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tower_cookies::{Cookie, Cookies};

use crate::audit::{audit_log, ActionKind};
use crate::db::{UserDB, DB};
use crate::error::{self, to_anyhow, Result};
use crate::jobs;
use crate::jobs::{get_latest_hash_for_path, JobPayload};
use crate::users::Authed;
use crate::utils::not_found_if_none;
use crate::workspaces::WorkspaceSettings;
use crate::BaseUrl;

pub fn global_service() -> Router {
    Router::new()
        .route("/login/:client", get(login))
        .route("/login_callback/:client", post(login_callback))
        .route("/connect/:client", get(connect))
        .route("/connect_callback/:client", post(connect_callback))
        .route("/connect_slack", get(connect_slack))
        .route("/connect_slack_callback", post(connect_slack_callback))
        .route(
            "/slack_command",
            post(slack_command).route_layer(axum::middleware::from_extractor::<SlackSig>()),
        )
        .route("/list_logins", get(list_logins))
        .route("/list_connects", get(list_connects))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/disconnect/:account_id", post(disconnect))
        .route("/disconnect_slack", post(disconnect_slack))
        .route("/set_workspace_slack", post(set_workspace_slack))
        .route("/set_account/:client_name", post(set_account))
        .route("/delete_account/:id", post(delete_account))
}

pub struct ClientWithScopes {
    client: OClient,
    scopes: Vec<String>,
}

pub type BasicClientsMap = HashMap<String, ClientWithScopes>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    auth_url: String,
    token_url: String,
    scopes: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthClient {
    id: String,
    secret: String,
}
pub struct AllClients {
    pub logins: BasicClientsMap,
    pub connects: BasicClientsMap,
    pub slack: Option<OClient>,
}

pub async fn build_oauth_clients(base_url: &str) -> anyhow::Result<AllClients> {
    let connect_configs = serde_json::from_str::<HashMap<String, OAuthConfig>>(include_str!(
        "../oauth_connect.json"
    ))?;
    let login_configs =
        serde_json::from_str::<HashMap<String, OAuthConfig>>(include_str!("../oauth_login.json"))?;

    let mut content = String::new();
    let path = "./oauth.json";
    if std::path::Path::new(path).exists() {
        let mut file = File::open(path).await?;
        file.read_to_string(&mut content).await?;
    } else {
        content.push_str("{}");
    }

    let oauths: HashMap<String, OAuthClient> =
        match serde_json::from_str::<HashMap<String, OAuthClient>>(&content) {
            Ok(clients) => clients,
            Err(e) => {
                tracing::error!("Error while deserializing oauth.json: {e}");
                HashMap::new()
            }
        }
        .into_iter()
        .collect();

    tracing::info!("OAuth loaded clients: {}", oauths.keys().join(", "));

    let logins = login_configs
        .into_iter()
        .filter(|x| oauths.contains_key(&x.0))
        .map(|(k, v)| {
            let scopes = v.scopes.clone();
            let named_client = build_basic_client(
                k.clone(),
                v,
                oauths.get(&k).unwrap().clone(),
                true,
                base_url,
            );
            (
                named_client.0,
                ClientWithScopes {
                    client: named_client.1,
                    scopes: scopes.unwrap_or(vec![]),
                },
            )
        })
        .collect();

    let connects = connect_configs
        .into_iter()
        .filter(|x| oauths.contains_key(&x.0))
        .map(|(k, v)| {
            let scopes = v.scopes.clone();
            let named_client = build_basic_client(
                k.clone(),
                v,
                oauths.get(&k).unwrap().clone(),
                false,
                base_url,
            );
            (
                named_client.0,
                ClientWithScopes {
                    client: named_client.1,
                    scopes: scopes.unwrap_or(vec![]),
                },
            )
        })
        .collect();

    let slack = oauths.get("slack").map(|v| {
        build_basic_client(
            "slack".to_string(),
            OAuthConfig {
                auth_url: "https://slack.com/oauth/authorize".to_string(),
                token_url: "https://slack.com/api/oauth.access".to_string(),
                scopes: None,
            },
            v.clone(),
            false,
            base_url,
        )
        .1
    });

    Ok(AllClients {
        logins,
        connects,
        slack,
    })
}

pub fn build_basic_client(
    name: String,
    config: OAuthConfig,
    client_params: OAuthClient,
    login: bool,
    base_url: &str,
) -> (String, OClient) {
    let auth_url = Url::parse(&config.auth_url).expect("Invalid authorization endpoint URL");
    let token_url = Url::parse(&config.token_url).expect("Invalid token endpoint URL");

    let redirect_url = if login {
        format!("{base_url}/user/login_callback/{name}")
    } else {
        format!("{base_url}/oauth/callback/{name}")
    };

    let mut client = OClient::new(client_params.id, auth_url, token_url);
    client.set_client_secret(client_params.secret.clone());
    client.set_redirect_url(Url::parse(&redirect_url).expect("Invalid redirect URL"));
    // Set up the config for the Github OAuth2 process.
    (name.to_string(), client)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SlackTokenResponse {
    access_token: AccessToken,
    team_id: String,
    team_name: String,
    #[serde(rename = "scope")]
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    scopes: Option<Vec<Scope>>,
    bot: SlackBotToken,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    access_token: AccessToken,
    expires_in: Option<u64>,
    refresh_token: Option<RefreshToken>,
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(default)]
    scope: Option<Vec<Scope>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SlackBotToken {
    bot_access_token: String,
}

#[derive(Deserialize)]
struct ConnectScopes {
    scopes: Option<String>,
}
async fn connect(
    Path(client_name): Path<String>,
    Query(ConnectScopes { scopes }): Query<ConnectScopes>,
    Extension(clients): Extension<Arc<AllClients>>,
    cookies: Cookies,
) -> error::Result<Redirect> {
    let connects = &clients.connects;
    oauth_redirect(
        connects,
        client_name,
        cookies,
        scopes.map(|x| x.split('+').map(|x| x.to_owned()).collect()),
    )
}

#[derive(Deserialize)]
struct SetAccount {
    refresh_token: String,
    expires_in: i64,
}
async fn set_account(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, client)): Path<(String, String)>,
    Json(payload): Json<SetAccount>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let expires_at = chrono::Utc::now() + Duration::seconds(payload.expires_in);
    let id = sqlx::query_scalar!(
        "INSERT INTO account (workspace_id, client, expires_at, refresh_token) VALUES ($1, $2, $3, $4) RETURNING id",
        w_id,
        client,
        expires_at,
        payload.refresh_token
    )
    .fetch_one(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(id.to_string())
}

async fn delete_account(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Query((w_id, id)): Query<(String, i32)>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let exists = sqlx::query!(
        "DELETE FROM account WHERE workspace_id = $1 AND id = $2 RETURNING id",
        w_id,
        id,
    )
    .fetch_optional(&mut tx)
    .await?;

    let id_str = id.to_string();
    not_found_if_none(exists, "Account", &id_str)?;

    audit_log(
        &mut tx,
        &authed.username,
        "account.delete",
        ActionKind::Delete,
        &w_id,
        Some(&id_str),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Deleted account id {id}"))
}

async fn list_logins(
    Extension(clients): Extension<Arc<AllClients>>,
) -> error::JsonResult<Vec<String>> {
    Ok(Json(
        clients
            .logins
            .keys()
            .map(|x| x.to_owned())
            .collect::<Vec<String>>(),
    ))
}

async fn list_connects(
    Extension(clients): Extension<Arc<AllClients>>,
) -> error::JsonResult<HashMap<String, Vec<String>>> {
    Ok(Json(
        (&clients.connects)
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.scopes.clone()))
            .collect::<HashMap<String, Vec<String>>>(),
    ))
}

async fn connect_slack(
    Extension(clients): Extension<Arc<AllClients>>,
    cookies: Cookies,
) -> error::Result<Redirect> {
    let mut client = clients
        .slack
        .as_ref()
        .ok_or_else(|| error::Error::BadRequest("slack client not setup".to_string()))?
        .to_owned();
    let state = State::new_random();

    client.add_scope("bot");
    client.add_scope("commands");
    let url = client.authorize_url(&state);

    set_cookie(&state, cookies);
    Ok(Redirect::to(url.as_str()))
}

async fn disconnect(
    authed: Authed,
    Path((w_id, id)): Path<(String, i32)>,
    Extension(user_db): Extension<UserDB>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM account WHERE id = $1 AND workspace_id = $2",
        id,
        w_id
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(format!("account {id} disconnected"))
}

async fn disconnect_slack(
    authed: Authed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "UPDATE workspace_settings
            SET slack_team_id = null, slack_name = null WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(format!("slack disconnected"))
}

async fn login(
    Extension(clients): Extension<Arc<AllClients>>,
    Path(client_name): Path<String>,
    cookies: Cookies,
) -> error::Result<Redirect> {
    let clients = &clients.logins;
    oauth_redirect(clients, client_name, cookies, None)
}

#[derive(Deserialize)]
pub struct OAuthCallback {
    code: String,
    state: String,
}

async fn connect_callback(
    cookies: Cookies,
    Path(client_name): Path<String>,
    Json(callback): Json<OAuthCallback>,
    Extension(clients): Extension<Arc<AllClients>>,
    Extension(http_client): Extension<Client>,
) -> error::JsonResult<TokenResponse> {
    let client = (&clients
        .connects
        .get(&client_name)
        .ok_or_else(|| error::Error::BadRequest("invalid client".to_string()))?
        .client)
        .to_owned();

    let token_response =
        exchange_code::<TokenResponse>(callback, &cookies, client, &http_client).await?;

    Ok(Json(token_response))
}

async fn connect_slack_callback(
    cookies: Cookies,
    Json(callback): Json<OAuthCallback>,
    Extension(clients): Extension<Arc<AllClients>>,
    Extension(http_client): Extension<Client>,
) -> error::JsonResult<SlackTokenResponse> {
    let client = clients
        .slack
        .as_ref()
        .ok_or_else(|| error::Error::BadRequest("slack client not setup".to_string()))?
        .to_owned();
    let token =
        exchange_code::<SlackTokenResponse>(callback, &cookies, client, &http_client).await?;

    Ok(Json(token))
}

async fn set_workspace_slack(
    Path(w_id): Path<String>,
    Json(token): Json<SlackTokenResponse>,
    Extension(user_db): Extension<UserDB>,
    authed: Authed,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO workspace_settings
            (workspace_id, slack_team_id, slack_name)
            VALUES ($1, $2, $3) ON CONFLICT (workspace_id) DO UPDATE SET slack_team_id = $2, slack_name = $3",
        &w_id,
        token.team_id,
        token.team_name
    )
    .execute(&mut tx)
    .await?;
    sqlx::query!(
        "INSERT INTO group_
            (workspace_id, name, summary)
            VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        "slack",
        "The group that runs the script triggered by the slack /windmill command.
                     Share scripts to this group to make them executable from slack and add
                     members to this group to let them manage the slack related owner space."
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;
    Ok("slack workspace connected".to_string())
}

#[derive(Deserialize, Debug)]
pub struct SlackCommand {
    team_id: String,
    user_name: String,
    text: String,
    response_url: String,
}

#[derive(Clone, Debug)]
pub struct SlackSig {
    sig: String,
    ts: String,
}

#[async_trait]
impl<B> FromRequest<B> for SlackSig
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
        let hm = req.headers();
        Ok(Self {
            sig: hm
                .get("X-Slack-Signature")
                .map(|x| x.to_str().unwrap_or(""))
                .unwrap_or("")
                .to_string(),
            ts: hm
                .get("X-Slack-Request-Timestamp")
                .map(|x| x.to_str().unwrap_or(""))
                .unwrap_or("")
                .to_string(),
        })
    }
}

async fn slack_command(
    SlackSig { sig, ts }: SlackSig,
    Extension(slack_verifier): Extension<Arc<Option<SlackVerifier>>>,
    Extension(db): Extension<DB>,
    Extension(base_url): Extension<BaseUrl>,
    body: Bytes,
) -> error::Result<String> {
    let form: SlackCommand = serde_urlencoded::from_bytes(&body)
        .map_err(|_| error::Error::BadRequest("invalid payload".to_string()))?;

    let body = String::from_utf8_lossy(&body);
    if slack_verifier
        .as_ref()
        .as_ref()
        .map(|sv| sv.verify(&ts, &body, &sig).ok())
        .flatten()
        .is_none()
    {
        return Err(error::Error::BadRequest("verification failed".to_owned()));
    }

    let mut tx = db.begin().await?;
    let settings = sqlx::query_as!(
        WorkspaceSettings,
        "SELECT * FROM workspace_settings WHERE slack_team_id = $1",
        form.team_id,
    )
    .fetch_optional(&mut tx)
    .await?;

    if let Some(settings) = settings {
        if let Some(script) = &settings.slack_command_script {
            let script_hash =
                get_latest_hash_for_path(&mut tx, &settings.workspace_id, script).await?;
            let mut map = serde_json::Map::new();
            map.insert("text".to_string(), serde_json::Value::String(form.text));
            map.insert(
                "response_url".to_string(),
                serde_json::Value::String(form.response_url),
            );

            let (uuid, tx) = jobs::push(
                tx,
                &settings.workspace_id,
                JobPayload::ScriptHash {
                    hash: script_hash,
                    path: script.to_owned(),
                },
                Some(map),
                &form.user_name,
                "g/slack".to_string(),
                None,
                None,
                None,
                false,
            )
            .await?;
            tx.commit().await?;
            let url = base_url.0;
            return Ok(format!("Job launched. See details at {url}/run/{uuid}"));
        }
    }

    return Ok(format!(
        "workspace not properly configured (did you set the script to trigger in the settings?)"
    ));
}

#[derive(Deserialize)]
pub struct UserInfo {
    name: Option<String>,
    company: Option<String>,
}

async fn login_callback(
    Path(client_name): Path<String>,
    Json(callback): Json<OAuthCallback>,
    cookies: Cookies,
    Extension(clients): Extension<Arc<AllClients>>,
    Extension(db): Extension<DB>,
    Extension(http_client): Extension<Client>,
) -> error::Result<String> {
    let client = (&clients
        .logins
        .get(&client_name)
        .ok_or_else(|| error::Error::BadRequest("invalid client".to_string()))?
        .client)
        .to_owned();
    let token_res = exchange_code::<TokenResponse>(callback, &cookies, client, &http_client).await;

    if let Ok(token) = token_res {
        let token = &token.access_token.to_string();

        let email = get_email(&http_client, &client_name, token).await?;

        let mut tx = db.begin().await?;

        let login: Option<(String, String, bool)> =
            sqlx::query_as("SELECT email, login_type, super_admin FROM password WHERE email = $1")
                .bind(&email)
                .fetch_optional(&mut tx)
                .await?;

        if let Some((email, login_type, super_admin)) = login {
            let login_type = serde_json::json!(login_type);
            if login_type == client_name {
                crate::users::create_session_token(&email, super_admin, &mut tx, cookies).await?;
            } else {
                return Err(error::Error::BadRequest(format!(
                "an user with the email associated to this login exists but with a different login type {login_type}")
            ));
            }
        } else {
            let user = get_user_info(&http_client, &client_name, &token).await?;

            sqlx::query(
                &format!("INSERT INTO password (email, name, company, login_type, verified) VALUES ($1, $2, $3, '{}', true)", &client_name)
            )
            .bind(&email)
            .bind(&user.name)
            .bind(user.company)
            .execute(&mut tx)
            .await?;
            crate::users::create_session_token(&email, false, &mut tx, cookies).await?;
            audit_log(
                &mut tx,
                &email,
                "oauth.signup",
                ActionKind::Create,
                "global",
                Some("github"),
                None,
            )
            .await?;
            let demo_exists =
                sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = 'demo')")
                    .fetch_one(&mut tx)
                    .await?
                    .unwrap_or(false);
            if demo_exists {
                sqlx::query!(
                    "INSERT INTO workspace_invite
            (workspace_id, email, is_admin)
            VALUES ('demo', $1, false)",
                    &email
                )
                .execute(&mut tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok("Successfully logged in".to_string())
    } else {
        Err(error::Error::BadRequest(format!(
            "failed to exchange code: {:?}",
            token_res.err().unwrap()
        )))
    }
}

async fn exchange_code<T: DeserializeOwned>(
    callback: OAuthCallback,
    cookies: &Cookies,
    client: OClient,
    http_client: &Client,
) -> error::Result<T> {
    let csrf_state = cookies
        .get("csrf")
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());
    if callback.state != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }
    client
        .exchange_code(callback.code)
        .with_client(http_client)
        .execute::<T>()
        .await
        .map_err(|e| error::Error::InternalErr(format!("{:?}", e)))
}

#[derive(Deserialize)]
pub struct GHEmailInfo {
    email: String,
    verified: bool,
    primary: bool,
}

#[derive(Deserialize)]
pub struct EmailInfo {
    email: String,
}

async fn get_email(http_client: &Client, client_name: &str, token: &str) -> error::Result<String> {
    let email = match client_name {
        "github" => http_get_user_info::<Vec<GHEmailInfo>>(
            http_client,
            "https://api.github.com/user/emails",
            token,
        )
        .await?
        .iter()
        .find(|x| x.primary && x.verified)
        .ok_or(error::Error::BadRequest(format!(
            "user does not have any primary and verified address"
        )))?
        .email
        .to_string(),
        "gitlab" => {
            http_get_user_info::<EmailInfo>(http_client, "https://gitlab.com/api/v4/user", token)
                .await?
                .email
                .to_string()
        }
        _ => {
            return Err(error::Error::BadRequest(
                "client name not recognized".to_string(),
            ))
        }
    };
    Ok(email)
}

async fn get_user_info(
    http_client: &Client,
    client_name: &str,
    token: &str,
) -> error::Result<UserInfo> {
    let email = match client_name {
        "github" => http_get_user_info(http_client, "https://api.github.com/user", token).await?,
        "gitlab" => {
            http_get_user_info(http_client, "https://gitlab.com/api/v4/user", token).await?
        }
        _ => {
            return Err(error::Error::BadRequest(
                "client name not recognized".to_string(),
            ))
        }
    };
    Ok(email)
}

async fn http_get_user_info<T: DeserializeOwned>(
    http_client: &Client,
    url: &str,
    token: &str,
) -> error::Result<T> {
    Ok(http_client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(to_anyhow)?
        .json::<T>()
        .await
        .map_err(to_anyhow)?)
}

fn oauth_redirect(
    clients: &HashMap<String, ClientWithScopes>,
    client_name: String,
    cookies: Cookies,
    scopes: Option<Vec<String>>,
) -> error::Result<Redirect> {
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
    let url = client.authorize_url(&state);
    set_cookie(&state, cookies);
    Ok(Redirect::to(url.as_str()))
}

fn set_cookie(state: &State, cookies: Cookies) {
    let csrf = state.to_base64();
    let mut cookie = Cookie::new("csrf", csrf);
    cookie.set_path("/");
    cookies.add(cookie);
}
