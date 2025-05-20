use crate::db::{ApiAuthed, DB};
use crate::trigger_helpers::TriggerJobArgs;
use axum::{extract::Request, Router};
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::prelude::FromRow;
use sqlx::types::Json as SqlxJson;
use std::collections::HashMap;
use windmill_common::db::UserDB;
use windmill_common::worker::to_raw_value;
#[cfg(feature = "private")]
use crate::gcp_triggers_ee;

use windmill_common::{
    error::{Error as WindmillError, Result as WindmillResult},
    utils::empty_as_none,
};
use windmill_queue::TriggerKind;

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
    #[serde(deserialize_with = "empty_as_none")]
    route_path: Option<String>,
    #[serde(deserialize_with = "empty_as_none")]
    audience: Option<String>,
    authenticate: bool,
    base_endpoint: String,
}
#[derive(Default, Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct CreateUpdateConfig {
    pub delivery_type: DeliveryType,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub subscription_id: Option<String>,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExistingGcpSubscription {
    pub subscription_id: String,
    pub base_endpoint: String,
}

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "GCP_SUBSCRIPTION_MODE", rename_all = "snake_case")]
pub enum SubscriptionMode {
    Existing,
    CreateUpdate,
}

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return gcp_triggers_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

pub fn start_consuming_gcp_pubsub_event(
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    #[cfg(feature = "private")]
    {
        gcp_triggers_ee::start_consuming_gcp_pubsub_event(db, killpill_rx);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, killpill_rx);
        // implementation is not open source
    }
}

pub async fn manage_google_subscription(
    authed: ApiAuthed,
    db: &DB,
    workspace_id: &str,
    gcp_resource_path: &str,
    path: &str,
    topic_id: &str,
    subscription_id: &mut Option<String>,
    base_endpoint: &mut Option<String>,
    subscription_mode: SubscriptionMode,
    create_update_config: Option<CreateUpdateConfig>,
    trigger_mode: bool,
    is_flow: bool
) -> WindmillResult<CreateUpdateConfig> {
    #[cfg(feature = "private")]
    {
        return gcp_triggers_ee::manage_google_subscription(authed, db, workspace_id, gcp_resource_path, path, topic_id, subscription_id, base_endpoint, subscription_mode, create_update_config, trigger_mode, is_flow).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (authed, db, workspace_id, gcp_resource_path, path, topic_id, subscription_id, base_endpoint, subscription_mode, create_update_config, trigger_mode, is_flow);
        Ok(CreateUpdateConfig::default())
    }
}

pub async fn process_google_push_request(
    headers: HeaderMap,
    request: Request,
) -> Result<(String, HashMap<String, Box<RawValue>>), WindmillError> {
    #[cfg(feature = "private")]
    {
        return gcp_triggers_ee::process_google_push_request(headers, request).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (headers, request);
        Ok((String::new(), HashMap::new()))
    }
}

pub async fn validate_jwt_token(
    db: &DB,
    user_db: UserDB,
    authed: ApiAuthed,
    headers: &HeaderMap,
    gcp_resource_path: &str,
    workspace_id: &str,
    delivery_config: &PushConfig,
) -> Result<(), windmill_common::error::Error> {
    #[cfg(feature = "private")]
    {
        return gcp_triggers_ee::validate_jwt_token(db, user_db, authed, headers, gcp_resource_path, workspace_id, delivery_config).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, user_db, authed, headers, gcp_resource_path, workspace_id, delivery_config);
        Ok(())
    }
}

pub fn gcp_push_route_handler() -> Router {
    #[cfg(feature = "private")]
    {
        return gcp_triggers_ee::gcp_push_route_handler();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
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
}

impl TriggerJobArgs<String> for GcpTrigger {
    fn v1_payload_fn(payload: String) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_string(), to_raw_value(&payload))])
    }

    fn trigger_kind() -> TriggerKind {
        TriggerKind::Gcp
    }
}
