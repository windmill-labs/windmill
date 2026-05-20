#[cfg(feature = "bedrock")]
use crate::bedrock;
use crate::db::{ApiAuthed, DB};

#[cfg(feature = "bedrock")]
use axum::routing::get;
#[cfg(feature = "bedrock")]
use axum::Json;
use axum::{body::Bytes, extract::Path, response::IntoResponse, routing::post, Extension, Router};
use futures::StreamExt;
use http::{HeaderMap, Method};
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use std::collections::HashMap;
use std::time::Duration;
use windmill_ai::ai_cache::current_instance_ai_config_revision;
use windmill_ai::ai_providers::{
    empty_string_as_none, AIPlatform, AIProvider, ProviderConfig, ProviderModel,
};
use windmill_ai::providers::{
    create_proxy_query_builder,
    google_ai::{
        handle_google_ai_chat_proxy, handle_google_ai_models_proxy, GoogleAIProxyResponse,
        GoogleAIProxyResponseBody,
    },
};
use windmill_ai::proxy::{
    proxy_execution_mode, supports_query_builder_proxy, ProviderCredentials, ProxyBuildArgs,
    ProxyExecutionMode, ProxyRequest,
};
use windmill_ai::utils::AI_HTTP_HEADERS;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::db::UserDB;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::utils::configure_client;
use windmill_common::variables::{get_variable_or_self, get_variable_or_self_as};

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
        .user_agent("windmill/beta"))
        .build()
        .expect("Failed to build AI HTTP client - check system TLS configuration");

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringAIRequestConfig> = Cache::new(500);

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

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AIResource {
    OAuth(AIOAuthResource),
    Standard(AIStandardResource),
}

#[derive(Deserialize, Clone, Debug)]
struct AIRequestConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub organization_id: Option<String>,
    pub user: Option<String>,
    #[allow(dead_code)]
    pub region: Option<String>,
    #[allow(dead_code)]
    pub aws_access_key_id: Option<String>,
    #[allow(dead_code)]
    pub aws_secret_access_key: Option<String>,
    #[allow(dead_code)]
    pub aws_session_token: Option<String>,
    pub platform: AIPlatform,
    pub enable_1m_context: bool,
    pub custom_headers: HashMap<String, String>,
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

impl AIRequestConfig {
    pub async fn new(
        provider: &AIProvider,
        db: &DB,
        w_id: &str,
        resource: AIResource,
        authed: Option<&ApiAuthed>,
    ) -> Result<Self> {
        // When authed is provided, resolve $var: references through RLS so that
        // users can only read variables they have permission to access.
        let user_db = authed.map(|_| UserDB::new(db.clone()));

        let (
            api_key,
            access_token,
            organization_id,
            base_url,
            user,
            region,
            aws_access_key_id,
            aws_secret_access_key,
            aws_session_token,
            platform,
            enable_1m_context,
            custom_headers,
        ) = match resource {
            AIResource::Standard(resource) => {
                let region = resource.region.clone();
                let platform = resource.platform.clone();
                let enable_1m_context = resource.enable_1m_context;
                let custom_headers = resource.headers.clone();
                // Skip get_base_url for Bedrock - it uses SDK directly, not HTTP
                let base_url = if matches!(provider, AIProvider::AWSBedrock) {
                    String::new()
                } else {
                    provider.get_base_url(resource.base_url, db).await?
                };
                let api_key = if let Some(api_key) = resource.api_key {
                    Some(resolve_var(api_key, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
                let organization_id = if let Some(organization_id) = resource.organization_id {
                    Some(resolve_var(organization_id, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
                let aws_access_key_id = if let Some(access_key_id) = resource.aws_access_key_id {
                    Some(resolve_var(access_key_id, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
                let aws_secret_access_key = if let Some(secret_access_key) =
                    resource.aws_secret_access_key
                {
                    Some(resolve_var(secret_access_key, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
                let aws_session_token = if let Some(session_token) = resource.aws_session_token {
                    Some(resolve_var(session_token, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };

                (
                    api_key,
                    None,
                    organization_id,
                    base_url,
                    None,
                    region,
                    aws_access_key_id,
                    aws_secret_access_key,
                    aws_session_token,
                    platform,
                    enable_1m_context,
                    custom_headers,
                )
            }
            AIResource::OAuth(resource) => {
                let user = if let Some(user) = resource.user.clone() {
                    Some(resolve_var(user, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
                let token =
                    Self::get_token_using_oauth(resource, db, w_id, user_db.as_ref(), authed)
                        .await?;
                let base_url = provider.get_base_url(None, db).await?;

                (
                    None,
                    Some(token),
                    None,
                    base_url,
                    user,
                    None,
                    None,
                    None,
                    None,
                    AIPlatform::Standard,
                    false,
                    HashMap::new(),
                )
            }
        };

        Ok(Self {
            base_url,
            organization_id,
            api_key,
            access_token,
            user,
            region,
            aws_access_key_id,
            aws_secret_access_key,
            aws_session_token,
            platform,
            enable_1m_context,
            custom_headers,
        })
    }

    async fn get_token_using_oauth(
        mut resource: AIOAuthResource,
        db: &DB,
        w_id: &str,
        user_db: Option<&UserDB>,
        authed: Option<&ApiAuthed>,
    ) -> Result<String> {
        resource.client_id = resolve_var(resource.client_id, db, w_id, user_db, authed).await?;
        resource.client_secret =
            resolve_var(resource.client_secret, db, w_id, user_db, authed).await?;
        resource.token_url = resolve_var(resource.token_url, db, w_id, user_db, authed).await?;
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

    fn into_provider_credentials(self, provider: AIProvider) -> ProviderCredentials {
        ProviderCredentials {
            provider,
            base_url: self.base_url,
            api_key: self.api_key,
            access_token: self.access_token,
            organization_id: self.organization_id,
            user: self.user,
            region: self.region,
            aws_access_key_id: self.aws_access_key_id,
            aws_secret_access_key: self.aws_secret_access_key,
            aws_session_token: self.aws_session_token,
            platform: self.platform,
            enable_1m_context: self.enable_1m_context,
            custom_headers: self.custom_headers,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExpiringAIRequestConfig {
    config: AIRequestConfig,
    expires_at: std::time::Instant,
    instance_ai_config_revision: Option<u64>,
}

impl ExpiringAIRequestConfig {
    fn new(config: AIRequestConfig, instance_ai_config_revision: Option<u64>) -> Self {
        Self {
            config,
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

// FIM (Fill-in-the-Middle) simulation for providers that don't support native FIM
#[derive(Deserialize, Debug)]
struct FimRequest {
    model: String,
    prompt: String,         // code before cursor
    suffix: Option<String>, // code after cursor
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stop: Option<Vec<String>>,
}

/// Checks if the AI provider supports native FIM (Fill-in-the-Middle) endpoint
fn supports_native_fim(provider: &AIProvider) -> bool {
    matches!(provider, AIProvider::Mistral)
}

/// Transforms a FIM request to chat/completions format for providers that don't support native FIM.
fn transform_fim_to_chat_completions(body: &Bytes) -> Result<(Bytes, String)> {
    let fim_req: FimRequest = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse FIM request: {}", e)))?;

    let suffix = fim_req.suffix.unwrap_or_default();

    let system_prompt = "You are a code completion assistant. Complete the code at the <CURSOR/> position between the given prefix and suffix. Output ONLY the code that goes at the cursor - no explanations, no markdown, no repeating the prefix or suffix.";

    let user_content = format!(
        "<PREFIX>\n{}\n<CURSOR/>\n<SUFFIX>\n{}",
        fim_req.prompt, suffix
    );

    let chat_req = json!({
        "model": fim_req.model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content}
        ],
        "temperature": fim_req.temperature.unwrap_or(0.0),
        "max_tokens": fim_req.max_tokens.unwrap_or(256),
        "stop": fim_req.stop
    });

    let chat_body = serde_json::to_vec(&chat_req)
        .map_err(|e| Error::internal_err(format!("Failed to serialize chat request: {}", e)))?;

    Ok((Bytes::from(chat_body), "chat/completions".to_string()))
}

pub fn global_service() -> Router {
    Router::new().route("/proxy/{*ai}", post(global_proxy).get(global_proxy))
}

pub fn workspaced_service() -> Router {
    let router = Router::new().route("/proxy/{*ai}", post(proxy).get(proxy));

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

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let Some(api_key) = api_key else {
        return Err(Error::BadRequest("API key is required".to_string()));
    };

    let base_url = provider.get_base_url(None, &db).await?;

    let request = if supports_query_builder_proxy(&provider) {
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
            custom_headers: HashMap::new(),
        };
        let query_builder = create_proxy_query_builder(&credentials);
        let proxy_request = query_builder.build_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        })?;
        proxy_request_to_request_builder(proxy_request)
    } else {
        let url = format!("{}/{}", base_url, ai_path);
        let mut request = HTTP_CLIENT
            .request(method, url)
            .header("content-type", "application/json")
            .header("Authorization", format!("Bearer {}", &api_key));

        // Apply custom headers from AI_HTTP_HEADERS environment variable
        for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
            request = request.header(header_name.as_str(), header_value.as_str());
        }

        request.body(body)
    };

    let response = request.send().await.map_err(to_anyhow)?;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "ai.global_request",
        ActionKind::Execute,
        "global",
        Some(&authed.email),
        None,
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

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
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
    let request_config = match workspace_cache {
        Some(request_cache) if !request_cache.is_expired() && forced_resource_path.is_none() => {
            request_cache.config
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

            let resource = sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
                "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
            )
            .bind(&resource_path)
            .bind(&resource_workspace)
            .fetch_optional(&db)
            .await?
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
            let request_config = AIRequestConfig::new(
                &provider,
                &db,
                &resource_workspace,
                resource,
                enforce_authed,
            )
            .await?;
            if save_to_cache {
                AI_REQUEST_CACHE.insert(
                    (w_id.clone(), provider.clone()),
                    ExpiringAIRequestConfig::new(
                        request_config.clone(),
                        instance_ai_config_revision,
                    ),
                );
            }
            request_config
        }
    };

    // Check if this is a FIM request to a provider that doesn't support native FIM endpoint
    // For such providers, transform to use FIM sentinel tokens with the chat/completions endpoint
    let is_fim_request = ai_path.contains("fim/completions");
    if is_fim_request && !supports_native_fim(&provider) {
        tracing::debug!(
            "Transforming FIM request to chat/completions with FIM tokens for provider {:?}",
            provider
        );
        let (chat_body, chat_path) = transform_fim_to_chat_completions(&body)?;
        body = chat_body;
        ai_path = chat_path;
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

        let credentials = request_config.into_provider_credentials(provider.clone());
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
    {
        // Extract model and streaming flag for Bedrock transformation (only for POST requests)
        let (model, is_streaming) = if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock)
            && method == Method::POST
        {
            #[derive(Deserialize, Debug)]
            struct BedrockRequest {
                model: String,
                #[serde(default)]
                stream: bool,
            }
            let parsed: BedrockRequest = serde_json::from_slice(&body)
                .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;
            (Some(parsed.model), parsed.stream)
        } else {
            (None, false)
        };

        // For Bedrock requests, use the SDK-based approach
        if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
            let region = request_config
                .region
                .as_deref()
                .unwrap_or(windmill_ai::ai_providers::USE_ENV_REGION);

            // Audit log before making the SDK request
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

            // Handle GET requests for control plane operations
            if method == Method::GET {
                if ai_path == "foundation-models" {
                    return bedrock::list_foundation_models(
                        request_config.api_key.as_deref(),
                        request_config.aws_access_key_id.as_deref(),
                        request_config.aws_secret_access_key.as_deref(),
                        request_config.aws_session_token.as_deref(),
                        region,
                    )
                    .await;
                } else if ai_path == "inference-profiles" {
                    return bedrock::list_inference_profiles(
                        request_config.api_key.as_deref(),
                        request_config.aws_access_key_id.as_deref(),
                        request_config.aws_secret_access_key.as_deref(),
                        request_config.aws_session_token.as_deref(),
                        region,
                    )
                    .await;
                }
            }

            // Handle POST requests for inference
            if method == Method::POST && model.is_some() {
                if is_streaming {
                    return bedrock::handle_bedrock_sdk_streaming(
                        model.as_ref().unwrap(),
                        &body,
                        request_config.api_key.as_deref(),
                        request_config.aws_access_key_id.as_deref(),
                        request_config.aws_secret_access_key.as_deref(),
                        request_config.aws_session_token.as_deref(),
                        region,
                    )
                    .await;
                } else {
                    return bedrock::handle_bedrock_sdk_non_streaming(
                        model.as_ref().unwrap(),
                        &body,
                        request_config.api_key.as_deref(),
                        request_config.aws_access_key_id.as_deref(),
                        request_config.aws_secret_access_key.as_deref(),
                        request_config.aws_session_token.as_deref(),
                        region,
                    )
                    .await;
                }
            }
        }
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
            let credentials = request_config.into_provider_credentials(provider.clone());
            let query_builder = create_proxy_query_builder(&credentials);
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

    fn sample_request_config() -> AIRequestConfig {
        AIRequestConfig {
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
    fn maps_request_config_to_provider_credentials() {
        let mut custom_headers = HashMap::new();
        custom_headers.insert("X-Test".to_string(), "yes".to_string());

        let config = AIRequestConfig {
            base_url: "https://example.com".to_string(),
            api_key: Some("api-key".to_string()),
            access_token: Some("access-token".to_string()),
            organization_id: Some("org-id".to_string()),
            user: Some("user-id".to_string()),
            region: Some("us-east-1".to_string()),
            aws_access_key_id: Some("aws-access-key".to_string()),
            aws_secret_access_key: Some("aws-secret-key".to_string()),
            aws_session_token: Some("aws-session-token".to_string()),
            platform: AIPlatform::GoogleVertexAi,
            enable_1m_context: true,
            custom_headers,
        };

        let credentials = config.into_provider_credentials(AIProvider::Anthropic);

        assert_eq!(credentials.provider, AIProvider::Anthropic);
        assert_eq!(credentials.base_url, "https://example.com");
        assert_eq!(credentials.api_key.as_deref(), Some("api-key"));
        assert_eq!(credentials.access_token.as_deref(), Some("access-token"));
        assert_eq!(credentials.organization_id.as_deref(), Some("org-id"));
        assert_eq!(credentials.user.as_deref(), Some("user-id"));
        assert_eq!(credentials.region.as_deref(), Some("us-east-1"));
        assert_eq!(
            credentials.aws_access_key_id.as_deref(),
            Some("aws-access-key")
        );
        assert_eq!(
            credentials.aws_secret_access_key.as_deref(),
            Some("aws-secret-key")
        );
        assert_eq!(
            credentials.aws_session_token.as_deref(),
            Some("aws-session-token")
        );
        assert_eq!(credentials.platform, AIPlatform::GoogleVertexAi);
        assert!(credentials.enable_1m_context);
        assert_eq!(
            credentials.custom_headers.get("X-Test").map(String::as_str),
            Some("yes")
        );
    }

    #[test]
    fn invalidates_all_cached_providers_for_workspace() {
        let _guard = TEST_LOCK.lock().unwrap();
        AI_REQUEST_CACHE.clear();
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::OpenAI),
            ExpiringAIRequestConfig::new(sample_request_config(), None),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::Anthropic),
            ExpiringAIRequestConfig::new(sample_request_config(), None),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-b".to_string(), AIProvider::OpenAI),
            ExpiringAIRequestConfig::new(sample_request_config(), None),
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

        let cached = ExpiringAIRequestConfig::new(
            sample_request_config(),
            Some(current_instance_ai_config_revision()),
        );
        assert!(!cached.is_expired());

        bump_instance_ai_config_revision();

        assert!(cached.is_expired());
    }
}
