#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::gcp_triggers_ee::*;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::prelude::FromRow;
use sqlx::types::Json as SqlxJson;
use std::collections::HashMap;

#[cfg(not(feature = "private"))]
use crate::db::DB;

#[cfg(not(feature = "private"))]
pub fn start_consuming_gcp_pubsub_event(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    // implementation is not open source
}

#[derive(FromRow, Deserialize, Serialize, Debug)]
pub struct GcpTrigger {
    pub gcp_resource_path: String,
    pub subscription_id: String,
    pub delivery_type: DeliveryType,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
    pub subscription_mode: SubscriptionMode,
    pub topic_id: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<SqlxJson<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SqlxJson<windmill_common::flows::Retry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_acknowledge_msg: Option<bool>,
}
