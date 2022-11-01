/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sql_builder::prelude::*;

use std::collections::HashMap;

use windmill_common::{
    error::{Error, Result},
    utils::Pagination,
};

use serde::{Deserialize, Serialize};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};

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

#[tracing::instrument(level = "trace", skip_all)]
pub async fn audit_log<'c>(
    db: &mut Transaction<'c, Postgres>,
    username: &str,
    operation: &str,
    action_kind: ActionKind,
    w_id: &str,
    resource: Option<&str>,
    parameters: Option<HashMap<&str, &str>>,
) -> Result<()> {
    let p_json: serde_json::Value = serde_json::to_value(&parameters).unwrap();

    tracing::info!(
        operation = operation,
        action_kind = ?action_kind,
        resource = resource,
        parameters = %p_json,
        workspace_id = w_id,
        username = username,
    );
    sqlx::query(
        "INSERT INTO audit
            (workspace_id, username, operation, action_kind, resource, parameters)
            VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(w_id)
    .bind(username)
    .bind(operation)
    .bind(action_kind)
    .bind(resource)
    .bind(p_json)
    .execute(db)
    .await?;
    Ok(())
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

pub async fn list_audit(
    mut tx: Transaction<'_, sqlx::Postgres>,
    w_id: String,
    pagination: Pagination,
    lq: ListAuditLogQuery,
) -> Result<Vec<AuditLog>> {
    let (per_page, offset) = windmill_common::utils::paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("audit")
        .field("*")
        .order_by("id", true)
        .and_where_eq("workspace_id", "?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if let Some(u) = &lq.username {
        sqlb.and_where_eq("username", "?".bind(u));
    }
    if let Some(o) = &lq.operation {
        sqlb.and_where_eq("operation", "?".bind(o));
    }
    if let Some(ak) = &lq.action_kind {
        sqlb.and_where_eq("action_kind", "?".bind(ak));
    }
    if let Some(r) = &lq.resource {
        sqlb.and_where_eq("resource", "?".bind(r));
    }
    if let Some(b) = &lq.before {
        sqlb.and_where_le("timestamp", format!("to_timestamp({})", b.timestamp()));
    }
    if let Some(a) = &lq.after {
        sqlb.and_where_gt("timestamp", format!("to_timestamp({})", a.timestamp()));
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, AuditLog>(&sql)
        .fetch_all(&mut tx)
        .await?;
    Ok(rows)
}

pub async fn get_audit(mut tx: Transaction<'_, sqlx::Postgres>, id: i32) -> Result<AuditLog> {
    let audit_o = sqlx::query_as::<_, AuditLog>("SELECT * FROM audit WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut tx)
        .await?;
    tx.commit().await?;
    let audit = windmill_common::utils::not_found_if_none(audit_o, "AuditLog", &id.to_string())?;
    Ok(audit)
}
