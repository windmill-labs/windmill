use crate::{
    db::{ApiAuthed, DB},
    variables::get_variable_or_self,
};

use anthropic::AnthropicCache;
use anyhow::Context;
use axum::{body::Bytes, extract::Path, response::IntoResponse, routing::post, Extension, Router};
use http::HeaderMap;
use lazy_static::lazy_static;
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::value::{RawValue, Value};
use std::collections::HashMap;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::error::{to_anyhow, Error, Result};

use mistral::MistralCache;
use openai::OpenaiCache;
use openai_api_compatible::OpenaiApiCompatibleCache;

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();
}

mod openai_api_compatible {
    use super::*;

    #[derive(Deserialize, Clone, Debug)]
    pub struct OpenaiApiCompatibleCache {
        pub base_url: String,
        pub api_key: Option<String>,
    }

    impl OpenaiApiCompatibleCache {
        pub fn prepare_request(self, path: &str, body: Bytes) -> Result<RequestBuilder> {
            let url = format!("{}/{}", self.base_url, path);

            let mut request = HTTP_CLIENT
                .post(url)
                .header("content-type", "application/json")
                .body(body);

            if let Some(api_key) = self.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            Ok(request)
        }
    }

    pub async fn get_cached_value(
        db: &DB,
        w_id: &str,
        resource: Value,
        base_url: Option<String>,
    ) -> Result<KeyCache> {
        let mut resource: OpenaiApiCompatibleCache = if let Some(base_url) = base_url {
            let api_key = match resource {
                Value::Object(mut obj) => obj
                    .remove("api_key")
                    .map(|v| serde_json::from_value::<String>(v.clone()).ok())
                    .flatten(),
                _ => None,
            };
            OpenaiApiCompatibleCache { base_url, api_key }
        } else {
            serde_json::from_value(resource).with_context(|| "validating custom AI resource")?
        };

        if let Some(api_key) = resource.api_key {
            resource.api_key = Some(get_variable_or_self(api_key, db, w_id).await?);
        }

        Ok(KeyCache::OpenaiApiCompatible(resource))
    }
}

mod openai {
    use super::*;

    const API_VERSION: &str = "2024-10-21";

    #[derive(Deserialize, Debug)]
    struct OpenaiResource {
        api_key: String,
        organization_id: Option<String>,
    }

    #[derive(Deserialize, Debug)]
    struct OpenaiClientCredentialsOauthResource {
        client_id: String,
        client_secret: String,
        token_url: String,
        user: Option<String>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged, rename_all = "snake_case")]
    enum OpenaiConfig {
        Resource(OpenaiResource),
        ClientCredentialsOauthResource(OpenaiClientCredentialsOauthResource),
    }

    lazy_static::lazy_static! {
        pub static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();
    }

    #[derive(Deserialize, Debug)]
    struct OpenaiCredentials {
        access_token: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct OpenaiCache {
        api_key: String,
        organization_id: Option<String>,
        azure_base_path: Option<String>,
        user: Option<String>,
    }

    impl OpenaiCache {
        pub fn new(
            api_key: String,
            organization_id: Option<String>,
            azure_base_path: Option<String>,
            user: Option<String>,
        ) -> Self {
            Self { api_key, organization_id, azure_base_path, user }
        }
    }

    const BASE_URL: &str = "https://api.openai.com/v1";
    impl OpenaiCache {
        pub fn prepare_request(self, openai_path: &str, mut body: Bytes) -> Result<RequestBuilder> {
            let OpenaiCache { api_key, azure_base_path, organization_id, user } = self;
            if user.is_some() {
                tracing::debug!("Adding user to request body");
                let mut json_body: HashMap<String, Box<RawValue>> = serde_json::from_slice(&body)
                    .map_err(|e| {
                    Error::internal_err(format!("Failed to parse request body: {}", e))
                })?;

                let user_json_string = serde_json::Value::String(user.unwrap()).to_string(); // makes sure to escape characters

                json_body.insert(
                    "user".to_string(),
                    RawValue::from_string(user_json_string)
                        .map_err(|e| Error::internal_err(format!("Failed to parse user: {}", e)))?,
                );

                body = serde_json::to_vec(&json_body)
                    .map_err(|e| {
                        Error::internal_err(format!("Failed to reserialize request body: {}", e))
                    })?
                    .into();
            }

            let base_url = if let Some(base_url) = azure_base_path {
                base_url
            } else {
                BASE_URL.to_string()
            };
            let url = format!("{}/{}", base_url, openai_path);
            let mut request = HTTP_CLIENT
                .post(url)
                .header("content-type", "application/json")
                .body(body);

            if base_url != BASE_URL {
                request = request
                    .header("api-key", api_key)
                    .query(&[("api-version", API_VERSION)])
            } else {
                request = request.header("authorization", format!("Bearer {}", api_key))
            }

            if let Some(org_id) = organization_id {
                request = request.header("OpenAI-Organization", org_id);
            }

            Ok(request)
        }
    }

    async fn get_openai_key_using_credentials_flow(
        mut resource: OpenaiClientCredentialsOauthResource,
        db: &DB,
        w_id: &str,
    ) -> Result<String> {
        resource.client_id = get_variable_or_self(resource.client_id, db, w_id).await?;
        resource.client_secret = get_variable_or_self(resource.client_secret, db, w_id).await?;
        resource.token_url = get_variable_or_self(resource.token_url, db, w_id).await?;
        let mut params = HashMap::new();
        params.insert("grant_type", "client_credentials");
        let response = HTTP_CLIENT
            .post(resource.token_url)
            .form(&params)
            .basic_auth(resource.client_id, Some(resource.client_secret))
            .send()
            .await
            .map_err(|err| {
                Error::internal_err(format!(
                    "Failed to get OpenAI credentials using credentials flow: {}",
                    err
                ))
            })?;
        let response = response.json::<OpenaiCredentials>().await.map_err(|err| {
            Error::internal_err(format!(
                "Failed to parse OpenAI credentials from credentials flow: {}",
                err
            ))
        })?;
        Ok(response.access_token)
    }

    pub async fn get_cached_value(db: &DB, w_id: &str, resource: Value) -> Result<KeyCache> {
        let config = serde_json::from_value(resource)
            .map_err(|e| Error::internal_err(format!("validating openai resource {e:#}")))?;

        let mut user = None::<String>;
        let mut resource = match config {
            OpenaiConfig::Resource(resource) => {
                tracing::debug!("Getting OpenAI key from static resource");
                resource
            }
            OpenaiConfig::ClientCredentialsOauthResource(resource) => {
                tracing::debug!("Getting OpenAI key with client credentials flow");
                user = resource.user.clone();
                let token = get_openai_key_using_credentials_flow(resource, db, w_id).await?;
                OpenaiResource { api_key: token, organization_id: None }
            }
        };

        resource.api_key = get_variable_or_self(resource.api_key, db, w_id).await?;

        if let Some(organization_id) = resource.organization_id {
            resource.organization_id = Some(get_variable_or_self(organization_id, db, w_id).await?);
        }

        if user.is_some() {
            user = Some(get_variable_or_self(user.unwrap(), db, w_id).await?);
        }

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

        let workspace_cache = OpenaiCache::new(
            resource.api_key.clone(),
            resource.organization_id.clone(),
            azure_base_path.clone(),
            user.clone(),
        );
        Ok(KeyCache::Openai(workspace_cache))
    }
}

mod anthropic {

    use super::*;

    #[derive(Clone, Deserialize, Debug)]
    pub struct AnthropicCache {
        #[serde(rename = "apiKey")]
        pub api_key: String,
    }

    const API_VERSION: &str = "2023-06-01";

    const BASE_URL: &str = "https://api.anthropic.com";
    impl AnthropicCache {
        pub fn prepare_request(self, anthropic_path: &str, body: Bytes) -> Result<RequestBuilder> {
            let AnthropicCache { api_key } = self;
            let url = format!("{}/{}", BASE_URL, anthropic_path);
            let request = HTTP_CLIENT
                .post(url)
                .header("x-api-key", api_key)
                .header("anthropic-version", API_VERSION)
                .header("content-type", "application/json")
                .body(body);
            Ok(request)
        }
    }

    pub async fn get_cached_value(db: &DB, w_id: &str, resource: Value) -> Result<KeyCache> {
        let mut resource: AnthropicCache = serde_json::from_value(resource)
            .map_err(|e| Error::internal_err(format!("validating anthropic resource {e:#}")))?;
        resource.api_key = get_variable_or_self(resource.api_key, db, w_id).await?;
        Ok(KeyCache::Anthropic(resource))
    }
}

mod mistral {
    use super::*;
    #[derive(Deserialize, Clone, Debug)]
    pub struct MistralCache {
        #[serde(rename = "apiKey")]
        pub api_key: String,
    }

    const BASE_URL: &str = "https://api.mistral.ai";
    impl MistralCache {
        pub fn prepare_request(self, mistral_path: &str, body: Bytes) -> Result<RequestBuilder> {
            let MistralCache { api_key } = self;

            let url = format!("{}/{}", BASE_URL, mistral_path);
            let request = HTTP_CLIENT
                .post(url)
                .header("content-type", "application/json")
                .header("Accept", "application/json")
                .header("authorization", format!("Bearer {}", api_key))
                .body(body);
            Ok(request)
        }
    }

    pub async fn get_cached_value(db: &DB, w_id: &str, resource: Value) -> Result<KeyCache> {
        let mut resource: MistralCache = serde_json::from_value(resource)
            .map_err(|e| Error::internal_err(format!("validating mistral resource {e:#}")))?;
        resource.api_key = get_variable_or_self(resource.api_key, db, w_id).await?;
        Ok(KeyCache::Mistral(resource))
    }
}

#[derive(Clone, Debug)]
pub enum KeyCache {
    Openai(OpenaiCache),
    Anthropic(AnthropicCache),
    Mistral(MistralCache),
    OpenaiApiCompatible(OpenaiApiCompatibleCache),
}

#[derive(Clone, Debug)]
pub struct AICache {
    pub path: String,
    pub cached_key: KeyCache,
    pub expires_at: std::time::Instant,
}

impl AICache {
    pub fn new(path: String, cached_key: KeyCache) -> Self {
        Self {
            path,
            cached_key,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
        }
    }
    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
    }
}

lazy_static! {
    pub static ref AI_KEY_CACHE: Cache<String, AICache> = Cache::new(500);
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    OpenAI,
    Anthropic,
    Mistral,
    DeepSeek,
    Groq,
    OpenRouter,
    CustomAI,
}

impl AIProvider {
    pub fn get_openai_compatible_base_url(&self) -> Result<Option<String>> {
        match self {
            AIProvider::DeepSeek => Ok(Some("https://api.deepseek.com/v1".to_string())),
            AIProvider::Groq => Ok(Some("https://api.groq.com/openai/v1".to_string())),
            AIProvider::OpenRouter => Ok(Some("https://openrouter.ai/api/v1".to_string())),
            AIProvider::CustomAI => Ok(None),
            _ => Err(Error::BadRequest(
                "Please use the specific provider instead of the OpenAI compatible one".to_string(),
            )),
        }
    }
}

impl TryFrom<&str> for AIProvider {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self> {
        match s {
            "openai" => Ok(AIProvider::OpenAI),
            "anthropic" => Ok(AIProvider::Anthropic),
            "mistral" => Ok(AIProvider::Mistral),
            "groq" => Ok(AIProvider::Groq),
            "openrouter" => Ok(AIProvider::OpenRouter),
            "deepseek" => Ok(AIProvider::DeepSeek),
            "customai" => Ok(AIProvider::CustomAI),
            _ => Err(Error::BadRequest(format!("Invalid AI provider: {}", s))),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct AIResource {
    pub path: String,
    pub provider: AIProvider,
}

pub fn workspaced_service() -> Router {
    let router = Router::new().route("/proxy/*ai", post(proxy));

    router
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, ai_path)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let workspace_cache = AI_KEY_CACHE.get(&w_id);
    let forced_resource_path = headers
        .get("X-Resource-Path")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let ai_cache = match workspace_cache {
        Some(cache) if !cache.is_expired() && forced_resource_path.is_none() => cache.cached_key,
        _ => {
            let (resource, resource_path, ai_provider) = if let Some(resource_path) =
                forced_resource_path
            {
                // guess the provider from the resource type
                let record = sqlx::query!(
                    "SELECT value, resource_type FROM resource WHERE path = $1 AND workspace_id = $2",
                    &resource_path,
                    &w_id
                )
                .fetch_optional(&db)
                .await?
                .ok_or_else(|| {
                    Error::NotFound(format!(
                        "Could not find the resource {}, update the resource path in the workspace settings", resource_path
                    ))
                })?;

                (
                    record.value,
                    resource_path,
                    AIProvider::try_from(record.resource_type.as_str())?,
                )
            } else {
                let ai_resource = sqlx::query_scalar!(
                    "SELECT ai_resource FROM workspace_settings WHERE workspace_id = $1",
                    &w_id
                )
                .fetch_one(&db)
                .await?;

                if ai_resource.is_none() {
                    return Err(Error::internal_err("AI resource not configured".to_string()));
                }

                let ai_resource = serde_json::from_value::<AIResource>(ai_resource.unwrap())
                    .map_err(|e| Error::BadRequest(e.to_string()))?;

                let resource = sqlx::query_scalar!(
                    "SELECT value
                    FROM resource
                    WHERE path = $1 AND workspace_id = $2",
                    &ai_resource.path,
                    &w_id
                )
                .fetch_optional(&db)
                .await?
                .ok_or_else(|| {
                    Error::NotFound(format!(
                        "Could not find the {:?} resource at path {}, update the resource path in the workspace settings", ai_resource.provider, ai_resource.path
                    ))
                })?;

                (resource, ai_resource.path, ai_resource.provider)
            };

            if resource.is_none() {
                return Err(Error::internal_err(format!(
                    "{:?} resource missing value",
                    ai_provider
                )));
            }

            let resource = resource.unwrap();

            let ai_cache = match ai_provider {
                AIProvider::OpenAI => openai::get_cached_value(&db, &w_id, resource).await,
                AIProvider::Anthropic => anthropic::get_cached_value(&db, &w_id, resource).await,
                AIProvider::Mistral => mistral::get_cached_value(&db, &w_id, resource).await,
                _ => {
                    openai_api_compatible::get_cached_value(
                        &db,
                        &w_id,
                        resource,
                        ai_provider.get_openai_compatible_base_url()?,
                    )
                    .await
                }
            };
            let ai_cache = ai_cache?;
            AI_KEY_CACHE.insert(w_id.clone(), AICache::new(resource_path, ai_cache.clone()));
            ai_cache
        }
    };

    let request = match ai_cache {
        KeyCache::Openai(cached) => cached.prepare_request(&ai_path, body),
        KeyCache::Anthropic(cached) => cached.prepare_request(&ai_path, body),
        KeyCache::Mistral(cached) => cached.prepare_request(&ai_path, body),
        KeyCache::OpenaiApiCompatible(cached) => cached.prepare_request(&ai_path, body),
    };

    let response = request?.send().await.map_err(to_anyhow)?;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "ai.request",
        ActionKind::Execute,
        &w_id,
        Some(&authed.email),
        Some([("ai_resource_path", &format!("{:?}", ai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AiError(err_msg));
    }

    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();
    Ok((status_code, headers, axum::body::Body::from_stream(stream)))
}
