use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
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
    utils::{not_found_if_none, paginate, truncate_with_ellipsis, Pagination},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_conversations))
        .route("/delete/{conversation_id}", delete(delete_conversation))
        .route("/update/{conversation_id}", post(update_conversation))
        .route("/{conversation_id}/messages", get(list_messages))
}

#[derive(Serialize, FromRow, Debug)]
pub struct FlowConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub message_type: MessageType,
    pub content: String,
    pub job_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub created_seq: i64,
    pub step_name: Option<String>,
    pub success: bool,
    /// The call behind a tool row whose tool has no job of its own — an MCP tool, or a
    /// provider-native one. Read back from the job otherwise, and null here.
    pub tool_arguments: Option<String>,
    pub tool_result: Option<String>,
    /// The thinking that produced an answer, streamed by the provider and stored here
    /// because nothing else keeps it.
    pub reasoning: Option<String>,
}

/// Which conversations a listing holds. A test chat was started from the editor's test
/// panel; a deployed one from the flow itself.
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ConversationKind {
    Test,
    /// The default: a deployed flow's chat should not surface someone's trial runs.
    #[default]
    Deployed,
    All,
}

#[derive(Deserialize)]
pub struct ListConversationsQuery {
    pub flow_path: Option<String>,
    pub kind: Option<ConversationKind>,
}

#[derive(Deserialize)]
pub struct ListMessagesQuery {
    pub after_seq: Option<i64>,
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
        "is_test",
    ])
    .and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(flow_path) = &query.flow_path {
        sqlb.and_where_eq("flow_path", "?".bind(flow_path));
    }

    match query.kind.unwrap_or_default() {
        ConversationKind::Test => {
            sqlb.and_where_eq("is_test", "true");
        }
        ConversationKind::Deployed => {
            sqlb.and_where_eq("is_test", "false");
        }
        ConversationKind::All => {}
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
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by, is_test
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

#[derive(Deserialize)]
pub struct UpdateConversation {
    /// The chat's name. Set from the first message when the chat is created, and left
    /// alone afterwards, so a typed one stays typed.
    pub title: String,
}

async fn update_conversation(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
    Json(update): Json<UpdateConversation>,
) -> Result<String> {
    // The column is VARCHAR(255) and the helper appends an ellipsis to what it cuts, so the
    // bound it takes is three short of the column's. A longer title would otherwise reach
    // Postgres as a 22001 and come back a 500.
    let title = truncate_with_ellipsis(update.title.trim(), 252);

    let mut tx = user_db.clone().begin(&authed).await?;

    let updated = sqlx::query_scalar!(
        "UPDATE flow_conversation SET title = $1, updated_at = updated_at
         WHERE id = $2 AND workspace_id = $3
         RETURNING id",
        title,
        conversation_id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    not_found_if_none(updated, "Conversation", conversation_id.to_string())?;

    tx.commit().await?;

    Ok(format!("Conversation {} updated", conversation_id))
}

async fn list_messages(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
    Query(pagination): Query<Pagination>,
    Query(query): Query<ListMessagesQuery>,
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

    let messages = if let Some(after_seq) = query.after_seq {
        sqlx::query_as!(
            FlowConversationMessage,
            r#"SELECT id, conversation_id, message_type as "message_type: MessageType", content, job_id, created_at, created_seq, step_name, success, tool_arguments, tool_result, reasoning
             FROM flow_conversation_message
             WHERE conversation_id = $1
               AND created_seq > $2
             ORDER BY created_seq ASC
             LIMIT $3
             "#,
            conversation_id,
            after_seq,
            per_page as i64
        )
        .fetch_all(&mut *tx)
        .await?
    } else {
        // Fetch messages for this conversation, oldest first, but reverse the order of the messages for easy rendering on the frontend
        sqlx::query_as!(
            FlowConversationMessage,
            r#"SELECT id, conversation_id, message_type as "message_type: MessageType", content, job_id, created_at, created_seq, step_name, success, tool_arguments, tool_result, reasoning
             FROM (
                SELECT id, conversation_id, message_type, content, job_id, created_at, created_seq, step_name, success, tool_arguments, tool_result, reasoning
                FROM flow_conversation_message
                WHERE conversation_id = $1
                ORDER BY created_seq DESC
                LIMIT $2 OFFSET $3
             ) AS messages
             ORDER BY created_seq ASC
             "#,
            conversation_id,
            per_page as i64,
            offset as i64
        )
        .fetch_all(&mut *tx)
        .await?
    };

    tx.commit().await?;
    Ok(Json(messages))
}
