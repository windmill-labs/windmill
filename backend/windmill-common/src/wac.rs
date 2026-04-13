//! Workflow-as-Code v2 checkpoint model and persistence primitives.
//!
//! Lives in `windmill-common` (not `windmill-worker`) so the API server can
//! write checkpoint deltas directly from the SDK fast path without pulling in
//! the entire worker crate. The worker still re-exports the same symbols from
//! `windmill_worker::wac_executor` for its own historical call sites.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::{self, Error};
use crate::DB;

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
/// SET running = false` in the same transaction — restoring the original
/// all-or-nothing atomicity — while the API fast path simply commits after
/// the helper returns.
///
/// Concurrency: the first DB operation is a `SELECT ... FOR UPDATE` that
/// row-locks the `v2_job_status` row for the duration of the transaction. If
/// two callers race on the same `job_id`, the second one waits until the
/// first commits, then reads the freshly-persisted checkpoint — no
/// read-modify-write loss.
pub async fn persist_inline_checkpoint_delta(
    tx: &mut Transaction<'_, Postgres>,
    job_id: &Uuid,
    source_hash_hint: Option<&str>,
    key: &str,
    result: Value,
    started_at: Option<&str>,
    duration_ms: Option<u64>,
) -> error::Result<()> {
    // Row-lock the existing checkpoint row (if any) for the duration of the
    // transaction. NULL if the row doesn't exist yet — that's fine, the
    // INSERT below handles both paths.
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

    Ok(())
}
