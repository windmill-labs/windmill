use crate::db::{ApiAuthed, DB};
use axum::{extract::Request, Router};
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::prelude::FromRow;
use sqlx::types::Json as SqlxJson;
use std::collections::HashMap;
use windmill_common::db::UserDB;
use windmill_common::{
    error::{Error as WindmillError, Result as WindmillResult},
    utils::empty_string_as_none,
};

#[derive(sqlx::Type, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[sqlx(type_name = "DELIVERY_MODE", rename_all = "lowercase")]
#[allow(unused)]
pub enum DeliveryType {
    Pull,
    Push,
}

impl Default for DeliveryType {
    fn default() -> Self {
        Self::Pull
    }
}

#[derive(FromRow, Deserialize, Serialize, Debug)]
#[allow(unused)]
pub struct PushConfig {
    #[serde(deserialize_with = "empty_string_as_none")]
    route_path: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    audience: Option<String>,
    authenticate: bool,
    base_endpoint: String,
}
#[derive(Default, Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct CreateUpdateConfig {
    pub delivery_type: DeliveryType,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub subscription_id: Option<String>,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExistingGcpSubscription {
    pub subscription_id: String,
    pub base_endpoint: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "subscription_mode", rename_all = "snake_case")]
pub enum SubscriptionMode {
    Existing(ExistingGcpSubscription),
    CreateUpdate(CreateUpdateConfig),
}

pub fn workspaced_service() -> Router {
    Router::new()
}

pub fn start_consuming_gcp_pubsub_event(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    // implementation is not open source
}

pub async fn manage_google_subscription(
    _authed: ApiAuthed,
    _db: &DB,
    _workspace_id: &str,
    _gcp_resource_path: &str,
    _path: &str,
    _topic_id: &str,
    _subscription_mode: SubscriptionMode,
) -> WindmillResult<CreateUpdateConfig> {
    Ok(CreateUpdateConfig::default())
}

pub async fn process_google_push_request(
    _headers: HeaderMap,
    _request: Request,
) -> Result<
    (
        HashMap<String, Box<RawValue>>,
        Option<HashMap<String, Box<RawValue>>>,
    ),
    WindmillError,
> {
    Ok((HashMap::new(), None))
}

pub async fn validate_jwt_token(
    _db: &DB,
    _user_db: UserDB,
    _authed: ApiAuthed,
    _headers: &HeaderMap,
    _gcp_resource_path: &str,
    _workspace_id: &str,
    _delivery_config: &PushConfig,
) -> Result<(), windmill_common::error::Error> {
    Ok(())
}

pub fn gcp_push_route_handler() -> Router {
    Router::new()
}

#[derive(FromRow, Deserialize, Serialize, Debug)]
pub struct GcpTrigger {
    pub gcp_resource_path: String,
    pub subscription_id: String,
    pub delivery_type: DeliveryType,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
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
}
