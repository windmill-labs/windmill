use std::path::PathBuf;
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;
use windmill_common::worker::TMP_LOGS_DIR;

/// Get the file path for storing memory for a specific AI agent step
fn path_for(workspace_id: &str, conversation_id: Uuid, step_id: &str) -> PathBuf {
    PathBuf::from(TMP_LOGS_DIR)
        .join("memory")
        .join(workspace_id)
        .join(conversation_id.to_string())
        .join(format!("{step_id}.json"))
}

/// Read existing messages from memory for a specific AI agent step
/// Returns None if the memory file doesn't exist
pub async fn read_messages(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
    let path = path_for(workspace_id, conversation_id, step_id);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).await?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)?;
    Ok(Some(v))
}

/// Append new messages to the memory for a specific AI agent step
/// Creates the file and directory structure if they don't exist
/// new_messages should be a JSON array of message objects
pub async fn append_messages(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    new_messages: &serde_json::Value,
) -> anyhow::Result<()> {
    // Expect new_messages to be an array
    let mut new_msgs = match new_messages {
        serde_json::Value::Array(v) => v.clone(),
        _ => {
            tracing::warn!(
                "Expected messages array for memory persistence, got: {:?}",
                new_messages
            );
            return Ok(()); // nothing to write
        }
    };

    if new_msgs.is_empty() {
        return Ok(());
    }

    let path = path_for(workspace_id, conversation_id, step_id);

    // Ensure parent directories exist
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).await?;
    }

    // Read existing messages and merge with new ones
    let mut merged = match read_messages(workspace_id, conversation_id, step_id).await? {
        Some(serde_json::Value::Array(existing)) => existing,
        _ => vec![],
    };
    merged.append(&mut new_msgs);

    // Write atomically using a temporary file
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp).await?;
    f.write_all(serde_json::to_string_pretty(&serde_json::Value::Array(merged))?.as_bytes())
        .await?;
    f.flush().await?;
    drop(f);

    // Atomic rename
    fs::rename(tmp, &path).await?;

    tracing::info!(
        "Persisted {} messages to memory for workspace={}, conversation={}, step={}",
        new_msgs.len(),
        workspace_id,
        conversation_id,
        step_id
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_persistence() {
        // Use a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let original = env::var("TMP_LOGS_DIR").unwrap_or_default();
        env::set_var("TMP_LOGS_DIR", temp_dir.path());

        let workspace_id = "test_workspace";
        let conversation_id = Uuid::new_v4();
        let step_id = "test_step";

        // Test reading non-existent memory
        let result = read_messages(workspace_id, conversation_id, step_id)
            .await
            .unwrap();
        assert!(result.is_none());

        // Test appending first messages
        let messages1 = json!([
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there"}
        ]);
        append_messages(workspace_id, conversation_id, step_id, &messages1)
            .await
            .unwrap();

        // Test reading back the messages
        let result = read_messages(workspace_id, conversation_id, step_id)
            .await
            .unwrap();
        assert!(result.is_some());
        let stored = result.unwrap();
        assert_eq!(stored, messages1);

        // Test appending more messages
        let messages2 = json!([
            {"role": "user", "content": "How are you?"},
            {"role": "assistant", "content": "I'm doing well"}
        ]);
        append_messages(workspace_id, conversation_id, step_id, &messages2)
            .await
            .unwrap();

        // Test that messages are merged
        let result = read_messages(workspace_id, conversation_id, step_id)
            .await
            .unwrap();
        assert!(result.is_some());
        let stored = result.unwrap();
        let expected = json!([
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there"},
            {"role": "user", "content": "How are you?"},
            {"role": "assistant", "content": "I'm doing well"}
        ]);
        assert_eq!(stored, expected);

        // Restore original environment
        if original.is_empty() {
            env::remove_var("TMP_LOGS_DIR");
        } else {
            env::set_var("TMP_LOGS_DIR", original);
        }
    }
}
