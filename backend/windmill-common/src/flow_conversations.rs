use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{self, FromRow};
use uuid::Uuid;
use windmill_types::s3::S3Object;

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
}

pub async fn get_or_create_conversation_with_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    flow_path: &str,
    username: &str,
    title: &str,
    conversation_id: Uuid,
) -> Result<FlowConversation> {
    if let Some(existing) = lock_conversation(tx, w_id, conversation_id).await? {
        return Ok(existing);
    }

    // Truncate title to 25 characters max
    let title = truncate_with_ellipsis(title, 25);

    // Every turn released by the same collector's commit finds no row: the first insert
    // wins, the others wait on it, do nothing, and read the row it created.
    let created = sqlx::query_as!(
        FlowConversation,
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by, title)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (id) DO NOTHING
         RETURNING id, workspace_id, flow_path, title, created_at, updated_at, created_by",
        conversation_id,
        w_id,
        flow_path,
        username,
        title
    )
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(conversation) = created {
        return Ok(conversation);
    }

    lock_conversation(tx, w_id, conversation_id)
        .await?
        .ok_or_else(|| {
            crate::error::Error::BadRequest(format!(
                "conversation {conversation_id} belongs to another workspace"
            ))
        })
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
        "SELECT id, workspace_id, flow_path, title, created_at, updated_at, created_by
         FROM flow_conversation
         WHERE id = $1 AND workspace_id = $2
         FOR UPDATE",
        conversation_id,
        w_id
    )
    .fetch_optional(&mut **tx)
    .await?)
}

/// What a row carries beyond its text. A chat is rebuilt from its rows alone, without
/// reading jobs, so every tool row carries the model's call and what the model got back: a
/// Windmill tool's job holds the args its input transforms produced rather than the model's,
/// and an MCP tool's call sits among every call of the turn in the agent's job. That job's
/// `reasoning` is one string for the whole turn, where the rows keep it per iteration.
#[derive(Debug, Clone, Default)]
pub struct MessageExtras {
    pub tool_arguments: Option<String>,
    pub tool_result: Option<String>,
    pub reasoning: Option<String>,
    /// The files a user message carried; see `message_attachments`.
    pub attachments: Vec<MessageAttachment>,
}

/// The most files a user message keeps references to; the rest are dropped.
pub const MAX_MESSAGE_ATTACHMENTS: usize = 20;

/// A file a user message carried, as the object-storage reference its run received.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageAttachment {
    /// The flow input that held it.
    pub input: String,
    pub s3: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// The files a run's args carry for its user message: every top-level input other than
/// `user_message` whose value is an object-storage reference or a list of them, in input
/// name order, capped at `MAX_MESSAGE_ATTACHMENTS`. Only the reference is kept: `presigned`
/// grants access to the file, and any other value may be the file's bytes.
pub fn message_attachments(args: &HashMap<String, Box<RawValue>>) -> Vec<MessageAttachment> {
    let mut inputs: Vec<_> = args
        .iter()
        .filter(|(name, _)| name.as_str() != "user_message")
        .collect();
    inputs.sort_by(|a, b| a.0.cmp(b.0));
    inputs
        .into_iter()
        .flat_map(|(name, value)| {
            serde_json::from_str::<S3Object>(value.get())
                .map(|object| vec![object])
                .or_else(|_| serde_json::from_str::<Vec<S3Object>>(value.get()))
                .unwrap_or_default()
                .into_iter()
                .filter(|object| !object.s3.is_empty())
                .map(move |object| MessageAttachment {
                    input: name.clone(),
                    s3: object.s3,
                    storage: object.storage,
                    filename: object.filename,
                })
        })
        .take(MAX_MESSAGE_ATTACHMENTS)
        .collect()
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
    extras: Option<&MessageExtras>,
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
        "INSERT INTO flow_conversation_message (conversation_id, message_type, content, job_id, step_name, success, tool_arguments, tool_result, reasoning, attachments)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        conversation_id,
        message_type as MessageType,
        content,
        job_id,
        step_name,
        success,
        extras.and_then(|e| e.tool_arguments.as_deref()),
        extras.and_then(|e| e.tool_result.as_deref()),
        extras.and_then(|e| e.reasoning.as_deref()),
        extras
            .map(|e| &e.attachments)
            .filter(|attachments| !attachments.is_empty())
            .map(sqlx::types::Json) as Option<sqlx::types::Json<&Vec<MessageAttachment>>>
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, value::to_raw_value};

    fn args(values: serde_json::Value) -> HashMap<String, Box<RawValue>> {
        values
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| (name.clone(), to_raw_value(value).unwrap()))
            .collect()
    }

    #[test]
    fn keeps_only_object_storage_references() {
        let attachments = message_attachments(&args(json!({
            "user_message": { "s3": "not/an/attachment.png" },
            "avatar": { "s3": "u/a.png", "storage": "secondary", "presigned": "https://signed" },
            "files": [
                { "s3": "u/b.pdf", "filename": "b.pdf" },
                { "s3": "" }
            ],
            "photo": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
            "count": 3
        })));
        assert_eq!(
            serde_json::to_value(&attachments).unwrap(),
            json!([
                { "input": "avatar", "s3": "u/a.png", "storage": "secondary" },
                { "input": "files", "s3": "u/b.pdf", "filename": "b.pdf" }
            ])
        );
    }

    #[test]
    fn caps_the_references_of_one_message() {
        let files: Vec<_> = (0..25)
            .map(|i| json!({ "s3": format!("u/{i}.png") }))
            .collect();
        let attachments = message_attachments(&args(json!({ "files": files })));
        assert_eq!(attachments.len(), MAX_MESSAGE_ATTACHMENTS);
        assert_eq!(attachments.last().unwrap().s3, "u/19.png");
    }
}
