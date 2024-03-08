use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub mod audit_ee;

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
    pub id: i32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub username: String,
    pub operation: String,
    pub action_kind: ActionKind,
    pub resource: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ListAuditLogQuery {
    pub username: Option<String>,
    pub operation: Option<String>,
    pub action_kind: Option<String>,
    pub resource: Option<String>,
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    pub after: Option<chrono::DateTime<chrono::Utc>>,
}
