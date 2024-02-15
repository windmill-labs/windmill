use std::collections::HashMap;

use crate::{
    db::{ApiAuthed, DB},
    HTTP_CLIENT,
};

use axum::{
    body::{Bytes, StreamBody},
    extract::{Extension, Path, Query},
    response::IntoResponse,
    routing::post,
    Router,
};
use quick_cache::sync::Cache;
use serde_json::value::RawValue;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    error::{to_anyhow, Error},
    variables::build_crypt,
};

use crate::variables::decrypt;
use serde::Deserialize;

pub fn workspaced_service() -> Router {
    let router = Router::new().route("/proxy/*openai_path", post(proxy));

    router
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

struct Variable {
    value: String,
    is_secret: bool,
}
async fn get_variable_or_self(path: String, db: &DB, w_id: &String) -> Result<String, Error> {
    if !path.starts_with("$var:") {
        return Ok(path);
    }
    let path = path.strip_prefix("$var:").unwrap().to_string();
    let mut tx = db.begin().await?;
    let mut variable = sqlx::query_as!(
        Variable,
        "SELECT value, is_secret
        FROM variable
        WHERE path = $1 AND workspace_id = $2",
        &path,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if variable.is_secret {
        let mc = build_crypt(&mut tx, &w_id).await?;
        variable.value = decrypt(&mc, variable.value)?;
    }
    tx.commit().await?;
    Ok(variable.value)
}

lazy_static::lazy_static! {
    pub static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();
    static ref OPENAI_KEY_CACHE: Cache<String, OpenaiKeyCache> = Cache::new(500);
}

#[derive(Deserialize)]
struct OpenaiCredentials {
    access_token: String,
}
async fn get_openai_key_using_credentials_flow(
    mut resource: OpenaiClientCredentialsOauthResource,
    db: &DB,
    w_id: &String,
) -> Result<String, Error> {
    resource.client_id = get_variable_or_self(resource.client_id, &db, &w_id).await?;
    resource.client_secret = get_variable_or_self(resource.client_secret, &db, &w_id).await?;
    resource.token_url = get_variable_or_self(resource.token_url, &db, &w_id).await?;
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

#[derive(Clone)]
struct OpenaiKeyCache {
    api_key: String,
    organization_id: Option<String>,
    azure_base_path: Option<String>,
    user: Option<String>,
    expires_at: std::time::Instant,
}

impl OpenaiKeyCache {
    fn new(
        api_key: String,
        organization_id: Option<String>,
        azure_base_path: Option<String>,
        expires_at: std::time::Instant,
        user: Option<String>,
    ) -> Self {
        Self { api_key, organization_id, azure_base_path, expires_at, user }
    }
    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
    }
}

#[derive(Deserialize)]
struct ProxyQueryParams {
    no_cache: Option<bool>,
}
async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, openai_path)): Path<(String, String)>,
    Query(query_params): Query<ProxyQueryParams>,
    mut body: Bytes,
) -> impl IntoResponse {
    let workspace_cache = OPENAI_KEY_CACHE.get(&w_id);
    let (api_key, organization_id, azure_base_path, user) = if query_params
        .no_cache
        .unwrap_or(false)
        || workspace_cache.is_none()
        || workspace_cache.clone().unwrap().is_expired()
    {
        let openai_resource_path = sqlx::query_scalar!(
            "SELECT openai_resource_path FROM workspace_settings WHERE workspace_id = $1",
            &w_id
        )
        .fetch_one(&db)
        .await?;

        if openai_resource_path.is_none() {
            return Err(Error::InternalErr(
                "OpenAI resource not configured".to_string(),
            ));
        }

        let openai_resource_path = openai_resource_path.unwrap();

        let resource = sqlx::query_scalar!(
            "SELECT value
            FROM resource
            WHERE path = $1 AND workspace_id = $2",
            &openai_resource_path,
            &w_id
        )
        .fetch_optional(&db)
        .await?
        .ok_or_else(|| {
            Error::InternalErr(format!(
                "Could not find the OpenAI resource at path {openai_resource_path}, update the resource path in the workspace settings"
            ))
        })?;

        if resource.is_none() {
            return Err(Error::InternalErr(
                "OpenAI resource missing value".to_string(),
            ));
        }

        let config: OpenaiConfig = serde_json::from_value(resource.unwrap())
            .map_err(|e| Error::InternalErr(format!("validating openai resource {e}")))?;

        let mut user = None::<String>;
        let mut resource = match config {
            OpenaiConfig::Resource(resource) => {
                tracing::debug!("Getting OpenAI key from static resource");
                resource
            }
            OpenaiConfig::ClientCredentialsOauthResource(resource) => {
                tracing::debug!("Getting OpenAI key with client credentials flow");
                user = resource.user.clone();
                let token = get_openai_key_using_credentials_flow(resource, &db, &w_id).await?;
                OpenaiResource { api_key: token, organization_id: None }
            }
        };

        resource.api_key = get_variable_or_self(resource.api_key, &db, &w_id).await?;

        if resource.organization_id.is_some() {
            resource.organization_id =
                Some(get_variable_or_self(resource.organization_id.unwrap(), &db, &w_id).await?);
        }

        if user.is_some() {
            user = Some(get_variable_or_self(user.unwrap(), &db, &w_id).await?);
        }

        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(60);

        let azure_base_path = sqlx::query_scalar!(
            "SELECT value
            FROM global_settings
            WHERE name = 'openai_azure_base_path'",
        )
        .fetch_optional(&db)
        .await?;

        let azure_base_path = if let Some(azure_base_path) = azure_base_path {
            Some(
                serde_json::from_value::<String>(azure_base_path).map_err(|e| {
                    Error::InternalErr(format!("validating openai azure base path {e}"))
                })?,
            )
        } else {
            OPENAI_AZURE_BASE_PATH.clone()
        };

        let workspace_cache = OpenaiKeyCache::new(
            resource.api_key.clone(),
            resource.organization_id.clone(),
            azure_base_path.clone(),
            expires_at,
            user.clone(),
        );
        OPENAI_KEY_CACHE.insert(w_id.clone(), workspace_cache);
        (
            resource.api_key,
            resource.organization_id,
            azure_base_path,
            user,
        )
    } else {
        tracing::debug!("Using cached OpenAI key");
        let workspace_cache = workspace_cache.unwrap();
        (
            workspace_cache.api_key.clone(),
            workspace_cache.organization_id.clone(),
            workspace_cache.azure_base_path.clone(),
            workspace_cache.user.clone(),
        )
    };

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
        "https://api.openai.com/v1".to_string()
    };

    let url = format!("{}/{}", base_url, openai_path);
    let mut request = HTTP_CLIENT
        .post(url)
        .header("content-type", "application/json")
        .body(body);

    if base_url != "https://api.openai.com/v1" {
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
        &authed.username,
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

    Ok((status_code, headers, StreamBody::new(stream)))
}
