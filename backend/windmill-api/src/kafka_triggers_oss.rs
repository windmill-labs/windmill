use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct KafkaResourceSecurity {}

pub fn workspaced_service() -> Router {
    Router::new()
}

pub fn start_kafka_consumers(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    // implementation is not open source
}

#[derive(Serialize, Deserialize)]
pub enum KafkaTriggerConfigConnection {}

#[derive(Serialize, Clone)]
pub struct KafkaTrigger {
    pub workspace_id: String,
    pub path: String,
    pub kafka_resource_path: String,
    pub group_id: String,
    pub topics: Vec<String>,
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