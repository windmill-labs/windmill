use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow};
use std::{collections::HashMap, fmt::Debug};
use windmill_common::jobs::JobTriggerKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HandlerAction {
    Trigger { path: String, trigger_kind: JobTriggerKind },
    // Future variants can be added here (e.g., Script, Flow, etc.)
}

#[cfg(all(feature = "smtp", feature = "enterprise", feature = "private"))]
pub mod email;
#[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
pub mod gcp;
#[cfg(feature = "http_trigger")]
pub mod http;
#[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
pub mod kafka;
#[cfg(feature = "mqtt_trigger")]
pub mod mqtt;
#[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
pub mod nats;
#[cfg(feature = "postgres_trigger")]
pub mod postgres;
#[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
pub mod sqs;
#[cfg(feature = "websocket")]
pub mod websocket;

pub mod global_handler;
mod handler;
mod listener;
pub mod trigger_helpers;

#[allow(unused)]
pub(crate) use handler::TriggerCrud;
pub use handler::{generate_trigger_routers, get_triggers_count_internal, TriggersCount};
pub use listener::start_all_listeners;
#[allow(unused)]
pub(crate) use listener::Listener;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct BaseTrigger {
    pub workspace_id: String,
    pub path: String,
    pub script_path: String,
    pub mode: TriggerMode,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ServerState {
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
    pub error_handler_args: Option<SqlxJson<HashMap<String, serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<sqlx::types::Json<windmill_common::flows::Retry>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Trigger<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    #[serde(flatten)]
    pub base: BaseTrigger,

    #[serde(flatten)]
    pub config: T,

    #[serde(flatten)]
    pub server_state: Option<ServerState>,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

impl<T> FromRow<'_, sqlx::postgres::PgRow> for Trigger<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    fn from_row(row: &sqlx::postgres::PgRow) -> std::result::Result<Self, sqlx::Error> {
        let base = BaseTrigger::from_row(row)?;

        Ok(Trigger {
            base,
            config: T::from_row(row)?,
            server_state: ServerState::from_row(row).ok(),
            error_handling: TriggerErrorHandling::from_row(row)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseTriggerData {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    #[deprecated(note = "Use mode instead")]
    enabled: Option<bool>, // Kept for backwards compatibility, use mode instead
    mode: Option<TriggerMode>,
}

impl BaseTriggerData {
    pub fn mode(&self) -> &TriggerMode {
        self.mode.as_ref().unwrap_or(
            #[allow(deprecated)]
            if self.enabled.unwrap_or(true) {
                &TriggerMode::Enabled
            } else {
                &TriggerMode::Disabled
            },
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerData<T: Debug> {
    #[serde(flatten)]
    pub base: BaseTriggerData,

    #[serde(flatten)]
    pub config: T,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

impl StandardTriggerQuery {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(0);
        let per_page = self.per_page.unwrap_or(100);
        (page * per_page) as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(100) as i64
    }
}

impl Default for StandardTriggerQuery {
    fn default() -> Self {
        Self { page: Some(0), per_page: Some(100), path: None, path_start: None, is_flow: None }
    }
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[sqlx(type_name = "TRIGGER_MODE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum TriggerMode {
    Enabled,
    Disabled,
    Suspended,
}
