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
use windmill_common::agent_workers::{BlacklistTokenRequest, blacklist_token, remove_token_from_blacklist};

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
    // For OSS version, return not implemented
    Err((StatusCode::NOT_IMPLEMENTED, "Blacklist functionality requires Enterprise Edition".to_string()))
}

#[cfg(not(feature = "private"))]
async fn remove_blacklist_token_handler(
    State(db): State<DB>,
    Json(req): Json<BlacklistTokenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // For OSS version, return not implemented
    Err((StatusCode::NOT_IMPLEMENTED, "Blacklist functionality requires Enterprise Edition".to_string()))
}
