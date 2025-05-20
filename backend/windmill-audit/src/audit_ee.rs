/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use std::collections::HashMap;

use windmill_common::{
    error::{Error, Result},
    utils::Pagination,
};

use crate::{ActionKind, AuditLog, ListAuditLogQuery};
use sqlx::{Postgres, Transaction};

#[cfg(feature = "private")]
use crate::audit_ee; // Points to the new audit_ee.rs

#[derive(Clone)]
pub struct AuditAuthor {
    pub username: String,
    pub email: String,
    pub username_override: Option<String>,
}

impl AuditAuthorable for AuditAuthor {
    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }
}

pub trait AuditAuthorable {
    fn username(&self) -> &str;
    fn email(&self) -> &str;
    fn username_override(&self) -> Option<&str>;
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn audit_log<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    _db: E,
    _author: &impl AuditAuthorable,
    mut _operation: &str,
    _action_kind: ActionKind,
    _w_id: &str,
    mut _resource: Option<&str>,
    _parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    #[cfg(feature = "private")]
    {
        audit_ee::audit_log(_db, _author, _operation, _action_kind, _w_id, _resource, _parameters).await
    }
    #[cfg(not(feature = "private"))]
    {
        // Original OSS body:
        // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
        Ok(())
    }
}

pub async fn list_audit(
    _tx: Transaction<'_, Postgres>,
    _w_id: String,
    _pagination: Pagination,
    _lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    #[cfg(feature = "private")]
    {
        audit_ee::list_audit(_tx, _w_id, _pagination, _lq).await
    }
    #[cfg(not(feature = "private"))]
    {
        // Original OSS body:
        // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
        return Ok(vec![]);
    }
}

pub async fn get_audit(tx: Transaction<'_, Postgres>, _id: i32, _w_id: &str) -> Result<AuditLog> {
    #[cfg(feature = "private")]
    {
        audit_ee::get_audit(tx, _id, _w_id).await
    }
    #[cfg(not(feature = "private"))]
    {
        // Original OSS body:
        // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
        tx.commit().await?;
        Err(Error::NotFound(
            "Audit log not not available in Windmill Community edition".to_string(),
        ))
    }
}
/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
// This file is compiled only when the "private" feature is enabled.
// It should contain the enterprise-specific implementations.

use std::collections::HashMap;
use windmill_common::{
    error::{Error, Result},
    utils::Pagination,
};
use crate::{ActionKind, AuditLog, ListAuditLogQuery}; // Assuming these types are accessible
use sqlx::{Postgres, Transaction};

// Re-declaring AuditAuthorable and AuditAuthor as they were in the original _ee file.
// If these are intended to be common, they could be defined in _oss and imported here.
// For this refactoring, mirroring the original structure.
pub trait AuditAuthorable {
    fn username(&self) -> &str;
    fn email(&self) -> &str;
    fn username_override(&self) -> Option<&str>;
}

#[derive(Clone)]
pub struct AuditAuthor {
    pub username: String,
    pub email: String,
    pub username_override: Option<String>,
}

impl AuditAuthorable for AuditAuthor {
    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn audit_log<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    _db: E,
    _author: &impl AuditAuthorable,
    _operation: &str, // Original had `mut _operation`
    _action_kind: ActionKind,
    _w_id: &str,
    _resource: Option<&str>, // Original had `mut _resource`
    _parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    let _ = (_db, _author, _operation, _action_kind, _w_id, _resource, _parameters); // silence warnings
    panic!("Enterprise version of audit_log is not available in this build.");
}

pub async fn list_audit(
    _tx: Transaction<'_, Postgres>,
    _w_id: String,
    _pagination: Pagination,
    _lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    let _ = (_tx, _w_id, _pagination, _lq); // silence warnings
    panic!("Enterprise version of list_audit is not available in this build.");
}

pub async fn get_audit(_tx: Transaction<'_, Postgres>, _id: i32, _w_id: &str) -> Result<AuditLog> {
    let _ = (_tx, _id, _w_id); // silence warnings
    panic!("Enterprise version of get_audit is not available in this build.");
}
