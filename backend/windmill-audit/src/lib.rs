use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[cfg(feature = "private")]
pub mod audit_ee;
pub mod audit_oss;
pub mod instance_config;

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "ACTION_KIND", rename_all = "lowercase")]
pub enum ActionKind {
    Create,
    Update,
    Delete,
    Execute,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct AuditLog {
    pub workspace_id: String,
    pub id: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub username: String,
    pub operation: String,
    pub action_kind: ActionKind,
    pub resource: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub span: Option<String>,
}

#[derive(Deserialize)]
pub struct ListAuditLogQuery {
    pub username: Option<String>,
    pub operation: Option<String>,
    pub operations: Option<String>,
    pub exclude_operations: Option<String>,
    pub action_kind: Option<String>,
    pub resource: Option<String>,
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    pub after: Option<chrono::DateTime<chrono::Utc>>,
    // Keyset cursor on the `timestamp DESC, id DESC` ordering: the id and timestamp of the last
    // row seen. Lets a client stream a page in small batches without paying a growing OFFSET on
    // every batch. `before_id` alone filters on the id only.
    pub before_id: Option<i64>,
    pub before_id_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub all_workspaces: Option<bool>,
    // The entries about one run: started, viewed, cancelled, resumed, deleted...
    pub job_id: Option<sqlx::types::Uuid>,
}
