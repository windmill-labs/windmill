use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow};
use uuid::Uuid;

use crate::db::DB;
use crate::error::Result;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "MESSAGE_TYPE", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    User,
    Assistant,
    System,
    Tool,
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

pub async fn get_or_create_conversation_with_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    flow_path: &str,
    username: &str,
    title: &str,
    conversation_id: Uuid,
) -> Result<FlowConversation> {
    // Check if conversation already exists
    let existing_conversation = sqlx::query_as!(
        FlowConversation,
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
         FROM flow_conversation
         WHERE id = $1 AND workspace_id = $2",
        conversation_id,
        w_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(existing) = existing_conversation {
        return Ok(existing);
    }

    // Truncate title to 25 char characters max
    let title = if title.len() > 25 {
        format!("{}...", &title[..25])
    } else {
        title.to_string()
    };

    // Create new conversation with provided ID
    let conversation = sqlx::query_as!(
        FlowConversation,
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by, title)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, workspace_id, flow_path, title, created_at, updated_at, created_by",
        conversation_id,
        w_id,
        flow_path,
        username,
        title
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(conversation)
}

/// Add a message to a conversation using an existing transaction
/// If the conversation doesn't exist, logs a warning and returns Ok (no error thrown)
/// This allows memory_id to be used for agent memory without requiring a conversation
pub async fn add_message_to_conversation_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    conversation_id: Uuid,
    job_id: Option<Uuid>,
    content: &str,
    message_type: MessageType,
    step_name: Option<&str>,
    success: bool,
) -> Result<()> {
    // Check if conversation exists first
    let conversation_exists = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM flow_conversation WHERE id = $1) as \"exists!\"",
        conversation_id
    )
    .fetch_one(&mut **tx)
    .await?
    .exists;

    if !conversation_exists {
        tracing::warn!(
            "Conversation {} does not exist. Skipping message insertion. This is expected when flows are called from apps (memory_id is used for agent memory only).",
            conversation_id
        );
        return Ok(());
    }

    // Insert the message
    sqlx::query!(
        "INSERT INTO flow_conversation_message (conversation_id, message_type, content, job_id, step_name, success)
         VALUES ($1, $2, $3, $4, $5, $6)",
        conversation_id,
        message_type as MessageType,
        content,
        job_id,
        step_name,
        success
    )
    .execute(&mut **tx)
    .await?;

    // Update conversation updated_at timestamp
    sqlx::query!(
        "UPDATE flow_conversation SET updated_at = NOW() WHERE id = $1",
        conversation_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Delete all memory for a conversation from the database
pub async fn delete_conversation_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM ai_agent_memory WHERE workspace_id = $1 AND conversation_id = $2",
        workspace_id,
        conversation_id
    )
    .execute(db)
    .await?;

    Ok(())
}
