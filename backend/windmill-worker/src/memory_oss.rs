#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::memory_ee::*;

#[cfg(not(feature = "private"))]
use {crate::ai::types::OpenAIMessage, crate::memory_common, uuid::Uuid};

/// Read AI agent memory from storage
/// In OSS: always reads from disk
#[cfg(not(feature = "private"))]
pub async fn read_from_memory(
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> anyhow::Result<Option<Vec<OpenAIMessage>>> {
    memory_common::read_from_disk(workspace_id, conversation_id, step_id).await
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

    memory_common::write_to_disk(workspace_id, conversation_id, step_id, messages).await
}
