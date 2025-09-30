use uuid::Uuid;
use windmill_common::DB;

// Import storage implementations from memory_oss
// which will re-export EE versions when feature = "private" is enabled
use crate::memory_oss::{default_disk_memory_storage, read_messages_from_storage, s3_memory_storage};

/// Read existing messages from memory for a specific AI agent step
/// Returns None if the memory file doesn't exist
pub async fn read_messages(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
    read_messages_from_storage(workspace_id, conversation_id, step_id).await
}

/// Append new messages to the memory for a specific AI agent step
/// Checks workspace settings for large_file_storage configuration:
/// - If S3/Azure/GCS is configured, writes to cloud storage (EE only)
/// - Otherwise, falls back to disk storage
/// new_messages should be a JSON array of message objects
pub async fn append_messages(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    new_messages: &serde_json::Value,
) -> anyhow::Result<()> {
    // Expect new_messages to be an array
    if !new_messages.is_array() {
        tracing::warn!(
            "Expected messages array for memory persistence, got: {:?}",
            new_messages
        );
        return Ok(());
    }

    if new_messages.as_array().map_or(true, |arr| arr.is_empty()) {
        return Ok(());
    }

    // Check if workspace has large_file_storage configured
    let raw_lfs_opt = sqlx::query_scalar!(
        "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
        workspace_id
    )
    .fetch_optional(db)
    .await?
    .flatten();

    let has_cloud_storage = raw_lfs_opt.is_some();

    if has_cloud_storage {
        // Use S3/cloud storage (EE implementation will handle actual S3, OSS will fall back to disk)
        s3_memory_storage(workspace_id, conversation_id, step_id, db, new_messages).await
    } else {
        // Use disk storage directly
        default_disk_memory_storage(workspace_id, conversation_id, step_id, db, new_messages).await
    }
}

