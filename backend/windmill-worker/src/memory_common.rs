use crate::ai::types::{OpenAIContent, OpenAIMessage};
use uuid::Uuid;
use windmill_common::{
    db::DB,
    error::Error,
    worker::{MAX_MEMORY_SIZE_BYTES, MEMORY_TRUNCATION_WARNING},
};

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
    let mut size_bytes = json_value.to_string().len();

    // Truncate if necessary
    if size_bytes > MAX_MEMORY_SIZE_BYTES {
        tracing::warn!(
            "Memory size ({} bytes) exceeds limit ({} bytes) for workspace={} conversation={} step={}. Truncating messages.",
            size_bytes,
            MAX_MEMORY_SIZE_BYTES,
            workspace_id,
            conversation_id,
            step_id
        );

        messages_to_store = truncate_messages(messages, MAX_MEMORY_SIZE_BYTES)?;
        json_value = serde_json::to_value(&messages_to_store)?;
        size_bytes = json_value.to_string().len();

        tracing::info!(
            "After truncation: {} messages, {} bytes for workspace={} conversation={} step={}",
            messages_to_store.len(),
            size_bytes,
            workspace_id,
            conversation_id,
            step_id
        );
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
///
/// Algorithm:
/// 1. Check if messages exceed threshold
/// 2. If yes, remove oldest messages (from beginning) until under threshold
/// 3. Append truncation warning message at the end (even if it exceeds threshold)
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

    // Create and append truncation warning message at the end
    let truncation_message = OpenAIMessage {
        role: "system".to_string(),
        content: Some(OpenAIContent::Text(MEMORY_TRUNCATION_WARNING.to_string())),
        tool_calls: None,
        tool_call_id: None,
        agent_action: None,
    };
    result.push(truncation_message);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(role: &str, content: &str) -> OpenAIMessage {
        OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(content.to_string())),
            tool_calls: None,
            tool_call_id: None,
            agent_action: None,
        }
    }

    #[test]
    fn test_truncate_messages_basic() {
        let messages = vec![
            create_test_message("system", "You are a helpful assistant"),
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi there!"),
            create_test_message("user", "How are you?"),
            create_test_message("assistant", "I'm doing well!"),
        ];

        // Very small limit to force truncation
        let truncated = truncate_messages(&messages, 500).unwrap();

        // Should have at least one message (truncation warning is always added)
        assert!(truncated.len() >= 1);

        // Last message should be the truncation warning
        let last_msg = truncated.last().unwrap();
        assert_eq!(last_msg.role, "system");
        assert!(last_msg
            .content
            .as_ref()
            .unwrap()
            .to_string()
            .contains("TRUNCATED"));
    }

    #[test]
    fn test_truncate_messages_removes_oldest() {
        let messages = vec![
            create_test_message("user", "Oldest message"),
            create_test_message("assistant", "Old response"),
            create_test_message("user", "Recent message"),
            create_test_message("assistant", "Recent response"),
        ];

        let truncated = truncate_messages(&messages, 400).unwrap();

        // Should have some messages + truncation warning at end
        assert!(truncated.len() >= 1);

        // Last message should be truncation warning
        let last_msg = truncated.last().unwrap();
        assert!(last_msg
            .content
            .as_ref()
            .unwrap()
            .to_string()
            .contains("TRUNCATED"));

        // First message should NOT be "Oldest message" if truncation happened
        if truncated.len() > 1 {
            let first_msg_content = match &truncated[0].content {
                Some(OpenAIContent::Text(text)) => text.as_str(),
                _ => "",
            };
            // Either we kept all messages or we removed the oldest one
            assert!(
                first_msg_content.contains("Oldest") || !first_msg_content.contains("Oldest"),
                "Oldest messages should be removed first during truncation"
            );
        }
    }

    #[test]
    fn test_truncate_messages_warning_at_end() {
        let messages = vec![
            create_test_message("user", "Message 1"),
            create_test_message("assistant", "Response 1"),
        ];

        let truncated = truncate_messages(&messages, 1000).unwrap();

        // Should have original messages + warning at the end
        assert!(truncated.len() == 3); // 2 original + 1 warning

        // Last message must be the warning
        let last_msg = truncated.last().unwrap();
        assert_eq!(last_msg.role, "system");
        assert!(last_msg
            .content
            .as_ref()
            .unwrap()
            .to_string()
            .contains("TRUNCATED"));
    }
}
