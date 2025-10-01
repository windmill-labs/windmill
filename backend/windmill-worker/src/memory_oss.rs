#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::memory_ee::*;

#[cfg(not(feature = "private"))]
use {
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
) -> anyhow::Result<Option<serde_json::Value>> {
    let path = path_for(workspace_id, conversation_id, step_id);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).await?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)?;
    Ok(Some(v))
}

/// Write AI agent memory to storage
/// In OSS: always writes to disk
#[cfg(not(feature = "private"))]
pub async fn write_to_memory(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    messages: &serde_json::Value,
) -> anyhow::Result<()> {
    // Expect messages to be an array
    if !messages.is_array() {
        tracing::warn!(
            "Expected messages array for memory persistence, got: {:?}",
            messages
        );
        return Ok(());
    }

    if messages.as_array().map_or(true, |arr| arr.is_empty()) {
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

    tracing::info!(
        "Persisted {} messages to disk memory for workspace={}, conversation={}, step={}",
        messages.as_array().map(|a| a.len()).unwrap_or(0),
        workspace_id,
        conversation_id,
        step_id
    );

    Ok(())
}
