/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::audit_ee;
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
    db: E,
    author: &impl AuditAuthorable,
    mut operation: &str,
    action_kind: ActionKind,
    w_id: &str,
    mut resource: Option<&str>,
    parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    #[cfg(feature = "private")]
    {
        audit_ee::audit_log(db, author, operation, action_kind, w_id, resource, parameters).await
    }
    #[cfg(not(feature = "private"))]
    {
        // Original OSS body:
        // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
        let _ = (db, author, operation, action_kind, w_id, resource, parameters); // Mark params as used
        Ok(())
    }
}

pub async fn list_audit(
    tx: Transaction<'_, Postgres>,
    w_id: String,
    pagination: Pagination,
    lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    #[cfg(feature = "private")]
    {
        audit_ee::list_audit(tx, w_id, pagination, lq).await
    }
    #[cfg(not(feature = "private"))]
    {
        // Original OSS body:
        // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
        let _ = (tx, w_id, pagination, lq); // Mark params as used
        return Ok(vec![]);
    }
}

pub async fn get_audit(tx: Transaction<'_, Postgres>, id: i32, w_id: &str) -> Result<AuditLog> {
    #[cfg(feature = "private")]
    {
        audit_ee::get_audit(tx, id, w_id).await
    }
    #[cfg(not(feature = "private"))]
    {
        // Original OSS body:
        // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
        let _ = (id, w_id); // Mark params as used, tx is used
        tx.commit().await?;
        Err(Error::NotFound(
            "Audit log not not available in Windmill Community edition".to_string(),
        ))
    }
}
