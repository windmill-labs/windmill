/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{ApiAuthed, DB},
    utils::require_super_admin,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use windmill_common::{
    error::{Error, JsonResult, Result},
    mailbox::{MailboxType, MsgPayload},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_mailbox_messages))
        .route("/:message_id", delete(delete_mailbox_message))
        .route("/bulk_delete", delete(bulk_delete_mailbox_messages))
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct MailboxMessage {
    pub message_id: i64,
    pub mailbox_id: Option<String>,
    pub workspace_id: String,
    #[sqlx(rename = "type")]
    pub r#type: MailboxType,
    pub created_at: DateTime<Utc>,
    pub payload: MsgPayload,
}

#[derive(Deserialize)]
pub struct ListMailboxQuery {
    pub mailbox_type: Option<MailboxType>,
    pub mailbox_id: Option<String>,
    pub message_id: Option<i64>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Deserialize)]
pub struct BulkDeleteRequest {
    pub message_ids: Vec<i64>,
}

async fn list_mailbox_messages(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListMailboxQuery>,
) -> JsonResult<Vec<MailboxMessage>> {
    require_super_admin(&db, &authed.email).await?;

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let mut sqlb = SqlBuilder::select_from("mailbox");

    sqlb.fields(&[
        "message_id",
        "mailbox_id",
        "workspace_id",
        "type",
        "created_at",
        "payload",
    ])
    .order_by("created_at", true)
    .and_where("workspace_id = ?".bind(&w_id));

    if let Some(mailbox_type) = &query.mailbox_type {
        sqlb.and_where("type = ?".bind(mailbox_type));
    }

    if let Some(mailbox_id) = &query.mailbox_id {
        sqlb.and_where("mailbox_id = ?".bind(mailbox_id));
    }

    if let Some(message_id) = query.message_id {
        sqlb.and_where("message_id = ?".bind(&message_id));
    }

    sqlb.offset(offset).limit(per_page);

    let sql = sqlb
        .sql()
        .map_err(|e| Error::InternalErr(format!("SQL error: {}", e)))?;

    let messages = sqlx::query_as(&sql).fetch_all(&db).await?;

    Ok(Json(messages))
}

async fn delete_mailbox_message(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, message_id)): Path<(String, i64)>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    let result = sqlx::query!(
        "DELETE FROM mailbox WHERE message_id = $1 AND workspace_id = $2",
        message_id,
        &w_id
    )
    .execute(&db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(windmill_common::error::Error::NotFound(format!(
            "Mailbox message {} not found in workspace {}",
            message_id, w_id
        )));
    }

    Ok(format!("Deleted mailbox message {}", message_id))
}

async fn bulk_delete_mailbox_messages(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    if req.message_ids.is_empty() {
        return Ok("No messages to delete".to_string());
    }

    let result = sqlx::query!(
        "DELETE FROM mailbox WHERE message_id = ANY($1) AND workspace_id = $2",
        &req.message_ids,
        &w_id
    )
    .execute(&db)
    .await?;

    Ok(format!(
        "Deleted {} mailbox messages",
        result.rows_affected()
    ))
}
