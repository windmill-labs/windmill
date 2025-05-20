/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::agent_workers_ee;

use crate::db::DB;

use axum::Router;

use serde::{Deserialize, Serialize};

pub fn global_service() -> Router {
    #[cfg(feature = "private")]
    {
        return agent_workers_ee::global_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

pub fn workspaced_service(
    db: DB,
    base_internal_url: String,
) -> (
    Router,
    Vec<tokio::task::JoinHandle<()>>,
    Option<windmill_worker::JobCompletedSender>,
) {
    #[cfg(feature = "private")]
    {
        return agent_workers_ee::workspaced_service(db, base_internal_url);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = base_internal_url; // Mark as used
        use windmill_common::worker::Connection;
        use windmill_worker::JobCompletedSender;

        let (job_completed_tx, _job_completed_rx) =
            JobCompletedSender::new(&Connection::Sql(db.clone()), 10);

        let router = Router::new();

        (router, vec![], Some(job_completed_tx))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAuth { // Stays in OSS
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

pub struct AgentCache {} // Stays in OSS

impl AgentCache { // Stays in OSS
    pub fn new() -> Self {
        AgentCache {}
    }
}
