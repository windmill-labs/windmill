use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow};
use uuid::Uuid;

use crate::db::DB;
use crate::error::Result;
use crate::utils::truncate_with_ellipsis;

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
    /// Started from the flow editor's test panel rather than a deployed run.
    pub is_test: bool,
}

/// `is_test` is written on insert. An existing conversation of the other kind refuses the
/// turn, so preview and deployed runs never share one.
pub async fn get_or_create_conversation_with_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    flow_path: &str,
    username: &str,
    title: &str,
    conversation_id: Uuid,
    is_test: bool,
) -> Result<FlowConversation> {
    if let Some(existing) = lock_conversation(tx, w_id, conversation_id).await? {
        return same_kind(existing, is_test);
    }

    // Truncate title to 25 characters max
    let title = truncate_with_ellipsis(title, 25);

    // Every turn released by the same collector's commit finds no row: the first insert
    // wins, the others wait on it, do nothing, and read the row it created.
    let created = sqlx::query_as!(
        FlowConversation,
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by, title, is_test)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO NOTHING
         RETURNING id, workspace_id, flow_path, title, created_at, updated_at, created_by, is_test",
        conversation_id,
        w_id,
        flow_path,
        username,
        title,
        is_test
    )
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(conversation) = created {
        return Ok(conversation);
    }

    // The concurrent first turn that won the insert may have been of the other kind.
    let existing = lock_conversation(tx, w_id, conversation_id)
        .await?
        .ok_or_else(|| {
            crate::error::Error::BadRequest(format!(
                "conversation {conversation_id} belongs to another workspace"
            ))
        })?;
    same_kind(existing, is_test)
}

/// `memory_id` is the caller's to choose, so a preview run could name a deployed
/// conversation and the reverse. A conversation's kind is fixed at creation and nothing
/// would show the mixing afterwards, so the turn is refused before it starts.
fn same_kind(existing: FlowConversation, is_test: bool) -> Result<FlowConversation> {
    if existing.is_test == is_test {
        return Ok(existing);
    }
    Err(crate::error::Error::BadRequest(if existing.is_test {
        "this conversation was started from the flow editor's test panel; start a new conversation to run the deployed flow".to_string()
    } else {
        "this conversation belongs to the deployed flow; start a new conversation to test from the flow editor".to_string()
    }))
}

/// Locked, so a turn orders against retention collecting the conversation
/// (windmill_common::jobs::delete_jobs): either the turn goes first and the collector then
/// sees its message, or it waits and finds the row gone and creates it again. Unlocked, the
/// message insert would wait on the parent row's lock instead and then fail its FK check.
async fn lock_conversation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    conversation_id: Uuid,
) -> Result<Option<FlowConversation>> {
    Ok(sqlx::query_as!(
        FlowConversation,
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by, is_test
         FROM flow_conversation
         WHERE id = $1 AND workspace_id = $2
         FOR UPDATE",
        conversation_id,
        w_id
    )
    .fetch_optional(&mut **tx)
    .await?)
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
