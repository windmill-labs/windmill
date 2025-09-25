/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres};
use uuid::Uuid;

use crate::db::{ApiAuthed, DB};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{JsonResult, Result},
    utils::{paginate, Pagination},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_conversations))
        .route("/create", post(create_conversation))
        .route("/delete/:conversation_id", delete(delete_conversation))
}

#[derive(Serialize, FromRow, Debug)]
pub struct FlowConversation {
    pub id: Uuid,
    pub workspace_id: String,
    pub flow_path: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Deserialize)]
pub struct ListConversationsQuery {
    pub flow_path: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateConversationRequest {
    pub flow_path: String,
    pub title: Option<String>,
}

async fn list_conversations(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(query): Query<ListConversationsQuery>,
) -> JsonResult<Vec<FlowConversation>> {
    let (per_page, offset) = paginate(pagination);
    let mut tx = user_db.begin(&authed).await?;

    let conversations = if let Some(flow_path) = &query.flow_path {
        sqlx::query_as::<Postgres, FlowConversation>(
            "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
             FROM flow_conversation
             WHERE workspace_id = $1 AND created_by = $2 AND flow_path = $3
             ORDER BY updated_at DESC
             LIMIT $4 OFFSET $5"
        )
        .bind(&w_id)
        .bind(&authed.username)
        .bind(flow_path)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&mut *tx)
        .await?
    } else {
        sqlx::query_as::<Postgres, FlowConversation>(
            "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
             FROM flow_conversation
             WHERE workspace_id = $1 AND created_by = $2
             ORDER BY updated_at DESC
             LIMIT $3 OFFSET $4"
        )
        .bind(&w_id)
        .bind(&authed.username)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&mut *tx)
        .await?
    };

    tx.commit().await?;
    Ok(Json(conversations))
}

async fn create_conversation(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(req): Json<CreateConversationRequest>,
) -> JsonResult<FlowConversation> {
    let mut tx = user_db.begin(&authed).await?;

    // Verify the flow exists
    let flow_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE workspace_id = $1 AND path = $2)",
        &w_id,
        &req.flow_path
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if !flow_exists {
        return Err(windmill_common::error::Error::NotFound(format!(
            "Flow not found: {}",
            req.flow_path
        )));
    }

    let conversation_id = Uuid::new_v4();

    let conversation = sqlx::query_as::<Postgres, FlowConversation>(
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, title, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, workspace_id, flow_path, title, created_at, updated_at, created_by"
    )
    .bind(conversation_id)
    .bind(&w_id)
    .bind(&req.flow_path)
    .bind(&req.title)
    .bind(&authed.username)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    audit_log(
        &db,
        &authed,
        "flow_conversation.create",
        ActionKind::Create,
        &w_id,
        Some(&format!("{}/{}", req.flow_path, conversation_id)),
        None,
    )
    .await?;

    Ok(Json(conversation))
}

async fn delete_conversation(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    // Verify the conversation exists and belongs to the user
    let conversation = sqlx::query_as::<Postgres, FlowConversation>(
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
         FROM flow_conversation
         WHERE id = $1 AND workspace_id = $2 AND created_by = $3"
    )
    .bind(conversation_id)
    .bind(&w_id)
    .bind(&authed.username)
    .fetch_optional(&mut *tx)
    .await?;

    let conversation = conversation.ok_or_else(|| {
        windmill_common::error::Error::NotFound(format!(
            "Conversation not found or access denied: {}",
            conversation_id
        ))
    })?;

    // Delete the conversation (messages will be cascade deleted)
    sqlx::query!(
        "DELETE FROM flow_conversation WHERE id = $1 AND workspace_id = $2 AND created_by = $3",
        conversation_id,
        &w_id,
        &authed.username
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    audit_log(
        &db,
        &authed,
        "flow_conversation.delete",
        ActionKind::Delete,
        &w_id,
        Some(&format!("{}/{}", conversation.flow_path, conversation_id)),
        None,
    )
    .await?;

    Ok(format!("Conversation {} deleted", conversation_id))
}