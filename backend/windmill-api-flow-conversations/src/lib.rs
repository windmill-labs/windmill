use axum::{
    extract::{Path, Query},
    routing::{delete, get},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sql_builder::prelude::*;
use sqlx::{FromRow, Postgres};
use uuid::Uuid;

use windmill_api_auth::ApiAuthed;
pub use windmill_common::flow_conversations::FlowConversation;
use windmill_common::{
    db::{UserDB, DB},
    error::{JsonResult, Result},
    flow_conversations::MessageType,
    utils::{not_found_if_none, paginate, Pagination},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_conversations))
        .route("/delete/:conversation_id", delete(delete_conversation))
        .route("/:conversation_id/messages", get(list_messages))
}

#[derive(Serialize, FromRow, Debug)]
pub struct FlowConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub message_type: MessageType,
    pub content: String,
    pub job_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub step_name: Option<String>,
    pub success: bool,
}

#[derive(Deserialize)]
pub struct ListConversationsQuery {
    pub flow_path: Option<String>,
    pub after_id: Option<Uuid>,
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

    let mut sqlb = SqlBuilder::select_from("flow_conversation");
    sqlb.fields(&[
        "id",
        "workspace_id",
        "flow_path",
        "title",
        "created_at",
        "updated_at",
        "created_by",
    ])
    .and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(flow_path) = &query.flow_path {
        sqlb.and_where_eq("flow_path", "?".bind(flow_path));
    }
    if let Some(after_id) = &query.after_id {
        let message_id_created_at = sqlx::query_scalar!(
            "SELECT created_at FROM flow_conversation_message WHERE id = $1",
            after_id
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlb.and_where_gt("created_at", "?".bind(&message_id_created_at.to_rfc3339()));
    }

    sqlb.order_by("updated_at", true)
        .limit(per_page as i64)
        .offset(offset as i64);

    let sql = sqlb.sql().map_err(|e| {
        windmill_common::error::Error::internal_err(format!("Failed to build SQL: {}", e))
    })?;

    let conversations = sqlx::query_as::<Postgres, FlowConversation>(&sql)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(conversations))
}

async fn delete_conversation(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
) -> Result<String> {
    let mut tx = user_db.clone().begin(&authed).await?;

    // Verify the conversation exists and belongs to the user
    let conversation = sqlx::query_as!(
        FlowConversation,
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
         FROM flow_conversation
         WHERE id = $1 AND workspace_id = $2",
        conversation_id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    not_found_if_none(conversation, "Conversation", conversation_id.to_string())?;

    // Delete the conversation (messages will be cascade deleted)
    sqlx::query!(
        "DELETE FROM flow_conversation WHERE id = $1 AND workspace_id = $2",
        conversation_id,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Delete associated memory in background (non-blocking cleanup)
    {
        let w_id_clone = w_id.clone();
        let db_clone = db.clone();
        tokio::spawn(async move {
            if let Err(e) = windmill_common::flow_conversations::delete_conversation_memory(
                &db_clone,
                &w_id_clone,
                conversation_id,
            )
            .await
            {
                tracing::error!(
                    "Failed to delete memory for conversation {} in workspace {}: {:?}",
                    conversation_id,
                    w_id_clone,
                    e
                );
            }
        });
    }

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
        "SELECT EXISTS(SELECT 1 FROM flow_conversation WHERE id = $1 AND workspace_id = $2)",
        conversation_id,
        &w_id
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

    // Fetch messages for this conversation, oldest first, but reverse the order of the messages for easy rendering on the frontend
    let messages = sqlx::query_as!(
        FlowConversationMessage,
        r#"SELECT id, conversation_id, message_type as "message_type: MessageType", content, job_id, created_at, step_name, success
         FROM (
            SELECT id, conversation_id, message_type, content, job_id, created_at, step_name, success
            FROM flow_conversation_message
            WHERE conversation_id = $1
            ORDER BY created_at DESC, CASE WHEN message_type = 'user' THEN 0 ELSE 1 END
            LIMIT $2 OFFSET $3
         ) AS messages
         ORDER BY created_at ASC, CASE WHEN message_type = 'user' THEN 0 ELSE 1 END
         "#,
        conversation_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(messages))
}
