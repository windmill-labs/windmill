use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NatsResourceAuth {}

pub fn workspaced_service() -> Router {
    Router::new()
}

pub fn start_nats_consumers(_db: DB, mut _killpill_rx: tokio::sync::broadcast::Receiver<()>) -> () {
    // implementation is not open source
}

#[derive(Serialize, Deserialize)]
pub enum NatsTriggerConfigConnection {}

#[derive(Serialize, Clone)]
pub struct NatsTrigger {
    pub workspace_id: String,
    pub path: String,
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer_name: Option<String>,
    pub use_jetstream: bool,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub enabled: bool,
}