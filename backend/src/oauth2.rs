use std::fmt::Debug;

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Extension, FromRequest, Path, Query, RequestParts};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum::{async_trait, Router};
use futures::TryFutureExt;
use hyper::StatusCode;
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
use serde::{Deserialize, Serialize};
use slack_http_verifier::SlackVerifier;
use tower_cookies::{Cookie, Cookies};

use crate::audit::{audit_log, ActionKind};
use crate::db::{UserDB, DB};
use crate::error::{self, to_anyhow, Error, Result};
use crate::jobs::{get_latest_hash_for_path, JobPayload};
use crate::users::{Authed, LoginType};
use crate::variables::build_crypt;
use crate::workspaces::WorkspaceSettings;
use crate::{jobs, BasicClientsMap};
use crate::{variables, BaseUrl};

pub fn global_service() -> Router {
    Router::new()
        .route("/login/:client", get(login))
        .route("/login_callback/:client", get(login_callback))
        .route(
            "/slack_command",
            post(slack_command).route_layer(axum::middleware::from_extractor::<SlackSig>()),
        )
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/connect/:client", get(connect))
        .route("/disconnect/:client", post(disconnect))
        .route("/connect_callback/:client", get(connect_callback))
}

pub fn build_gh_client(client_id: &str, client_secret: &str, base_uri: &str) -> BasicClient {
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.
    BasicClient::new(
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.to_string())),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{base_uri}/api/oauth/login_callback/github")).unwrap(),
    )
}

pub fn build_connect_client(w_id: &str, client_name: &str, base_uri: &str) -> Result<BasicClient> {
    let (auth_str, token_str) = match client_name {
        "gmail" => ("", ""),
        "slack" => (
            "https://slack.com/oauth/authorize",
            "https://slack.com/api/oauth.access",
        ),
        _ => Err(Error::BadRequest(format!("unrecognized client!")))?,
    };

    let auth_url = AuthUrl::new(auth_str.to_string()).expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new(token_str.to_string()).expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.
    Ok(BasicClient::new(
        ClientId::new(
            std::env::var(&format!("{}_OAUTH_CLIENT_ID", client_name.to_uppercase()))
                .ok()
                .ok_or(Error::BadRequest(format!(
                    "client id for {} not in env",
                    client_name
                )))?,
        ),
        Some(ClientSecret::new(
            std::env::var(&format!(
                "{}_OAUTH_CLIENT_SECRET",
                client_name.to_uppercase()
            ))
            .ok()
            .ok_or(Error::BadRequest(format!(
                "client secret for {} not in env",
                client_name
            )))?,
        )),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!(
            "{base_uri}/api/w/{w_id}/oauth/connect_callback/{client_name}"
        ))
        .unwrap(),
    ))
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

pub fn build_slack_client(w_id: &str, client_name: &str, base_uri: &str) -> Result<SlackClient> {
    let (auth_str, token_str) = (
        "https://slack.com/oauth/authorize",
        "https://slack.com/api/oauth.access",
    );

    let auth_url = AuthUrl::new(auth_str.to_string()).expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new(token_str.to_string()).expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.
    Ok(SlackClient::new(
        ClientId::new(
            std::env::var(&format!("{}_OAUTH_CLIENT_ID", client_name.to_uppercase()))
                .ok()
                .ok_or(Error::BadRequest(format!(
                    "client id for {} not in env",
                    client_name
                )))?,
        ),
        Some(ClientSecret::new(
            std::env::var(&format!(
                "{}_OAUTH_CLIENT_SECRET",
                client_name.to_uppercase()
            ))
            .ok()
            .ok_or(Error::BadRequest(format!(
                "client secret for {} not in env",
                client_name
            )))?,
        )),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!(
            "{base_uri}/api/w/{w_id}/oauth/connect_callback/{client_name}"
        ))
        .unwrap(),
    ))
}

async fn connect(
    Path((w_id, client_name)): Path<(String, String)>,
    Extension(base_url): Extension<BaseUrl>,
    cookies: Cookies,
) -> error::Result<Redirect> {
    let client = build_connect_client(&w_id, &client_name, &base_url.0)?;

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("bot".to_string()))
        .add_scope(Scope::new("commands".to_string()))
        .url();

    let csrf = csrf_state.secret().to_string();
    let mut cookie = Cookie::new("csrf", csrf);
    cookie.set_path("/");
    cookies.add(cookie);
    Ok(Redirect::to(authorize_url.as_str()))
}

async fn disconnect(
    authed: Authed,
    Path((w_id, client_name)): Path<(String, String)>,
    Extension(user_db): Extension<UserDB>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    match client_name.as_str() {
        "slack" => {
            sqlx::query!(
                "UPDATE workspace_settings
            SET slack_team_id = null, slack_name = null WHERE workspace_id = $1",
                &w_id
            )
            .execute(&mut tx)
            .await?;
        }
        _ => Err(error::Error::BadRequest(format!(
            "Not recognized client name {client_name}"
        )))?,
    }
    tx.commit().await?;
    Ok(format!("{client_name} disconnected"))
}

async fn login(
    Extension(clients): Extension<Arc<BasicClientsMap>>,
    Path(client_name): Path<String>,
    cookies: Cookies,
) -> error::Result<Redirect> {
    let client = clients
        .get(&client_name)
        .ok_or(Error::BadRequest(format!("client {} invalid", client_name)))?;
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        //        .add_scope(Scope::new("read:user".to_string()))
        .url();

    let csrf = csrf_state.secret().to_string();
    let mut cookie = Cookie::new("csrf", csrf);
    cookie.set_path("/");
    cookies.add(cookie);
    Ok(Redirect::to(authorize_url.as_str()))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn connect_callback(
    authed: Authed,
    Path((w_id, client_name)): Path<(String, String)>,
    Query(query): Query<CallbackQuery>,
    cookies: Cookies,
    Extension(user_db): Extension<UserDB>,
    Extension(base_url): Extension<BaseUrl>,
) -> error::Result<Redirect> {
    if let Some(error) = query.error {
        return Ok(Redirect::to(&format!(
            "/connection_added?error={}",
            urlencoding::encode(&error).into_owned()
        )));
    }

    let code = AuthorizationCode::new(query.code.unwrap());
    let state = CsrfToken::new(query.state.unwrap());

    let csrf_state = cookies
        .get("csrf")
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());

    if state.secret().to_string() != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let mut tx = user_db.begin(&authed).await?;

    let mc = build_crypt(&mut tx, &w_id).await?;

    let token_res = match client_name.as_str() {
        "slack" => {
            let t = build_slack_client(&w_id, &client_name, &base_url.0)?
                .exchange_code(code)
                .request_async(async_http_client)
                .await;
            if let Ok(token) = t {
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
                Ok(token.bot.bot_access_token.to_owned())
            } else {
                Err(t.unwrap_err())
            }
        }
        _ => {
            build_connect_client(&w_id, &client_name, &base_url.0)?
                .exchange_code(code)
                .request_async(async_http_client)
                .map_ok(|t| t.access_token().secret().to_owned())
                .await
        }
    };

    if let Ok(token) = token_res {
        tracing::info!("{token}");
        let variable_path = &format!("g/all/{}_token", &client_name);
        sqlx::query!(
            "INSERT INTO variable
            (workspace_id, path, value, is_secret, description)
            VALUES ($1, $2, $3, true, $4) ON CONFLICT (workspace_id, path) DO UPDATE SET value = $3",
            &w_id,
            variable_path,
            variables::encrypt(&mc, token.to_string()),
            format!("OAuth2 token for {client_name}"),
        )
        .execute(&mut tx)
        .await?;
        sqlx::query!(
            "INSERT INTO resource
            (workspace_id, path, value, description, resource_type)
            VALUES ($1, $2, $3, $4, $5) ON CONFLICT (workspace_id, path) DO UPDATE SET value = $3",
            &w_id,
            variable_path,
            serde_json::json!({ "token": format!("$var:{variable_path}") }),
            format!("OAuth2 token for {client_name}"),
            &client_name
        )
        .execute(&mut tx)
        .await?;
        audit_log(
            &mut tx,
            &authed.username,
            "oauth2.connect",
            ActionKind::Create,
            &w_id,
            Some(&client_name),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(Redirect::to(
            format!("/connection_added?client_name={}", &client_name).as_str(),
        ))
    } else {
        let error = token_res.unwrap_err().to_string();
        Ok(Redirect::to(&format!(
            "/connection_added?error={}",
            urlencoding::encode(&format!("error fetching token: {error}")).into_owned()
        )))
    }
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
    Query(query): Query<CallbackQuery>,
    cookies: Cookies,
    Extension(clients): Extension<Arc<BasicClientsMap>>,
    Extension(db): Extension<DB>,
) -> error::Result<Redirect> {
    if let Some(error) = query.error {
        return Ok(Redirect::to(&format!(
            "/user/login?error={}",
            urlencoding::encode(&error).into_owned()
        )));
    }

    let code = AuthorizationCode::new(query.code.unwrap());
    let state = CsrfToken::new(query.state.unwrap());

    let csrf_state = cookies
        .get("csrf")
        .map(|x| x.value().to_string())
        .unwrap_or("".to_string());

    if state.secret().to_string() != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let client = clients.get(&client_name).unwrap();

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

        let login: Option<(String, LoginType, bool)> =
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
        Ok(Redirect::to("/user/workspaces"))
    } else {
        Ok(Redirect::to(&format!(
            "/user/login?error={}",
            urlencoding::encode("invalid token").into_owned()
        )))
    }
}

#[derive(Deserialize)]
pub struct GHEmailInfo {
    email: String,
    verified: bool,
    primary: bool,
}

async fn get_email(http_client: &Client, client_name: &str, token: &str) -> error::Result<String> {
    let email = match client_name {
        "github" => http_client
            .get("https://api.github.com/user/emails")
            .bearer_auth(token)
            .send()
            .await
            .map_err(to_anyhow)?
            .json::<Vec<GHEmailInfo>>()
            .await
            .map_err(to_anyhow)?
            .iter()
            .find(|x| x.primary && x.verified)
            .ok_or(error::Error::BadRequest(format!(
                "user does not have any primary and verified address"
            )))?
            .email
            .to_string(),
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
        "github" => http_client
            .get("https://api.github.com/user")
            .bearer_auth(token)
            .send()
            .await
            .map_err(to_anyhow)?
            .json::<UserInfo>()
            .await
            .map_err(to_anyhow)?,
        _ => {
            return Err(error::Error::BadRequest(
                "client name not recognized".to_string(),
            ))
        }
    };
    Ok(email)
}
