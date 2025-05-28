use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx_json::Json as SqlxJson;
use std::collections::HashMap;
use windmill_common::{error::WindmillError, utils::TriggerJobArgs, error::Result as WindmillResult};

#[derive(Serialize, Deserialize)]
pub enum DeliveryType {
    Pull,
    Push,
}

#[derive(Serialize, Deserialize)]
pub enum SubscriptionMode {
    Existing,
    CreateUpdate,
}

#[derive(Serialize, Deserialize)]
pub struct PushConfig {
    route_path: Option<String>,
    audience: Option<String>,
    authenticate: bool,
    base_endpoint: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUpdateConfig {
    pub delivery_type: DeliveryType,
    pub subscription_id: Option<String>,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
}

#[derive(Serialize, Deserialize)]
pub struct ExistingGcpSubscription {
    pub subscription_id: String,
    pub base_endpoint: String,
}

#[derive(Serialize, Clone)]
pub struct GcpTrigger {
    pub workspace_id: String,
    pub path: String,
    pub gcp_resource_path: String,
    pub project_id: String,
    pub topic_id: String,
    pub subscription_mode: SubscriptionMode,
    pub existing_subscription: Option<ExistingGcpSubscription>,
    pub create_update_config: Option<CreateUpdateConfig>,
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

impl TriggerJobArgs<String> for GcpTrigger {
    fn get_workspace_id(&self) -> &str {
        &self.workspace_id
    }

    fn get_path(&self) -> &str {
        &self.path
    }

    fn get_script_path(&self) -> &str {
        &self.script_path
    }

    fn get_is_flow(&self) -> bool {
        self.is_flow
    }

    fn get_extra_perms(&self) -> &serde_json::Value {
        &self.extra_perms
    }

    fn get_args(&self) -> &String {
        &String::new()
    }
}

pub fn workspaced_service() -> Router {
    crate::gcp_triggers_ee::workspaced_service()
}

pub fn start_consuming_gcp_pubsub_event(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    crate::gcp_triggers_ee::start_consuming_gcp_pubsub_event(_db, _killpill_rx)
}

pub async fn manage_google_subscription(
    _authed: windmill_common::users::Authed,
    _path: axum::extract::Path<String>,
    _axum::extract::Json(_trigger): axum::extract::Json<GcpTrigger>,
) -> WindmillResult<CreateUpdateConfig> {
    crate::gcp_triggers_ee::manage_google_subscription(_authed, _path, axum::extract::Json(_trigger)).await
}

pub async fn process_google_push_request(
    _headers: axum::http::HeaderMap,
    _body: axum::body::Bytes,
    _base_endpoint: String,
) -> Result<(String, HashMap<String, Box<RawValue>>), WindmillError> {
    crate::gcp_triggers_ee::process_google_push_request(_headers, _body, _base_endpoint).await
}

pub async fn validate_jwt_token(
    _audience: &str,
    _jwt_token: &str,
) -> Result<(), windmill_common::error::Error> {
    crate::gcp_triggers_ee::validate_jwt_token(_audience, _jwt_token).await
}

pub fn gcp_push_route_handler() -> Router {
    crate::gcp_triggers_ee::gcp_push_route_handler()
}