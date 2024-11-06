use crate::{
    db::{ApiAuthed, DB},
    variables::decrypt,
    workspaces::AiRessource,
};
use anthropic::AnthropicCache;
use axum::{
    body::Bytes,
    extract::{Path, Query},
    response::IntoResponse,
    routing::post,
    Extension, Router,
};
use lazy_static::lazy_static;
use openai::OpenaiCache;
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::error::{to_anyhow, Result};
use windmill_common::variables::build_crypt;

use windmill_common::error::Error;

const OPENAI_BASE_API_URL: &str = "https://api.openai.com/v1";
const ANTHROPIC_BASE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MISTRAL_BASE_API_URL: &str = "";
use serde_json::value::{RawValue, Value};
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();
}

mod openai {
    use super::*;

    use super::{get_variable_or_self, AiCache, KeyCache, AI_KEY_CACHE, OPENAI_BASE_API_URL};

    const API_VERSION: &str = "2023-05-15";

    #[derive(Clone)]
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

        pub fn prepare_request(self, openai_path: &str, mut body: Bytes) -> Result<RequestBuilder> {
            let OpenaiCache { api_key, azure_base_path, organization_id, user } = self;
            if user.is_some() {
                tracing::debug!("Adding user to request body");
                let mut json_body: HashMap<String, Box<RawValue>> = serde_json::from_slice(&body)
                    .map_err(|e| {
                    Error::InternalErr(format!("Failed to parse request body: {}", e))
                })?;

                let user_json_string = serde_json::Value::String(user.unwrap()).to_string(); // makes sure to escape characters

                json_body.insert(
                    "user".to_string(),
                    RawValue::from_string(user_json_string)
                        .map_err(|e| Error::InternalErr(format!("Failed to parse user: {}", e)))?,
                );

                body = serde_json::to_vec(&json_body)
                    .map_err(|e| {
                        Error::InternalErr(format!("Failed to reserialize request body: {}", e))
                    })?
                    .into();
            }

            let cp = body.clone();

            println!("{:#?}", cp);

            let base_url = if let Some(base_url) = azure_base_path {
                base_url
            } else {
                OPENAI_BASE_API_URL.to_string()
            };

            let url = format!("{}/{}", base_url, openai_path);
            println!("URL {}", &url);
            let mut request = HTTP_CLIENT
                .post(url)
                .header("content-type", "application/json")
                .body(body);

            if base_url != OPENAI_BASE_API_URL {
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
    #[serde(untagged)]
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
                Error::InternalErr(format!(
                    "Failed to get OpenAI credentials using credentials flow: {}",
                    err
                ))
            })?;
        let response = response.json::<OpenaiCredentials>().await.map_err(|err| {
            Error::InternalErr(format!(
                "Failed to parse OpenAI credentials from credentials flow: {}",
                err
            ))
        })?;
        Ok(response.access_token)
    }

    pub async fn get_cached_value(db: &DB, w_id: &str, resource: Value) -> Result<KeyCache> {
        let config = serde_json::from_value(resource)
            .map_err(|e| Error::InternalErr(format!("validating openai resource {e:#}")))?;

        let mut user = None::<String>;
        let mut resource = match config {
            OpenaiConfig::Resource(resource) => {
                tracing::debug!("Getting OpenAI key from static resource");
                println!("Here 1");
                resource
            }
            OpenaiConfig::ClientCredentialsOauthResource(resource) => {
                tracing::debug!("Getting OpenAI key with client credentials flow");
                user = resource.user.clone();
                let token = get_openai_key_using_credentials_flow(resource, db, w_id).await?;
                println!("Here 2");
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
                    Error::InternalErr(format!("validating openai azure base path {e:#}"))
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
        let cached_key = KeyCache::Openai(workspace_cache);
        AI_KEY_CACHE.insert(w_id.to_string(), AiCache::new(cached_key.clone()));
        Ok(cached_key)
    }
}

mod anthropic {

    use super::*;

    #[derive(Clone, Deserialize, Debug)]
    pub struct AnthropicCache {
        pub api_key: String,
    }

    const API_VERSION: &str = "2023-06-01";

    impl AnthropicCache {
        pub fn new(api_key: String) -> Self {
            Self { api_key }
        }

        pub fn prepare_request(self, anthropic_path: &str, body: Bytes) -> Result<RequestBuilder> {
            let AnthropicCache { api_key } = self;
            let url = format!("{}/{}", ANTHROPIC_BASE_API_URL, anthropic_path);
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
            .map_err(|e| Error::InternalErr(format!("validating anthropic resource {e:#}")))?;
        println!("{:#?}", resource);
        resource.api_key = get_variable_or_self(resource.api_key, db, w_id).await?;
        let workspace_cache = AnthropicCache::new(resource.api_key);
        let cached_key = KeyCache::Anthropic(workspace_cache);
        AI_KEY_CACHE.insert(w_id.to_string(), AiCache::new(cached_key.clone()));
        Ok(cached_key)
    }
}
mod mistral {}
#[derive(Clone)]
enum KeyCache {
    Openai(OpenaiCache),
    Anthropic(AnthropicCache),
}

#[derive(Clone)]
struct AiCache {
    cached_key: KeyCache,
    expires_at: std::time::Instant,
}

impl AiCache {
    pub fn new(cached_key: KeyCache) -> Self {
        Self {
            cached_key,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
        }
    }
    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
    }
}

lazy_static! {
    static ref AI_KEY_CACHE: Cache<String, AiCache> = Cache::new(500);
}

struct Variable {
    value: String,
    is_secret: bool,
}

#[derive(Deserialize)]
struct ProxyQueryParams {
    no_cache: Option<bool>,
}

async fn get_variable_or_self(path: String, db: &DB, w_id: &str) -> Result<String> {
    if !path.starts_with("$var:") {
        return Ok(path);
    }
    let path = path.strip_prefix("$var:").unwrap().to_string();
    let mut variable = sqlx::query_as!(
        Variable,
        "SELECT value, is_secret
        FROM variable
        WHERE path = $1 AND workspace_id = $2",
        &path,
        &w_id
    )
    .fetch_one(db)
    .await?;
    if variable.is_secret {
        let mc = build_crypt(db, w_id).await?;
        variable.value = decrypt(&mc, variable.value)?;
    }
    Ok(variable.value)
}

pub fn workspaced_service() -> Router {
    let router = Router::new().route("/proxy/*ai", post(proxy));

    router
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, ai_path)): Path<(String, String)>,
    Query(query_params): Query<ProxyQueryParams>,
    body: Bytes,
) -> impl IntoResponse {
    let workspace_cache = AI_KEY_CACHE.get(&w_id);
    let ai_cache = match workspace_cache {
        Some(cache) if !cache.is_expired() || !query_params.no_cache.unwrap_or(false) => {
            cache.cached_key
        }
        _ => {
            let ai_resource = sqlx::query_scalar!(
                "SELECT ai_resource FROM workspace_settings WHERE workspace_id = $1",
                &w_id
            )
            .fetch_one(&db)
            .await?;

            if ai_resource.is_none() {
                return Err(Error::InternalErr(
                    "OpenAI resource not configured".to_string(),
                ));
            }

            let ai_resource = serde_json::from_value::<AiRessource>(ai_resource.unwrap()).unwrap();

            let ai_resource_path = ai_resource.path;

            let resource = sqlx::query_scalar!(
                "SELECT value
                FROM resource
                WHERE path = $1 AND workspace_id = $2",
                &ai_resource_path,
                &w_id
            )
            .fetch_optional(&db)
            .await?
            .ok_or_else(|| {
                Error::InternalErr(format!(
                    "Could not find the OpenAI resource at path {ai_resource_path}, update the resource path in the workspace settings"
                ))
            })?;

            if resource.is_none() {
                return Err(Error::InternalErr(format!(
                    "{} resource missing value",
                    ai_resource.provider
                )));
            }

            let resource = resource.unwrap();

            let ai_cache = match ai_resource.provider.as_str() {
                "openai" => openai::get_cached_value(&db, &w_id, resource).await,
                "anthropic" => anthropic::get_cached_value(&db, &w_id, resource).await,
                provider => {
                    return Err(Error::BadRequest(format!("{} is not supported", provider)))
                }
            };
            ai_cache?
        }
    };

    let (path, request) = match ai_cache {
        KeyCache::Openai(cached) => ("openai_path", cached.prepare_request(&ai_path, body)),
        KeyCache::Anthropic(cached) => ("anthropic_path", cached.prepare_request(&ai_path, body)),
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
        Some([(path, &format!("{:?}", ai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    if response.error_for_status_ref().is_err() {
        return Err(Error::OpenAIError(
            response.text().await.unwrap_or("".to_string()),
        ));
    }

    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();

    Ok((status_code, headers, axum::body::Body::from_stream(stream)))
}
