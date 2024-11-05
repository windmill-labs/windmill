use std::collections::HashMap;

use crate::db::{ApiAuthed, DB};

use axum::{body::Bytes, response::IntoResponse};
use reqwest::Client;
use serde_json::value::{RawValue, Value};
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;

use serde::Deserialize;
use windmill_common::error::{to_anyhow, Error};

use super::{get_variable_or_self, AiCache, KeyCache, AI_KEY_CACHE, OPENAI_BASE_API_URL};

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();
}

#[derive(Clone)]
pub struct OpenaiKeyCache {
    api_key: String,
    organization_id: Option<String>,
    azure_base_path: Option<String>,
    user: Option<String>,
}

impl OpenaiKeyCache {
    pub fn new(
        api_key: String,
        organization_id: Option<String>,
        azure_base_path: Option<String>,
        user: Option<String>,
    ) -> Self {
        Self { api_key, organization_id, azure_base_path, user }
    }
}

#[derive(Deserialize)]
struct OpenaiResource {
    api_key: String,
    organization_id: Option<String>,
}

#[derive(Deserialize)]
struct OpenaiClientCredentialsOauthResource {
    client_id: String,
    client_secret: String,
    token_url: String,
    user: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OpenaiConfig {
    Resource(OpenaiResource),
    ClientCredentialsOauthResource(OpenaiClientCredentialsOauthResource),
}

lazy_static::lazy_static! {
    pub static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();
}

#[derive(Deserialize)]
struct OpenaiCredentials {
    access_token: String,
}
async fn get_openai_key_using_credentials_flow(
    mut resource: OpenaiClientCredentialsOauthResource,
    db: &DB,
    w_id: &str,
) -> Result<String, Error> {
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

pub async fn update_and_retrieve_cached_value(
    db: &DB,
    w_id: &str,
    resource: Value,
) -> Result<KeyCache, Error> {
    let config = serde_json::from_value(resource)
        .map_err(|e| Error::InternalErr(format!("validating openai resource {e:#}")))?;

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

    if resource.organization_id.is_some() {
        resource.organization_id =
            Some(get_variable_or_self(resource.organization_id.unwrap(), db, w_id).await?);
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

    let workspace_cache = OpenaiKeyCache::new(
        resource.api_key.clone(),
        resource.organization_id.clone(),
        azure_base_path.clone(),
        user.clone(),
    );
    let cached_key = KeyCache::Openai(workspace_cache);
    AI_KEY_CACHE.insert(w_id.to_string(), AiCache::new(cached_key.clone()));
    Ok(cached_key)
}

pub async fn proxy(
    authed: ApiAuthed,
    db: &DB,
    (w_id, openai_path): (String, String),
    cached_key: OpenaiKeyCache,
    mut body: Bytes,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let OpenaiKeyCache { api_key, azure_base_path, organization_id, user } = cached_key;
    if user.is_some() {
        tracing::debug!("Adding user to request body");
        let mut json_body: HashMap<String, Box<RawValue>> = serde_json::from_slice(&body)
            .map_err(|e| Error::InternalErr(format!("Failed to parse request body: {}", e)))?;

        let user_json_string = serde_json::Value::String(user.unwrap()).to_string(); // makes sure to escape characters

        json_body.insert(
            "user".to_string(),
            RawValue::from_string(user_json_string)
                .map_err(|e| Error::InternalErr(format!("Failed to parse user: {}", e)))?,
        );

        body = serde_json::to_vec(&json_body)
            .map_err(|e| Error::InternalErr(format!("Failed to reserialize request body: {}", e)))?
            .into();
    }

    let base_url = if let Some(base_url) = azure_base_path {
        base_url
    } else {
        OPENAI_BASE_API_URL.to_string()
    };

    let url = format!("{}/{}", base_url, openai_path);
    let mut request = HTTP_CLIENT
        .post(url)
        .header("content-type", "application/json")
        .body(body);

    if base_url != OPENAI_BASE_API_URL {
        request = request
            .header("api-key", api_key)
            .query(&[("api-version", "2023-05-15")])
    } else {
        request = request.header("authorization", format!("Bearer {}", api_key))
    }

    if let Some(org_id) = organization_id {
        request = request.header("OpenAI-Organization", org_id);
    }

    let response = request.send().await.map_err(to_anyhow)?;

    let mut tx = db.begin().await?;
    audit_log(
        &mut *tx,
        &authed,
        "openai.request",
        ActionKind::Execute,
        &w_id,
        Some(&authed.email),
        Some([("openai_path", &format!("{:?}", openai_path)[..])].into()),
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
