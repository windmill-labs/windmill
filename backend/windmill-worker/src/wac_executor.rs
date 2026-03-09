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
    /// Accumulated map of step_key → child job UUID across all dispatch rounds.
    /// Unlike `pending_steps.job_ids` (cleared after completion), this persists
    /// so the frontend can always resolve step keys to child job names.
    #[serde(default)]
    pub job_ids: serde_json::Map<String, Value>,
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
    /// Suspend the workflow waiting for an external approval event.
    /// No child job is dispatched — the parent suspends directly and resumes
    /// when a user hits the resume/cancel endpoint.
    #[serde(rename = "approval")]
    Approval { key: String, timeout: Option<u32>, form: Option<Value> },
}

/// A step dispatched by the WAC SDK.
///
/// `dispatch_type` determines how the child job is created:
/// - `"inline"` (default): re-runs the parent workflow with `_executing_key` set
/// - `"script"`: runs a separate Windmill script resolved from `script` path
/// - `"flow"`: runs a separate Windmill flow resolved from `script` path
#[derive(Debug, Deserialize, Clone)]
pub struct WacStepDispatch {
    pub name: String,
    pub script: String,
    pub args: serde_json::Map<String, Value>,
    pub key: String,
    #[serde(default = "default_dispatch_type")]
    pub dispatch_type: String,
}

fn default_dispatch_type() -> String {
    "inline".to_string()
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
    let ids_map: serde_json::Map<String, Value> = job_ids
        .iter()
        .map(|(key, id)| (key.clone(), Value::String(id.to_string())))
        .collect();
    // Accumulate into persistent job_ids (survives pending_steps clearing)
    for (k, v) in ids_map.iter() {
        checkpoint.job_ids.insert(k.clone(), v.clone());
    }
    let pending = WacPendingSteps {
        mode: mode.to_string(),
        keys: steps.iter().map(|s| s.key.clone()).collect(),
        job_ids: ids_map,
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

/// Detect WAC v2 patterns in TypeScript/Bun code.
/// Checks for `import ... from "windmill-client"` containing workflow/task,
/// skipping comment lines.
pub fn is_wac_v2_ts(code: &str) -> bool {
    let mut has_wac_import = false;
    let mut has_workflow = false;
    let mut has_task = false;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.contains("windmill-client")
            && (trimmed.starts_with("import") || trimmed.starts_with("from"))
        {
            has_wac_import = true;
            if trimmed.contains("workflow") {
                has_workflow = true;
            }
            if trimmed.contains("task") {
                has_task = true;
            }
        }
        if trimmed.contains("export") && trimmed.contains("workflow(") {
            has_workflow = true;
        }
    }
    has_wac_import && has_workflow && has_task
}

/// Detect WAC v2 patterns in Python code.
/// Checks for `@workflow` decorator and `@task` decorator with wmill import,
/// skipping comment lines.
pub fn is_wac_v2_py(code: &str) -> bool {
    let mut has_wmill_import = false;
    let mut has_workflow_decorator = false;
    let mut has_task_decorator = false;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("import wmill") || trimmed.starts_with("from wmill") {
            has_wmill_import = true;
        }
        if trimmed == "@workflow" || trimmed.starts_with("@workflow(") {
            has_workflow_decorator = true;
        }
        if trimmed == "@task" || trimmed.starts_with("@task(") {
            has_task_decorator = true;
        }
    }
    has_wmill_import && has_workflow_decorator && has_task_decorator
}
