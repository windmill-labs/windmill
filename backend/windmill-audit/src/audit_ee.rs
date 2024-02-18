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

#[tracing::instrument(level = "trace", skip_all)]
pub async fn audit_log<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    _db: E,
    _username: &str,
    mut _operation: &str,
    _action_kind: ActionKind,
    _w_id: &str,
    mut _resource: Option<&str>,
    _parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
    Ok(())
}

pub async fn list_audit(
    _tx: Transaction<'_, Postgres>,
    _w_id: String,
    _pagination: Pagination,
    _lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
    return Ok(vec![]);
}

pub async fn get_audit(tx: Transaction<'_, Postgres>, _id: i32, _w_id: &str) -> Result<AuditLog> {
    // Implementation is not open source as Audit logs is a Windmill Enterprise Edition feature
    tx.commit().await?;
    Err(Error::NotFound(
        "Audit log not not available in Windmill Community edition".to_string(),
    ))
}
