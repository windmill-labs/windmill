use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use windmill_common::error::Result;

#[derive(Serialize, Deserialize)]
pub struct AuditAuthor {
    pub username: String,
    pub email: String,
    pub username_override: Option<String>,
}

pub trait AuditAuthorable {
    fn username(&self) -> &str;
    fn email(&self) -> &str;
    fn username_override(&self) -> Option<&str>;
}

impl AuditAuthorable for AuditAuthor {
    fn username(&self) -> &str {
        &self.username
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }
}

pub struct AuditLog;

pub async fn audit_log<A: AuditAuthorable>(
    _tx: &mut Transaction<'_, Postgres>,
    _author: &A,
    _operation: &str,
    _action_kind: &str,
    _workspace_id: &str,
    _resource: Option<&str>,
    _parameters: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<()> {
    crate::audit_ee::audit_log(_tx, _author, _operation, _action_kind, _workspace_id, _resource, _parameters).await
}

pub async fn list_audit(
    _tx: Transaction<'_, Postgres>,
    _workspace_id: &str,
    _username: Option<String>,
    _operation: Option<String>,
    _action_kind: Option<String>,
    _resource: Option<String>,
    _before: Option<chrono::DateTime<chrono::Utc>>,
    _after: Option<chrono::DateTime<chrono::Utc>>,
    _per_page: usize,
    _offset: usize,
) -> Result<Vec<AuditLog>> {
    crate::audit_ee::list_audit(_tx, _workspace_id, _username, _operation, _action_kind, _resource, _before, _after, _per_page, _offset).await
}

pub async fn get_audit(
    tx: Transaction<'_, Postgres>,
    _id: i32,
    _w_id: &str,
) -> Result<AuditLog> {
    crate::audit_ee::get_audit(tx, _id, _w_id).await
}