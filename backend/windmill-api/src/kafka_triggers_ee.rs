#[cfg(feature = "private")]
use crate::kafka_triggers_ee;

use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct KafkaResourceSecurity {} // Stays in OSS

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return kafka_triggers_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

pub fn start_kafka_consumers(
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    #[cfg(feature = "private")]
    {
        kafka_triggers_ee::start_kafka_consumers(db, killpill_rx);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, killpill_rx);
        // implementation is not open source
    }
}

#[derive(Serialize, Deserialize)]
pub enum KafkaTriggerConfigConnection {} // Stays in OSS

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
