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
pub async fn add_message_to_conversation_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    conversation_id: Uuid,
    job_id: Option<Uuid>,
    content: &str,
    message_type: MessageType,
    step_name: Option<&str>,
    success: bool,
) -> Result<()> {
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
