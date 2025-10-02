use crate::ai::types::OpenAIMessage;
use std::path::PathBuf;
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;
use windmill_common::worker::TMP_LOGS_DIR;

/// Get the file path for storing memory for a specific AI agent step
pub fn path_for(workspace_id: &str, conversation_id: Uuid, step_id: &str) -> PathBuf {
    PathBuf::from(TMP_LOGS_DIR)
        .join("memory")
        .join(workspace_id)
        .join(conversation_id.to_string())
        .join(format!("{step_id}.json"))
}

/// Read messages from disk storage
pub async fn read_from_disk(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> anyhow::Result<Option<Vec<OpenAIMessage>>> {
    let path = path_for(workspace_id, conversation_id, step_id);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).await?;
    let messages: Vec<OpenAIMessage> = serde_json::from_slice(&bytes)?;
    Ok(Some(messages))
}

/// Write messages to disk storage
pub async fn write_to_disk(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    messages: &[OpenAIMessage],
) -> anyhow::Result<()> {
    let path = path_for(workspace_id, conversation_id, step_id);

    // Ensure parent directories exist
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).await?;
    }

    // Write atomically using a temporary file
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp).await?;
    f.write_all(&serde_json::to_vec(messages)?).await?;
    f.flush().await?;
    drop(f);

    // Atomic rename
    fs::rename(tmp, &path).await?;

    Ok(())
}

/// Delete all memory for a conversation from disk storage
pub async fn delete_conversation_from_disk(
    workspace_id: &str,
    conversation_id: Uuid,
) -> anyhow::Result<()> {
    let conversation_path = PathBuf::from(TMP_LOGS_DIR)
        .join("memory")
        .join(workspace_id)
        .join(conversation_id.to_string());

    if conversation_path.exists() {
        fs::remove_dir_all(&conversation_path).await?;
    }

    Ok(())
}
