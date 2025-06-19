#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::agent_workers_ee::*;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use crate::db::DB;

#[cfg(not(feature = "private"))]
use axum::{routing::post, Router, extract::State, Json, http::StatusCode};
#[cfg(not(feature = "private"))]
use chrono::{DateTime, Utc};
#[cfg(not(feature = "private"))]
use windmill_common::agent_workers::{BlacklistTokenRequest, blacklist_token_with_optional_expiry, remove_token_from_blacklist};

#[cfg(not(feature = "private"))]
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new()
        .route("/api/agent_workers/blacklist_token", post(blacklist_token_handler))
        .route("/api/agent_workers/remove_blacklist_token", post(remove_blacklist_token_handler))
}

#[cfg(not(feature = "private"))]
pub fn workspaced_service(
    db: DB,
    _base_internal_url: String,
) -> (
    Router,
    Vec<tokio::task::JoinHandle<()>>,
    Option<windmill_worker::JobCompletedSender>,
) {
    use windmill_common::worker::Connection;
    use windmill_worker::JobCompletedSender;

    let (job_completed_tx, _job_completed_rx) =
        JobCompletedSender::new(&Connection::Sql(db.clone()), 10);

    let router = Router::new();

    (router, vec![], Some(job_completed_tx))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg(not(feature = "private"))]
pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

#[cfg(not(feature = "private"))]
pub struct AgentCache {}

#[cfg(not(feature = "private"))]
impl AgentCache {
    pub fn new() -> Self {
        AgentCache {}
    }
}

#[cfg(not(feature = "private"))]
async fn blacklist_token_handler(
    State(db): State<DB>,
    Json(req): Json<BlacklistTokenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Extract blacklisted_by from request context or use default
    let blacklisted_by = "system"; // TODO: Extract from auth context if available
    
    match blacklist_token_with_optional_expiry(&db, &req.token, req.expires_at, blacklisted_by).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to blacklist token: {}", e)))
    }
}

#[cfg(not(feature = "private"))]
async fn remove_blacklist_token_handler(
    State(db): State<DB>,
    Json(req): Json<BlacklistTokenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match remove_token_from_blacklist(&db, &req.token).await {
        Ok(removed) => {
            if removed {
                Ok(StatusCode::OK)
            } else {
                Err((StatusCode::NOT_FOUND, "Token not found in blacklist".to_string()))
            }
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to remove token from blacklist: {}", e)))
    }
}
