//! Workflow-as-Code v2 checkpoint model and persistence primitives.
//!
//! Lives in `windmill-common` (not `windmill-worker`) so the API server can
//! write checkpoint deltas directly from the SDK fast path without pulling in
//! the entire worker crate. The worker still re-exports the same symbols from
//! `windmill_worker::wac_executor` for its own historical call sites.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

use crate::error::{self, Error};
use crate::DB;

/// Set when a child completion brought a parked WAC parent's `suspend` counter to
/// zero, so a worker's pull loop tries the suspended-jobs query first instead of
/// waiting for its next periodic attempt.
pub static WAC_SUSPEND_READY: AtomicBool = AtomicBool::new(false);

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
    /// With `_executing_key`: the SDK's fingerprint of the task's code and the
    /// arguments the task was called with, what its cached result is keyed on.
    /// A child whose SDK sent no fingerprint is keyed on its step key and its
    /// parent's arguments instead (`cached_result_path`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _executing_fn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _executing_args: Option<serde_json::Map<String, Value>>,
    /// `resume_job.id` values already consumed by earlier approval steps (the
    /// row primary key, not the distinct integer `resume_id` column). Rows are
    /// never deleted, so a workflow with several sequential wait_for_approval()
    /// calls accumulates one per approval; excluding these lets each step read
    /// its own row rather than the oldest.
    #[serde(default)]
    pub consumed_resume_row_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WacPendingSteps {
    pub mode: String,
    pub keys: Vec<String>,
    pub job_ids: serde_json::Map<String, Value>,
}

/// `resume_id` bound to a WAC `wait_for_approval` step key.
///
/// Two callers must agree on it — the worker minting the inline resume/cancel
/// buttons at suspend time, and the API signing URLs the workflow asked for
/// ahead of time — so the derivation must be stable across processes and
/// releases. `DefaultHasher` is explicitly not (std makes no cross-release
/// guarantee), hence SHA-256 truncated to the `u32` the resume routes take.
/// Distinctness per key is what matters: `resume_job`'s primary key is
/// `job_id ^ resume_id`, so two steps sharing a resume_id would collide on
/// one row.
pub fn approval_resume_id(step_key: &str) -> u32 {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(step_key.as_bytes());
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

#[cfg(test)]
mod tests {
    use super::approval_resume_id;

    /// Golden values: worker and API must agree on this mapping, and they can run
    /// different builds during a rolling deploy. Changing it strands every resume
    /// URL already in the hands of an approver, so a diff here is a deliberate
    /// break, not a refactor.
    #[test]
    fn approval_resume_id_is_a_stable_cross_process_contract() {
        assert_eq!(approval_resume_id("approval"), 0x9deb_65b8);
        assert_eq!(approval_resume_id("approval_2"), 0x50d1_eeca);
        assert_eq!(approval_resume_id("manager"), 0x6ee4_a469);
    }

    use super::wac_failure_record;
    use serde_json::json;

    /// The point of the function: a task failure and a step failure describing
    /// the same error must be indistinguishable to the handler that catches
    /// them, apart from the child job only a task has.
    #[test]
    fn a_task_and_a_step_failure_read_the_same() {
        let from_child = wac_failure_record(
            "fetch",
            Some("abc-123"),
            &json!({"error": {"name": "ValueError", "message": "nope", "stack": "frames"}}),
        );
        let from_step = wac_failure_record(
            "fetch",
            None,
            &json!({"error": {"name": "ValueError", "message": "nope", "stack": "frames"}}),
        );
        assert_eq!(from_child["result"], from_step["result"]);
        assert_eq!(from_child["message"], json!("nope"));
        assert_eq!(from_step["message"], json!("nope"));
        assert_eq!(from_child["child_job_id"], json!("abc-123"));
        assert_eq!(from_step.get("child_job_id"), None);
    }

    /// A child job's result is whatever the failing job produced — a cancel, a
    /// timeout, an executor that writes a bare string. The handler is still
    /// promised `name` and `message`, so they cannot be conjured per-caller.
    #[test]
    fn an_unshaped_child_result_still_yields_name_and_message() {
        for raw in [
            json!({"error": "boom"}),
            json!({"error": {"message": "boom"}}),
            json!("boom"),
            json!(null),
        ] {
            let rec = wac_failure_record("s", None, &raw);
            assert_eq!(rec["result"]["error"]["name"], json!("Error"), "{raw}");
            assert!(
                rec["result"]["error"]["message"].is_string(),
                "{raw} produced no message"
            );
            assert_eq!(rec["result"]["error"].get("stack"), None, "{raw}");
        }
        // Nothing to say beats saying `{}`: the fallback exists for these.
        for empty in [json!(null), json!({}), json!([]), json!({"error": ""})] {
            assert_eq!(
                wac_failure_record("s", None, &empty)["message"],
                json!("WAC step 's' failed"),
                "{empty}"
            );
        }
    }

    /// Extra keys are the failing side's own, and dropping them would lose a
    /// custom error's fields; the three normalized ones still win.
    #[test]
    fn extra_error_fields_survive_normalization() {
        let rec = wac_failure_record(
            "s",
            None,
            &json!({"error": {"name": "HttpError", "message": "429", "code": 429, "stack": 12}}),
        );
        assert_eq!(rec["result"]["error"]["code"], json!(429));
        assert_eq!(rec["result"]["error"]["name"], json!("HttpError"));
        // a non-string stack is not something a handler can be told to read
        assert_eq!(rec["result"]["error"].get("stack"), None);
    }

    use super::normalize_posted_step_result;

    /// An SDK that predates the echoed record raises from the copy it built, so
    /// rewriting what it posted would make its live round and its replays
    /// disagree — the very thing being fixed here. It keeps its own shape.
    #[test]
    fn a_legacy_sdk_marker_is_stored_untouched() {
        let legacy = json!({
            "__wmill_error": true,
            "message": "nope",
            "step_key": "s",
            "result": {"error": "nope", "type": "TypeError"},
        });
        assert_eq!(normalize_posted_step_result("s", legacy.clone()), legacy);
    }

    #[test]
    fn a_current_sdk_marker_is_normalized_and_a_success_is_not() {
        let posted = json!({
            "__wmill_error": true,
            "message": "nope",
            "step_key": "s",
            "result": {"error": {"name": "ValueError", "message": "nope"}},
        });
        let stored = normalize_posted_step_result("s", posted);
        assert_eq!(stored["result"]["error"]["name"], json!("ValueError"));
        assert_eq!(stored["step_key"], json!("s"));

        let success = json!({"rows": 3});
        assert_eq!(normalize_posted_step_result("s", success.clone()), success);
    }

    #[test]
    fn an_oversized_stack_is_truncated() {
        let rec = wac_failure_record(
            "s",
            None,
            &json!({"error": {"message": "m", "stack": "x".repeat(100_000)}}),
        );
        let stack = rec["result"]["error"]["stack"].as_str().unwrap();
        assert!(stack.len() < 100_000, "stack was not truncated");
        assert!(stack.ends_with("... (truncated)"));
    }

    /// The cap bounds what lands in the checkpoint, so it has to be bytes: a
    /// multibyte traceback counted in characters would be up to 4x over.
    #[test]
    fn the_stack_cap_counts_bytes_not_characters() {
        let rec = wac_failure_record(
            "s",
            None,
            &json!({"error": {"message": "m", "stack": "é".repeat(50_000)}}),
        );
        let stack = rec["result"]["error"]["stack"].as_str().unwrap();
        assert!(
            stack.len() <= 8 * 1024 + "\n... (truncated)".len(),
            "kept {} bytes",
            stack.len()
        );
    }

    /// `extra` is the failing side's own attributes, so it can carry a response
    /// body straight past the cap that exists to bound the checkpoint.
    #[test]
    fn an_oversized_extra_is_dropped_rather_than_stored() {
        let rec = wac_failure_record(
            "s",
            None,
            &json!({"error": {"message": "m", "extra": {"body": "x".repeat(100_000)}}}),
        );
        assert_eq!(rec["result"]["error"].get("extra"), None);
        assert_eq!(rec["result"]["error"]["extra_omitted"], json!(true));

        // one that fits is kept whole
        let small = wac_failure_record(
            "s",
            None,
            &json!({"error": {"message": "m", "extra": {"code": 429}}}),
        );
        assert_eq!(small["result"]["error"]["extra"], json!({"code": 429}));
        assert_eq!(small["result"]["error"].get("extra_omitted"), None);
    }

    /// `message` is the failure's own message; `Value::to_string` on a string
    /// would hand the handler `"boom"` with the JSON quotes still on it.
    #[test]
    fn a_bare_string_failure_keeps_its_message_unquoted() {
        assert_eq!(
            wac_failure_record("s", None, &json!({"error": "boom"}))["message"],
            json!("boom")
        );
        assert_eq!(
            wac_failure_record("s", None, &json!("boom"))["message"],
            json!("boom")
        );
    }
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

/// Marks a `completed_steps` entry as a failure rather than a step result.
pub(crate) const WAC_ERROR_MARKER: &str = "__wmill_error";

/// Per-field budget for the two unbounded things a failure record carries, the
/// stack and `extra`. `persist_inline_checkpoint_delta` rewrites the whole
/// checkpoint on every step, so either one left unbounded is re-serialized once
/// per subsequent step for the rest of the workflow. The two are additive, so a
/// record costs at most twice this.
const MAX_CHECKPOINT_FIELD_BYTES: usize = 8 * 1024;

fn truncate_stack(stack: &str) -> String {
    if stack.len() <= MAX_CHECKPOINT_FIELD_BYTES {
        return stack.to_string();
    }
    // Byte budget, not characters: the cap exists to bound what goes into the
    // checkpoint, and a multibyte traceback would otherwise be up to 4x it.
    let mut cut = MAX_CHECKPOINT_FIELD_BYTES;
    while cut > 0 && !stack.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}\n... (truncated)", &stack[..cut])
}

/// A failure's own message, without the JSON quoting `Value::to_string` puts
/// around a string. `None` when the value carries no message at all, so the
/// caller's fallback wins — a reader handed `{}` learns less than one handed
/// "WAC step 'x' failed".
fn value_message(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        Value::String(s) => Some(s.clone()),
        Value::Object(o) if o.is_empty() => None,
        Value::Array(a) if a.is_empty() => None,
        other => Some(other.to_string()),
    }
}

fn message_only(message: Option<String>) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    if let Some(message) = message {
        m.insert("message".to_string(), Value::String(message));
    }
    m
}

/// Build the failure record a caught WAC failure reads, from whatever the
/// failing side produced.
///
/// The single place this shape is decided, for both a task failure (arriving as
/// the child job's own result) and a `step()` failure (as the SDK posted it).
/// Assembling it per caller instead is how the two come to disagree on `name`
/// or on `stack` while both claim to be one shape. `name`, `message` and
/// `stack` are normalized here; any other key the failing side attached to its
/// error is passed through untouched, so a custom error's own fields survive.
pub fn wac_failure_record(step_key: &str, child_job_id: Option<&str>, raw_result: &Value) -> Value {
    let mut error = match raw_result.get("error") {
        Some(Value::Object(o)) => o.clone(),
        // A bare-string error (some executors), or a result not shaped like a
        // failure at all: keep whatever it says as the message rather than
        // dropping it.
        Some(other) => message_only(value_message(other)),
        None => message_only(value_message(raw_result)),
    };

    let name = error
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Error")
        .to_string();
    let message = error
        .get("message")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("WAC step '{step_key}' failed"));
    error.insert("name".to_string(), Value::String(name));
    error.insert("message".to_string(), Value::String(message.clone()));
    match error.get("stack").and_then(|v| v.as_str()) {
        Some(stack) => {
            error.insert("stack".to_string(), Value::String(truncate_stack(stack)));
        }
        // Never invent one, and never keep a non-string in the field a handler
        // is told it can read.
        None => {
            error.remove("stack");
        }
    }

    // `extra` is the failing side's own attributes, so it can hold a response
    // body or a dataframe repr and route straight around the stack cap into the
    // checkpoint this record is rewritten into on every later step. Dropped
    // wholesale past the same budget rather than truncated, since half a
    // structure is worse than a flag saying it was too big.
    if let Some(extra) = error.get("extra") {
        if serde_json::to_string(extra).map_or(true, |s| s.len() > MAX_CHECKPOINT_FIELD_BYTES) {
            error.remove("extra");
            error.insert("extra_omitted".to_string(), Value::Bool(true));
        }
    }

    let mut record = serde_json::Map::new();
    record.insert(WAC_ERROR_MARKER.to_string(), Value::Bool(true));
    // `str(e)` / `e.message` reads the failure's own message whether it came
    // from a task or a step; which task, and which child job, are the fields
    // below rather than prose baked into the message.
    record.insert("message".to_string(), Value::String(message));
    record.insert("step_key".to_string(), Value::String(step_key.to_string()));
    if let Some(child) = child_job_id {
        record.insert("child_job_id".to_string(), Value::String(child.to_string()));
    }
    record.insert(
        "result".to_string(),
        Value::Object(
            [("error".to_string(), Value::Object(error))]
                .into_iter()
                .collect(),
        ),
    );
    Value::Object(record)
}

/// Decide what to store for a step result an SDK posted.
///
/// A failure is normalized through `wac_failure_record`, the same function that
/// shapes task failures, so the two cannot drift apart.
///
/// A marker an older SDK posted is stored untouched instead. Those are
/// recognizable by an `error` that is a message string rather than an object,
/// and the SDK that posted one raises from the copy it built and ignores the
/// record echoed back to it. Rewriting it here would leave the round that ran
/// the failing body reading one shape and every replay of it reading another —
/// the divergence this whole mechanism exists to remove. It keeps its own shape
/// until it upgrades.
pub(crate) fn normalize_posted_step_result(key: &str, posted: Value) -> Value {
    if !is_wac_failure(&posted) {
        return posted;
    }
    let normalizable = posted
        .get("result")
        .and_then(|r| r.get("error"))
        .map(|e| e.is_object())
        .unwrap_or(false);
    if !normalizable {
        return posted;
    }
    wac_failure_record(key, None, posted.get("result").unwrap_or(&Value::Null))
}

/// Whether a `completed_steps` entry is a failure record.
pub(crate) fn is_wac_failure(value: &Value) -> bool {
    value
        .get(WAC_ERROR_MARKER)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
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

/// Persist a single inline-step checkpoint delta into the given transaction:
/// validate the source hash, add the step to `completed_steps`, save the
/// checkpoint, and write the `_step/<key>` timeline entry.
///
/// The caller owns the transaction and commits it. This lets the worker-side
/// `WacOutput::InlineCheckpoint` fallback arm add its own `UPDATE v2_job_queue
/// SET running = false` in the same transaction — preserving the original
/// all-or-nothing atomicity — while the API fast path simply commits after
/// the helper returns.
///
/// ## Concurrency model
///
/// The helper does a read-modify-write: `SELECT ... FOR UPDATE` → parse
/// `WacCheckpoint` → modify in Rust via `add_completed_step` → write the
/// full serialized `_checkpoint` back via `INSERT ... ON CONFLICT DO UPDATE`
/// plus a separate `UPDATE` for the `_step/<key>` timeline entry. The
/// important property of this pattern: each call **replaces the whole
/// `_checkpoint` object**, not individual `completed_steps[key]` entries.
/// That means distinct step keys do NOT protect concurrent callers from
/// overwriting each other — two writers that start from the same loaded
/// checkpoint will each produce a new serialized object that lacks the
/// other's step.
///
/// **Steady state (row exists)** — `SELECT ... FOR UPDATE` holds the row
/// lock until commit. The second concurrent caller blocks on the lock,
/// then re-reads the post-commit checkpoint (which already contains the
/// first caller's step), applies its own delta, and writes. No loss.
///
/// **First write (row does not yet exist)** — `SELECT ... FOR UPDATE` on a
/// WHERE clause that matches zero rows acquires no lock. Two concurrent
/// callers would both see `None`, both build a fresh `WacCheckpoint` from
/// scratch, and then race on the final `INSERT ... ON CONFLICT DO UPDATE`:
/// the second writer's `DO UPDATE SET workflow_as_code_status = jsonb_set(
/// ..., '{_checkpoint}', $2)` replaces the `_checkpoint` the first writer
/// just inserted, so the first writer's step is lost.
///
/// That race window is closed **on the client side** by the SDKs:
/// `WorkflowCtx._inline_lock` (Python `asyncio.Lock`) and
/// `WorkflowCtx._inlineChain` (TypeScript promise chain) serialize the
/// fast-path POSTs per workflow invocation. The lock wraps only the HTTP
/// call — `fn()` itself still runs in parallel across `asyncio.gather` /
/// `Promise.all` — so the only thing actually ordered is the sequence of
/// API requests, which is exactly what the helper needs to rely on.
///
/// **Future contributors: do not remove the SDK-side lock without also
/// fixing the server-side first-write guarantee (e.g. via a single-statement
/// merge-UPDATE that's cheap enough — see note below — or a pre-created
/// `v2_job_status` row).** The comment used to claim the SDKs could fire in
/// parallel without client-side serialization; that was wrong, because the
/// helper writes the whole `_checkpoint`.
///
/// Cross-process concurrency with the worker-side legacy fallback arm is
/// safe by construction: both paths receive the same `_StepSuspend` payload
/// (same `key`, same `result`, same `started_at`, same `duration_ms`), so
/// even if the fast path's commit and the worker arm's commit land out of
/// order for the same step, the worst case is a redundant idempotent write,
/// not a divergence.
///
/// ## Why not a single-statement merge-UPDATE?
///
/// A pure-SQL single-statement variant (pushing load-modify-save entirely
/// into `jsonb_set` + `jsonb_build_object` so correctness on the first
/// write comes from Postgres row locking rather than a client-side lock)
/// was prototyped and measured at ~80 ms per call in debug mode — the
/// nested `COALESCE(v2_job_status.workflow_as_code_status->'_checkpoint'
/// ->...)` accesses cause Postgres to evaluate the growing JSONB subtree
/// multiple times per call, and the `||` merges re-serialize the whole
/// object. The two-statement Rust-side load-modify-save below is ~10×
/// faster in practice, so we keep it and rely on the SDK-level lock.
///
/// Returns the value actually stored: a failure posted by an SDK is normalized
/// through `wac_failure_record` first, so the round that ran the failing body
/// can raise from the same record every replay will read instead of building
/// its own copy of it.
pub async fn persist_inline_checkpoint_delta(
    tx: &mut Transaction<'_, Postgres>,
    job_id: &Uuid,
    source_hash_hint: Option<&str>,
    key: &str,
    result: Value,
    started_at: Option<&str>,
    duration_ms: Option<u64>,
) -> error::Result<Option<Value>> {
    // Row-lock the existing checkpoint row (if any) for the duration of the
    // transaction. NULL if the row doesn't exist yet — see the doc comment
    // above for why the first-write race is accepted.
    let row: Option<Option<Value>> = sqlx::query_scalar(
        "SELECT workflow_as_code_status->'_checkpoint'
         FROM v2_job_status WHERE id = $1 FOR UPDATE",
    )
    .bind(job_id)
    .fetch_optional(&mut **tx)
    .await?;

    let mut checkpoint: WacCheckpoint = match row.flatten() {
        Some(status) => serde_json::from_value(status).unwrap_or_else(|e| {
            tracing::warn!(
                job_id = %job_id,
                error = %e,
                "Failed to deserialize WAC checkpoint, resetting to empty"
            );
            WacCheckpoint::default()
        }),
        None => WacCheckpoint::default(),
    };

    // Source hash validation: detect if code changed between replays.
    match source_hash_hint {
        Some(hint) if !hint.is_empty() => {
            if checkpoint.source_hash.is_empty() {
                checkpoint.source_hash = hint.to_string();
            } else if checkpoint.source_hash != hint {
                return Err(Error::ExecutionErr(
                    "Workflow source code changed between replays. \
                     Cannot safely resume from checkpoint — step keys may have shifted. \
                     Please restart this workflow."
                        .to_string(),
                ));
            }
        }
        _ => {
            // Preview / inline jobs have no `runnable_id`, so the caller passes
            // None (or Some("")). We can't validate drift for these — log once
            // so operators can tell which jobs are running unguarded.
            tracing::debug!(
                job_id = %job_id,
                "WAC v2 inline checkpoint without runnable hash — source-hash drift protection is off for this job"
            );
        }
    }

    tracing::info!(
        job_id = %job_id,
        step_key = %key,
        "WAC v2 inline checkpoint — persisting step result"
    );

    let result = normalize_posted_step_result(key, result);

    // Only a failure is ever read back, so only a failure is copied: a
    // successful step's result can be large and moves straight into the
    // checkpoint.
    let failure = is_wac_failure(&result).then(|| result.clone());
    add_completed_step(&mut checkpoint, key, result);

    let status_json = serde_json::to_value(&checkpoint)
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
    .execute(&mut **tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to save WAC checkpoint: {e}")))?;

    // Write the `_step/<key>` timeline entry. Fall back to now() when the
    // client doesn't provide started_at (older SDK versions omit it).
    let now_str = chrono::Utc::now().to_rfc3339();
    let sa = started_at.unwrap_or(&now_str);
    let mut timeline_val = serde_json::json!({
        "scheduled_for": sa,
        "started_at": sa,
        "name": key,
    });
    if let Some(dur) = duration_ms {
        timeline_val["duration_ms"] = serde_json::json!(dur);
    }
    let step_timeline_key = format!("_step/{}", key);
    sqlx::query(
        "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
            COALESCE(workflow_as_code_status, '{}'::jsonb),
            ARRAY[$2],
            $3
        ) WHERE id = $1",
    )
    .bind(job_id)
    .bind(&step_timeline_key)
    .bind(&timeline_val)
    .execute(&mut **tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to write step timeline: {e}")))?;

    Ok(failure)
}

/// The step key a WAC v2 parent is waiting on `child_job` for, if any.
///
/// `job_ids` is the parent's `pending_steps.job_ids` (step key → child job id).
/// A child absent from it is not a step this round is waiting on: a completion
/// arriving after the parent re-dispatched the key to a new job, or a child the
/// body launched directly (`runScript` and friends).
fn pending_step_key(job_ids: &Value, child_job: &Uuid) -> Option<String> {
    let child = child_job.to_string();
    job_ids
        .as_object()?
        .iter()
        .find_map(|(key, id)| (id.as_str() == Some(child.as_str())).then(|| key.clone()))
}

/// Record a completed child on its WAC parent, in the child's completion transaction.
///
/// Every path that brings a child to a terminal state — a worker's result, the
/// zombie monitor, a cancel — completes it through `add_completed_job`, and this is
/// where the parent learns of it. For a child the parent is parked on, the step's
/// result (or failure record) is merged into the checkpoint's `completed_steps` and
/// the parent's `suspend` counter drops by one, atomically with the child's own
/// completion: there is no window in which the child is completed but the parent
/// still waits for it. For any other child, only the timeline entry is stamped.
///
/// Returns whether the counter reached zero, i.e. the parent is ready to be pulled.
///
/// Exactly once: the merge is refused when `completed_steps` already holds the key
/// or `job_ids` no longer maps it to this child, and the decrement follows only a
/// merge that happened. Two completions of one child (a worker and the monitor
/// racing) therefore decrement once, and a stale completion never touches a
/// counter that belongs to a later round.
///
/// Lock order: the parent's queue row, then its status row, then (by the caller)
/// the child's queue row. A cancel walks parent then children, the parent's own
/// completion deletes its queue row and cascades to its status row, and the park
/// (`suspend_wac_parent`) locks the queue row before writing the checkpoint, so
/// any other order can deadlock against one of them.
///
/// Authorization: none is checked here. `child_job` is the job the caller is
/// completing, which it already holds, and `parent_job` must be that job's
/// persisted `v2_job.parent_job` (both callers read it from the child's row).
/// The read that gates the step merge and the counter decrement joins on that
/// relationship, so a mismatched pair changes no parent's `completed_steps` or
/// `suspend`; only the timeline stamp at the end runs unconditionally. Job ids
/// are global, so no workspace scoping is needed on top.
pub async fn record_child_completion(
    tx: &mut Transaction<'_, Postgres>,
    parent_job: &Uuid,
    child_job: &Uuid,
    success: bool,
    duration_ms: i64,
    result: &str,
) -> error::Result<bool> {
    let job_ids: Option<Option<Value>> = sqlx::query_scalar(
        "SELECT s.workflow_as_code_status->'_checkpoint'->'pending_steps'->'job_ids' \
         FROM v2_job_status s JOIN v2_job c ON c.parent_job = s.id \
         WHERE s.id = $1 AND c.id = $2",
    )
    .bind(parent_job)
    .bind(child_job)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to read WAC parent {parent_job}: {e}")))?;

    let step_key = job_ids
        .flatten()
        .and_then(|ids| pending_step_key(&ids, child_job));

    let mut parent_ready = false;
    if let Some(step_key) = step_key {
        let parked: Option<i32> =
            sqlx::query_scalar("SELECT suspend FROM v2_job_queue WHERE id = $1 FOR UPDATE")
                .bind(parent_job)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| {
                    Error::internal_err(format!("Failed to lock WAC parent {parent_job}: {e}"))
                })?;

        if parked.is_some() {
            let step_value = if success {
                result.to_string()
            } else {
                let raw: Value = serde_json::from_str(result).unwrap_or(Value::Null);
                wac_failure_record(&step_key, Some(&child_job.to_string()), &raw).to_string()
            };
            let merged: Option<i32> = sqlx::query_scalar(
                "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                    workflow_as_code_status,
                    '{_checkpoint,completed_steps}',
                    COALESCE(workflow_as_code_status->'_checkpoint'->'completed_steps', '{}'::jsonb)
                        || jsonb_build_object($2::text, $3::text::jsonb)
                 ) WHERE id = $1
                   AND workflow_as_code_status->'_checkpoint'->'pending_steps'->'job_ids'->>$2 = $4
                   AND NOT COALESCE(workflow_as_code_status->'_checkpoint'->'completed_steps' ? $2, false)
                 RETURNING 1",
            )
            .bind(parent_job)
            .bind(&step_key)
            .bind(&step_value)
            .bind(child_job.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| {
                Error::internal_err(format!("Failed to add WAC completed step: {e}"))
            })?;

            if merged.is_some() {
                // `suspend_until` stays set: the suspended pull query is what takes a
                // parked parent back, and it selects on `suspend_until IS NOT NULL`.
                let suspend: Option<i32> = sqlx::query_scalar(
                    "UPDATE v2_job_queue SET suspend = GREATEST(suspend - 1, 0) \
                     WHERE id = $1 RETURNING suspend",
                )
                .bind(parent_job)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| Error::internal_err(format!("Failed to unsuspend WAC parent: {e}")))?;
                parent_ready = suspend == Some(0);
                if parent_ready {
                    sqlx::query(
                        "UPDATE v2_job_status SET workflow_as_code_status = \
                         workflow_as_code_status #- '{_checkpoint,pending_steps}' WHERE id = $1",
                    )
                    .bind(parent_job)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| {
                        Error::internal_err(format!("Failed to clear WAC pending steps: {e}"))
                    })?;
                }
            }
            tracing::info!(
                parent_job = %parent_job,
                child_job = %child_job,
                step_key = %step_key,
                success,
                recorded = merged.is_some(),
                parent_ready,
                "WAC v2 child job completed"
            );
        }
    }

    // The child's entry in the parent's timeline, keyed by child id. The parent may
    // already be completed (cancelled with its children still running), in which
    // case the entry lives on its completed row instead. Errors propagate: a failed
    // statement has already aborted the transaction, so there is nothing to continue with.
    let stamped: Option<i32> = sqlx::query_scalar(
        "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
            jsonb_set(
                workflow_as_code_status,
                ARRAY[$1],
                COALESCE(workflow_as_code_status->$1, '{}'::jsonb)
            ),
            ARRAY[$1, 'duration_ms'],
            to_jsonb($2::bigint)
         ) WHERE id = $3 AND workflow_as_code_status IS NOT NULL RETURNING 1",
    )
    .bind(child_job.to_string())
    .bind(duration_ms)
    .bind(parent_job)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "Could not update parent job `duration_ms` in workflow as code status: {e}"
        ))
    })?;
    if stamped.is_none() {
        sqlx::query(
            "UPDATE v2_job_completed SET workflow_as_code_status = jsonb_set(
                jsonb_set(
                    workflow_as_code_status,
                    ARRAY[$1],
                    COALESCE(workflow_as_code_status->$1, '{}'::jsonb)
                ),
                ARRAY[$1, 'duration_ms'],
                to_jsonb($2::bigint)
             ) WHERE id = $3 AND workflow_as_code_status IS NOT NULL",
        )
        .bind(child_job.to_string())
        .bind(duration_ms)
        .bind(parent_job)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Could not update completed parent job `duration_ms` in workflow as code status: {e}"
            ))
        })?;
    }

    Ok(parent_ready)
}
