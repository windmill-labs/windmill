/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow, Pool, Postgres};
use std::{collections::HashMap, fmt::Debug};
use windmill_common::{db::Authable, error::Result, jobs::JobTriggerKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HandlerAction {
    Trigger { path: String, trigger_kind: JobTriggerKind },
    // Future variants can be added here (e.g., Script, Flow, etc.)
}

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
    /// Optional email for deployment - when set, the trigger will run jobs as this user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// If true and user is admin/wm_deployers, preserve the provided email instead of using deploying user's email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_email: Option<bool>,
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

    pub async fn resolve_email(
        &self,
        authed: &impl Authable,
        db: &Pool<Postgres>,
        w_id: &str,
    ) -> Result<String> {
        if let Some(ref username) = self.email {
            if self.preserve_email.unwrap_or(false)
                && windmill_common::can_preserve_on_behalf_of(authed)
            {
                let email = sqlx::query_scalar!(
                    "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
                    username,
                    w_id
                )
                .fetch_optional(db)
                .await?;
                if let Some(email) = email {
                    return Ok(email);
                }
            }
        }
        Ok(authed.email().to_string())
    }

    pub fn resolve_edited_by(&self, authed: &impl Authable) -> String {
        if let Some(ref username) = self.email {
            if self.preserve_email.unwrap_or(false)
                && windmill_common::can_preserve_on_behalf_of(authed)
            {
                return username.clone();
            }
        }
        authed.username().to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- TriggerMode serde ---

    #[test]
    fn test_trigger_mode_serialize() {
        assert_eq!(
            serde_json::to_value(TriggerMode::Enabled).unwrap(),
            json!("enabled")
        );
        assert_eq!(
            serde_json::to_value(TriggerMode::Disabled).unwrap(),
            json!("disabled")
        );
        assert_eq!(
            serde_json::to_value(TriggerMode::Suspended).unwrap(),
            json!("suspended")
        );
    }

    #[test]
    fn test_trigger_mode_deserialize() {
        let enabled: TriggerMode = serde_json::from_value(json!("enabled")).unwrap();
        assert_eq!(enabled, TriggerMode::Enabled);
        let disabled: TriggerMode = serde_json::from_value(json!("disabled")).unwrap();
        assert_eq!(disabled, TriggerMode::Disabled);
    }

    #[test]
    fn test_trigger_mode_invalid() {
        let result: std::result::Result<TriggerMode, _> = serde_json::from_value(json!("paused"));
        assert!(result.is_err());
    }

    // --- StandardTriggerQuery ---

    #[test]
    fn test_query_default() {
        let q = StandardTriggerQuery::default();
        assert_eq!(q.offset(), 0);
        assert_eq!(q.limit(), 100);
    }

    #[test]
    fn test_query_offset_calculation() {
        let q = StandardTriggerQuery {
            page: Some(2),
            per_page: Some(50),
            path: None,
            is_flow: None,
            path_start: None,
        };
        assert_eq!(q.offset(), 100);
        assert_eq!(q.limit(), 50);
    }

    #[test]
    fn test_query_offset_defaults() {
        let q = StandardTriggerQuery {
            page: None,
            per_page: None,
            path: None,
            is_flow: None,
            path_start: None,
        };
        assert_eq!(q.offset(), 0);
        assert_eq!(q.limit(), 100);
    }

    // --- BaseTriggerData backward compatibility ---

    #[test]
    fn test_base_trigger_data_mode_field() {
        let json = r#"{
            "path": "test",
            "script_path": "f/test/script",
            "is_flow": false,
            "mode": "enabled"
        }"#;
        let data: BaseTriggerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.mode(), &TriggerMode::Enabled);
    }

    #[test]
    fn test_base_trigger_data_legacy_enabled_true() {
        let json = r#"{
            "path": "test",
            "script_path": "f/test/script",
            "is_flow": false,
            "enabled": true
        }"#;
        let data: BaseTriggerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.mode(), &TriggerMode::Enabled);
    }

    #[test]
    fn test_base_trigger_data_legacy_enabled_false() {
        let json = r#"{
            "path": "test",
            "script_path": "f/test/script",
            "is_flow": false,
            "enabled": false
        }"#;
        let data: BaseTriggerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.mode(), &TriggerMode::Disabled);
    }

    #[test]
    fn test_base_trigger_data_mode_takes_precedence() {
        let json = r#"{
            "path": "test",
            "script_path": "f/test/script",
            "is_flow": false,
            "mode": "suspended",
            "enabled": true
        }"#;
        let data: BaseTriggerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.mode(), &TriggerMode::Suspended);
    }

    #[test]
    fn test_base_trigger_data_neither_field() {
        let json = r#"{
            "path": "test",
            "script_path": "f/test/script",
            "is_flow": false
        }"#;
        let data: BaseTriggerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.mode(), &TriggerMode::Enabled);
    }

    // --- HandlerAction ---

    #[test]
    fn test_handler_action_serialization() {
        let action = HandlerAction::Trigger {
            path: "f/test/trigger".to_string(),
            trigger_kind: JobTriggerKind::Webhook,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "trigger");
        assert_eq!(json["path"], "f/test/trigger");
    }

    #[test]
    fn test_handler_action_deserialization() {
        let json = r#"{"type": "trigger", "path": "f/test/trigger", "trigger_kind": "webhook"}"#;
        let action: HandlerAction = serde_json::from_str(json).unwrap();
        match action {
            HandlerAction::Trigger { path, trigger_kind } => {
                assert_eq!(path, "f/test/trigger");
                assert_eq!(
                    serde_json::to_value(&trigger_kind).unwrap(),
                    serde_json::to_value(&JobTriggerKind::Webhook).unwrap()
                );
            }
        }
    }

    // --- ServerState ---

    #[test]
    fn test_server_state_skip_none_fields() {
        let state = ServerState { server_id: None, last_server_ping: None, error: None };
        let json = serde_json::to_value(&state).unwrap();
        assert!(!json.as_object().unwrap().contains_key("server_id"));
        assert!(!json.as_object().unwrap().contains_key("error"));
    }

    #[test]
    fn test_server_state_with_error() {
        let state = ServerState {
            server_id: Some("srv-1".to_string()),
            last_server_ping: None,
            error: Some("connection timeout".to_string()),
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["server_id"], "srv-1");
        assert_eq!(json["error"], "connection timeout");
    }
}
