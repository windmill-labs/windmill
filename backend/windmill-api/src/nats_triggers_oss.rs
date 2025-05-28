use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NatsResourceAuth {}

#[derive(Serialize, Deserialize)]
pub enum NatsTriggerConfigConnection {}

#[derive(Serialize, Clone)]
pub struct NatsTrigger {
    pub workspace_id: String,
    pub path: String,
    pub nats_resource_path: String,
    pub subject: String,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub server_id: Option<String>,
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub extra_perms: serde_json::Value,
    pub error: Option<String>,
    pub enabled: bool,
}

pub fn workspaced_service() -> Router {
    crate::nats_triggers_ee::workspaced_service()
}

pub fn start_nats_consumers(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    crate::nats_triggers_ee::start_nats_consumers(_db, _killpill_rx)
}