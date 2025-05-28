use crate::db::DB;
use axum::Router;

pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

pub struct AgentCache {}

impl AgentCache {
    pub fn new() -> Self {
        crate::agent_workers_ee::AgentCache::new()
    }
}

pub fn global_service() -> Router {
    crate::agent_workers_ee::global_service()
}

pub fn workspaced_service(
    db: DB,
    base_internal_url: String,
) -> (
    Router,
    Vec<tokio::task::JoinHandle<()>>,
    Option<windmill_worker::JobCompletedSender>,
) {
    crate::agent_workers_ee::workspaced_service(db, base_internal_url)
}