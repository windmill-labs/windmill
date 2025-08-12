use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow};
use std::collections::HashMap;

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct BaseTrigger {
    pub workspace_id: String,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
    pub extra_perms: serde_json::Value,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct TriggerErrorHandling {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<SqlxJson<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SqlxJson<crate::flows::Retry>>,
}

impl Default for TriggerErrorHandling {
    fn default() -> Self {
        Self { error_handler_path: None, error_handler_args: None, retry: None }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self { enabled: true, server_id: None, last_server_ping: None, error: None }
    }
}

impl BaseTrigger {
    pub fn new(
        workspace_id: String,
        path: String,
        script_path: String,
        is_flow: bool,
        edited_by: String,
        email: String,
    ) -> Self {
        Self {
            workspace_id,
            path,
            script_path,
            is_flow,
            edited_by,
            email,
            edited_at: Utc::now(),
            extra_perms: serde_json::json!({}),
        }
    }
}

#[derive(FromRow, Serialize, Deserialize, Clone)]
pub struct Trigger<T> {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub base: BaseTrigger,

    #[sqlx(flatten)]
    #[serde(flatten)]
    pub config: T,

    #[sqlx(flatten)]
    #[serde(flatten)]
    pub server_state: ServerState,
    
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEditTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTrigger<T> {
    #[serde(flatten)]
    pub base: BaseEditTrigger,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,

    #[serde(flatten)]
    pub config: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseCreateTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTrigger<T> {
    #[serde(flatten)]
    pub base: BaseCreateTrigger,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,

    #[serde(flatten)]
    pub config: T,
}
