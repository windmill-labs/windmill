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
    InlineCheckpoint {
        key: String,
        result: Value,
        #[serde(default)]
        started_at: Option<String>,
        #[serde(default)]
        duration_ms: Option<u64>,
    },
    /// Suspend the workflow waiting for an external approval event.
    /// No child job is dispatched — the parent suspends directly and resumes
    /// when a user hits the resume/cancel endpoint.
    #[serde(rename = "approval")]
    Approval {
        key: String,
        timeout: Option<u32>,
        form: Option<Value>,
        #[serde(default)]
        self_approval_disabled: Option<bool>,
    },
    /// Server-side sleep — suspend the workflow for a duration without holding a worker.
    #[serde(rename = "sleep")]
    Sleep { key: String, seconds: u32 },
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
    // Per-task options forwarded to push()
    #[serde(default)]
    pub timeout: Option<i32>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub cache_ttl: Option<i32>,
    #[serde(default)]
    pub priority: Option<i16>,
    #[serde(default)]
    pub concurrent_limit: Option<i32>,
    #[serde(default)]
    pub concurrency_key: Option<String>,
    #[serde(default)]
    pub concurrency_time_window_s: Option<i32>,
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
            let checkpoint: WacCheckpoint = match serde_json::from_value(status) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        job_id = %job_id,
                        error = %e,
                        "Failed to deserialize WAC checkpoint, resetting to empty"
                    );
                    WacCheckpoint::default()
                }
            };
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

/// If the checkpoint has a pending approval or sleep, inject the resume result
/// into `completed_steps` and save back to DB. Returns the (possibly modified) checkpoint.
///
/// Called by both bun and python executors before writing checkpoint.json to disk.
pub async fn prepare_checkpoint_for_resume(
    db: &DB,
    job_id: &Uuid,
    mut checkpoint: WacCheckpoint,
) -> error::Result<WacCheckpoint> {
    let pending_mode = checkpoint.pending_steps.as_ref().map(|p| p.mode.as_str());

    match pending_mode {
        Some("approval") => {
            let approval_key = checkpoint
                .pending_steps
                .as_ref()
                .and_then(|p| p.keys.first().cloned())
                .unwrap_or_default();

            let resume_row = sqlx::query_as::<_, (sqlx::types::Json<Box<serde_json::value::RawValue>>, Option<String>, bool)>(
                "SELECT value, approver, approved FROM resume_job WHERE job = $1 ORDER BY created_at ASC LIMIT 1",
            )
            .bind(job_id)
            .fetch_optional(db)
            .await?;

            let approval_result = if let Some((value, approver, approved)) = resume_row {
                serde_json::json!({
                    "value": serde_json::from_str::<Value>(value.get()).unwrap_or(Value::Null),
                    "approver": approver.unwrap_or_else(|| "anonymous".to_string()),
                    "approved": approved,
                })
            } else {
                serde_json::json!({
                    "value": null,
                    "approver": null,
                    "approved": false,
                })
            };
            checkpoint
                .completed_steps
                .insert(approval_key.clone(), approval_result);
            checkpoint.pending_steps = None;
            save_checkpoint(db, job_id, &checkpoint).await?;

            // Update the approval step's timeline entry with duration_ms
            let step_timeline_key = format!("_step/{}", approval_key);
            sqlx::query(
                "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                    workflow_as_code_status,
                    ARRAY[$2, 'duration_ms'],
                    to_jsonb(EXTRACT(EPOCH FROM (now() - (workflow_as_code_status->$2->>'started_at')::timestamptz)) * 1000)
                ) WHERE id = $1 AND workflow_as_code_status ? $2",
            )
            .bind(job_id)
            .bind(&step_timeline_key)
            .execute(db)
            .await
            .ok(); // best-effort

            tracing::info!(
                job_id = %job_id,
                approval_key = %approval_key,
                "WAC v2 injected approval result into checkpoint"
            );
        }
        Some("sleep") => {
            let sleep_key = checkpoint
                .pending_steps
                .as_ref()
                .and_then(|p| p.keys.first().cloned())
                .unwrap_or_default();

            checkpoint
                .completed_steps
                .insert(sleep_key.clone(), Value::Bool(true));
            checkpoint.pending_steps = None;
            save_checkpoint(db, job_id, &checkpoint).await?;

            tracing::info!(
                job_id = %job_id,
                sleep_key = %sleep_key,
                "WAC v2 resumed from sleep"
            );
        }
        _ => {}
    }

    Ok(checkpoint)
}

/// Detect WAC v2 patterns in TypeScript/Bun code.
/// Checks for `import ... from "windmill-client"` containing workflow,
/// skipping comment lines. Handles both single-line and multi-line imports.
pub fn is_wac_v2_ts(code: &str) -> bool {
    let mut has_wac_import = false;
    let mut has_workflow = false;
    let mut in_import_block = false;
    let mut import_block_has_workflow = false;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        // Single-line import: import { workflow, task } from "windmill-client"
        if trimmed.contains("windmill-client")
            && (trimmed.starts_with("import") || trimmed.starts_with("from"))
        {
            has_wac_import = true;
            if trimmed.contains("workflow") {
                has_workflow = true;
            }
            in_import_block = false;
        }
        // Start of multi-line import: import {
        else if trimmed.starts_with("import") && trimmed.contains("{") && !trimmed.contains("}") {
            in_import_block = true;
            import_block_has_workflow = trimmed.contains("workflow");
        }
        // Inside multi-line import block
        else if in_import_block {
            if trimmed.contains("workflow") {
                import_block_has_workflow = true;
            }
            // End of multi-line import: } from "windmill-client"
            if trimmed.contains("windmill-client") {
                has_wac_import = true;
                if import_block_has_workflow {
                    has_workflow = true;
                }
                in_import_block = false;
            }
            // End of import block but not windmill-client
            if trimmed.contains("}") {
                in_import_block = false;
            }
        }
        if trimmed.contains("export") && trimmed.contains("workflow(") {
            has_workflow = true;
        }
    }
    has_wac_import && has_workflow
}

/// Inject the variable name as the first argument to `task()` calls in WAC v2 scripts.
/// `const double = task(async ...` → `const double = task("double", async ...`
/// Skips calls that already have a string argument.
pub fn inject_wac_task_names(content: &str) -> String {
    use regex::Regex;
    use std::borrow::Cow;
    lazy_static::lazy_static! {
        static ref TASK_RE: Regex =
            Regex::new(r#"(?m)((?:export\s+)?(?:const|let|var)\s+)(\w+)(\s*=\s*task\s*(?:<[^>]*>)?\s*\(\s*)(async\b)"#).unwrap();
    }
    let replaced = TASK_RE.replace_all(content, r#"${1}${2}${3}"${2}", ${4}"#);
    match replaced {
        Cow::Borrowed(_) => content.to_string(),
        Cow::Owned(s) => s,
    }
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
