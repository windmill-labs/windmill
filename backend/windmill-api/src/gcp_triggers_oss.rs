#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::gcp_triggers_ee::*;

use crate::trigger_helpers::TriggerJobArgs;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::prelude::FromRow;
use sqlx::types::Json as SqlxJson;
use std::collections::HashMap;
use windmill_common::triggers::TriggerKind;
use windmill_common::worker::to_raw_value;

#[cfg(not(feature = "private"))]
use {crate::db::DB, windmill_common::utils::empty_as_none};

#[derive(sqlx::Type, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[sqlx(type_name = "DELIVERY_MODE", rename_all = "lowercase")]
#[allow(unused)]
#[cfg(not(feature = "private"))]
pub enum DeliveryType {
    Pull,
    Push,
}

#[cfg(not(feature = "private"))]
impl Default for DeliveryType {
    fn default() -> Self {
        Self::Pull
    }
}

#[derive(FromRow, Deserialize, Serialize, Debug)]
#[allow(unused)]
#[cfg(not(feature = "private"))]
pub struct PushConfig {
    #[serde(deserialize_with = "empty_as_none")]
    route_path: Option<String>,
    #[serde(deserialize_with = "empty_as_none")]
    audience: Option<String>,
    authenticate: bool,
    base_endpoint: String,
}
#[derive(Default, Debug, Serialize, Deserialize)]
#[allow(unused)]
#[cfg(not(feature = "private"))]
pub struct CreateUpdateConfig {
    pub delivery_type: DeliveryType,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub subscription_id: Option<String>,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg(not(feature = "private"))]
pub struct ExistingGcpSubscription {
    pub subscription_id: String,
    pub base_endpoint: String,
}

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "GCP_SUBSCRIPTION_MODE", rename_all = "snake_case")]
#[cfg(not(feature = "private"))]
pub enum SubscriptionMode {
    Existing,
    CreateUpdate,
}

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

impl TriggerJobArgs<String> for GcpTrigger {
    fn v1_payload_fn(payload: String) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_string(), to_raw_value(&payload))])
    }

    fn trigger_kind() -> TriggerKind {
        TriggerKind::Gcp
    }
}
