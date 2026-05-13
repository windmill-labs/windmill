/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Runtime fan-out for asset-triggered scripts.
//!
//! When a producer pipeline script (`// pipeline`) writes an asset and a
//! downstream script subscribes to that asset via `// on s3://...`, this
//! module pushes a job for each subscriber after the producer's job
//! completes successfully.
//!
//! Eligibility (V1, narrow on purpose):
//!   - Producer kind is `Script` or `Preview`. Flows defer.
//!   - Producer is top-level (no `parent_job`, no `flow_step_id`).
//!   - Producer succeeded.
//!   - The producer's args do not contain `_wmill_skip_asset_dispatch: true`.
//!   - Cascade depth (carried in args under `trigger.depth`) is below
//!     `MAX_CHAIN_DEPTH`.
//!
//! Subscribers (V1):
//!   - Only `script` runnables. Flow subscribers defer.
//!   - The subscriber must have at least one non-archived script row.
//!   - A subscriber is skipped if its path equals the producer's path
//!     (avoids trivial self-loops).
//!
//! Args sent to subscribers:
//!   ```json
//!   {
//!     "trigger": {
//!       "kind": "asset",
//!       "asset_kind": "s3object",
//!       "asset_path": "...",
//!       "producer_path": "...",
//!       "producer_job_id": "...",
//!       "depth": 1
//!     }
//!   }
//!   ```
//!
//! Errors are logged but never bubble up to fail the producer's job.

use crate::{push, MiniCompletedJob, PushArgs, PushIsolationLevel};
use serde_json::value::RawValue;
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::assets::{parse_asset_trigger_ref, AssetKind};
use windmill_common::error::{self, Result};
use windmill_common::get_latest_hash_for_path;
use windmill_common::jobs::{JobKind, JobPayload, JobTriggerKind};
use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
use windmill_common::triggers::TriggerMetadata;
use windmill_common::users::username_to_permissioned_as;
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

/// Reserved arg key that suppresses asset-trigger dispatch for a single run.
/// Set by the test panel when the user opts out of the cascade.
pub const SKIP_ASSET_DISPATCH_ARG: &str = "_wmill_skip_asset_dispatch";

/// Reserved arg key (under `trigger.depth`) that carries cascade depth.
const CHAIN_DEPTH_KEY: &str = "depth";

/// Cap cascade depth so a misconfigured ring doesn't fan out unbounded.
const MAX_CHAIN_DEPTH: i64 = 5;

/// Returned to the caller (the worker's completed-job hook) so logs can
/// reference the dispatched ids.
#[derive(Debug, Default)]
pub struct DispatchResult {
    pub dispatched: Vec<Uuid>,
}

/// Top-level entry. Returns `Ok(default)` and logs on any internal failure
/// rather than propagating, because dispatch is best-effort and must not
/// retroactively fail the producer.
pub async fn dispatch_asset_triggers(db: &DB, job: &MiniCompletedJob) -> DispatchResult {
    match try_dispatch(db, job).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("asset-trigger dispatch failed for job {}: {e:#}", job.id);
            DispatchResult::default()
        }
    }
}

async fn try_dispatch(db: &DB, job: &MiniCompletedJob) -> Result<DispatchResult> {
    if !is_eligible_kind(job) {
        return Ok(DispatchResult::default());
    }
    let runnable_path = match job.runnable_path.as_deref() {
        Some(p) if !p.is_empty() => p,
        _ => return Ok(DispatchResult::default()),
    };

    // Args were moved from v2_job to v2_job_completed by add_completed_job
    // before dispatch runs. Fetch from v2_job_completed.
    let args = fetch_args(db, &job.workspace_id, job.id).await?;
    if read_skip_arg(args.as_ref()) {
        return Ok(DispatchResult::default());
    }
    let depth = read_chain_depth(args.as_ref());
    if depth >= MAX_CHAIN_DEPTH {
        tracing::warn!(
            "asset-trigger dispatch skipped: chain depth {} >= cap {} (job {}, path {})",
            depth,
            MAX_CHAIN_DEPTH,
            job.id,
            runnable_path
        );
        return Ok(DispatchResult::default());
    }

    let writes = fetch_producer_writes(db, &job.workspace_id, runnable_path).await?;
    if writes.is_empty() {
        return Ok(DispatchResult::default());
    }

    let mut dispatched = Vec::new();
    for (asset_kind, asset_path) in writes {
        let Some(prefix) = prefix_for(asset_kind) else {
            continue;
        };
        let trigger_ref = format!("{}{}", prefix, asset_path);
        let subs = fetch_subscribers(db, &job.workspace_id, &trigger_ref).await?;
        for sub_path in subs {
            if sub_path == runnable_path {
                continue;
            }
            match push_subscriber(
                db,
                job,
                &sub_path,
                asset_kind,
                &asset_path,
                runnable_path,
                depth + 1,
            )
            .await
            {
                Ok(id) => dispatched.push(id),
                Err(e) => {
                    tracing::error!("failed to push asset-triggered job for {}: {e:#}", sub_path)
                }
            }
        }
    }

    if !dispatched.is_empty() {
        tracing::info!(
            "asset-trigger dispatch from job {} ({}): pushed {} downstream jobs",
            job.id,
            runnable_path,
            dispatched.len()
        );
    }
    Ok(DispatchResult { dispatched })
}

fn is_eligible_kind(job: &MiniCompletedJob) -> bool {
    if !matches!(job.kind, JobKind::Script | JobKind::Preview) {
        return false;
    }
    if job.parent_job.is_some() || job.flow_step_id.is_some() {
        return false;
    }
    true
}

async fn fetch_args(
    db: &Pool<Postgres>,
    workspace_id: &str,
    job_id: Uuid,
) -> Result<Option<HashMap<String, Box<RawValue>>>> {
    // Read from v2_job because args live there permanently — v2_job_completed
    // is the *result* row and doesn't carry args. The producer's v2_job row
    // is still present at dispatch time (deletion happens later in the
    // completion pipeline, after this hook).
    let row = sqlx::query!(
        r#"SELECT args AS "args!: Json<HashMap<String, Box<RawValue>>>"
           FROM v2_job
           WHERE workspace_id = $1 AND id = $2"#,
        workspace_id,
        job_id,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.args.0))
}

fn read_skip_arg(args: Option<&HashMap<String, Box<RawValue>>>) -> bool {
    args.and_then(|a| a.get(SKIP_ASSET_DISPATCH_ARG))
        .and_then(|v| serde_json::from_str::<bool>(v.get()).ok())
        .unwrap_or(false)
}

fn read_chain_depth(args: Option<&HashMap<String, Box<RawValue>>>) -> i64 {
    let Some(args) = args else {
        return 0;
    };
    let Some(trigger) = args.get("trigger") else {
        return 0;
    };
    let Ok(map) = serde_json::from_str::<HashMap<String, Box<RawValue>>>(trigger.get()) else {
        return 0;
    };
    map.get(CHAIN_DEPTH_KEY)
        .and_then(|v| serde_json::from_str::<i64>(v.get()).ok())
        .unwrap_or(0)
}

fn prefix_for(kind: AssetKind) -> Option<&'static str> {
    match kind {
        AssetKind::S3Object => Some("s3://"),
        AssetKind::Resource => Some("$res:"),
        AssetKind::Ducklake => Some("ducklake://"),
        AssetKind::DataTable => Some("datatable://"),
        AssetKind::Volume => Some("volume://"),
        // Deprecated kind from before the parser was unified — has no
        // canonical trigger ref and never produced trigger rows.
        AssetKind::Variable => None,
    }
}

async fn fetch_producer_writes(
    db: &Pool<Postgres>,
    workspace_id: &str,
    runnable_path: &str,
) -> Result<Vec<(AssetKind, String)>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            kind AS "kind!: AssetKind",
            path AS "path!"
        FROM asset
        WHERE workspace_id = $1
          AND usage_kind = 'script'
          AND usage_path = $2
          AND usage_access_type IN ('w', 'rw')
        "#,
        workspace_id,
        runnable_path,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| (r.kind, r.path)).collect())
}

async fn fetch_subscribers(
    db: &Pool<Postgres>,
    workspace_id: &str,
    trigger_ref: &str,
) -> Result<Vec<String>> {
    // V1: script subscribers only. Flow subscribers (`runnable_kind = 'flow'`)
    // are intentionally excluded — wiring them is straightforward but the
    // payload shape and permissioning need their own pass.
    let rows = sqlx::query!(
        r#"
        SELECT runnable_path AS "runnable_path!"
        FROM script_trigger
        WHERE workspace_id = $1
          AND trigger_kind = 'asset'
          AND trigger_ref = $2
          AND runnable_kind = 'script'
        "#,
        workspace_id,
        trigger_ref,
    )
    .fetch_all(db)
    .await?;
    // Sanity check: parsing the stored trigger_ref must succeed. If it
    // doesn't, the row is corrupt; skip it loudly rather than silently.
    if parse_asset_trigger_ref(trigger_ref).is_none() {
        tracing::warn!(
            "asset-trigger dispatch: trigger_ref {} did not round-trip through parse_asset_trigger_ref",
            trigger_ref
        );
    }
    Ok(rows.into_iter().map(|r| r.runnable_path).collect())
}

async fn push_subscriber(
    db: &DB,
    producer: &MiniCompletedJob,
    subscriber_path: &str,
    asset_kind: AssetKind,
    asset_path: &str,
    producer_path: &str,
    depth: i64,
) -> Result<Uuid> {
    let (
        hash,
        tag,
        _concurrency_key,
        _concurrent_limit,
        _concurrency_time_window_s,
        _debounce_key,
        _debounce_delay_s,
        cache_ttl,
        cache_ignore_s3_path,
        language,
        dedicated_worker,
        priority,
        _timeout,
        on_behalf_of_email,
        created_by,
        _runnable_settings_handle,
        labels,
    ) = get_latest_hash_for_path(db, &producer.workspace_id, subscriber_path, false).await?;

    let payload = JobPayload::ScriptHash {
        hash,
        path: subscriber_path.to_string(),
        cache_ttl,
        cache_ignore_s3_path,
        dedicated_worker,
        language,
        priority,
        apply_preprocessor: false,
        // V1: skip debouncing/concurrency for asset-triggered runs. The
        // trigger fan-out is the user's intent — we don't want a noisy
        // upstream's writes to silently drop downstream runs because a
        // debounce key collides. Revisit if we see lots of dups.
        debouncing_settings: DebouncingSettings::default(),
        concurrency_settings: ConcurrencySettings::default(),
        labels,
    };

    // Subscriber's own on_behalf_of_email controls identity when set;
    // otherwise we run as the producer. This keeps the asset cascade
    // attributable to whoever originally wrote the asset, while still
    // honoring scripts that explicitly opted into a service-account email.
    let (permissioned_as, email) = if let Some(obo) = on_behalf_of_email {
        (username_to_permissioned_as(&created_by), obo)
    } else {
        (
            producer.permissioned_as.clone(),
            producer.permissioned_as_email.clone(),
        )
    };

    let mut args: HashMap<String, Box<RawValue>> = HashMap::new();
    let trigger_payload = serde_json::json!({
        "kind": "asset",
        "asset_kind": serde_json::to_value(&asset_kind).unwrap_or(serde_json::Value::Null),
        "asset_path": asset_path,
        "producer_path": producer_path,
        "producer_job_id": producer.id.to_string(),
        CHAIN_DEPTH_KEY: depth,
    });
    args.insert("trigger".to_string(), to_raw_value(&trigger_payload));

    // Attribute the dispatched run to a synthetic user so audit logs reflect
    // it came from the asset cascade, not the original human runner.
    let pseudo_user = format!("asset-{producer_path}");

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (id, tx) = push(
        db,
        tx,
        &producer.workspace_id,
        payload,
        PushArgs { args: &args, extra: None },
        &pseudo_user,
        &email,
        permissioned_as,
        Some(producer_path),
        None,
        Some(producer_path.to_string()),
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        tag,
        None,
        None,
        None,
        None,
        false,
        None,
        Some(TriggerMetadata::new(
            Some(producer_path.to_string()),
            JobTriggerKind::Asset,
        )),
        None,
    )
    .await
    .map_err(|e| error::Error::internal_err(format!("push asset-triggered job: {e:#}")))?;
    tx.commit().await?;
    Ok(id)
}

