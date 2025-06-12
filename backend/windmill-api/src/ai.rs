use crate::{
    db::{ApiAuthed, DB},
    variables::get_variable_or_self,
};

use axum::{body::Bytes, extract::Path, response::IntoResponse, routing::post, Extension, Router};
use http::HeaderMap;
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::error::{to_anyhow, Error, Result};

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringAIRequestConfig> = Cache::new(500);
}

const AZURE_API_VERSION: &str = "2025-04-01-preview";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

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
                let base_url = provider.get_base_url(resource.base_url, db).await?;
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
                let base_url = provider.get_base_url(None, db).await?;

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
        body: Bytes,
    ) -> Result<RequestBuilder> {
        let body = if let Some(user) = self.user {
            Self::add_user_to_body(body, user)?
        } else {
            body
        };

        let base_url = self.base_url.trim_end_matches('/');

        let is_azure = matches!(provider, AIProvider::OpenAI) && base_url != OPENAI_BASE_URL
            || matches!(provider, AIProvider::AzureOpenAI);

        let url = if is_azure {
            if base_url.ends_with("/deployments") {
                let model = Self::get_azure_model(&body)?;
                format!("{}/{}/{}", base_url, model, path)
            } else if base_url.ends_with("/openai") {
                let model = Self::get_azure_model(&body)?;
                format!("{}/deployments/{}/{}", base_url, model, path)
            } else {
                format!("{}/{}", base_url, path)
            }
        } else {
            format!("{}/{}", base_url, path)
        };

        tracing::debug!("AI request URL: {}", url);

        let mut request = HTTP_CLIENT
            .post(url)
            .header("content-type", "application/json")
            .body(body);

        if is_azure {
            request = request.query(&[("api-version", AZURE_API_VERSION)])
        }

        if let Some(api_key) = self.api_key {
            if is_azure {
                request = request.header("api-key", api_key)
            } else {
                request = request.header("authorization", format!("Bearer {}", api_key))
            }
        }

        if let Some(access_token) = self.access_token {
            request = request.header("authorization", format!("Bearer {}", access_token))
        }

        if let Some(org_id) = self.organization_id {
            request = request.header("OpenAI-Organization", org_id);
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

    fn get_azure_model(body: &Bytes) -> Result<String> {
        #[derive(Deserialize, Debug)]
        struct AzureModel {
            model: String,
        }

        let azure_model: AzureModel = serde_json::from_slice(body)
            .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;

        Ok(azure_model.model)
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    OpenAI,
    #[serde(rename = "azure_openai")]
    AzureOpenAI,
    Anthropic,
    Mistral,
    DeepSeek,
    GoogleAI,
    Groq,
    OpenRouter,
    TogetherAI,
    CustomAI,
}

impl AIProvider {
    pub async fn get_base_url(&self, resource_base_url: Option<String>, db: &DB) -> Result<String> {
        match self {
            AIProvider::OpenAI => {
                let azure_base_path = sqlx::query_scalar!(
                    "SELECT value
                    FROM global_settings
                    WHERE name = 'openai_azure_base_path'",
                )
                .fetch_optional(db)
                .await?;

                let azure_base_path = if let Some(azure_base_path) = azure_base_path {
                    Some(
                        serde_json::from_value::<String>(azure_base_path).map_err(|e| {
                            Error::internal_err(format!("validating openai azure base path {e:#}"))
                        })?,
                    )
                } else {
                    OPENAI_AZURE_BASE_PATH.clone()
                };

                Ok(azure_base_path.unwrap_or(OPENAI_BASE_URL.to_string()))
            }
            AIProvider::DeepSeek => Ok("https://api.deepseek.com/v1".to_string()),
            AIProvider::GoogleAI => {
                Ok("https://generativelanguage.googleapis.com/v1beta/openai".to_string())
            }
            AIProvider::Groq => Ok("https://api.groq.com/openai/v1".to_string()),
            AIProvider::OpenRouter => Ok("https://openrouter.ai/api/v1".to_string()),
            AIProvider::TogetherAI => Ok("https://api.together.xyz/v1".to_string()),
            AIProvider::Anthropic => Ok("https://api.anthropic.com/v1".to_string()),
            AIProvider::Mistral => Ok("https://api.mistral.ai/v1".to_string()),
            p @ (AIProvider::CustomAI | AIProvider::AzureOpenAI) => {
                if let Some(base_url) = resource_base_url {
                    Ok(base_url)
                } else {
                    Err(Error::BadRequest(format!(
                        "{:?} provider requires a base URL in the resource",
                        p
                    )))
                }
            }
        }
    }
}

impl TryFrom<&str> for AIProvider {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self> {
        let s = serde_json::from_value::<AIProvider>(serde_json::Value::String(s.to_string()))
            .map_err(|e| Error::BadRequest(format!("Invalid AI provider: {}", e)))?;
        Ok(s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderConfig {
    pub resource_path: String,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderModel {
    pub model: String,
    pub provider: AIProvider,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AIConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<HashMap<AIProvider, ProviderConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_completion_model: Option<ProviderModel>,
}

pub fn global_service() -> Router {
    Router::new().route("/proxy/*ai", post(global_proxy))
}

pub fn workspaced_service() -> Router {
    Router::new().route("/proxy/*ai", post(proxy))
}

async fn global_proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(ai_path): Path<String>,
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

    let url = format!("{}/{}", base_url, ai_path);

    let request = HTTP_CLIENT
        .post(url)
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .body(body);

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

    let request = request_config.prepare_request(&provider, &ai_path, body)?;

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
    Ok((status_code, headers, axum::body::Body::from_stream(stream)))
}
