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

/// Changing it detaches every memory stored under a string memory id.
const MEMORY_ID_NAMESPACE: Uuid = Uuid::from_u128(0x6f1c2d4e_8a3b_5c7d_9e0f_1a2b3c4d5e6f);

/// Memory is stored and carried in `flow_status.memory_id` as a uuid, which names the same memory
/// wherever it is passed, as a chat conversation id must. Any other string names a memory through a
/// name-based (v5) uuid scoped to its workspace and flow, so the same key in two flows or two
/// workspaces names two memories, and chat conversation ids stay unique across workspaces.
pub fn memory_key(workspace_id: &str, flow_path: &str, memory_id: &str) -> Uuid {
    let memory_id = memory_id.trim();
    Uuid::parse_str(memory_id).unwrap_or_else(|_| {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(MEMORY_ID_NAMESPACE.as_bytes());
        for part in [workspace_id, flow_path, memory_id] {
            hasher.update(part.as_bytes());
            hasher.update([0u8]);
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hasher.finalize()[..16]);
        uuid::Builder::from_sha1_bytes(bytes).into_uuid()
    })
}

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
        let existing = same_kind(existing, is_test)?;
        refuse_running_turn(tx, conversation_id).await?;
        return Ok(existing);
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
    let existing = same_kind(existing, is_test)?;
    refuse_running_turn(tx, conversation_id).await?;
    Ok(existing)
}

/// The turn a conversation is still answering: its newest user message, while the flow run
/// that message started is still queued or running.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct RunningTurn {
    pub job_id: Uuid,
    /// `created_seq` of the user message that started the turn.
    pub user_seq: i64,
}

/// One running turn per conversation holds its agent memory; a second run would write the
/// same memory concurrently. Checked under the conversation's row lock, so two runs sent at
/// once cannot both pass: the second waits, then sees the first's message and queued job.
async fn refuse_running_turn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    conversation_id: Uuid,
) -> Result<()> {
    let Some(turn) = running_turns(&mut **tx, &[conversation_id])
        .await?
        .remove(&conversation_id)
    else {
        return Ok(());
    };
    // A JSON body, so a chat client can follow the running turn instead of failing.
    Err(crate::error::Error::Generic(
        axum::http::StatusCode::CONFLICT,
        serde_json::json!({
            "error": "this conversation is still answering a message; wait for it to finish or stop it before sending another",
            "running_turn": turn,
        })
        .to_string(),
    ))
}

/// The running turn of each of `conversation_ids` that has one.
///
/// It answers for whatever ids it is given and checks no permission of its own, so the
/// executor must be one the caller is entitled to read those conversations through: a
/// `user_db` transaction under RLS, or a transaction holding ids the caller has already
/// authorized. Handed a raw pool and ids from a request, it would report other users' jobs.
pub async fn running_turns<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    conversation_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, RunningTurn>> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, i64)>(
        "SELECT c.id, u.job_id, u.created_seq
         FROM unnest($1::uuid[]) AS c(id)
         CROSS JOIN LATERAL (
             SELECT job_id, created_seq
             FROM flow_conversation_message
             WHERE conversation_id = c.id AND message_type = 'user'
             ORDER BY created_seq DESC
             LIMIT 1
         ) u
         WHERE u.job_id IS NOT NULL
           AND EXISTS (SELECT 1 FROM v2_job_queue q WHERE q.id = u.job_id)",
    )
    .bind(conversation_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(conversation_id, job_id, user_seq)| {
            (conversation_id, RunningTurn { job_id, user_seq })
        })
        .collect())
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

    /// A string names a memory only within its workspace and flow; a uuid is used as is.
    #[test]
    fn memory_key_scopes_strings_but_not_uuids() {
        let key = memory_key("ws", "f/support/triage", " customer-1 ");
        assert_eq!(key, memory_key("ws", "f/support/triage", "customer-1"));
        assert_ne!(
            key,
            memory_key("other_ws", "f/support/triage", "customer-1")
        );
        assert_ne!(key, memory_key("ws", "f/sales/triage", "customer-1"));
        let conversation = Uuid::from_u128(7).to_string();
        assert_eq!(memory_key("ws", "f/a", &conversation), Uuid::from_u128(7));
        assert_eq!(
            memory_key("other_ws", "f/b", &conversation),
            Uuid::from_u128(7)
        );
    }
}
