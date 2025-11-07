use crate::ai::types::OpenAIMessage;
use uuid::Uuid;
use windmill_common::{db::DB, error::Error};

pub const MAX_MEMORY_SIZE_BYTES: usize = 100_000; // 100KB per memory entry in database

/// Read AI agent memory from database
pub async fn read_from_db(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> Result<Option<Vec<OpenAIMessage>>, Error> {
    let result = sqlx::query!(
        "SELECT messages FROM ai_agent_memory
         WHERE workspace_id = $1 AND conversation_id = $2 AND step_id = $3",
        workspace_id,
        conversation_id,
        step_id
    )
    .fetch_optional(db)
    .await?;

    match result {
        Some(row) => {
            let messages: Vec<OpenAIMessage> = serde_json::from_value(row.messages)?;
            Ok(Some(messages))
        }
        None => Ok(None),
    }
}

/// Write AI agent memory to database with size checking and truncation
pub async fn write_to_db(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    messages: &[OpenAIMessage],
) -> Result<(), Error> {
    if messages.is_empty() {
        return Ok(());
    }

    // Serialize messages and check size
    let mut messages_to_store = messages.to_vec();
    let mut json_value = serde_json::to_value(&messages_to_store)?;
    let size_bytes = json_value.to_string().len();

    // Truncate if necessary
    if size_bytes > MAX_MEMORY_SIZE_BYTES {
        tracing::warn!(
            "Memory size ({} bytes) exceeds limit ({} bytes) for workspace={} conversation={} step={}. Truncating messages. Use S3 storage in workspace settings to store full conversation history.",
            size_bytes,
            MAX_MEMORY_SIZE_BYTES,
            workspace_id,
            conversation_id,
            step_id
        );

        messages_to_store = truncate_messages(messages, MAX_MEMORY_SIZE_BYTES)?;
        json_value = serde_json::to_value(&messages_to_store)?;
    }

    // Insert or update using UPSERT
    sqlx::query!(
        "INSERT INTO ai_agent_memory (workspace_id, conversation_id, step_id, messages, created_at, updated_at)
         VALUES ($1, $2, $3, $4, NOW(), NOW())
         ON CONFLICT (workspace_id, conversation_id, step_id)
         DO UPDATE SET
            messages = EXCLUDED.messages,
            updated_at = NOW()",
        workspace_id,
        conversation_id,
        step_id,
        json_value
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Delete all memory for a conversation from database
pub async fn delete_conversation_from_db(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
) -> Result<(), Error> {
    sqlx::query!(
        "DELETE FROM ai_agent_memory
         WHERE workspace_id = $1 AND conversation_id = $2",
        workspace_id,
        conversation_id
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Truncate messages to fit within the size limit
fn truncate_messages(
    messages: &[OpenAIMessage],
    max_size_bytes: usize,
) -> Result<Vec<OpenAIMessage>, Error> {
    let mut result = messages.to_vec();

    // Keep removing oldest messages until we're under the threshold
    while !result.is_empty() {
        let test_json = serde_json::to_value(&result)?;
        let test_size = test_json.to_string().len();

        if test_size <= max_size_bytes {
            break;
        }

        // Remove the first (oldest) message
        result.remove(0);
    }

    Ok(result)
}
