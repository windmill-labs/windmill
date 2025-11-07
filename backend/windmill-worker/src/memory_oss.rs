#[cfg(all(feature = "private", feature = "enterprise"))]
#[allow(unused)]
pub use crate::memory_ee::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
use {crate::ai::types::OpenAIMessage, crate::memory_common, uuid::Uuid, windmill_common::db::DB};

/// Read AI agent memory from storage
/// In OSS: always reads from database
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn read_from_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> anyhow::Result<Option<Vec<OpenAIMessage>>> {
    memory_common::read_from_db(db, workspace_id, conversation_id, step_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database read failed: {e:?}"))
}

/// Write AI agent memory to storage
/// In OSS: always writes to database
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn write_to_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    messages: &[OpenAIMessage],
) -> anyhow::Result<()> {
    if messages.is_empty() {
        return Ok(());
    }

    memory_common::write_to_db(db, workspace_id, conversation_id, step_id, messages)
        .await
        .map_err(|e| anyhow::anyhow!("Database write failed: {e:?}"))
}

/// Delete all memory for a conversation from storage
/// In OSS: always deletes from database
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn delete_conversation_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
) -> anyhow::Result<()> {
    memory_common::delete_conversation_from_db(db, workspace_id, conversation_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database delete failed: {e:?}"))
}
