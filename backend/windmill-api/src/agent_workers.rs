/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{ApiAuthed, DB},
    utils::require_super_admin,
};

use axum::{
    async_trait,
    extract::{Extension, FromRequestParts},
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
    jwt::{decode_with_internal_secret, encode_with_internal_secret},
    worker::{update_ping_http, Ping},
};
use windmill_queue::{pull, push_init_job, PulledJobResult};

pub fn global_service() -> Router {
    Router::new()
        .route("/queue_init_job", post(queue_init_job))
        .route("/create_agent_token", post(create_agent_token))
        .route("/update_ping", post(update_worker_ping))
        .route("/pull_job", post(pull_job))
}

#[derive(Clone, Debug)]
pub struct AgentAuth {
    pub worker_name: String,
    pub tags: Vec<String>,
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
                        decoded.worker_name.push_str("-");
                        decoded.worker_name.push_str(suffix);
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
    AgentClaims { worker_name_prefix, .. }: AgentClaims,
    Extension(db): Extension<DB>,
    Json(QueueInitJob { content, worker_name }): Json<QueueInitJob>,
) -> Result<StatusCode> {
    if !worker_name.starts_with(&worker_name_prefix) {
        return Err(anyhow::anyhow!("Worker name must start with {}", worker_name_prefix).into());
    }
    push_init_job(&db, content, &worker_name).await?;
    Ok(StatusCode::OK)
}

#[derive(Serialize, Deserialize)]
struct AgentClaims {
    worker_name_prefix: String,
    tags: Vec<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for AgentClaims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        if parts.method == http::Method::OPTIONS {
            return Ok(AgentClaims { worker_name_prefix: "".to_string(), tags: Vec::new() });
        };

        let auth_header = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        if let Some(token) = auth_header {
            let claims = decode_with_internal_secret::<AgentClaims>(&token).await;
            match claims {
                Ok(claims) => Ok(claims),
                Err(_) => Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned())),
            }
        } else {
            Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
        }
    }
}

async fn create_agent_token(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(claims): Json<AgentClaims>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let token = encode_with_internal_secret(claims).await?;
    Ok(token)
}

async fn update_worker_ping(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(update_ping): Json<Ping>,
) -> JsonResult<()> {
    //require_super_admin(&db, &authed.email).await?;
    update_ping_http(update_ping, &db).await?;
    Ok(Json(()))
}

async fn pull_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    // Json(request): Json<PullJobRequest>,
) -> JsonResult<PulledJobResult> {
    let job = pull(&db, false, todo!()).await?;
    Ok(Json(job))
}
