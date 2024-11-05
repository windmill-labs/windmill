use crate::{
    db::{ApiAuthed, DB},
    variables::decrypt,
    workspaces::AiRessource,
};
use axum::{
    body::Bytes,
    extract::{Path, Query},
    response::IntoResponse,
    Extension,
};
use http::StatusCode;
use lazy_static::lazy_static;
use openai::OpenaiKeyCache;
use quick_cache::sync::Cache;
use serde::Deserialize;
use windmill_common::variables::build_crypt;

use windmill_common::error::Error;

const OPENAI_BASE_API_URL: &str = "https://api.openai.com/v1";
const ANTHROPIC_BASE_API_URL: &str = "";
const MISTRAL_BASE_API_URL: &str = "";

mod anthropic;
mod mistral;
mod openai;

#[derive(Clone)]
enum KeyCache {
    Openai(OpenaiKeyCache),
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

async fn get_variable_or_self(path: String, db: &DB, w_id: &str) -> Result<String, Error> {
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
        let mc = build_crypt(&db, &w_id).await?;
        variable.value = decrypt(&mc, variable.value)?;
    }
    Ok(variable.value)
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, ai_path)): Path<(String, String)>,
    Query(query_params): Query<ProxyQueryParams>,
    mut body: Bytes,
) -> impl IntoResponse {
    let workspace_cache = AI_KEY_CACHE.get(&w_id);
    let ai_cache = if query_params.no_cache.unwrap_or(false)
        || workspace_cache.is_none()
        || workspace_cache.unwrap().is_expired()
    {
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
            Err(Error::InternalErr(format!(
                "Could not find the OpenAI resource at path {ai_resource_path}, update the resource path in the workspace settings"
            )))
        })?;

        if resource.is_none() {
            return Err(Error::InternalErr(format!(
                "{} resource missing value",
                ai_resource.provider
            )));
        }

        let resource = resource.unwrap();

        let ai_cache = match ai_resource.provider.as_str() {
            "openai" => openai::update_and_retrieve_cached_value(&db, &w_id, resource),
        }
        .await;
        ai_cache?
    } else {
        tracing::debug!("Using cached OpenAI key");
        workspace_cache.unwrap().cached_key
    };

    let result_proxy = match ai_cache {
        KeyCache::Openai(cached) => openai::proxy(authed, &db, (w_id, ai_path), cached, body),
    };

    Ok(result_proxy.await)
}
