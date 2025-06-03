#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::sqs_triggers_ee::*;

#[cfg(not(feature = "private"))]
use crate::db::DB;
#[cfg(not(feature = "private"))]
use axum::Router;
#[cfg(not(feature = "private"))]
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "private"))]
use windmill_common::auth::aws::AwsAuthResourceType;

#[cfg(not(feature = "private"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(not(feature = "private"))]
pub fn start_sqs(_db: DB, mut _killpill_rx: tokio::sync::broadcast::Receiver<()>) -> () {
    // implementation is not open source
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg(not(feature = "private"))]
pub struct SqsTrigger {
    pub queue_url: String,
    pub aws_auth_resource_type: AwsAuthResourceType,
    pub aws_resource_path: String,
    pub message_attributes: Option<Vec<String>>,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: Option<serde_json::Value>,
    pub error: Option<String>,
    pub server_id: Option<String>,
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}
