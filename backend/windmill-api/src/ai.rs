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
use tokio_stream::wrappers::ReceiverStream;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::error::{to_anyhow, Error, Result};

#[cfg(feature = "bedrock")]
use aws_sdk_bedrockruntime::{
    config::{BehaviorVersion, Credentials, Region},
    primitives::Blob,
    Client as BedrockClient,
};

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringAIRequestConfig> = Cache::new(500);
}

const AZURE_API_VERSION: &str = "2024-10-21";
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

#[cfg(feature = "bedrock")]
#[derive(Deserialize, Debug)]
struct AIBedrockResource {
    region: String,
    #[serde(rename = "accessKeyId")]
    access_key_id: String,
    #[serde(rename = "secretAccessKey")]
    secret_access_key: String,
}

#[derive(Deserialize, Debug)]
struct OAuthTokens {
    access_token: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AIResource {
    #[cfg(feature = "bedrock")]
    Bedrock(AIBedrockResource),
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
    pub region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

impl AIRequestConfig {
    pub async fn new(
        provider: &AIProvider,
        db: &DB,
        w_id: &str,
        resource: AIResource,
    ) -> Result<Self> {
        tracing::debug!("Creating AI request config for provider: {:?}", provider);
        tracing::debug!("Resource: {:?}", resource);
        match resource {
            #[cfg(feature = "bedrock")]
            AIResource::Bedrock(resource) => {
                tracing::debug!("Creating Bedrock request config");
                let base_url = provider.get_base_url(None, db).await?;
                let access_key_id = get_variable_or_self(resource.access_key_id, db, w_id).await?;
                let secret_access_key =
                    get_variable_or_self(resource.secret_access_key, db, w_id).await?;
                let region = get_variable_or_self(resource.region, db, w_id).await?;
                Ok(Self {
                    base_url,
                    access_key_id: Some(access_key_id),
                    secret_access_key: Some(secret_access_key),
                    region: Some(region),
                    organization_id: None,
                    api_key: None,
                    access_token: None,
                    user: None,
                })
            }
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

                Ok(Self {
                    base_url,
                    organization_id,
                    api_key,
                    access_token: None,
                    user: None,
                    region: None,
                    access_key_id: None,
                    secret_access_key: None,
                })
            }
            AIResource::OAuth(resource) => {
                let user = if let Some(user) = resource.user.clone() {
                    Some(get_variable_or_self(user, db, w_id).await?)
                } else {
                    None
                };
                let token = Self::get_token_using_oauth(resource, db, w_id).await?;
                let base_url = provider.get_base_url(None, db).await?;

                Ok(Self {
                    base_url,
                    organization_id: None,
                    api_key: None,
                    access_token: Some(token),
                    user,
                    region: None,
                    access_key_id: None,
                    secret_access_key: None,
                })
            }
        }
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
        let url = format!("{}/{}", self.base_url, path);

        let body = if let Some(user) = self.user {
            Self::add_user_to_body(body, user)?
        } else {
            body
        };

        let is_azure = matches!(provider, AIProvider::OpenAI) && self.base_url != OPENAI_BASE_URL
            || matches!(provider, AIProvider::AzureOpenAI);

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
    #[cfg(feature = "bedrock")]
    #[serde(rename = "aws_bedrock")]
    AwsBedrock,
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
            #[cfg(feature = "bedrock")]
            AIProvider::AwsBedrock => Ok("".to_string()), // Bedrock uses AWS SDK directly, not REST API
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

    #[cfg(feature = "bedrock")]
    if matches!(provider, AIProvider::AwsBedrock) {
        let region = headers
            .get("X-Bedrock-Region")
            .map(|v| v.to_str().unwrap_or("").to_string())
            .ok_or_else(|| Error::BadRequest("Bedrock region is required".to_string()))?;

        let model_id = headers
            .get("X-Bedrock-Model-ID")
            .map(|v| v.to_str().unwrap_or("").to_string())
            .ok_or_else(|| Error::BadRequest("Bedrock model ID is required".to_string()))?;

        // Get AWS credentials from headers if provided
        let access_key_id = headers
            .get("X-AWS-Access-Key-ID")
            .map(|v| v.to_str().unwrap_or("").to_string());

        let secret_access_key = headers
            .get("X-AWS-Secret-Access-Key")
            .map(|v| v.to_str().unwrap_or("").to_string());

        tracing::debug!(
            "Global Bedrock request - Model: {}, Region: {}, Access Key Provided: {}",
            model_id,
            region,
            access_key_id.is_some()
        );

        // Use the provided credentials or default AWS credentials from environment
        let (status, headers, body) = send_bedrock_request(
            body,
            Some(&region),
            access_key_id.as_deref(),
            secret_access_key.as_deref(),
        )
        .await?;

        let mut tx = db.begin().await?;
        audit_log(
            &mut *tx,
            &authed,
            "ai.global_request.bedrock",
            ActionKind::Execute,
            "global",
            Some(&authed.email),
            None,
        )
        .await?;
        tx.commit().await?;

        return Ok((status, headers, body));
    }

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

    let result = if matches!(provider, AIProvider::AwsBedrock) {
        send_bedrock_request(
            body,
            request_config.region.as_deref(),
            request_config.access_key_id.as_deref(),
            request_config.secret_access_key.as_deref(),
        )
        .await?
    } else {
        let request = request_config.prepare_request(&provider, &ai_path, body)?;
        let response = request.send().await.map_err(to_anyhow)?;

        if response.error_for_status_ref().is_err() {
            let err_msg = response.text().await.unwrap_or("".to_string());
            return Err(Error::AIError(err_msg));
        }

        let status_code = response.status();
        let headers = response.headers().clone();
        let stream = response.bytes_stream();
        (status_code, headers, axum::body::Body::from_stream(stream))
    };

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

    Ok(result)
}

#[cfg(feature = "bedrock")]
async fn create_bedrock_client(
    region: Option<&str>,
    access_key_id: Option<&str>,
    secret_access_key: Option<&str>,
) -> Result<BedrockClient> {
    if let (Some(access_key), Some(secret_key), Some(region)) =
        (access_key_id, secret_access_key, region)
    {
        tracing::debug!(
            "Creating Bedrock client with provided credentials in region: {}",
            region
        );

        let credentials = Credentials::new(access_key, secret_key, None, None, "DirectTest");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .credentials_provider(credentials)
            .load()
            .await;

        Ok(BedrockClient::new(&config))
    } else {
        tracing::debug!("Creating Bedrock client with default credentials");

        let config = aws_config::defaults(BehaviorVersion::latest()).load().await;

        Ok(BedrockClient::new(&config))
    }
}

#[cfg(feature = "bedrock")]
pub async fn send_bedrock_request(
    body: Bytes,
    region: Option<&str>,
    access_key_id: Option<&str>,
    secret_access_key: Option<&str>,
) -> Result<(axum::http::StatusCode, HeaderMap, axum::body::Body)> {
    let region = region.ok_or_else(|| Error::BadRequest("Region is required".to_string()))?;
    let access_key_id =
        access_key_id.ok_or_else(|| Error::BadRequest("Access key ID is required".to_string()))?;
    let secret_access_key = secret_access_key
        .ok_or_else(|| Error::BadRequest("Secret access key is required".to_string()))?;

    tracing::debug!("Sending Bedrock request to region: {}", region);
    tracing::debug!("Body: {:#?}", body);


    let mut json_body: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;
    let model_id = json_body["model"]
        .as_str()
        .ok_or_else(|| Error::internal_err("Model ID not found in request body"))?
        .to_string();

    // Update anthropic_version default
    json_body.as_object_mut().map(|obj| {
        obj.entry("anthropic_version")
            .or_insert_with(|| serde_json::Value::String("bedrock-2023-05-31".to_string()));
        obj.entry("messages")
            .or_insert(serde_json::Value::Array(vec![]));
        // Add any other required fields with default values
    });

    // Handle "system" role
    if let Some(messages) = json_body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for message in messages {
            if let Some(role) = message.get_mut("role") {
                // Change "system" role to "user"
                if role == "system" {
                    *role = serde_json::Value::String("user".to_string());
                }
            }
        }
    }

    // Remove "tool_choice"
    json_body.as_object_mut().map(|obj| {
        obj.remove("stream");
        obj.remove("model");
        obj.remove("tools");
        obj.remove("tool_choice");
        // Add any other fields you need to remove
    });

    // Serialize the modified JSON back to bytes
    let modified_body = serde_json::to_vec(&json_body)
        .map_err(|e| Error::internal_err(format!("Failed to serialize modified body: {}", e)))?;

    let client =
        create_bedrock_client(Some(region), Some(access_key_id), Some(secret_access_key)).await?;

    tracing::debug!("Sending request to Bedrock...");
    tracing::debug!("Body: {:#?}", json_body);

    let response = client
        .invoke_model_with_response_stream()
        .model_id(model_id.clone())
        .body(Blob::new(modified_body))
        .content_type("application/json")
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to send Bedrock request: {:#?}", e)))?;

    let mut stream = response.body;

    let (tx, rx): (
        tokio::sync::mpsc::Sender<std::result::Result<String, Error>>,
        tokio::sync::mpsc::Receiver<std::result::Result<String, Error>>,
    ) = tokio::sync::mpsc::channel(32);

    let tx_clone1 = tx.clone();
    let tx_clone2 = tx.clone();

    // Spawn a task to process the stream and send chunks
    tokio::spawn(async move {
        while let Ok(Some(output)) = stream.recv().await {
            if let Ok(chunk) = output.as_chunk() {
                let chunk_bytes = if let Some(blob) = chunk.bytes() {
                    blob.as_ref().to_vec()
                } else {
                    tracing::warn!("Received a chunk with no bytes");
                    continue;
                };

                // Parse JSON from chunk
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&chunk_bytes) {
                    if let Some(text) = json
                        .get("delta")
                        .and_then(|d| d.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        let event = serde_json::json!({
                            "id": "chatcmpl-123",
                            "object": "chat.completion.chunk",
                            "created": chrono::Utc::now().timestamp(),
                            "model": model_id,
                            "choices": [{
                                "index": 0,
                                "delta": {
                                    "content": text
                                },
                                "finish_reason": null
                            }]
                        });

                        if let Err(e) = tx_clone1.send(Ok(format!("data: {}\n\n", event.to_string())))
                            .await
                        {
                            tracing::error!("Failed to send chunk: {}", e);
                            break;
                        }
                    }
                } else {
                    tracing::warn!("Non-JSON chunk: {:?}", chunk_bytes);
                    tracing::debug!("Raw chunk bytes: {:?}", String::from_utf8_lossy(&chunk_bytes));
                }
            }
        }

        // Finish event
        let final_event = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": chrono::Utc::now().timestamp(),
            "model": model_id,
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        });

        let _ = tx_clone1.send(Ok(format!("data: {}\n\n", final_event.to_string())))
            .await;
    });

    // Send heartbeat ping
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            if let Err(e) = tx_clone2.send(Ok(":\n\n".to_string())).await {
                tracing::error!("Failed to send heartbeat: {}", e);
                break;
            }
        }
    });

    // Set up response headers for SSE
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        http::header::CONTENT_TYPE,
        "text/event-stream".parse().unwrap(),
    );
    response_headers.insert(http::header::CACHE_CONTROL, "no-cache".parse().unwrap());
    response_headers.insert(http::header::CONNECTION, "keep-alive".parse().unwrap());

    // Create a streaming body from the receiver
    let body = axum::body::Body::from_stream(ReceiverStream::new(rx));

    Ok((axum::http::StatusCode::OK, response_headers, body))
}
