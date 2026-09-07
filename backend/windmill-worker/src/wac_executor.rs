use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use windmill_common::error::{self, Error};
use windmill_common::scripts::ScriptLang;
use windmill_common::DB;
use windmill_queue::CanceledBy;

// Checkpoint model + persistence primitives live in windmill-common so the
// API server can use them without pulling in the full worker crate. Re-export
// here for historical call sites inside windmill-worker.
pub use windmill_common::wac::{
    load_checkpoint, persist_inline_checkpoint_delta, save_checkpoint, WacCheckpoint,
    WacPendingSteps,
};

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

/// What `suspend_wac_parent` did with the parent's queue row.
#[derive(Debug)]
pub enum WacPark {
    /// Parked. Carries the segment that just ended, in milliseconds, for `end_wac_segment`.
    Parked(Option<i64>),
    /// A cancel reached the row while this segment was running, so the park was skipped.
    /// Carries who cancelled, for the completion that must happen instead.
    Cancelled(CanceledBy),
}

/// Park a WAC v2 parent in the queue until `suspend` reaches 0 or `suspend_secs`
/// elapses, whichever comes first. `running` stays true so the normal pull query
/// skips the row; only the suspended pull query takes it back. The `id`/`workspace_id`
/// pair is a consistency check, not an authorization one — callers must already hold
/// the job (every one of them passes a job its own worker pulled).
///
/// `started_at` is cleared because the parent holds no worker while parked. The pull
/// re-stamps it (`started_at = coalesce(started_at, now())`), and every path that
/// completes a job without a worker-measured duration — a cancel, the child-failure
/// handler — falls back to `now() - started_at`. Left pointing at the first segment,
/// that fallback reports the whole sleep or approval wait as execution time.
pub async fn suspend_wac_parent(
    tx: &mut Transaction<'_, Postgres>,
    job_id: &Uuid,
    w_id: &str,
    suspend: i32,
    suspend_secs: f64,
) -> error::Result<WacPark> {
    // `FOR UPDATE` orders this against a concurrent soft cancel, which writes `suspend = 0`
    // and leaves acting on `canceled_by` to the next pull. Parking on top of that keeps the
    // row unpullable until `suspend_until` — up to the full `sleep()` — so a cancel already
    // on the row has to stand the park down rather than be overwritten by it.
    let prev = sqlx::query!(
        "SELECT canceled_by, canceled_reason,
                (extract(epoch FROM now() - started_at) * 1000)::bigint AS segment_ms
         FROM v2_job_queue WHERE id = $1 AND workspace_id = $2 FOR UPDATE",
        job_id,
        w_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to read WAC parent job {job_id}: {e}")))?
    // Silently parking nothing is unrecoverable on the dispatch arm: the children are
    // pushed right after and decrement a `suspend` that was never set, so the parent
    // sits out its whole suspend window instead of resuming.
    .ok_or_else(|| {
        Error::internal_err(format!(
            "WAC parent job {job_id} not in the queue of workspace {w_id} to suspend"
        ))
    })?;

    if let Some(username) = prev.canceled_by {
        return Ok(WacPark::Cancelled(CanceledBy {
            username: Some(username),
            reason: prev.canceled_reason,
        }));
    }

    sqlx::query!(
        "UPDATE v2_job_queue
         SET suspend = $3, suspend_until = now() + make_interval(secs => $4), started_at = null
         WHERE id = $1 AND workspace_id = $2",
        job_id,
        w_id,
        suspend,
        suspend_secs,
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to suspend WAC parent job {job_id}: {e}")))?;

    Ok(WacPark::Parked(prev.segment_ms))
}

/// Turn a cancel that landed mid-segment into the error the executor returns, so the job
/// completes on this pass instead of parking. Setting the worker's `canceled_by` is what
/// makes it land as `canceled` rather than `failure`: the row was cancelled after this
/// worker pulled the job, so the in-memory copy still reads as uncancelled.
///
/// The completion charges the segment that just ended, so callers must not also hand it to
/// `end_wac_segment`.
pub(crate) fn wac_cancelled_mid_segment(
    cancel: CanceledBy,
    canceled_by: &mut Option<CanceledBy>,
) -> Error {
    let payload = windmill_common::worker::to_raw_value(&windmill_queue::canceled_result(
        cancel.reason.as_deref(),
        cancel.username.as_deref(),
    ));
    *canceled_by = Some(cancel);
    Error::ExecutionRawError(payload)
}

/// Charge the execution segment a WAC parent just finished. Segments are metered as they
/// end rather than summed at completion, so a workflow that sleeps for days is billed for
/// the compute it used, when it used it — and the final segment is charged by the ordinary
/// completion path.
///
/// Call this only where the parent really parks. On a rollback that goes on to complete
/// the job, the completion charges the same segment and it would be billed twice.
pub(crate) fn end_wac_segment(
    _conn: &windmill_common::worker::Connection,
    _job: &windmill_queue::MiniPulledJob,
    _segment_ms: Option<i64>,
) {
    #[cfg(feature = "cloud")]
    if let (windmill_common::worker::Connection::Sql(db), Some(segment_ms)) = (_conn, _segment_ms) {
        windmill_queue::meter_execution_seconds(
            db,
            &_job.workspace_id,
            &_job.permissioned_as_email,
            segment_ms,
        );
    }
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

            // Exclude rows already consumed by earlier approvals so each step
            // reads its own (rows are never deleted). resume_id can't key this:
            // it's only hash(step_key) for the inline URL, while the approval
            // page, in-run button, Slack, Teams and resume-as-owner store a
            // random id. A timed-out step matches no row -> else branch below.
            let consumed = checkpoint.consumed_resume_row_ids.clone();

            // A row carrying another step's bound resume_id answers that step, not
            // this one, so it must never be picked up here however it got in — the
            // API rejects such resumes but cannot do so atomically with the insert.
            // Only keys this workflow minted a URL for are known to be bound; every
            // other resume_id stays eligible, preserving WIN-2241 for the channels
            // that sign random ids.
            let foreign_bound_ids: Vec<i32> = sqlx::query_scalar::<_, String>(
                "SELECT jsonb_object_keys(
                        COALESCE(workflow_as_code_status->'_minted_approval_keys', '{}'::jsonb))
                     FROM v2_job_status WHERE id = $1",
            )
            .bind(job_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .filter(|k| *k != approval_key)
            .map(|k| windmill_common::wac::approval_resume_id(&k) as i32)
            .collect();

            let resume_row = sqlx::query_as::<
                _,
                (
                    Uuid,
                    sqlx::types::Json<Box<serde_json::value::RawValue>>,
                    Option<String>,
                    bool,
                ),
            >(
                "SELECT id, value, approver, approved FROM resume_job \
                 WHERE job = $1 AND id <> ALL($2) AND resume_id <> ALL($3) \
                 ORDER BY created_at ASC LIMIT 1",
            )
            .bind(job_id)
            .bind(&consumed)
            .bind(&foreign_bound_ids)
            .fetch_optional(db)
            .await?;

            let approval_result = if let Some((row_id, value, approver, approved)) = resume_row {
                checkpoint.consumed_resume_row_ids.push(row_id);
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
/// Checks for a wmill import plus a `@workflow` decorator, skipping comment
/// lines. `@task` is optional: a workflow that only uses inline `step()` calls
/// (no child-job `@task`) is still WAC v2 and must go through the WAC runner
/// so its coroutine gets awaited.
pub fn is_wac_v2_py(code: &str) -> bool {
    let mut has_wmill_import = false;
    let mut has_workflow_decorator = false;
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
    }
    has_wmill_import && has_workflow_decorator
}

/// Whether `content` is a workflow-as-code v2 entrypoint, for the languages whose
/// executor routes it through the WAC runner.
///
/// Mirrors exactly what `handle_bun_job` / `handle_python_job` test: a language
/// missing here (Deno) runs a WAC-shaped script as a plain `main`, so claiming it
/// is WAC would deny it paths it uses correctly today.
pub fn is_wac_v2(lang: Option<ScriptLang>, content: &str) -> bool {
    match lang {
        Some(ScriptLang::Bun) | Some(ScriptLang::Bunnative) => is_wac_v2_ts(content),
        Some(ScriptLang::Python3) => is_wac_v2_py(content),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAC_TS: &str = r#"import { task, workflow } from "windmill-client";
export default workflow(async function main(x: number) { return x; });"#;
    const WAC_PY: &str = "import wmill\n@workflow\ndef main(x: int):\n    return x\n";

    /// Callers use this to decide whether a script may run somewhere that only knows how
    /// to call `main`. Widening it to a language whose executor ignores WAC (Deno) would
    /// take that path away from scripts that use it correctly, and narrowing it would let
    /// a workflow reach a runner that cannot run it.
    #[test]
    fn only_the_languages_whose_executor_runs_wac_report_it() {
        assert!(is_wac_v2(Some(ScriptLang::Bun), WAC_TS));
        assert!(is_wac_v2(Some(ScriptLang::Bunnative), WAC_TS));
        assert!(is_wac_v2(Some(ScriptLang::Python3), WAC_PY));
        assert!(!is_wac_v2(Some(ScriptLang::Deno), WAC_TS));
        assert!(!is_wac_v2(None, WAC_TS));
        assert!(!is_wac_v2(
            Some(ScriptLang::Bun),
            "export async function main() {}"
        ));
    }
}
