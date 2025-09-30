#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::memory_ee::*;

#[cfg(not(feature = "private"))]
use {
    std::path::PathBuf,
    tokio::{fs, io::AsyncWriteExt},
    uuid::Uuid,
    windmill_common::worker::TMP_LOGS_DIR,
    windmill_common::DB,
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

/// Read existing messages from memory for a specific AI agent step
/// Returns None if the memory file doesn't exist
#[cfg(not(feature = "private"))]
pub(crate) async fn read_messages_from_storage(
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

/// Internal function to append messages to disk storage only
#[cfg(not(feature = "private"))]
async fn append_messages_to_disk(
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
    let mut merged = match read_messages_from_storage(workspace_id, conversation_id, step_id).await? {
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
        "Persisted {} messages to disk memory for workspace={}, conversation={}, step={}",
        new_msgs.len(),
        workspace_id,
        conversation_id,
        step_id
    );

    Ok(())
}

/// OSS stub for S3 memory storage - logs that S3 would be used but isn't available in OSS
/// Falls back to disk storage
#[cfg(not(feature = "private"))]
pub(crate) async fn s3_memory_storage(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    _db: &DB,
    messages: &serde_json::Value,
) -> anyhow::Result<()> {
    tracing::info!(
        "Memory storage for conversation {} step {} would use S3 but implementation is not in OSS, falling back to disk",
        conversation_id,
        step_id
    );
    // Fall back to disk storage in OSS
    append_messages_to_disk(workspace_id, conversation_id, step_id, messages).await
}

/// OSS implementation uses disk storage directly
#[cfg(not(feature = "private"))]
pub(crate) async fn default_disk_memory_storage(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    _db: &DB,
    messages: &serde_json::Value,
) -> anyhow::Result<()> {
    // For OSS, use the basic disk storage
    append_messages_to_disk(workspace_id, conversation_id, step_id, messages).await
}