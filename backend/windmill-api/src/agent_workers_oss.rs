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
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
#[cfg(not(feature = "private"))]
use chrono::{DateTime, Utc};
#[cfg(not(feature = "private"))]
use windmill_common::agent_workers::{
    blacklist_token_with_optional_expiry, remove_token_from_blacklist, BlacklistTokenRequest,
};

#[cfg(not(feature = "private"))]
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new()
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
