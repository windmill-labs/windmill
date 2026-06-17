/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Runtime fan-out for asset-triggered scripts.
//!
//! When a script writes an asset and a downstream script subscribes to
//! that asset via `// on s3://...`, this module pushes a job for each
//! subscriber after the producer's job completes successfully. Any
//! asset-writing top-level script cascades — there is no `// pipeline`
//! gate on the producer side; subscriptions alone define the graph.
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
use windmill_common::assets::{AssetKind, PARTITION_TOKEN};
use windmill_common::error::{self, Result};
use windmill_common::get_latest_deployed_hash_for_path;
use windmill_common::jobs::{JobKind, JobPayload, JobTriggerKind};
use windmill_common::partition::PARTITION_ARG;
use windmill_common::runnable_settings::DebouncingSettings;
use windmill_common::scripts::ScriptHash;
use windmill_common::triggers::TriggerMetadata;
use windmill_common::users::{get_email_from_permissioned_as, username_to_permissioned_as};
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

/// Reserved arg key that suppresses asset-trigger dispatch for a single run.
/// Set by the test panel when the user opts out of the cascade.
pub const SKIP_ASSET_DISPATCH_ARG: &str = "_wmill_skip_asset_dispatch";

/// Arg key holding the cascade trigger object (carries `depth`, `partition`,
/// producer metadata) injected into every dispatched subscriber.
const TRIGGER_ARG: &str = "trigger";

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

/// Per-decision outcome persisted to `dispatch_event` so the producer's
/// job detail page can show what happened to each subscriber. Mirrors
/// the `DISPATCH_OUTCOME` Postgres enum exactly.
#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "DISPATCH_OUTCOME", rename_all = "snake_case")]
enum DispatchOutcome {
    Dispatched,
    JoinPending,
    Skipped,
}

/// Outcome-specific fields. Event constructors take the four "always-present"
/// columns positionally and bundle the rest here so each call site only
/// names what it actually carries.
#[derive(Debug, Default)]
struct EventOptions<'a> {
    child_job_id: Option<Uuid>,
    partition: Option<&'a str>,
    received_inputs: Option<i32>,
    required_inputs: Option<i32>,
    debounce_s: Option<i32>,
    reason: Option<&'a str>,
}

/// One accumulated `dispatch_event` row. Owned (not borrowed) so the whole
/// dispatch pass can collect rows and flush them in a single batched INSERT
/// at the end, avoiding an N+1 (one INSERT per subscriber × asset write).
#[derive(Debug)]
struct EventRow {
    subscriber_path: String,
    asset_kind: AssetKind,
    asset_path: String,
    outcome: DispatchOutcome,
    child_job_id: Option<Uuid>,
    partition: Option<String>,
    received_inputs: Option<i32>,
    required_inputs: Option<i32>,
    debounce_s: Option<i32>,
    reason: Option<String>,
}

impl EventRow {
    fn new(
        subscriber_path: &str,
        asset_kind: AssetKind,
        asset_path: &str,
        outcome: DispatchOutcome,
        opts: EventOptions<'_>,
    ) -> Self {
        EventRow {
            subscriber_path: subscriber_path.to_string(),
            asset_kind,
            asset_path: asset_path.to_string(),
            outcome,
            child_job_id: opts.child_job_id,
            partition: opts.partition.map(str::to_string),
            received_inputs: opts.received_inputs,
            required_inputs: opts.required_inputs,
            debounce_s: opts.debounce_s,
            reason: opts.reason.map(str::to_string),
        }
    }
}

/// AND-join slot progress at the moment a partition-bearing input
/// arrived. `fired` = all required inputs are now present (the slot has
/// been cleared and the subscriber will be dispatched). `received` /
/// `required` are the counts observed inside the slot's transaction
/// (monotonic up to that point), surfaced so the dispatch_event row can
/// show partial progress like "2/3".
#[derive(Debug, Clone, Copy)]
struct JoinSlotStatus {
    fired: bool,
    received: i32,
    required: i32,
}

/// Outcome of evaluating an AND-join barrier for one (subscriber, input).
/// The caller turns this into a `dispatch_event` row and decides whether to
/// push, all in one place.
enum JoinDecision {
    /// Input does not advance the join (recorded as Skipped with `reason`).
    Skip(&'static str),
    /// Join advanced but is not yet complete (recorded as JoinPending).
    Pending { received: i32, required: i32 },
    /// All required inputs are present — push the subscriber.
    Fire,
}

/// Evaluate the AND-join barrier for a partition-bearing subscriber input.
/// Only a partition-bearing input carrying a concrete partition advances the
/// join — a reference input or an unpartitioned producer must never fire a
/// partitioned join (the case-3 silent-wrong guard). On `Err` the caller
/// should log and skip without recording an event.
async fn handle_join(
    db: &DB,
    workspace_id: &str,
    sub_path: &str,
    trigger_ref: &str,
    partition: Option<&str>,
) -> Result<JoinDecision> {
    if !is_partition_bearing_ref(trigger_ref) {
        tracing::debug!(
            "AND subscriber {}: non-partition-bearing input {} does not fire the join",
            sub_path,
            trigger_ref
        );
        return Ok(JoinDecision::Skip("case3_non_partition_bearing"));
    }
    let Some(pv) = partition else {
        tracing::warn!(
            "AND subscriber {}: partition-bearing input {} arrived with no resolved \
             partition; not dispatching (case-3 guard)",
            sub_path,
            trigger_ref
        );
        return Ok(JoinDecision::Skip("case3_missing_partition"));
    };
    match record_and_check_join_slot(db, workspace_id, sub_path, pv, trigger_ref).await? {
        JoinSlotStatus { fired: false, received, required } => {
            Ok(JoinDecision::Pending { received, required })
        }
        JoinSlotStatus { fired: true, .. } => Ok(JoinDecision::Fire),
    }
}

/// Best-effort batched insert into `dispatch_event`. Never propagates — the
/// dispatch contract is "logging failures must not retroactively fail the
/// producer's job." All rows accumulated over a dispatch pass go in one
/// INSERT (UNNEST) to avoid an N+1 across (subscriber × asset write).
async fn flush_events(db: &DB, workspace_id: &str, producer_job_id: Uuid, events: &[EventRow]) {
    if events.is_empty() {
        return;
    }
    // Column-oriented arrays for UNNEST. Each Vec is one column across all rows.
    let subscriber_paths: Vec<String> = events.iter().map(|e| e.subscriber_path.clone()).collect();
    let asset_kinds: Vec<AssetKind> = events.iter().map(|e| e.asset_kind).collect();
    let asset_paths: Vec<String> = events.iter().map(|e| e.asset_path.clone()).collect();
    let outcomes: Vec<DispatchOutcome> = events.iter().map(|e| e.outcome).collect();
    let child_job_ids: Vec<Option<Uuid>> = events.iter().map(|e| e.child_job_id).collect();
    let partitions: Vec<Option<String>> = events.iter().map(|e| e.partition.clone()).collect();
    let received_inputs: Vec<Option<i32>> = events.iter().map(|e| e.received_inputs).collect();
    let required_inputs: Vec<Option<i32>> = events.iter().map(|e| e.required_inputs).collect();
    let debounce_s: Vec<Option<i32>> = events.iter().map(|e| e.debounce_s).collect();
    let reasons: Vec<Option<String>> = events.iter().map(|e| e.reason.clone()).collect();

    let res = sqlx::query!(
        r#"INSERT INTO dispatch_event (
             workspace_id, producer_job_id, subscriber_path,
             asset_kind, asset_path, outcome,
             child_job_id, partition,
             received_inputs, required_inputs,
             debounce_s, reason
           )
           SELECT $1, $2, sp, ak, ap, oc, cj, pt, ri, rq, db, rs
           FROM unnest(
             $3::text[], $4::ASSET_KIND[], $5::text[], $6::DISPATCH_OUTCOME[],
             $7::uuid[], $8::text[], $9::int[], $10::int[], $11::int[], $12::text[]
           ) AS t(sp, ak, ap, oc, cj, pt, ri, rq, db, rs)"#,
        workspace_id,
        producer_job_id,
        &subscriber_paths,
        asset_kinds as Vec<AssetKind>,
        &asset_paths,
        outcomes as Vec<DispatchOutcome>,
        &child_job_ids as &[Option<Uuid>],
        &partitions as &[Option<String>],
        &received_inputs as &[Option<i32>],
        &required_inputs as &[Option<i32>],
        &debounce_s as &[Option<i32>],
        &reasons as &[Option<String>],
    )
    .execute(db)
    .await;
    if let Err(e) = res {
        tracing::error!(
            "failed to record {} dispatch_event row(s) for producer {}: {e:#}",
            events.len(),
            producer_job_id
        );
    }
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

    // Cheapest filter first: this hook runs on every top-level script
    // completion, and the overwhelmingly common case is a script that
    // writes no asset — one indexed query and out, before touching the
    // job's args JSONB.
    let writes = fetch_producer_writes(db, &job.workspace_id, runnable_path).await?;
    if writes.is_empty() {
        return Ok(DispatchResult::default());
    }

    let args = fetch_args(db, &job.workspace_id, job.id).await?;
    if read_skip_arg(args.as_ref()) {
        return Ok(DispatchResult::default());
    }
    // Parse the cascade `trigger` object once; both depth and the
    // propagated partition are read from it.
    let trigger_map = args
        .as_ref()
        .and_then(|a| a.get(TRIGGER_ARG))
        .and_then(|t| serde_json::from_str::<HashMap<String, Box<RawValue>>>(t.get()).ok());
    let depth = read_chain_depth(trigger_map.as_ref());
    let partition = read_partition(args.as_ref(), trigger_map.as_ref());
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

    let mut dispatched = Vec::new();
    // Best-effort dispatch_event rows accumulated over the whole pass and
    // flushed in one batched INSERT at the end (avoids an N+1 over
    // subscriber × asset write). The mid-pass join-slot writes
    // (record_and_check_join_slot) are a separate table and unaffected.
    let mut events: Vec<EventRow> = Vec::new();
    for (asset_kind, asset_path) in writes {
        let Some(prefix) = asset_kind.canonical_prefix() else {
            continue;
        };
        let trigger_ref = format!("{}{}", prefix, asset_path);
        let subs = fetch_subscribers(db, &job.workspace_id, &trigger_ref).await?;
        for sub in subs {
            let Subscriber { path: sub_path, join_all, debounce_s, retry_count, retry_delay_s } =
                sub;
            if sub_path == runnable_path {
                events.push(EventRow::new(
                    &sub_path,
                    asset_kind,
                    &asset_path,
                    DispatchOutcome::Skipped,
                    EventOptions { reason: Some("self_loop"), ..Default::default() },
                ));
                continue;
            }
            if join_all {
                match handle_join(
                    db,
                    &job.workspace_id,
                    &sub_path,
                    &trigger_ref,
                    partition.as_deref(),
                )
                .await
                {
                    Ok(JoinDecision::Skip(reason)) => {
                        events.push(EventRow::new(
                            &sub_path,
                            asset_kind,
                            &asset_path,
                            DispatchOutcome::Skipped,
                            EventOptions { reason: Some(reason), ..Default::default() },
                        ));
                        continue;
                    }
                    Ok(JoinDecision::Pending { received, required }) => {
                        events.push(EventRow::new(
                            &sub_path,
                            asset_kind,
                            &asset_path,
                            DispatchOutcome::JoinPending,
                            EventOptions {
                                partition: partition.as_deref(),
                                received_inputs: Some(received),
                                required_inputs: Some(required),
                                ..Default::default()
                            },
                        ));
                        continue; // slot incomplete — wait for the rest
                    }
                    Ok(JoinDecision::Fire) => {} // fall through to push
                    Err(e) => {
                        tracing::error!("join-slot check failed for {}: {e:#}", sub_path);
                        continue;
                    }
                }
            }
            match push_subscriber(
                db,
                job,
                &sub_path,
                asset_kind,
                &asset_path,
                runnable_path,
                depth + 1,
                partition.as_deref(),
                debounce_s,
                retry_count,
                retry_delay_s,
            )
            .await
            {
                Ok(id) => {
                    events.push(EventRow::new(
                        &sub_path,
                        asset_kind,
                        &asset_path,
                        DispatchOutcome::Dispatched,
                        EventOptions {
                            child_job_id: Some(id),
                            partition: partition.as_deref(),
                            debounce_s,
                            ..Default::default()
                        },
                    ));
                    dispatched.push(id);
                }
                Err(e) => {
                    tracing::error!("failed to push asset-triggered job for {}: {e:#}", sub_path)
                }
            }
        }
    }

    flush_events(db, &job.workspace_id, job.id, &events).await;

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

fn read_chain_depth(trigger_map: Option<&HashMap<String, Box<RawValue>>>) -> i64 {
    trigger_map
        .and_then(|m| m.get(CHAIN_DEPTH_KEY))
        .and_then(|v| serde_json::from_str::<i64>(v.get()).ok())
        .unwrap_or(0)
}

/// The partition value the producer ran with, if any. Resolved once at the
/// top of a chain (run-start) and threaded down here so every cascaded job
/// materializes the same partition without re-resolving. Top-level
/// `partition` arg (run-start injection) takes precedence over the
/// `trigger.partition` carried from an upstream cascade hop.
fn read_partition(
    args: Option<&HashMap<String, Box<RawValue>>>,
    trigger_map: Option<&HashMap<String, Box<RawValue>>>,
) -> Option<String> {
    if let Some(v) = args.and_then(|a| a.get(PARTITION_ARG)) {
        if let Ok(s) = serde_json::from_str::<String>(v.get()) {
            return Some(s);
        }
    }
    serde_json::from_str::<String>(trigger_map?.get(PARTITION_ARG)?.get()).ok()
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

/// A subscriber row resolved from `script_trigger`. Bundles the per-edge
/// options (debounce) and the script-level policy fields (`join_all`,
/// retry) that travel together to dispatch.
struct Subscriber {
    path: String,
    join_all: bool,
    debounce_s: Option<i32>,
    retry_count: Option<i16>,
    retry_delay_s: Option<i32>,
}

async fn fetch_subscribers(
    db: &Pool<Postgres>,
    workspace_id: &str,
    trigger_ref: &str,
) -> Result<Vec<Subscriber>> {
    // V1: script subscribers only. Flow subscribers (`runnable_kind = 'flow'`)
    // are intentionally excluded — wiring them is straightforward but the
    // payload shape and permissioning need their own pass.
    // `join_all` = `// trigger all` (AND join); `debounce_s` = the opt-in
    // debounce window resolved at deploy (NULL = fan-out, the default).
    // `retry_count` / `retry_delay_s` = the `// retry <n> [<delay>]` policy
    // (NULL = no retry).
    let rows = sqlx::query!(
        r#"
        SELECT runnable_path AS "runnable_path!", join_all AS "join_all!", debounce_s,
               retry_count, retry_delay_s
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
    Ok(rows
        .into_iter()
        .map(|r| Subscriber {
            path: r.runnable_path,
            join_all: r.join_all,
            debounce_s: r.debounce_s,
            retry_count: r.retry_count,
            retry_delay_s: r.retry_delay_s,
        })
        .collect())
}

/// A `// on <asset>` whose stored ref carries the literal `{partition}`
/// token is partition-bearing — its concrete partition is the AND-join
/// key. Non-token asset refs are reference/presence-only inputs.
fn is_partition_bearing_ref(trigger_ref: &str) -> bool {
    trigger_ref.contains(PARTITION_TOKEN)
}

/// Record an AND input arrival for `(subscriber, partition)` and report
/// whether every partition-bearing input is now present for that
/// partition. The record -> count -> clear sequence runs in one
/// transaction guarded by a transaction-scoped advisory lock keyed on the
/// slot: the same subscriber's last two partition-bearing inputs can
/// complete concurrently on different workers, and a check-then-act on a
/// pooled connection would let both observe "complete" and dispatch
/// twice. Serializing per `(workspace, subscriber, partition)` makes the
/// gate fire exactly once; the lock is released on commit/rollback.
/// Idempotent per input (PK conflict ignored); the slot is cleared on
/// fire so later writes re-accumulate and can re-materialize.
///
/// `required` = the distinct partition-bearing inputs the subscriber
/// declares (its `{partition}`-token `// on` lines). Reference inputs
/// (no token) and non-asset triggers are presence-only and excluded.
async fn record_and_check_join_slot(
    db: &DB,
    workspace_id: &str,
    subscriber_path: &str,
    partition: &str,
    trigger_ref: &str,
) -> Result<JoinSlotStatus> {
    let mut tx = db.begin().await?;
    let lock_key = format!("{workspace_id}|{subscriber_path}|{partition}");
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        lock_key,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        r#"INSERT INTO join_pending_inputs
             (workspace_id, subscriber_path, partition, trigger_ref)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT DO NOTHING"#,
        workspace_id,
        subscriber_path,
        partition,
        trigger_ref,
    )
    .execute(&mut *tx)
    .await?;
    let required = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT trigger_ref) AS "n!"
           FROM script_trigger
           WHERE workspace_id = $1
             AND runnable_path = $2
             AND trigger_kind = 'asset'
             AND runnable_kind = 'script'
             AND trigger_ref LIKE '%' || $3 || '%'"#,
        workspace_id,
        subscriber_path,
        PARTITION_TOKEN,
    )
    .fetch_one(&mut *tx)
    .await?;
    let received = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT trigger_ref) AS "n!"
           FROM join_pending_inputs
           WHERE workspace_id = $1 AND subscriber_path = $2 AND partition = $3"#,
        workspace_id,
        subscriber_path,
        partition,
    )
    .fetch_one(&mut *tx)
    .await?;
    let fire = required > 0 && received >= required;
    if fire {
        sqlx::query!(
            r#"DELETE FROM join_pending_inputs
               WHERE workspace_id = $1 AND subscriber_path = $2 AND partition = $3"#,
            workspace_id,
            subscriber_path,
            partition,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    // i64 -> i32: the COUNT(DISTINCT trigger_ref) values are bounded by the
    // number of `// on` lines on a single subscriber — fits trivially.
    Ok(JoinSlotStatus { fired: fire, received: received as i32, required: required as i32 })
}

/// Default time a partial AND-join slot may sit with no new input before
/// it is abandoned. A slot is normally cleared the moment the join fires;
/// this only reaps slots whose inputs never all arrive (an upstream was
/// removed/renamed, a one-off `dynamic` partition key, permanent skew).
/// Conservative so a legitimately slow join is never reaped; a per-join
/// configurable TTL via the annotation is a planned follow-up.
pub const JOIN_SLOT_TTL_SECS: i64 = 60 * 24 * 60 * 60; // 60 days

/// Reap abandoned AND-join slots. Keyed on the *slot's most recent row*
/// (`HAVING max(received_at)`), never per-row, so a join whose inputs
/// trickle in over a window longer than one input's age is not corrupted
/// mid-accumulation. Called periodically from the monitor loop.
pub async fn reap_stale_join_slots(db: &DB) -> Result<()> {
    sqlx::query!(
        "DELETE FROM join_pending_inputs jpi
         USING (
             SELECT workspace_id, subscriber_path, partition
             FROM join_pending_inputs
             GROUP BY workspace_id, subscriber_path, partition
             HAVING max(received_at) <= now() - ($1::bigint::text || ' s')::interval
         ) stale
         WHERE jpi.workspace_id = stale.workspace_id
           AND jpi.subscriber_path = stale.subscriber_path
           AND jpi.partition = stale.partition",
        JOIN_SLOT_TTL_SECS,
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn push_subscriber(
    db: &DB,
    producer: &MiniCompletedJob,
    subscriber_path: &str,
    asset_kind: AssetKind,
    asset_path: &str,
    producer_path: &str,
    depth: i64,
    partition: Option<&str>,
    debounce_s: Option<i32>,
    retry_count: Option<i16>,
    retry_delay_s: Option<i32>,
) -> Result<Uuid> {
    // Same resolution as every other trigger path (`script_path_to_payload`):
    // latest deployed hash plus the script's own runnable settings
    // (concurrency, debounce, timeout), resolved through the
    // runnable-settings handle. The cascade must not bypass a subscriber's
    // concurrency limit just because it was triggered by an asset write.
    let script = get_latest_deployed_hash_for_path(
        None,
        db.clone(),
        &producer.workspace_id,
        subscriber_path,
    )
    .await?
    .prefetch_cached(db)
    .await?;
    let hash = ScriptHash(script.hash);
    let tag = script.tag;
    let concurrency_settings = script.runnable_settings.concurrency_settings;

    // Debounce is opt-in per subscriber edge (`// debounce` /
    // `// on … debounce=`). When set, the window is keyed by
    // (subscriber, partition) so distinct partitions never collapse and
    // "latest within the window" falls out for free. When not set, the
    // subscriber's own script-level debounce settings (if any) apply.
    let debouncing_settings = match debounce_s {
        Some(s) if s > 0 => DebouncingSettings {
            debounce_key: Some(format!(
                "asset-cascade:{}:{}",
                subscriber_path,
                partition.unwrap_or("")
            )),
            debounce_delay_s: Some(s),
            ..DebouncingSettings::default()
        },
        _ => script.runnable_settings.debouncing_settings,
    };

    // Retry is only available via the flow runtime — wrap the script in a
    // one-step flow with `Retry { constant: { attempts, seconds } }` when
    // the cascade declares `// retry`. The exponential delay and retry_if
    // expression are intentionally unused: the annotation grammar is
    // count-plus-constant-delay only (parser-light). Empty retry =
    // unwrapped `ScriptHash` push, matching the previous behaviour.
    let payload = if let Some(count) = retry_count.filter(|c| *c > 0) {
        let delay = retry_delay_s.unwrap_or(0).max(0).min(u16::MAX as i32) as u16;
        let retry = windmill_common::flows::Retry {
            constant: windmill_common::flows::ConstantDelay {
                attempts: count as u32,
                seconds: delay,
            },
            exponential: windmill_common::flows::ExponentialDelay::default(),
            retry_if: None,
        };
        JobPayload::SingleStepFlow {
            path: subscriber_path.to_string(),
            hash: Some(hash),
            flow_version: None,
            args: HashMap::new(),
            retry: Some(retry),
            error_handler_path: None,
            error_handler_args: None,
            skip_handler: None,
            cache_ttl: script.cache_ttl,
            cache_ignore_s3_path: script.cache_ignore_s3_path,
            priority: script.priority,
            tag_override: tag.clone(),
            trigger_path: None,
            apply_preprocessor: false,
            concurrency_settings,
            debouncing_settings,
        }
    } else {
        JobPayload::ScriptHash {
            hash,
            path: subscriber_path.to_string(),
            cache_ttl: script.cache_ttl,
            cache_ignore_s3_path: script.cache_ignore_s3_path,
            dedicated_worker: script.dedicated_worker,
            language: script.language,
            priority: script.priority,
            apply_preprocessor: false,
            debouncing_settings,
            concurrency_settings,
            labels: script.labels,
        }
    };

    // Run the subscriber under its deployer's identity — never the
    // producer's. Subscriptions are workspace-wide, so attributing the run
    // to the producer would let anyone who can deploy a `// on` script
    // execute code with the permissions of whoever happens to write the
    // asset (e.g. an admin's scheduled job). `on_behalf_of_email` (an
    // explicit service-account opt-in at deploy) takes precedence for the
    // email; otherwise the deployer's email is resolved from their
    // username.
    let permissioned_as = username_to_permissioned_as(&script.created_by);
    let email = match script.on_behalf_of_email {
        Some(obo) => obo,
        None => {
            get_email_from_permissioned_as(&permissioned_as, &producer.workspace_id, db).await?
        }
    };

    let mut args: HashMap<String, Box<RawValue>> = HashMap::new();
    let trigger_payload = serde_json::json!({
        "kind": "asset",
        "asset_kind": serde_json::to_value(&asset_kind).expect("AssetKind serializes"),
        "asset_path": asset_path,
        "producer_path": producer_path,
        "producer_job_id": producer.id.to_string(),
        CHAIN_DEPTH_KEY: depth,
        PARTITION_ARG: partition,
    });
    args.insert(TRIGGER_ARG.to_string(), to_raw_value(&trigger_payload));
    // Carry the producer's resolved partition forward as a top-level arg so
    // the subscriber's body can read it and the next cascade hop's
    // `read_partition` picks it up — keeps the whole chain on one partition,
    // resolved once at the top. Omitted entirely for non-partitioned chains.
    if let Some(p) = partition {
        args.insert(PARTITION_ARG.to_string(), to_raw_value(&p));
    }

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
        script.timeout,
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
