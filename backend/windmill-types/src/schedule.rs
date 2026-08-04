use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::flows::Retry;

#[derive(FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct Schedule {
    pub workspace_id: String,
    pub path: String,
    pub edited_by: String,
    pub edited_at: DateTime<chrono::Utc>,
    pub schedule: String,
    pub timezone: String,
    pub enabled: bool,
    pub script_path: String,
    pub is_flow: bool,
    pub args: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    pub extra_perms: serde_json::Value,
    pub permissioned_as: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure_times: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure_exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure_extra_args: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_recovery: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_recovery_times: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_recovery_extra_args: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success_extra_args: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    pub ws_error_handler_muted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<serde_json::Value>,
    pub no_flow_overlap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_until: Option<DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_skip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

impl Schedule {
    pub fn parse_retry(self) -> Option<Retry> {
        self.retry.map(|r| serde_json::from_value(r).ok()).flatten()
    }
}

/// A [`Schedule`] plus the address of the identity it runs as. [`Schedule`] itself does not map
/// that column: `permissioned_as` is the identity, and the address is a function of it. The
/// database row still has an `email` column, and every save still writes it; removing it stops
/// those writes a release before dropping the column (`docs/schedule-email-removal.md`).
///
/// The two read paths fill it differently, and deliberately. `get_schedule` derives it, so the
/// response says what the *next run* will resolve to. The workspace export reads the stored
/// column, so a synced file reproduces the row — a principal whose account has since been
/// removed keeps the address the file already had instead of turning into a synthetic
/// `@unknown.windmill.dev`. When the column goes, this field stays, filled by deriving on both
/// paths.
// No `Deserialize`: `Schedule` is flattened in, and serde's flatten buffers through an untagged
// representation that `RawValue` (this type's `args`) cannot be read back from. `FromRow` is
// unaffected — it reads columns by name.
#[derive(Serialize, FromRow, Debug, Clone)]
pub struct ScheduleWithEmail {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub schedule: Schedule,
    pub email: String,
}

pub fn schedule_to_user(path: &str) -> String {
    format!("schedule-{}", path.replace('/', "-"))
}
