use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct SqsTrigger {
    pub workspace_id: String,
    pub path: String,
    pub sqs_resource_path: String,
    pub queue_url: String,
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
    crate::sqs_triggers_ee::workspaced_service()
}

pub fn start_sqs(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    crate::sqs_triggers_ee::start_sqs(_db, _killpill_rx)
}