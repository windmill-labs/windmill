use serde::{Deserialize, Serialize};
use sqlx;
use uuid::Uuid;

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
