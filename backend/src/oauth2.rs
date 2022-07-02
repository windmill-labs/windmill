use std::collections::HashMap;
use std::fmt::Debug;

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Extension, FromRequest, Path, Query, RequestParts};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum::{async_trait, Json, Router};
use hyper::StatusCode;
use itertools::Itertools;
use oauth2::basic::{
    BasicClient, BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
    BasicTokenType,
};
use oauth2::reqwest::async_http_client;
use oauth2::{helpers, TokenType};
use oauth2::{AccessToken, Client as OClient, RefreshToken, StandardRevocableToken};
// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
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
}

pub struct ClientWithScopes {
    client: BasicClient,
    scopes: Vec<String>,
}

pub type BasicClientsMap = HashMap<String, ClientWithScopes>;

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    auth_url: String,
    token_url: String,
    scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthClient {
    id: String,
    secret: String,
}
pub struct AllClients {
    pub logins: BasicClientsMap,
    pub connects: BasicClientsMap,
    pub slack: Option<SlackClient>,
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

            let named_client =
                build_basic_client(k.clone(), v, oauths.get(&k).unwrap(), true, base_url);
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
            let named_client =
                build_basic_client(k.clone(), v, oauths.get(&k).unwrap(), false, base_url);
            (
                named_client.0,
                ClientWithScopes {
                    client: named_client.1,
                    scopes: scopes.unwrap_or(vec![]),
                },
            )
        })
        .collect();

    let slack = oauths.get("slack").map(|v| build_slack_client(v, base_url));

    Ok(AllClients {
        logins,
        connects,
        slack,
    })
}

pub fn build_basic_client(
    name: String,
    config: OAuthConfig,
    client: &OAuthClient,
    login: bool,
    base_url: &str,
) -> (String, BasicClient) {
    let auth_url =
        AuthUrl::new(config.auth_url.to_string()).expect("Invalid authorization endpoint URL");
    let token_url =
        TokenUrl::new(config.token_url.to_string()).expect("Invalid token endpoint URL");

    let redirect_url = if login {
        format!("{base_url}/user/login_callback/{name}")
    } else {
        format!("{base_url}/oauth/callback/{name}")
    };

    // Set up the config for the Github OAuth2 process.
    (
        name.to_string(),
        BasicClient::new(
            ClientId::new(client.id.to_string()),
            Some(ClientSecret::new(client.secret.to_string())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap()),
    )
}

pub fn build_slack_client(client: &OAuthClient, base_url: &str) -> SlackClient {
    let auth_url = AuthUrl::new("https://slack.com/oauth/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://slack.com/api/oauth.access".to_string())
        .expect("Invalid token endpoint URL");

    let redirect_url = format!("{base_url}/oauth/callback_slack");

    SlackClient::new(
        ClientId::new(client.id.to_string()),
        Some(ClientSecret::new(client.secret.to_string())),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

type SlackClient = OClient<
    BasicErrorResponse,
    SlackTokenResponse,
    BasicTokenType,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
>;

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
pub struct SlackBotToken {
    bot_access_token: String,
}

impl TokenResponse<BasicTokenType> for SlackTokenResponse
where
    BasicTokenType: TokenType,
{
    ///
    /// REQUIRED. The access token issued by the authorization server.
    ///
    fn access_token(&self) -> &AccessToken {
        &self.access_token
    }
    ///
    /// REQUIRED. The type of the token issued as described in
    /// [Section 7.1](https://tools.ietf.org/html/rfc6749#section-7.1).
    /// Value is case insensitive and deserialized to the generic `TokenType` parameter.
    /// But in this particular case as the service is non compliant, it has a default value
    ///
    fn token_type(&self) -> &BasicTokenType {
        &BasicTokenType::Bearer
    }
    ///
    /// RECOMMENDED. The lifetime in seconds of the access token. For example, the value 3600
    /// denotes that the access token will expire in one hour from the time the response was
    /// generated. If omitted, the authorization server SHOULD provide the expiration time via
    /// other means or document the default value.
    ///
    fn expires_in(&self) -> Option<Duration> {
        None
    }
    ///
    /// OPTIONAL. The refresh token, which can be used to obtain new access tokens using the same
    /// authorization grant as described in
    /// [Section 6](https://tools.ietf.org/html/rfc6749#section-6).
    ///
    fn refresh_token(&self) -> Option<&RefreshToken> {
        None
    }
    ///
    /// OPTIONAL, if identical to the scope requested by the client; otherwise, REQUIRED. The
    /// scipe of the access token as described by
    /// [Section 3.3](https://tools.ietf.org/html/rfc6749#section-3.3). If included in the response,
    /// this space-delimited field is parsed into a `Vec` of individual scopes. If omitted from
    /// the response, this field is `None`.
    ///
    fn scopes(&self) -> Option<&Vec<Scope>> {
        self.scopes.as_ref()
    }
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
    let client = clients
        .slack
        .as_ref()
        .ok_or_else(|| error::Error::BadRequest("slack client not setup".to_string()))?
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("bot".to_string()))
        .add_scope(Scope::new("commands".to_string()));
    let authorize_url = set_csrf_and_retrieve_auth_url(client, cookies);
    Ok(Redirect::to(authorize_url.as_str()))
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
    code: Option<String>,
    state: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    token: String,
}

async fn connect_callback(
    cookies: Cookies,
    Path(client_name): Path<String>,
    Json(oauth): Json<OAuthCallback>,
    Extension(clients): Extension<Arc<AllClients>>,
) -> error::JsonResult<ConnectResponse> {
    let code = AuthorizationCode::new(oauth.code.unwrap());
    let state = CsrfToken::new(oauth.state.unwrap());

    let csrf_state = cookies
        .get("csrf")
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());

    if state.secret().to_string() != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let token = (&clients
        .connects
        .get(&client_name)
        .ok_or_else(|| error::Error::BadRequest("invalid client".to_string()))?
        .client
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .map_err(|e| error::Error::InternalErr(format!("invalid code: {e:?}")))?
        .access_token()
        .secret())
        .to_string();

    Ok(Json(ConnectResponse { token }))
}

async fn connect_slack_callback(
    cookies: Cookies,
    Json(oauth): Json<OAuthCallback>,
    Extension(clients): Extension<Arc<AllClients>>,
) -> error::JsonResult<SlackTokenResponse> {
    let code = AuthorizationCode::new(oauth.code.unwrap());
    let state = CsrfToken::new(oauth.state.unwrap());

    let csrf_state = cookies
        .get("csrf")
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());

    if state.secret().to_string() != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let slack_token = (&clients
        .slack
        .as_ref()
        .ok_or_else(|| error::Error::BadRequest("slack client not setup".to_string()))?
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .map_err(|e| error::Error::InternalErr(format!("invalid code: {e:?}")))?)
        .to_owned();

    Ok(Json(slack_token))
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
) -> error::Result<String> {
    let code = AuthorizationCode::new(callback.code.unwrap());
    let state = CsrfToken::new(callback.state.unwrap());

    let csrf_state = cookies
        .get("csrf")
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());

    if state.secret().to_string() != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let client = &clients.logins.get(&client_name).unwrap().client;

    // Exchange the code with a token.
    let token_res = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await;

    if let Ok(token) = token_res {
        let token = token.access_token().secret();
        let http_client = reqwest::ClientBuilder::new()
            .user_agent("windmill/beta")
            .build()
            .map_err(to_anyhow)?;

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
        Err(error::Error::BadRequest(
            "failed to exchange code".to_string(),
        ))
    }
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
    tracing::info!("{token}");
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
    let mut client = client_w_scopes.client.authorize_url(CsrfToken::new_random);
    let scopes_iter = if let Some(scopes) = scopes {
        scopes
    } else {
        client_w_scopes.scopes.clone()
    };
    for scope in scopes_iter.iter() {
        client = client.add_scope(oauth2::Scope::new(scope.to_string()));
    }
    let authorize_url = set_csrf_and_retrieve_auth_url(client, cookies);
    Ok(Redirect::to(authorize_url.as_str()))
}

fn set_csrf_and_retrieve_auth_url(
    client: oauth2::AuthorizationRequest,
    cookies: Cookies,
) -> url::Url {
    let (authorize_url, csrf_state) = client.url();
    let csrf = csrf_state.secret().to_string();
    let mut cookie = Cookie::new("csrf", csrf);
    cookie.set_path("/");
    cookies.add(cookie);
    authorize_url
}
