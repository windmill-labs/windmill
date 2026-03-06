use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::Value;
use uuid::Uuid;

use windmill_common::error::{self, Error};
use windmill_common::DB;

/// Checkpoint state persisted across workflow invocations.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WacCheckpoint {
    #[serde(default)]
    pub source_hash: String,
    #[serde(default)]
    pub completed_steps: serde_json::Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_steps: Option<WacPendingSteps>,
    #[serde(default)]
    pub input_args: serde_json::Map<String, Value>,
    /// When set on a child job's checkpoint, indicates which step this child
    /// should execute directly (instead of dispatching).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _executing_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WacPendingSteps {
    pub mode: String,
    pub keys: Vec<String>,
    pub job_ids: serde_json::Map<String, Value>,
}

/// Output from a single WAC invocation (parsed from result.json).
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WacOutput {
    #[serde(rename = "dispatch")]
    Dispatch { mode: String, steps: Vec<WacStepDispatch> },
    #[serde(rename = "complete")]
    Complete { result: Value },
    /// An inline step executed in the parent process — persist result to
    /// checkpoint and re-run immediately (no child job, no suspend).
    #[serde(rename = "inline_checkpoint")]
    InlineCheckpoint { key: String, result: Value },
}

#[derive(Debug, Deserialize, Clone)]
pub struct WacStepDispatch {
    pub name: String,
    pub script: String,
    pub args: serde_json::Map<String, Value>,
    pub key: String,
}

/// Load the WAC checkpoint from `v2_job_status.workflow_as_code_status._checkpoint`.
pub async fn load_checkpoint(db: &DB, job_id: &Uuid) -> error::Result<WacCheckpoint> {
    let row: Option<Option<Value>> = sqlx::query_scalar(
        "SELECT workflow_as_code_status->'_checkpoint' FROM v2_job_status WHERE id = $1",
    )
    .bind(job_id)
    .fetch_optional(db)
    .await?;

    match row {
        Some(Some(status)) => {
            let checkpoint: WacCheckpoint = serde_json::from_value(status).unwrap_or_default();
            Ok(checkpoint)
        }
        _ => Ok(WacCheckpoint::default()),
    }
}

/// Save the WAC checkpoint to `v2_job_status.workflow_as_code_status._checkpoint`.
/// The top level of workflow_as_code_status is reserved for per-child-job timeline data.
pub async fn save_checkpoint(
    db: &DB,
    job_id: &Uuid,
    checkpoint: &WacCheckpoint,
) -> error::Result<()> {
    let status_json = serde_json::to_value(checkpoint)
        .map_err(|e| Error::InternalErr(format!("Failed to serialize checkpoint: {e}")))?;

    sqlx::query(
        "INSERT INTO v2_job_status (id, workflow_as_code_status)
         VALUES ($1, jsonb_build_object('_checkpoint', $2::jsonb))
         ON CONFLICT (id) DO UPDATE SET
            workflow_as_code_status = jsonb_set(
                COALESCE(v2_job_status.workflow_as_code_status, '{}'::jsonb),
                '{_checkpoint}',
                $2::jsonb
            )",
    )
    .bind(job_id)
    .bind(&status_json)
    .execute(db)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to save WAC checkpoint: {e}")))?;

    Ok(())
}

/// Parse the WAC result from result.json content.
pub fn parse_wac_output(result: &RawValue) -> error::Result<WacOutput> {
    serde_json::from_str(result.get())
        .map_err(|e| Error::InternalErr(format!("Failed to parse WAC output: {e}")))
}

/// Process a "dispatch" result: update checkpoint with pending steps info.
pub fn update_checkpoint_for_dispatch(
    checkpoint: &mut WacCheckpoint,
    steps: &[WacStepDispatch],
    mode: &str,
    job_ids: &[(String, Uuid)],
) {
    let pending = WacPendingSteps {
        mode: mode.to_string(),
        keys: steps.iter().map(|s| s.key.clone()).collect(),
        job_ids: job_ids
            .iter()
            .map(|(key, id)| (key.clone(), Value::String(id.to_string())))
            .collect(),
    };
    checkpoint.pending_steps = Some(pending);
}

/// Process a completed child job result: add to checkpoint's completed_steps.
pub fn add_completed_step(checkpoint: &mut WacCheckpoint, step_key: &str, result: Value) {
    checkpoint
        .completed_steps
        .insert(step_key.to_string(), result);
    // If all pending steps are complete, clear pending
    if let Some(ref pending) = checkpoint.pending_steps {
        let all_done = pending
            .keys
            .iter()
            .all(|k| checkpoint.completed_steps.contains_key(k));
        if all_done {
            checkpoint.pending_steps = None;
        }
    }
}

/// Check if all pending parallel steps are complete.
pub fn all_pending_complete(checkpoint: &WacCheckpoint) -> bool {
    match &checkpoint.pending_steps {
        None => true,
        Some(pending) => pending
            .keys
            .iter()
            .all(|k| checkpoint.completed_steps.contains_key(k)),
    }
}
