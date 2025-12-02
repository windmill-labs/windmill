use crate::bedrock;
use crate::db::{ApiAuthed, DB};

use axum::{body::Bytes, extract::Path, response::IntoResponse, routing::post, Extension, Router};
use http::{HeaderMap, Method};
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::ai_providers::{AIProvider, ProviderConfig, ProviderModel};
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::utils::configure_client;
use windmill_common::variables::get_variable_or_self;

// AI timeout configuration constants
const AI_TIMEOUT_MIN_SECS: u64 = 1;
const AI_TIMEOUT_MAX_SECS: u64 = 86400; // 24 hours
const AI_TIMEOUT_DEFAULT_SECS: u64 = 3600; // 1 hour
const HTTP_POOL_MAX_IDLE_PER_HOST: usize = 10;
const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 90;

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

    static ref HTTP_CLIENT: Client = configure_client(reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(*AI_TIMEOUT_SECS))
        .pool_max_idle_per_host(HTTP_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Some(std::time::Duration::from_secs(HTTP_POOL_IDLE_TIMEOUT_SECS)))
        .user_agent("windmill/beta"))
        .build()
        .expect("Failed to build AI HTTP client - check system TLS configuration");

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringAIRequestConfig> = Cache::new(500);

    /// Parse AI_HTTP_HEADERS environment variable into a vector of (header_name, header_value) tuples
    /// Format: "header1: value1, header2: value2"
    static ref AI_HTTP_HEADERS: Vec<(String, String)> = {
        std::env::var("AI_HTTP_HEADERS")
            .ok()
            .map(|headers_str| {
                headers_str
                    .split(',')
                    .filter_map(|header| {
                        let parts: Vec<&str> = header.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let name = parts[0].trim().to_string();
                            let value = parts[1].trim().to_string();
                            if !name.is_empty() && !value.is_empty() {
                                Some((name, value))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
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
    #[serde(alias = "baseUrl")]
    base_url: Option<String>,
    #[serde(alias = "apiKey")]
    api_key: Option<String>,
    organization_id: Option<String>,
    region: Option<String>,
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
}

impl AIRequestConfig {
    pub async fn new(
        provider: &AIProvider,
        db: &DB,
        w_id: &str,
        resource: AIResource,
    ) -> Result<Self> {
        let (api_key, access_token, organization_id, base_url, user) = match resource {
            AIResource::Standard(resource) => {
                let base_url = provider
                    .get_base_url(resource.base_url, resource.region, db)
                    .await?;
                let api_key = if let Some(api_key) = resource.api_key {
                    Some(get_variable_or_self(api_key, db, w_id).await?)
                } else {
                    None
                };
                let organization_id = if let Some(organization_id) = resource.organization_id {
                    Some(get_variable_or_self(organization_id, db, w_id).await?)
                } else {
                    None
                };

                (api_key, None, organization_id, base_url, None)
            }
            AIResource::OAuth(resource) => {
                let user = if let Some(user) = resource.user.clone() {
                    Some(get_variable_or_self(user, db, w_id).await?)
                } else {
                    None
                };
                let token = Self::get_token_using_oauth(resource, db, w_id).await?;
                let base_url = provider.get_base_url(None, None, db).await?;

                (None, Some(token), None, base_url, user)
            }
        };

        Ok(Self { base_url, organization_id, api_key, access_token, user })
    }

    async fn get_token_using_oauth(
        mut resource: AIOAuthResource,
        db: &DB,
        w_id: &str,
    ) -> Result<String> {
        resource.client_id = get_variable_or_self(resource.client_id, db, w_id).await?;
        resource.client_secret = get_variable_or_self(resource.client_secret, db, w_id).await?;
        resource.token_url = get_variable_or_self(resource.token_url, db, w_id).await?;
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

    pub fn prepare_request(
        self,
        provider: &AIProvider,
        path: &str,
        method: Method,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<RequestBuilder> {
        let body = if let Some(user) = self.user {
            Self::add_user_to_body(body, user)?
        } else {
            body
        };

        let base_url = self.base_url.trim_end_matches('/');

        let is_azure = provider.is_azure_openai(base_url);
        let is_anthropic = matches!(provider, AIProvider::Anthropic);
        let is_anthropic_sdk = headers.get("X-Anthropic-SDK").is_some();
        let is_bedrock = matches!(provider, AIProvider::AWSBedrock);

        // Handle AWS Bedrock transformation
        let (url, body) = if is_bedrock && method != Method::GET {
            let (model, transformed_body, is_streaming) =
                bedrock::transform_openai_to_bedrock(&body)?;
            let endpoint = if is_streaming {
                "converse-stream"
            } else {
                "converse"
            };
            let bedrock_url = format!("{}/model/{}/{}", base_url, model, endpoint);
            (bedrock_url, transformed_body)
        } else if is_bedrock && (path == "foundation-models" || path == "inference-profiles") {
            // AWS Bedrock foundation-models and inference-profiles endpoints use different base URL (without -runtime)
            let bedrock_base_url = base_url.replace("bedrock-runtime.", "bedrock.");
            let bedrock_url = format!("{}/{}", bedrock_base_url, path);
            (bedrock_url, body)
        } else if is_azure {
            let azure_url = AIProvider::build_azure_openai_url(base_url, path);
            (azure_url, body)
        } else if is_anthropic_sdk {
            let truncated_base_url = base_url.trim_end_matches("/v1");
            let anthropic_url = format!("{}/{}", truncated_base_url, path);
            (anthropic_url, body)
        } else {
            let default_url = format!("{}/{}", base_url, path);
            (default_url, body)
        };

        tracing::debug!("AI request URL: {}", url);

        let mut request = HTTP_CLIENT
            .request(method, url)
            .header("content-type", "application/json");

        for (header_name, header_value) in headers.iter() {
            if header_name.to_string().starts_with("anthropic-") {
                request = request.header(header_name, header_value);
            }
        }

        request = request.body(body);

        if let Some(api_key) = self.api_key {
            if is_azure {
                request = request.header("api-key", api_key.clone())
            } else {
                request = request.header("authorization", format!("Bearer {}", api_key.clone()))
            }
            if is_anthropic {
                request = request.header("X-API-Key", api_key);
            }
        }

        if let Some(access_token) = self.access_token {
            request = request.header("authorization", format!("Bearer {}", access_token))
        }

        if let Some(org_id) = self.organization_id {
            request = request.header("OpenAI-Organization", org_id);
        }

        // Apply custom headers from AI_HTTP_HEADERS environment variable
        for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
            request = request.header(header_name.as_str(), header_value.as_str());
        }

        Ok(request)
    }

    fn add_user_to_body(body: Bytes, user: String) -> Result<Bytes> {
        tracing::debug!("Adding user to request body");
        let mut json_body: HashMap<String, Box<RawValue>> = serde_json::from_slice(&body)
            .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;

        let user_json_string = serde_json::Value::String(user).to_string(); // makes sure to escape characters

        json_body.insert(
            "user".to_string(),
            RawValue::from_string(user_json_string)
                .map_err(|e| Error::internal_err(format!("Failed to parse user: {}", e)))?,
        );

        Ok(serde_json::to_vec(&json_body)
            .map_err(|e| Error::internal_err(format!("Failed to reserialize request body: {}", e)))?
            .into())
    }
}

#[derive(Clone, Debug)]
pub struct ExpiringAIRequestConfig {
    config: AIRequestConfig,
    expires_at: std::time::Instant,
}

impl ExpiringAIRequestConfig {
    fn new(config: AIRequestConfig) -> Self {
        Self { config, expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60) }
    }
    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

pub fn global_service() -> Router {
    Router::new().route("/proxy/*ai", post(global_proxy).get(global_proxy))
}

pub fn workspaced_service() -> Router {
    Router::new().route("/proxy/*ai", post(proxy).get(proxy))
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

    let base_url = provider.get_base_url(None, None, &db).await?;

    let url = format!("{}/{}", base_url, ai_path);

    let mut request = HTTP_CLIENT
        .request(method, url)
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key));

    // Apply custom headers from AI_HTTP_HEADERS environment variable
    for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
        request = request.header(header_name.as_str(), header_value.as_str());
    }

    let request = request.body(body);

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
    Ok((status_code, headers, axum::body::Body::from_stream(stream)))
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, ai_path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
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
    let request_config = match workspace_cache {
        Some(request_cache) if !request_cache.is_expired() && forced_resource_path.is_none() => {
            request_cache.config
        }
        _ => {
            let (resource_path, save_to_cache) = if let Some(resource_path) = forced_resource_path {
                // forced resource path
                (resource_path, false)
            } else {
                let ai_config = sqlx::query_scalar!(
                    "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
                    &w_id
                )
                .fetch_one(&db)
                .await?;

                if ai_config.is_none() {
                    return Err(Error::internal_err(
                        "AI resource not configured".to_string(),
                    ));
                }

                let mut ai_config = serde_json::from_value::<AIConfig>(ai_config.unwrap())
                    .map_err(|e| Error::BadRequest(e.to_string()))?;

                let provider_config = ai_config
                    .providers
                    .as_mut()
                    .map(|providers| providers.remove(&provider))
                    .flatten()
                    .ok_or_else(|| {
                        Error::BadRequest(format!("Provider {:?} not configured", provider))
                    })?;

                if provider_config.resource_path.is_empty() {
                    return Err(Error::BadRequest("Resource path is empty".to_string()));
                }

                (provider_config.resource_path, true)
            };

            let resource= sqlx::query_scalar!(
                "SELECT value as \"value: sqlx::types::Json<Box<RawValue>>\" FROM resource WHERE path = $1 AND workspace_id = $2",
                &resource_path,
                &w_id
            )
            .fetch_optional(&db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Could not find the resource {}, update the resource path in the workspace settings", resource_path)))?
            .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", resource_path)))?;

            let resource = serde_json::from_str::<AIResource>(resource.0.get())
                .map_err(|e| Error::BadRequest(e.to_string()))?;

            let request_config = AIRequestConfig::new(&provider, &db, &w_id, resource).await?;
            if save_to_cache {
                AI_REQUEST_CACHE.insert(
                    (w_id.clone(), provider.clone()),
                    ExpiringAIRequestConfig::new(request_config.clone()),
                );
            }
            request_config
        }
    };

    // Extract model and streaming flag for Bedrock transformation (only for POST requests)
    let (model, is_streaming) =
        if matches!(provider, AIProvider::AWSBedrock) && method == Method::POST {
            #[derive(Deserialize, Debug)]
            struct BedrockRequest {
                model: String,
                stream: bool,
            }
            let parsed: BedrockRequest = serde_json::from_slice(&body)
                .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;
            (Some(parsed.model), parsed.stream)
        } else {
            (None, false)
        };

    let request = request_config.prepare_request(&provider, &ai_path, method, headers, body)?;

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

    // Transform Bedrock responses back to OpenAI format
    if matches!(provider, AIProvider::AWSBedrock) && model.is_some() {
        if is_streaming {
            // Transform streaming response
            use http::StatusCode;

            let mut response_headers = HeaderMap::new();
            response_headers.insert("content-type", "text/event-stream".parse().unwrap());
            response_headers.insert("cache-control", "no-cache".parse().unwrap());
            response_headers.insert("connection", "keep-alive".parse().unwrap());

            let stream = response.bytes_stream();
            let transformed_stream =
                bedrock::transform_bedrock_stream_to_openai(stream, model.unwrap());

            Ok((
                StatusCode::OK,
                response_headers,
                axum::body::Body::from_stream(transformed_stream),
            ))
        } else {
            // Transform non-streaming response
            let transformed_body =
                bedrock::transform_bedrock_to_openai(response, model.unwrap()).await?;

            let mut response_headers = HeaderMap::new();
            response_headers.insert("content-type", "application/json".parse().unwrap());

            Ok((
                http::StatusCode::OK,
                response_headers,
                axum::body::Body::from(transformed_body),
            ))
        }
    } else {
        // Pass through for other providers
        let status_code = response.status();
        let headers = response.headers().clone();
        let stream = response.bytes_stream();
        Ok((status_code, headers, axum::body::Body::from_stream(stream)))
    }
}
