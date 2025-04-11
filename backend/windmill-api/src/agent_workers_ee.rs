/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::DB;

use axum::Router;

use serde::{Deserialize, Serialize};

pub fn global_service() -> Router {
    Router::new()
}

pub fn workspaced_service(
    db: DB,
    _base_internal_url: String,
) -> (
    Router,
    Option<tokio::task::JoinHandle<()>>,
    windmill_worker::JobCompletedSender,
) {
    use windmill_common::worker::Connection;
    use windmill_worker::JobCompletedSender;

    let (job_completed_tx, _job_completed_rx) =
        JobCompletedSender::new(&Connection::Sql(db.clone()), 100);

    let router = Router::new();

    (router, None, job_completed_tx)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

pub struct AgentCache {}

impl AgentCache {
    pub fn new() -> Self {
        AgentCache {}
    }
}
