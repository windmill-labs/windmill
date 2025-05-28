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
    crate::agent_workers_ee::global_service()
}

pub fn workspaced_service(
    db: DB,
    _base_internal_url: String,
) -> (
    Router,
    Vec<tokio::task::JoinHandle<()>>,
    Option<windmill_worker::JobCompletedSender>,
) {
    crate::agent_workers_ee::workspaced_service(db, _base_internal_url)
}

pub use crate::agent_workers_ee::AgentAuth;
pub use crate::agent_workers_ee::AgentCache;
