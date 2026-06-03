use crate::db::{ApiAuthed, DB};
use crate::utils::check_scopes;

#[cfg(feature = "bedrock")]
use axum::routing::get;
use axum::{
    body::Bytes, extract::Path, response::IntoResponse, routing::post, Extension, Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use futures::StreamExt;
use http::{HeaderMap, Method};
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::time::Duration;
use windmill_ai::ai_cache::current_instance_ai_config_revision;
use windmill_ai::ai_providers::{
    empty_string_as_none, AIPlatform, AIProvider, ProviderConfig, ProviderModel,
};
use windmill_ai::credentials::ProviderCredentials;
#[cfg(feature = "bedrock")]
use windmill_ai::providers::bedrock::{
    handle_bedrock_proxy, BedrockProxyResponse, BedrockProxyResponseBody,
};
use windmill_ai::providers::{
    create_query_builder,
    google_ai::{
        handle_google_ai_chat_proxy, handle_google_ai_models_proxy, GoogleAIProxyResponse,
        GoogleAIProxyResponseBody,
    },
};
use windmill_ai::proxy::{
    fim::maybe_transform_fim_request, proxy_execution_mode, ProxyBuildArgs, ProxyExecutionMode,
    ProxyRequest,
};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::db::UserDB;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::utils::configure_client;
use windmill_common::variables::{get_variable_or_self, get_variable_or_self_as};

const CODEX_TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const CODEX_DEVICE_AUTHORIZATION_ENDPOINT: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
const CODEX_DEVICE_TOKEN_ENDPOINT: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const CODEX_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_ISSUER: &str = "https://auth.openai.com";
const CODEX_ACCOUNT_CLAIM: &str = "https://api.openai.com/auth";
const OPENAI_CHATGPT_ACCOUNT_RESOURCE: &str = "openai_chatgpt_account";

// AI timeout configuration constants
const AI_TIMEOUT_MIN_SECS: u64 = 1;
const AI_TIMEOUT_MAX_SECS: u64 = 86400; // 24 hours
const AI_TIMEOUT_DEFAULT_SECS: u64 = 3600; // 1 hour
const HTTP_POOL_MAX_IDLE_PER_HOST: usize = 10;
const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 90;
pub(crate) const KEEPALIVE_INTERVAL_SECS: u64 = 15;

lazy_static::lazy_static! {
    /// AI request timeout in seconds.
    ///
    /// This timeout applies to the TOTAL duration of AI HTTP requests,
    /// including streaming responses. Default is 3600 seconds (1 hour).
    ///
    /// Can be configured via AI_REQUEST_TIMEOUT_SECONDS environment variable.
    /// Valid range: 1-86400 seconds (24 hours).
    ///   - Minimum (1s): Prevents immediate timeout, allows minimal response time
    ///   - Maximum (24h): Prevents indefinite hangs while supporting long-running AI operations
    ///   - Default (1h): Balances responsiveness with support for complex AI tasks
    ///
    /// Note: This is a total request timeout, not an idle timeout.
    /// Long-running streaming responses that exceed this duration will be terminated,
    /// even if actively receiving data.
    ///
    /// CRITICAL: If using a reverse proxy (NGINX, Traefik, etc.), you MUST configure
    /// proxy timeouts to match or exceed this value. Without proper proxy configuration,
    /// connections will be terminated prematurely at the proxy layer regardless of this
    /// backend timeout setting.
    ///
    /// Example NGINX configuration:
    ///   location /api/ {
    ///     proxy_read_timeout 3600s;  # Must be >= AI_REQUEST_TIMEOUT_SECONDS
    ///     proxy_send_timeout 3600s;
    ///     proxy_connect_timeout 60s;
    ///   }
    static ref AI_TIMEOUT_SECS: u64 = {
        match std::env::var("AI_REQUEST_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            Some(timeout) if timeout >= AI_TIMEOUT_MIN_SECS && timeout <= AI_TIMEOUT_MAX_SECS => {
                tracing::info!("AI request timeout configured: {}s", timeout);
                timeout
            },
            Some(timeout) => {
                tracing::warn!(
                    "AI_REQUEST_TIMEOUT_SECONDS value {} is out of range ({}-{}), using default {}s",
                    timeout,
                    AI_TIMEOUT_MIN_SECS,
                    AI_TIMEOUT_MAX_SECS,
                    AI_TIMEOUT_DEFAULT_SECS
                );
                AI_TIMEOUT_DEFAULT_SECS
            },
            None => {
                tracing::info!(
                    "AI_REQUEST_TIMEOUT_SECONDS not set, using default {}s",
                    AI_TIMEOUT_DEFAULT_SECS
                );
                AI_TIMEOUT_DEFAULT_SECS
            },
        }
    };

    pub(crate) static ref HTTP_CLIENT: Client = configure_client(reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(*AI_TIMEOUT_SECS))
        .pool_max_idle_per_host(HTTP_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Some(std::time::Duration::from_secs(HTTP_POOL_IDLE_TIMEOUT_SECS)))
        // The SSRF check in `get_base_url` only validates the configured `base_url`.
        // reqwest follows up to 10 redirects by default and does not revalidate the
        // hops, so a public base_url could 3xx the server into a private/internal
        // address. Disable redirect following so the validated host is the only one
        // we ever connect to. AI APIs respond directly and do not rely on redirects,
        // so this holds even for ALLOW_PRIVATE_AI_BASE_URLS deployments.
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("windmill/beta"))
        .build()
        .expect("Failed to build AI HTTP client - check system TLS configuration");

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringProviderCredentials> = Cache::new(500);

}

pub(crate) fn invalidate_ai_request_cache_for_workspace(workspace_id: &str) {
    AI_REQUEST_CACHE.retain(|(cached_workspace_id, _), _| cached_workspace_id != workspace_id);
}

#[derive(Deserialize, Debug)]
struct AIOAuthResource {
    client_id: String,
    client_secret: String,
    token_url: String,
    user: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AIStandardResource {
    #[serde(alias = "baseUrl", default, deserialize_with = "empty_string_as_none")]
    base_url: Option<String>,
    #[serde(alias = "apiKey", default, deserialize_with = "empty_string_as_none")]
    api_key: Option<String>,
    #[serde(alias = "accessToken", default, deserialize_with = "empty_string_as_none")]
    access_token: Option<String>,
    #[serde(alias = "refreshToken", default, deserialize_with = "empty_string_as_none")]
    refresh_token: Option<String>,
    #[serde(alias = "expiresAt", default, deserialize_with = "empty_string_as_none")]
    expires_at: Option<String>,
    #[serde(alias = "accountId", default, deserialize_with = "empty_string_as_none")]
    account_id: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    organization_id: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    region: Option<String>,
    #[serde(
        alias = "awsAccessKeyId",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    aws_access_key_id: Option<String>,
    #[serde(
        alias = "awsSecretAccessKey",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    aws_secret_access_key: Option<String>,
    #[serde(
        alias = "awsSessionToken",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    aws_session_token: Option<String>,
    /// Platform (standard or google_vertex_ai)
    #[serde(default)]
    platform: AIPlatform,
    /// Enable 1M context window for Anthropic
    #[serde(alias = "enable_1M_context", default)]
    enable_1m_context: bool,
    /// Custom HTTP headers to include in AI requests
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct OAuthTokens {
    access_token: String,
}

#[derive(Serialize)]
struct OpenAIChatGPTAccountDeviceStartResponse {
    verification_uri: String,
    user_code: String,
    device_auth_id: String,
    interval_ms: u64,
    expires_at: String,
}

#[derive(Deserialize)]
struct OpenAIChatGPTAccountDeviceCompleteRequest {
    device_auth_id: String,
    user_code: String,
    resource_path: Option<String>,
}

#[derive(Serialize)]
struct OpenAIChatGPTAccountDeviceCompleteResponse {
    status: String,
    resource_path: Option<String>,
    account_id: Option<String>,
    expires_at: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIChatGPTAccountStatusRequest {
    resource_path: String,
}

#[derive(Serialize)]
struct OpenAIChatGPTAccountStatusResponse {
    connected: bool,
    resource_path: String,
    account_id: Option<String>,
    expires_at: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIChatGPTAccountResourceStatus {
    account_id: Option<String>,
    expires_at: Option<String>,
}

#[derive(Deserialize)]
struct CodexDeviceAuthorizationResponse {
    device_auth_id: String,
    user_code: String,
    interval: Option<CodexDeviceInterval>,
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CodexDeviceInterval {
    Seconds(u64),
    Text(String),
}

#[derive(Deserialize)]
struct CodexDeviceTokenResponse {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Deserialize)]
struct CodexTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AIResource {
    OAuth(AIOAuthResource),
    Standard(AIStandardResource),
}

/// Resolve a `$var:` reference. When `user_db`/`authed` are provided the query
/// goes through an RLS-scoped connection so the caller can only read variables
/// they are authorised to access.  Without auth context the raw pool is used
/// (appropriate for admin/system paths where the resource was already validated).
async fn resolve_var(
    path: String,
    db: &DB,
    w_id: &str,
    user_db: Option<&UserDB>,
    authed: Option<&ApiAuthed>,
) -> Result<String> {
    match (user_db, authed) {
        (Some(udb), Some(auth)) => Ok(get_variable_or_self_as(path, db, udb, auth, w_id).await?),
        _ => Ok(get_variable_or_self(path, db, w_id).await?),
    }
}

async fn resolve_provider_credentials(
    provider: &AIProvider,
    db: &DB,
    w_id: &str,
    resource_path: Option<&str>,
    resource: AIResource,
    authed: Option<&ApiAuthed>,
) -> Result<ProviderCredentials> {
    // When authed is provided, resolve $var: references through RLS so that
    // users can only read variables they have permission to access.
    let user_db = authed.map(|_| UserDB::new(db.clone()));

    match resource {
        AIResource::Standard(resource) => {
            // Skip get_base_url for Bedrock - it uses SDK directly, not HTTP
            let base_url = if matches!(provider, AIProvider::AWSBedrock) {
                String::new()
            } else {
                provider.get_base_url(resource.base_url.clone(), db).await?
            };
            let api_key = if let Some(api_key) = resource.api_key.clone() {
                Some(resolve_var(api_key, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let access_token = if let Some(access_token) = resource.access_token.clone() {
                Some(resolve_var(access_token, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let refresh_token = if let Some(refresh_token) = resource.refresh_token.clone() {
                Some(resolve_var(refresh_token, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let expires_at = if let Some(expires_at) = resource.expires_at.clone() {
                Some(resolve_var(expires_at, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let account_id = if let Some(account_id) = resource.account_id.clone() {
                Some(resolve_var(account_id, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let organization_id = if let Some(organization_id) = resource.organization_id.clone() {
                Some(resolve_var(organization_id, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let aws_access_key_id = if let Some(access_key_id) = resource.aws_access_key_id.clone() {
                Some(resolve_var(access_key_id, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let aws_secret_access_key =
                if let Some(secret_access_key) = resource.aws_secret_access_key.clone() {
                    Some(resolve_var(secret_access_key, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
            let aws_session_token = if let Some(session_token) = resource.aws_session_token.clone() {
                Some(resolve_var(session_token, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };

            let mut access_token = access_token;
            let mut account_id = account_id;
            if matches!(provider, AIProvider::OpenAIChatGPTAccount) {
                if let (Some(current_access_token), Some(refresh_token), Some(resource_path)) =
                    (access_token.as_ref(), refresh_token.as_ref(), resource_path)
                {
                    if token_expires_soon(current_access_token, expires_at.as_deref()) {
                        if let Some(authed) = authed {
                            check_scopes(authed, || format!("resources:write:{resource_path}"))?;
                            if let Some(access_token_path) =
                                variable_path(resource.access_token.as_deref())
                            {
                                check_scopes(authed, || {
                                    format!("variables:write:{access_token_path}")
                                })?;
                            }
                            if let Some(refresh_token_path) =
                                variable_path(resource.refresh_token.as_deref())
                            {
                                check_scopes(authed, || {
                                    format!("variables:write:{refresh_token_path}")
                                })?;
                            }
                        }
                        let session = refresh_codex_token(refresh_token).await?;
                        let refreshed_account_id = extract_chatgpt_account_id(&session.access_token)?;
                        let refreshed_expires_at = codex_session_expires_at(&session);
                        persist_refreshed_openai_chatgpt_account_session(
                            db,
                            w_id,
                            resource_path,
                            resource.access_token.as_deref(),
                            resource.refresh_token.as_deref(),
                            authed.map(|a| a.username.as_str()).unwrap_or("system"),
                            &session,
                            &refreshed_account_id,
                            refreshed_expires_at,
                        )
                        .await?;
                        access_token = Some(session.access_token);
                        account_id = Some(refreshed_account_id);
                        invalidate_ai_request_cache_for_workspace(w_id);
                    }
                }
            }

            let mut custom_headers = resource.headers.clone();
            if matches!(provider, AIProvider::OpenAIChatGPTAccount) {
                if let Some(account_id) = account_id.clone() {
                    custom_headers.insert("chatgpt-account-id".to_string(), account_id);
                }
            }

            Ok(ProviderCredentials {
                provider: provider.clone(),
                base_url,
                api_key,
                access_token,
                organization_id,
                user: None,
                region: resource.region,
                aws_access_key_id,
                aws_secret_access_key,
                aws_session_token,
                platform: resource.platform,
                enable_1m_context: resource.enable_1m_context,
                custom_headers,
            })
        }
        AIResource::OAuth(resource) => {
            let user = if let Some(user) = resource.user.clone() {
                Some(resolve_var(user, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let token = get_token_using_oauth(resource, db, w_id, user_db.as_ref(), authed).await?;
            let base_url = provider.get_base_url(None, db).await?;

            Ok(ProviderCredentials {
                provider: provider.clone(),
                base_url,
                api_key: None,
                access_token: Some(token),
                organization_id: None,
                user,
                region: None,
                aws_access_key_id: None,
                aws_secret_access_key: None,
                aws_session_token: None,
                platform: AIPlatform::Standard,
                enable_1m_context: false,
                custom_headers: HashMap::new(),
            })
        }
    }
}

async fn get_token_using_oauth(
    mut resource: AIOAuthResource,
    db: &DB,
    w_id: &str,
    user_db: Option<&UserDB>,
    authed: Option<&ApiAuthed>,
) -> Result<String> {
    resource.client_id = resolve_var(resource.client_id, db, w_id, user_db, authed).await?;
    resource.client_secret = resolve_var(resource.client_secret, db, w_id, user_db, authed).await?;
    resource.token_url = resolve_var(resource.token_url, db, w_id, user_db, authed).await?;
    // Validate the resolved token_url against SSRF rules before issuing the request,
    // mirroring the protection applied to base_url in `get_base_url` (same
    // ALLOW_PRIVATE_AI_BASE_URLS opt-in). Without this a workspace member could
    // point token_url at an internal/metadata address.
    if !*windmill_ai::ai_providers::ALLOW_PRIVATE_AI_BASE_URLS {
        use windmill_common::ssrf::SsrfValidationError;
        windmill_common::ssrf::validate_url_for_ssrf(&resource.token_url)
            .await
            .map_err(|e| match e {
                e @ SsrfValidationError::Private { .. } => Error::BadRequest(format!(
                    "{e}. If you need to use private/internal AI endpoints, \
                     set the ALLOW_PRIVATE_AI_BASE_URLS=true environment variable"
                )),
                e => Error::from(e),
            })?;
    }
    let mut params = HashMap::new();
    params.insert("grant_type", "client_credentials");
    params.insert("scope", "https://cognitiveservices.azure.com/.default");
    let response = HTTP_CLIENT
        .post(resource.token_url)
        .form(&params)
        .basic_auth(resource.client_id, Some(resource.client_secret))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|err| {
            Error::internal_err(format!(
                "Failed to get access token using credentials flow: {}",
                err
            ))
        })?;
    let response = response.json::<OAuthTokens>().await.map_err(|err| {
        Error::internal_err(format!(
            "Failed to parse access token from credentials flow: {}",
            err
        ))
    })?;
    Ok(response.access_token)
}

async fn start_openai_chatgpt_account_device_auth(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<Json<OpenAIChatGPTAccountDeviceStartResponse>> {
    check_scopes(&authed, || "resources:write".to_string())?;

    let response = HTTP_CLIENT
        .post(CODEX_DEVICE_AUTHORIZATION_ENDPOINT)
        .json(&json!({ "client_id": CODEX_CLIENT_ID }))
        .send()
        .await
        .map_err(to_anyhow)?;

    if response.error_for_status_ref().is_err() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadRequest(format!(
            "OpenAI device login failed ({status}): {body}"
        )));
    }

    let challenge = response
        .json::<CodexDeviceAuthorizationResponse>()
        .await
        .map_err(to_anyhow)?;
    let interval_seconds = challenge
        .interval
        .as_ref()
        .and_then(|interval| match interval {
            CodexDeviceInterval::Seconds(seconds) => Some(*seconds),
            CodexDeviceInterval::Text(value) => value.parse::<u64>().ok(),
        })
        .unwrap_or(5)
        .max(1);
    let expires_at = Utc::now()
        + ChronoDuration::seconds(challenge.expires_in.unwrap_or(600).max(1));

    audit_log_ai_auth(
        &db,
        &authed,
        &w_id,
        "ai.openai_chatgpt_account.device_start",
        None,
    )
    .await?;

    Ok(Json(OpenAIChatGPTAccountDeviceStartResponse {
        verification_uri: format!("{CODEX_ISSUER}/codex/device"),
        user_code: challenge.user_code,
        device_auth_id: challenge.device_auth_id,
        interval_ms: interval_seconds * 1000,
        expires_at: expires_at.to_rfc3339(),
    }))
}

async fn complete_openai_chatgpt_account_device_auth(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<OpenAIChatGPTAccountDeviceCompleteRequest>,
) -> Result<Json<OpenAIChatGPTAccountDeviceCompleteResponse>> {
    let resource_path = request.resource_path.unwrap_or_else(|| {
        default_openai_chatgpt_account_resource_path(authed.display_username())
    });
    check_scopes(&authed, || format!("resources:write:{resource_path}"))?;

    let response = HTTP_CLIENT
        .post(CODEX_DEVICE_TOKEN_ENDPOINT)
        .json(&json!({
            "device_auth_id": request.device_auth_id,
            "user_code": request.user_code,
        }))
        .send()
        .await
        .map_err(to_anyhow)?;

    if response.status().as_u16() == 403 || response.status().as_u16() == 404 {
        return Ok(Json(OpenAIChatGPTAccountDeviceCompleteResponse {
            status: "pending".to_string(),
            resource_path: None,
            account_id: None,
            expires_at: None,
        }));
    }

    if response.error_for_status_ref().is_err() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadRequest(format!(
            "OpenAI device login failed ({status}): {body}"
        )));
    }

    let device_token = response
        .json::<CodexDeviceTokenResponse>()
        .await
        .map_err(to_anyhow)?;
    let session = exchange_codex_authorization_code(&device_token).await?;
    if session.refresh_token.is_none() {
        return Err(Error::BadRequest(
            "OpenAI device login did not return a refresh token".to_string(),
        ));
    }
    let account_id = extract_chatgpt_account_id(&session.access_token)?;
    let expires_at = codex_session_expires_at(&session);

    persist_openai_chatgpt_account_session(
        &db,
        &w_id,
        &resource_path,
        &authed,
        &session,
        &account_id,
        expires_at,
    )
    .await?;
    invalidate_ai_request_cache_for_workspace(&w_id);
    audit_log_ai_auth(
        &db,
        &authed,
        &w_id,
        "ai.openai_chatgpt_account.device_complete",
        Some(&resource_path),
    )
    .await?;

    Ok(Json(OpenAIChatGPTAccountDeviceCompleteResponse {
        status: "connected".to_string(),
        resource_path: Some(resource_path),
        account_id: Some(account_id),
        expires_at: Some(expires_at.to_rfc3339()),
    }))
}

async fn get_openai_chatgpt_account_status(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(request): Json<OpenAIChatGPTAccountStatusRequest>,
) -> Result<Json<OpenAIChatGPTAccountStatusResponse>> {
    check_scopes(&authed, || format!("resources:read:{}", request.resource_path))?;

    let mut tx = user_db.begin(&authed).await?;
    let value = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = $3",
    )
    .bind(&w_id)
    .bind(&request.resource_path)
    .bind(OPENAI_CHATGPT_ACCOUNT_RESOURCE)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let Some(value) = value else {
        return Ok(Json(OpenAIChatGPTAccountStatusResponse {
            connected: false,
            resource_path: request.resource_path,
            account_id: None,
            expires_at: None,
        }));
    };

    let status = serde_json::from_value::<OpenAIChatGPTAccountResourceStatus>(value)
        .map_err(|e| Error::BadRequest(format!("Invalid ChatGPT account resource: {e}")))?;

    Ok(Json(OpenAIChatGPTAccountStatusResponse {
        connected: status.account_id.is_some(),
        resource_path: request.resource_path,
        account_id: status.account_id,
        expires_at: status.expires_at,
    }))
}

async fn audit_log_ai_auth(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    operation: &str,
    resource_path: Option<&str>,
) -> Result<()> {
    let mut tx = db.begin().await?;
    audit_log(
        &mut *tx,
        authed,
        operation,
        ActionKind::Execute,
        w_id,
        resource_path,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn exchange_codex_authorization_code(
    device_token: &CodexDeviceTokenResponse,
) -> Result<CodexTokenResponse> {
    let mut body = HashMap::new();
    body.insert("grant_type", "authorization_code");
    body.insert("code", device_token.authorization_code.as_str());
    body.insert("redirect_uri", CODEX_DEVICE_REDIRECT_URI);
    body.insert("client_id", CODEX_CLIENT_ID);
    body.insert("code_verifier", device_token.code_verifier.as_str());

    let response = HTTP_CLIENT
        .post(CODEX_TOKEN_ENDPOINT)
        .form(&body)
        .send()
        .await
        .map_err(to_anyhow)?;

    if response.error_for_status_ref().is_err() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadRequest(format!(
            "OpenAI token exchange failed ({status}): {body}"
        )));
    }

    Ok(response.json::<CodexTokenResponse>().await.map_err(to_anyhow)?)
}

async fn refresh_codex_token(refresh_token: &str) -> Result<CodexTokenResponse> {
    let mut body = HashMap::new();
    body.insert("grant_type", "refresh_token");
    body.insert("refresh_token", refresh_token);
    body.insert("client_id", CODEX_CLIENT_ID);

    let response = HTTP_CLIENT
        .post(CODEX_TOKEN_ENDPOINT)
        .form(&body)
        .send()
        .await
        .map_err(to_anyhow)?;

    if response.error_for_status_ref().is_err() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadRequest(format!(
            "OpenAI token refresh failed ({status}): {body}"
        )));
    }

    Ok(response.json::<CodexTokenResponse>().await.map_err(to_anyhow)?)
}

fn codex_session_expires_at(session: &CodexTokenResponse) -> DateTime<Utc> {
    Utc::now() + ChronoDuration::seconds(session.expires_in.unwrap_or(3600).max(60))
}

fn default_openai_chatgpt_account_resource_path(username: &str) -> String {
    let username = path_safe_user_segment(username);
    format!("u/{username}/{OPENAI_CHATGPT_ACCOUNT_RESOURCE}")
}

fn default_openai_chatgpt_account_access_token_path(username: &str) -> String {
    let username = path_safe_user_segment(username);
    format!("u/{username}/{OPENAI_CHATGPT_ACCOUNT_RESOURCE}_access_token")
}

fn default_openai_chatgpt_account_refresh_token_path(username: &str) -> String {
    let username = path_safe_user_segment(username);
    format!("u/{username}/{OPENAI_CHATGPT_ACCOUNT_RESOURCE}_refresh_token")
}

fn path_safe_user_segment(username: &str) -> String {
    let candidate = username.split_once('@').map(|(name, _)| name).unwrap_or(username);
    let sanitized: String = candidate
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "user".to_string()
    } else {
        sanitized
    }
}

async fn persist_openai_chatgpt_account_session(
    db: &DB,
    w_id: &str,
    resource_path: &str,
    authed: &ApiAuthed,
    session: &CodexTokenResponse,
    account_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    let access_token_path =
        default_openai_chatgpt_account_access_token_path(authed.display_username());
    let refresh_token_path =
        default_openai_chatgpt_account_refresh_token_path(authed.display_username());

    check_scopes(authed, || format!("variables:write:{access_token_path}"))?;
    check_scopes(authed, || format!("variables:write:{refresh_token_path}"))?;

    let encrypted_access_token =
        windmill_common::secret_backend::database::encrypt_for_database(
            db,
            w_id,
            &session.access_token,
        )
        .await?;

    let encrypted_refresh_token = if let Some(refresh_token) = session.refresh_token.as_ref() {
        Some(
            windmill_common::secret_backend::database::encrypt_for_database(
                db,
                w_id,
                refresh_token,
            )
            .await?,
        )
    } else {
        None
    };

    let mut tx = db.begin().await?;

    upsert_secret_variable_in_tx(
        &mut *tx,
        w_id,
        &access_token_path,
        encrypted_access_token,
        "OpenAI ChatGPT Account access token",
        &authed.username,
    )
    .await?;

    if let Some(encrypted_refresh_token) = encrypted_refresh_token {
        upsert_secret_variable_in_tx(
            &mut *tx,
            w_id,
            &refresh_token_path,
            encrypted_refresh_token,
            "OpenAI ChatGPT Account refresh token",
            &authed.username,
        )
        .await?;
    }

    let resource_value = json!({
        "auth_mode": "chatgpt",
        "token_source": "device_auth",
        "access_token": format!("$var:{access_token_path}"),
        "refresh_token": format!("$var:{refresh_token_path}"),
        "account_id": account_id,
        "expires_at": expires_at.to_rfc3339(),
        "client_id": CODEX_CLIENT_ID,
    });

    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, description, resource_type, created_by, edited_at)
         VALUES ($1, $2, $3, $4, $5, $6, now())
         ON CONFLICT (workspace_id, path) DO UPDATE SET
           value = EXCLUDED.value,
           description = EXCLUDED.description,
           resource_type = EXCLUDED.resource_type,
           edited_at = now()",
    )
    .bind(w_id)
    .bind(resource_path)
    .bind(sqlx::types::Json(resource_value))
    .bind("OpenAI ChatGPT account / Codex authentication")
    .bind(OPENAI_CHATGPT_ACCOUNT_RESOURCE)
    .bind(&authed.username)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

async fn persist_refreshed_openai_chatgpt_account_session(
    db: &DB,
    w_id: &str,
    resource_path: &str,
    access_token_reference: Option<&str>,
    refresh_token_reference: Option<&str>,
    edited_by: &str,
    session: &CodexTokenResponse,
    account_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    let access_token_path = variable_path(access_token_reference);
    let encrypted_access_token = if access_token_path.is_some() {
        Some(
            windmill_common::secret_backend::database::encrypt_for_database(
                db,
                w_id,
                &session.access_token,
            )
            .await?,
        )
    } else {
        None
    };

    let refresh_token_path = variable_path(refresh_token_reference);
    let encrypted_refresh_token = if let (Some(_), Some(refresh_token)) =
        (refresh_token_path, session.refresh_token.as_ref())
    {
        Some(
            windmill_common::secret_backend::database::encrypt_for_database(db, w_id, refresh_token)
                .await?,
        )
    } else {
        None
    };

    let mut tx = db.begin().await?;

    if let (Some(access_token_path), Some(encrypted_access_token)) =
        (access_token_path, encrypted_access_token)
    {
        upsert_secret_variable_in_tx(
            &mut *tx,
            w_id,
            access_token_path,
            encrypted_access_token,
            "OpenAI ChatGPT Account access token",
            edited_by,
        )
        .await?;
    }

    if let (Some(refresh_token_path), Some(encrypted_refresh_token)) =
        (refresh_token_path, encrypted_refresh_token)
    {
        upsert_secret_variable_in_tx(
            &mut *tx,
            w_id,
            refresh_token_path,
            encrypted_refresh_token,
            "OpenAI ChatGPT Account refresh token",
            edited_by,
        )
        .await?;
    }

    let mut resource_value = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = $3",
    )
    .bind(w_id)
    .bind(resource_path)
    .bind(OPENAI_CHATGPT_ACCOUNT_RESOURCE)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Could not find the resource {resource_path}")))?;

    let object = resource_value.as_object_mut().ok_or_else(|| {
        Error::BadRequest(format!("Invalid resource value for {resource_path}"))
    })?;

    if variable_path(access_token_reference).is_none() {
        object.insert(
            "access_token".to_string(),
            serde_json::Value::String(session.access_token.clone()),
        );
    }
    if variable_path(refresh_token_reference).is_none() {
        if let Some(refresh_token) = session.refresh_token.as_ref() {
            object.insert(
                "refresh_token".to_string(),
                serde_json::Value::String(refresh_token.clone()),
            );
        }
    }
    object.insert(
        "account_id".to_string(),
        serde_json::Value::String(account_id.to_string()),
    );
    object.insert(
        "expires_at".to_string(),
        serde_json::Value::String(expires_at.to_rfc3339()),
    );

    sqlx::query(
        "UPDATE resource SET value = $1, edited_at = now() WHERE workspace_id = $2 AND path = $3",
    )
        .bind(resource_value)
        .bind(w_id)
        .bind(resource_path)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

fn variable_path(reference: Option<&str>) -> Option<&str> {
    reference.and_then(|value| value.strip_prefix("$var:"))
}

async fn upsert_secret_variable_in_tx(
    conn: &mut sqlx::PgConnection,
    w_id: &str,
    path: &str,
    encrypted_value: String,
    description: &str,
    username: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, edited_by)
         VALUES ($1, $2, $3, true, $4, $5)
         ON CONFLICT (workspace_id, path) DO UPDATE SET
           value = EXCLUDED.value,
           is_secret = true,
           description = EXCLUDED.description,
           edited_by = EXCLUDED.edited_by,
           edited_at = now()",
    )
    .bind(w_id)
    .bind(path)
    .bind(encrypted_value)
    .bind(description)
    .bind(username)
    .execute(conn)
    .await?;

    Ok(())
}

fn extract_chatgpt_account_id(access_token: &str) -> Result<String> {
    let payload = decode_jwt_payload(access_token)?;
    payload
        .get(CODEX_ACCOUNT_CLAIM)
        .and_then(|claim| claim.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            Error::BadRequest(
                "OpenAI token does not contain a ChatGPT account id. Use Codex ChatGPT login."
                    .to_string(),
            )
        })
}

fn extract_jwt_expiry(access_token: &str) -> Option<DateTime<Utc>> {
    let payload = decode_jwt_payload(access_token).ok()?;
    let exp = payload.get("exp")?.as_i64()?;
    DateTime::<Utc>::from_timestamp(exp, 0)
}

fn decode_jwt_payload(access_token: &str) -> Result<serde_json::Value> {
    let payload = access_token
        .split('.')
        .nth(1)
        .ok_or_else(|| Error::BadRequest("Invalid JWT access token".to_string()))?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| Error::BadRequest(format!("Invalid JWT payload: {e}")))?;
    serde_json::from_slice(&decoded)
        .map_err(|e| Error::BadRequest(format!("Invalid JWT payload JSON: {e}")))
}

fn token_expires_soon(access_token: &str, expires_at: Option<&str>) -> bool {
    let expiry = expires_at
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| extract_jwt_expiry(access_token));

    expiry.is_some_and(|dt| dt <= Utc::now() + ChronoDuration::seconds(60))
}

#[derive(Clone, Debug)]
pub struct ExpiringProviderCredentials {
    credentials: ProviderCredentials,
    expires_at: std::time::Instant,
    instance_ai_config_revision: Option<u64>,
}

impl ExpiringProviderCredentials {
    fn new(credentials: ProviderCredentials, instance_ai_config_revision: Option<u64>) -> Self {
        Self {
            credentials,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
            instance_ai_config_revision,
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
            || self
                .instance_ai_config_revision
                .is_some_and(|revision| revision != current_instance_ai_config_revision())
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AIConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<HashMap<AIProvider, ProviderConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_completion_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_prompts: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_model: Option<HashMap<String, i32>>,
}

impl AIConfig {
    pub fn has_providers(&self) -> bool {
        self.providers
            .as_ref()
            .is_some_and(|providers| !providers.is_empty())
    }
}

pub fn global_service() -> Router {
    Router::new().route("/proxy/{*ai}", post(global_proxy).get(global_proxy))
}

pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/proxy/{*ai}", post(proxy).get(proxy))
        .route(
            "/openai_chatgpt_account/device/start",
            post(start_openai_chatgpt_account_device_auth),
        )
        .route(
            "/openai_chatgpt_account/device/complete",
            post(complete_openai_chatgpt_account_device_auth),
        )
        .route(
            "/openai_chatgpt_account/status",
            post(get_openai_chatgpt_account_status),
        );

    #[cfg(feature = "bedrock")]
    let router = router.route("/check_bedrock_credentials", get(check_bedrock_credentials));

    router
}

/// Check if AWS Bedrock credentials are available from environment variables.
#[cfg(feature = "bedrock")]
async fn check_bedrock_credentials(
    _authed: ApiAuthed,
    Path(_w_id): Path<String>,
) -> Result<Json<windmill_ai::ai_bedrock::BedrockCredentialsCheck>> {
    let response = windmill_ai::ai_bedrock::check_env_credentials().await;
    Ok(Json(response))
}

fn is_sse_response(headers: &HeaderMap) -> bool {
    headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false)
}

fn proxy_request_to_request_builder(proxy_request: ProxyRequest) -> RequestBuilder {
    let mut request = HTTP_CLIENT.request(proxy_request.method.clone(), &proxy_request.url);
    for (header_name, header_value) in &proxy_request.headers {
        request = request.header(header_name.as_str(), header_value.as_str());
    }
    request.body(proxy_request.body)
}

async fn audit_global_ai_request(db: &DB, authed: &ApiAuthed) -> Result<()> {
    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        authed,
        "ai.global_request",
        ActionKind::Execute,
        "global",
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(())
}

fn google_ai_proxy_response_to_body(
    response: GoogleAIProxyResponse,
) -> (http::StatusCode, HeaderMap, axum::body::Body) {
    let body = match response.body {
        GoogleAIProxyResponseBody::Fixed(body) => axum::body::Body::from(body),
        GoogleAIProxyResponseBody::Stream(stream) => axum::body::Body::from_stream(
            inject_keepalives(stream, Duration::from_secs(KEEPALIVE_INTERVAL_SECS)),
        ),
    };

    (response.status_code, response.headers, body)
}

#[cfg(feature = "bedrock")]
fn bedrock_proxy_response_to_body(
    response: BedrockProxyResponse,
) -> (http::StatusCode, HeaderMap, axum::body::Body) {
    let body = match response.body {
        BedrockProxyResponseBody::Fixed(body) => axum::body::Body::from(body),
        BedrockProxyResponseBody::Stream(stream) => axum::body::Body::from_stream(stream),
    };

    (response.status_code, response.headers, body)
}

pub(crate) fn inject_keepalives<S>(
    upstream: S,
    interval: Duration,
) -> impl futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>>
where
    S: futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin,
{
    async_stream::stream! {
        tokio::pin!(upstream);
        loop {
            tokio::select! {
                biased;
                chunk = upstream.next() => {
                    match chunk {
                        Some(item) => yield item,
                        None => break,
                    }
                }
                _ = tokio::time::sleep(interval) => {
                    yield Ok(Bytes::from(": keepalive\n\n"));
                }
            }
        }
    }
}

async fn global_proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(ai_path): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let api_key = headers
        .get("X-API-Key")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let account_id = headers
        .get("X-Account-Id")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let Some(api_key) = api_key else {
        return Err(Error::BadRequest("API key is required".to_string()));
    };

    let proxy_mode = proxy_execution_mode(&provider);

    if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
        return Err(Error::BadRequest(
            "AWS Bedrock global proxy is not supported; use a workspace AI resource with a region"
                .to_string(),
        ));
    }

    let base_url = provider.get_base_url(None, &db).await?;
    let mut custom_headers = HashMap::new();
    if matches!(provider, AIProvider::OpenAIChatGPTAccount) {
        if let Some(account_id) = account_id {
            custom_headers.insert("chatgpt-account-id".to_string(), account_id);
        }
    }

    let credentials = ProviderCredentials {
        provider: provider.clone(),
        base_url,
        api_key: Some(api_key.clone()),
        access_token: None,
        organization_id: None,
        user: None,
        region: None,
        aws_access_key_id: None,
        aws_secret_access_key: None,
        aws_session_token: None,
        platform: AIPlatform::Standard,
        enable_1m_context: false,
        custom_headers,
    };

    if matches!(proxy_mode, ProxyExecutionMode::NativeGoogleAi) {
        let proxy_args = ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        };

        audit_global_ai_request(&db, &authed).await?;

        let response = match ai_path.as_str() {
            "chat/completions" => handle_google_ai_chat_proxy(&HTTP_CLIENT, &proxy_args).await,
            "models" => handle_google_ai_models_proxy(&HTTP_CLIENT, &proxy_args).await,
            _ => Err(Error::BadRequest(format!(
                "Unsupported Google AI path: {}",
                ai_path
            ))),
        }?;

        return Ok(google_ai_proxy_response_to_body(response));
    }

    let request = match proxy_mode {
        ProxyExecutionMode::HttpForward => {
            let query_builder = create_query_builder(&credentials);
            let proxy_request = query_builder.build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: &ai_path,
                headers: &headers,
                body: &body,
                credentials: &credentials,
            })?;
            proxy_request_to_request_builder(proxy_request)
        }
        ProxyExecutionMode::NativeGoogleAi | ProxyExecutionMode::NativeAwsBedrock => {
            return Err(Error::BadRequest(format!(
                "Unsupported global proxy mode for provider {:?}",
                provider
            )))
        }
    };

    let response = request.send().await.map_err(to_anyhow)?;

    audit_global_ai_request(&db, &authed).await?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AIError(err_msg));
    }

    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();
    let body = if is_sse_response(&headers) {
        axum::body::Body::from_stream(inject_keepalives(
            stream,
            Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
        ))
    } else {
        axum::body::Body::from_stream(stream)
    };
    Ok((status_code, headers, body))
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, mut ai_path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    mut body: Bytes,
) -> impl IntoResponse {
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let workspace_cache = AI_REQUEST_CACHE.get(&(w_id.clone(), provider.clone()));

    let forced_resource_path = headers
        .get("X-Resource-Path")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let is_user_specified_resource = forced_resource_path.is_some();

    // When the caller supplies X-Resource-Path, the resource is treated as if it
    // were being read through the normal resource API: scope and RLS checks must
    // apply so that a low-privilege user cannot point the proxy at a restricted
    // AI resource (e.g. one in a folder they cannot read) to exfiltrate the
    // resource's provider credentials or use them via the proxy.
    if let Some(resource_path) = forced_resource_path.as_ref() {
        check_scopes(&authed, || format!("resources:read:{}", resource_path))?;
    }

    let mut credentials = match workspace_cache {
        Some(request_cache) if !request_cache.is_expired() && forced_resource_path.is_none() => {
            request_cache.credentials
        }
        _ => {
            let (resource_path, save_to_cache, resource_workspace, instance_ai_config_revision) =
                if let Some(resource_path) = forced_resource_path {
                    // forced resource path
                    (resource_path, false, w_id.clone(), None)
                } else {
                    let workspace_ai_config = sqlx::query_scalar!(
                        "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
                        &w_id
                    )
                    .fetch_one(&db)
                    .await?;

                    let (ai_config_value, resource_workspace, instance_ai_config_revision) = {
                        let ws_has_config = workspace_ai_config
                            .as_ref()
                            .and_then(|v| serde_json::from_value::<AIConfig>(v.clone()).ok())
                            .is_some_and(|config| config.has_providers());

                        if ws_has_config {
                            (workspace_ai_config.unwrap(), w_id.clone(), None)
                        } else {
                            let instance_config = sqlx::query_scalar!(
                                "SELECT value FROM global_settings WHERE name = 'ai_config'"
                            )
                            .fetch_optional(&db)
                            .await?;

                            match instance_config {
                                Some(config) => (
                                    config,
                                    "admins".to_string(),
                                    Some(current_instance_ai_config_revision()),
                                ),
                                None => {
                                    return Err(Error::internal_err(
                                        "AI resource not configured".to_string(),
                                    ));
                                }
                            }
                        }
                    };

                    let mut ai_config = serde_json::from_value::<AIConfig>(ai_config_value)
                        .map_err(|e| Error::BadRequest(e.to_string()))?;

                    let provider_config = ai_config
                        .providers
                        .as_mut()
                        .and_then(|providers| providers.remove(&provider))
                        .ok_or_else(|| {
                            Error::BadRequest(format!("Provider {:?} not configured", provider))
                        })?;

                    if provider_config.resource_path.is_empty() {
                        return Err(Error::BadRequest("Resource path is empty".to_string()));
                    }

                    (
                        provider_config.resource_path,
                        true,
                        resource_workspace,
                        instance_ai_config_revision,
                    )
                };

            // For user-specified resources, fetch through an RLS-scoped
            // connection so PostgreSQL row-level security enforces the same
            // folder/group boundaries as the regular resource API. For the
            // workspace/instance ai_config path, the resource_path was already
            // validated by an admin/devops user when configuring the workspace,
            // so the raw pool is used.
            let resource = if is_user_specified_resource {
                let mut tx = user_db.clone().begin(&authed).await?;
                let res = sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
                    "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
                )
                .bind(&resource_path)
                .bind(&resource_workspace)
                .fetch_optional(&mut *tx)
                .await?;
                tx.commit().await?;
                res
            } else {
                sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
                    "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
                )
                .bind(&resource_path)
                .bind(&resource_workspace)
                .fetch_optional(&db)
                .await?
            }
            .ok_or_else(|| Error::NotFound(format!("Could not find the resource {}, update the resource path in the workspace settings", resource_path)))?
            .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", resource_path)))?;

            let resource = serde_json::from_str::<AIResource>(resource.0.get())
                .map_err(|e| Error::BadRequest(e.to_string()))?;

            // Enforce RLS on $var: resolution when the resource path was
            // user-specified (X-Resource-Path header) so users can only read
            // variables they have permission to access.
            let enforce_authed = if is_user_specified_resource {
                Some(&authed)
            } else {
                None
            };
            let credentials = resolve_provider_credentials(
                &provider,
                &db,
                &resource_workspace,
                Some(resource_path.as_str()),
                resource,
                enforce_authed,
            )
            .await?;
            if save_to_cache {
                AI_REQUEST_CACHE.insert(
                    (w_id.clone(), provider.clone()),
                    ExpiringProviderCredentials::new(
                        credentials.clone(),
                        instance_ai_config_revision,
                    ),
                );
            }
            credentials
        }
    };

    if let Some(fim_transform) =
        maybe_transform_fim_request(&provider, &ai_path, &credentials.base_url, &body)?
    {
        if fim_transform.base_url.is_some() {
            tracing::debug!(
                "Routing native FIM request through provider-specific endpoint for {:?}",
                provider
            );
        } else {
            tracing::debug!(
                "Transforming FIM request to chat/completions with FIM tokens for provider {:?}",
                provider
            );
        }
        if let Some(base_url) = fim_transform.base_url {
            credentials.base_url = base_url;
        }
        body = fim_transform.body;
        ai_path = fim_transform.path;
    }

    let proxy_mode = proxy_execution_mode(&provider);

    // Handle GoogleAI (Gemini) using the native Gemini API
    if matches!(proxy_mode, ProxyExecutionMode::NativeGoogleAi) {
        let mut tx = db.begin().await?;
        audit_log(
            &mut *tx,
            &authed,
            "ai.request",
            ActionKind::Execute,
            &w_id,
            Some(&authed.email),
            Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
        )
        .await?;
        tx.commit().await?;

        let proxy_args = ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        };

        let response = match ai_path.as_str() {
            "chat/completions" => handle_google_ai_chat_proxy(&HTTP_CLIENT, &proxy_args).await,
            "models" => handle_google_ai_models_proxy(&HTTP_CLIENT, &proxy_args).await,
            _ => Err(Error::BadRequest(format!(
                "Unsupported Google AI path: {}",
                ai_path
            ))),
        }?;

        return Ok(google_ai_proxy_response_to_body(response));
    }

    // Handle Bedrock-specific logic when the feature is enabled
    #[cfg(feature = "bedrock")]
    if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
        let mut tx = db.begin().await?;
        audit_log(
            &mut *tx,
            &authed,
            "ai.request",
            ActionKind::Execute,
            &w_id,
            Some(&authed.email),
            Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
        )
        .await?;
        tx.commit().await?;

        let response = handle_bedrock_proxy(&ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        })
        .await?;

        return Ok(bedrock_proxy_response_to_body(response));
    }

    // When bedrock feature is disabled, return error for Bedrock provider
    #[cfg(not(feature = "bedrock"))]
    if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
        return Err(Error::BadRequest(
            "AWS Bedrock support is not enabled. Build with 'bedrock' feature.".to_string(),
        ));
    }

    let request = match proxy_mode {
        ProxyExecutionMode::HttpForward => {
            let query_builder = create_query_builder(&credentials);
            let proxy_request = query_builder.build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: &ai_path,
                headers: &headers,
                body: &body,
                credentials: &credentials,
            })?;
            proxy_request_to_request_builder(proxy_request)
        }
        ProxyExecutionMode::NativeGoogleAi => {
            return Err(Error::internal_err(
                "Google AI proxy route was not handled".to_string(),
            ))
        }
        ProxyExecutionMode::NativeAwsBedrock => {
            return Err(Error::BadRequest(
                "Unsupported AWS Bedrock proxy request".to_string(),
            ))
        }
    };

    let response = request.send().await.map_err(to_anyhow)?;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "ai.request",
        ActionKind::Execute,
        &w_id,
        Some(&authed.email),
        Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AIError(err_msg));
    }

    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();
    let body = if is_sse_response(&headers) {
        axum::body::Body::from_stream(inject_keepalives(
            stream,
            Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
        ))
    } else {
        axum::body::Body::from_stream(stream)
    };
    Ok((status_code, headers, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};
    use windmill_ai::ai_cache::bump_instance_ai_config_revision;
    use windmill_ai::ai_providers::AIPlatform;

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn sample_provider_credentials() -> ProviderCredentials {
        ProviderCredentials {
            provider: AIProvider::OpenAI,
            base_url: "https://example.com".to_string(),
            api_key: None,
            access_token: None,
            organization_id: None,
            user: None,
            region: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
            platform: AIPlatform::Standard,
            enable_1m_context: false,
            custom_headers: HashMap::new(),
        }
    }

    #[test]
    fn invalidates_all_cached_providers_for_workspace() {
        let _guard = TEST_LOCK.lock().unwrap();
        AI_REQUEST_CACHE.clear();
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::OpenAI),
            ExpiringProviderCredentials::new(sample_provider_credentials(), None),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::Anthropic),
            ExpiringProviderCredentials::new(sample_provider_credentials(), None),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-b".to_string(), AIProvider::OpenAI),
            ExpiringProviderCredentials::new(sample_provider_credentials(), None),
        );

        invalidate_ai_request_cache_for_workspace("workspace-a");

        assert!(AI_REQUEST_CACHE
            .get(&("workspace-a".to_string(), AIProvider::OpenAI))
            .is_none());
        assert!(AI_REQUEST_CACHE
            .get(&("workspace-a".to_string(), AIProvider::Anthropic))
            .is_none());
        assert!(AI_REQUEST_CACHE
            .get(&("workspace-b".to_string(), AIProvider::OpenAI))
            .is_some());
    }

    #[test]
    fn instance_backed_cache_entries_expire_when_revision_changes() {
        let _guard = TEST_LOCK.lock().unwrap();
        AI_REQUEST_CACHE.clear();

        let cached = ExpiringProviderCredentials::new(
            sample_provider_credentials(),
            Some(current_instance_ai_config_revision()),
        );
        assert!(!cached.is_expired());

        bump_instance_ai_config_revision();

        assert!(cached.is_expired());
    }
}
