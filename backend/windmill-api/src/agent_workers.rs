/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use crate::{
    db::{ApiAuthed, DB},
    jobs::check_license_key_valid,
    utils::require_super_admin,
};

use axum::{
    async_trait,
    extract::{Extension, FromRequestParts, Path},
    routing::post,
    Json, Router,
};
use http::request::Parts;
use hyper::StatusCode;
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use windmill_common::{
    agent_workers::QueueInitJob,
    error::{JsonResult, Result},
    jwt::encode_with_internal_secret,
    worker::{update_ping_http, Ping},
};
use windmill_queue::{pull, push_init_job, PulledJobResult};

#[cfg(feature = "enterprise")]
pub fn global_service() -> Router {
    use axum::routing::get;

    Router::new()
        .route("/queue_init_job", post(queue_init_job))
        .route("/create_agent_token", post(create_agent_token))
        .route("/update_ping", post(update_worker_ping))
        .route("/pull_job", post(pull_job))
        .route("/get_global_setting/:key", get(get_global_setting))
        .layer(Extension(Arc::new(AgentCache::new())))
}

#[cfg(not(feature = "enterprise"))]
pub fn global_service() -> Router {
    Router::new()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
}

impl AgentAuth {
    pub fn worker_name(&self) -> String {
        format!(
            "wk-{}-{}",
            self.worker_group,
            self.suffix.as_ref().unwrap_or(&"XXXX".to_string())
        )
    }
}

pub struct AgentCache {
    cache: Cache<String, AgentAuth>,
}

impl AgentCache {
    pub fn new() -> Self {
        AgentCache { cache: Cache::new(1000) }
    }

    pub async fn get_agent_authed(&self, token: &str) -> Option<AgentAuth> {
        let agent = self.cache.get(token);
        if agent.is_some() {
            agent
        } else {
            // #[cfg(feature = "enterprise")]
            if let Some(trimmed) = token.strip_prefix("jwt_agent_") {
                let splitted = trimmed.split_once("_");
                if let Some((suffix, jwt)) = splitted {
                    let decoded =
                        windmill_common::jwt::decode_with_internal_secret::<AgentAuth>(jwt).await;
                    if let Ok(mut decoded) = decoded {
                        decoded.suffix = Some(suffix.to_string());
                        self.cache.insert(token.to_string(), decoded.clone());
                        return Some(decoded);
                    } else {
                        tracing::error!("JWT_AGENT auth error: {:?}", decoded.unwrap_err());
                    }
                } else {
                    tracing::error!("jwt agent token missing suffix");
                }
            }
            None
        }
    }
}

async fn queue_init_job(
    authed: AgentAuth,
    Extension(db): Extension<DB>,
    Json(QueueInitJob { content }): Json<QueueInitJob>,
) -> Result<StatusCode> {
    push_init_job(&db, content, &authed.worker_name()).await?;
    Ok(StatusCode::OK)
}

#[async_trait]
impl<S> FromRequestParts<S> for AgentAuth
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        if parts.method == http::Method::OPTIONS {
            return Ok(AgentAuth { worker_group: "".to_string(), suffix: None, tags: Vec::new() });
        };

        let auth_header = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(|x| x.to_string()));

        if let Some(token) = auth_header {
            if let Ok(Extension(cache)) =
                Extension::<Arc<AgentCache>>::from_request_parts(parts, state).await
            {
                if let Some(authed) = cache.get_agent_authed(&token).await {
                    return Ok(authed);
                }
            }
        }
        Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
    }
}

async fn create_agent_token(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(claims): Json<AgentAuth>,
) -> JsonResult<String> {
    require_super_admin(&db, &authed.email).await?;
    let token = format!("jwt_agent_{}", encode_with_internal_secret(claims).await?);
    Ok(Json(token))
}

async fn update_worker_ping(
    authed: AgentAuth,
    Extension(db): Extension<DB>,
    Json(update_ping): Json<Ping>,
) -> JsonResult<()> {
    // check_license_key_valid().await?;
    update_ping_http(
        update_ping,
        &authed.worker_name(),
        &authed.worker_group,
        &db,
    )
    .await?;
    Ok(Json(()))
}

pub async fn get_global_setting(
    Extension(db): Extension<DB>,
    _authed: AgentAuth,
    Path(key): Path<String>,
) -> JsonResult<serde_json::Value> {
    let value = sqlx::query!("SELECT value FROM global_settings WHERE name = $1", key)
        .fetch_optional(&db)
        .await?
        .map(|x| x.value);

    Ok(Json(value.unwrap_or_else(|| serde_json::Value::Null)))
}

async fn pull_job(
    authed: AgentAuth,
    Extension(db): Extension<DB>,
    // Json(request): Json<PullJobRequest>,
) -> JsonResult<PulledJobResult> {
    let job = pull(&db, false, &authed.worker_name()).await?;
    Ok(Json(job))
}
