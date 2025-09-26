use axum::{
    extract::{Path, Query},
    routing::{delete, get},
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
        .route("/delete/:conversation_id", delete(delete_conversation))
        .route("/:conversation_id/messages", get(list_messages))
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

#[derive(Serialize, FromRow, Debug)]
pub struct FlowConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub message_type: String,
    pub content: String,
    pub job_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ListConversationsQuery {
    pub flow_path: Option<String>,
}

async fn list_conversations(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(query): Query<ListConversationsQuery>,
) -> JsonResult<Vec<FlowConversation>> {
    let (per_page, offset) = paginate(pagination);
    let mut tx = user_db.clone().begin(&authed).await?;

    let conversations = if let Some(flow_path) = &query.flow_path {
        sqlx::query_as::<Postgres, FlowConversation>(
            "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
             FROM flow_conversation
             WHERE workspace_id = $1 AND created_by = $2 AND flow_path = $3
             ORDER BY updated_at DESC
             LIMIT $4 OFFSET $5",
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
             LIMIT $3 OFFSET $4",
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

pub async fn create_conversation(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    flow_path: &str,
    username: &str,
    title: &str,
) -> Result<FlowConversation> {
    let mut tx = user_db.clone().begin(authed).await?;
    let conversation_id = Uuid::new_v4();
    let conversation = sqlx::query_as::<Postgres, FlowConversation>(
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by, title)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, workspace_id, flow_path, title, created_at, updated_at, created_by",
    )
    .bind(conversation_id)
    .bind(w_id)
    .bind(flow_path)
    .bind(username)
    .bind(title)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(conversation)
}

async fn delete_conversation(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
) -> Result<String> {
    let mut tx = user_db.clone().begin(&authed).await?;

    // Verify the conversation exists and belongs to the user
    let conversation = sqlx::query_as::<Postgres, FlowConversation>(
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
         FROM flow_conversation
         WHERE id = $1 AND workspace_id = $2 AND created_by = $3",
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

async fn list_messages(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<FlowConversationMessage>> {
    let (per_page, offset) = paginate(pagination);
    let mut tx = user_db.clone().begin(&authed).await?;

    // Verify the conversation exists and belongs to the user
    let conversation_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow_conversation WHERE id = $1 AND workspace_id = $2 AND created_by = $3)",
        conversation_id,
        &w_id,
        &authed.username
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if !conversation_exists {
        return Err(windmill_common::error::Error::NotFound(format!(
            "Conversation not found or access denied: {}",
            conversation_id
        )));
    }

    // Fetch messages for this conversation
    let messages = sqlx::query_as::<Postgres, FlowConversationMessage>(
        "SELECT id, conversation_id, message_type, content, job_id, created_at
         FROM flow_conversation_message
         WHERE conversation_id = $1
         ORDER BY created_at ASC
         LIMIT $2 OFFSET $3",
    )
    .bind(conversation_id)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(messages))
}

// Helper function to create a message using an existing transaction
pub async fn create_message(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    conversation_id: Uuid,
    message_type: &str,
    content: &str,
    job_id: Option<Uuid>,
    username: &str,
    workspace_id: &str,
) -> windmill_common::error::Result<()> {
    // Verify the conversation exists and belongs to the user
    let conversation_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow_conversation WHERE id = $1 AND workspace_id = $2 AND created_by = $3)",
        conversation_id,
        workspace_id,
        username
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);

    if !conversation_exists {
        return Err(windmill_common::error::Error::NotFound(format!(
            "Conversation not found or access denied: {}",
            conversation_id
        )));
    }

    // Insert the message
    sqlx::query!(
        "INSERT INTO flow_conversation_message (conversation_id, message_type, content, job_id)
         VALUES ($1, $2, $3, $4)",
        conversation_id,
        message_type,
        content,
        job_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
