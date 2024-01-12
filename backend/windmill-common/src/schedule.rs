/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

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
    pub args: Option<serde_json::Value>,
    pub extra_perms: serde_json::Value,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub on_failure: Option<String>,
    pub on_failure_times: Option<i32>,
    pub on_failure_exact: Option<bool>,
    pub on_failure_extra_args: Option<serde_json::Value>,
    pub on_recovery: Option<String>,
    pub on_recovery_times: Option<i32>,
    pub on_recovery_extra_args: Option<serde_json::Value>,
    pub ws_error_handler_muted: bool,
    pub retry: Option<serde_json::Value>,
    pub no_flow_overlap: bool,
    pub summary: Option<String>,
    pub tag: Option<String>,
}

impl Schedule {
    pub fn parse_retry(self) -> Option<Retry> {
        self.retry.map(|r| serde_json::from_value(r).ok()).flatten()
    }
}

pub fn schedule_to_user(path: &str) -> String {
    format!("schedule-{}", path.replace('/', "-"))
}
