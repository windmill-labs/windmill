#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::memory_ee::*;

#[cfg(not(feature = "private"))]
use {
    crate::ai::types::OpenAIMessage,
    std::path::PathBuf,
    tokio::{fs, io::AsyncWriteExt},
    uuid::Uuid,
    windmill_common::worker::TMP_LOGS_DIR,
};

/// Get the file path for storing memory for a specific AI agent step
#[cfg(not(feature = "private"))]
fn path_for(workspace_id: &str, conversation_id: Uuid, step_id: &str) -> PathBuf {
    PathBuf::from(TMP_LOGS_DIR)
        .join("memory")
        .join(workspace_id)
        .join(conversation_id.to_string())
        .join(format!("{step_id}.json"))
}

/// Read AI agent memory from storage
/// In OSS: always reads from disk
#[cfg(not(feature = "private"))]
pub async fn read_from_memory(
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

/// Write AI agent memory to storage
/// In OSS: always writes to disk
#[cfg(not(feature = "private"))]
pub async fn write_to_memory(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    messages: &[OpenAIMessage],
) -> anyhow::Result<()> {
    if messages.is_empty() {
        return Ok(());
    }

    let path = path_for(workspace_id, conversation_id, step_id);

    // Ensure parent directories exist
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).await?;
    }

    // Write atomically using a temporary file
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp).await?;
    f.write_all(serde_json::to_string_pretty(messages)?.as_bytes())
        .await?;
    f.flush().await?;
    drop(f);

    // Atomic rename
    fs::rename(tmp, &path).await?;
    Ok(())
}
