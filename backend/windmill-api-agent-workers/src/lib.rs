#[cfg(feature = "private")]
mod ee;
#[cfg(feature = "private")]
#[allow(unused)]
pub use ee::*;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use windmill_common::DB;

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn global_service(_job_completed_tx: windmill_worker::JobCompletedSender) -> Router {
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

#[cfg(not(feature = "private"))]
pub struct AgentCache {}

#[cfg(not(feature = "private"))]
impl AgentCache {
    pub fn new() -> Self {
        AgentCache {}
    }
}
