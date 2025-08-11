use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        Self {
            error_handler_path: None,
            error_handler_args: None,
            retry: None,
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            enabled: true,
            server_id: None,
            last_server_ping: None,
            error: None,
        }
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