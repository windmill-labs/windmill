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
//!
//! Subscribers (V1):
//!   - Only `script` runnables. Flow subscribers defer.
//!   - The subscriber must have at least one non-archived script row.
//!   - A subscriber is skipped if its path equals the producer's path
//!     (self-loop) or already appears in the cascade lineage
//!     (`trigger.chain`) — cycle detection, which bounds the cascade
//!     without capping legitimate depth.
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
//!       "chain": ["f/a/producer0", "f/a/producer1"]
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
use std::sync::Arc;
use uuid::Uuid;
use windmill_common::assets::AssetKind;
use windmill_common::error::{self, Result};
use windmill_common::get_latest_deployed_hash_for_path;
use windmill_common::jobs::{JobKind, JobPayload, JobTriggerKind};
use windmill_common::partition::PARTITION_ARG;
use windmill_common::scripts::ScriptHash;
use windmill_common::triggers::TriggerMetadata;
use windmill_common::users::{get_email_from_permissioned_as, username_to_permissioned_as};
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

/// Reserved arg key that suppresses asset-trigger dispatch for a single run.
/// Set by the test panel when the user opts out of the cascade.
pub const SKIP_ASSET_DISPATCH_ARG: &str = "_wmill_skip_asset_dispatch";

/// Arg key holding the cascade trigger object (carries `chain`, `partition`,
/// producer metadata) injected into every dispatched subscriber.
const TRIGGER_ARG: &str = "trigger";

/// Arg key (under `trigger.chain`) carrying the cascade lineage: the ordered
/// list of producer paths already run in this chain. Used to detect cycles
/// (a producer re-appearing) and stop only the cyclic edge — so deep but
/// *acyclic* pipelines are never truncated.
const CHAIN_KEY: &str = "chain";

/// Safety backstop on lineage length. Cycle detection already bounds an
/// acyclic cascade (a path can't repeat), so this only guards against a
/// runaway from a bug. Set far above any real pipeline depth.
const MAX_CHAIN_LEN: usize = 1000;

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
    // A parented script is dispatch-eligible only as a native retry attempt — a
    // re-run of the SAME runnable as its chain parent. Schedule/error/recovery
    // handlers are also parented `Script` children but run a DIFFERENT script;
    // excluding them stops a handler that happens to declare assets from
    // triggering a cascade (the pre-native-retry `parent_job IS NULL` guard
    // excluded every parented child).
    if job.parent_job.is_some() && !is_native_retry_attempt(db, job).await? {
        return Ok(DispatchResult::default());
    }
    let runnable_path = match job.runnable_path.as_deref() {
        Some(p) if !p.is_empty() => p,
        _ => return Ok(DispatchResult::default()),
    };

    // Producer gate (cached): this hook fires on every top-level
    // script/preview completion, and the overwhelmingly common case is a
    // script that writes no asset. The per-workspace producer→writes map is
    // cached and invalidated by a trigger on `asset`, so a non-producer
    // completion costs one in-memory lookup and zero queries. The map is
    // keyed on the deploy-time `asset` table by path, so an undeployed/new
    // preview (no asset rows for its path) is a non-producer and never
    // cascades — same as the previous per-completion lookup.
    let producers = workspace_producer_writes(db, &job.workspace_id).await?;
    let Some(writes) = producers.get(runnable_path).cloned() else {
        return Ok(DispatchResult::default());
    };

    let args = fetch_args(db, &job.workspace_id, job.id).await?;
    if read_skip_arg(args.as_ref()) {
        return Ok(DispatchResult::default());
    }
    // Parse the cascade `trigger` object once; both the lineage chain and
    // the propagated partition are read from it.
    let trigger_map = args
        .as_ref()
        .and_then(|a| a.get(TRIGGER_ARG))
        .and_then(|t| serde_json::from_str::<HashMap<String, Box<RawValue>>>(t.get()).ok());
    let chain = read_chain(trigger_map.as_ref());
    let partition = read_partition(args.as_ref(), trigger_map.as_ref());
    if chain.len() >= MAX_CHAIN_LEN {
        tracing::warn!(
            "asset-trigger dispatch skipped: cascade lineage length {} >= backstop {} (job {}, path {})",
            chain.len(),
            MAX_CHAIN_LEN,
            job.id,
            runnable_path
        );
        return Ok(DispatchResult::default());
    }
    // Lineage propagated to any subscriber pushed from this producer: the
    // ancestors that already ran, plus this producer.
    let mut next_chain = chain.clone();
    next_chain.push(runnable_path.to_string());

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
            // Cycle guard: a subscriber already in this producer's lineage
            // would re-enter the chain (A→…→A), looping forever. Stop only
            // this edge — sibling branches still dispatch, and acyclic chains
            // of any depth are unaffected.
            if chain.iter().any(|p| p == &sub_path) {
                events.push(EventRow::new(
                    &sub_path,
                    asset_kind,
                    &asset_path,
                    DispatchOutcome::Skipped,
                    EventOptions { reason: Some("cycle_detected"), ..Default::default() },
                ));
                continue;
            }
            if join_all {
                match crate::cascade::handle_join(
                    db,
                    &job.workspace_id,
                    &sub_path,
                    &trigger_ref,
                    partition.as_deref(),
                )
                .await
                {
                    Ok(crate::cascade::JoinDecision::Skip(reason)) => {
                        events.push(EventRow::new(
                            &sub_path,
                            asset_kind,
                            &asset_path,
                            DispatchOutcome::Skipped,
                            EventOptions { reason: Some(reason), ..Default::default() },
                        ));
                        continue;
                    }
                    Ok(crate::cascade::JoinDecision::Pending { received, required }) => {
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
                    Ok(crate::cascade::JoinDecision::Fire) => {} // fall through to push
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
                &next_chain,
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
    // Flow steps (and sub-flow jobs) carry `flow_step_id` and are ineligible.
    // Native script-retry attempts carry `parent_job` (the chain root) but no
    // `flow_step_id`; whether a parented job is actually a retry attempt (vs a
    // schedule/error handler child) is decided in `try_dispatch`.
    if job.flow_step_id.is_some() {
        return false;
    }
    true
}

// Native retry attempts carry an explicit `native_retry_attempt` marker; no
// other parented `Script` child (schedule handlers, WAC inline children, flow
// steps) does. One indexed point lookup, only for parented jobs.
async fn is_native_retry_attempt(db: &DB, job: &MiniCompletedJob) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM native_retry_attempt WHERE job_id = $1) AS \"exists!\"",
        job.id,
    )
    .fetch_one(db)
    .await?)
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

fn read_chain(trigger_map: Option<&HashMap<String, Box<RawValue>>>) -> Vec<String> {
    trigger_map
        .and_then(|m| m.get(CHAIN_KEY))
        .and_then(|v| serde_json::from_str::<Vec<String>>(v.get()).ok())
        .unwrap_or_default()
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

lazy_static::lazy_static! {
    /// Per-workspace map of producer script path → the assets it writes
    /// (`usage_access_type IN ('w','rw')`). Serves both the producer gate
    /// (is this path a producer?) and the writes themselves, so a completion
    /// that isn't a producer costs a single in-memory lookup and zero
    /// queries — the dispatch hook fires on every top-level script/preview
    /// completion instance-wide, the overwhelming majority of which write no
    /// asset. An empty map means the workspace has no asset producers (no
    /// pipelines). Invalidated per workspace by `notify_asset_producer_change`
    /// (a trigger on `asset`) through the polling notify system; until the
    /// next poll a freshly-deployed producer may not cascade (sub-poll lag,
    /// acceptable for a data pipeline).
    pub static ref ASSET_PRODUCER_WRITES_CACHE:
        quick_cache::sync::Cache<String, Arc<HashMap<String, Vec<(AssetKind, String)>>>> =
        quick_cache::sync::Cache::new(1000);
}

/// Test hook: disables the producer-writes cache so every dispatch reads the
/// current DB. Integration tests use `#[sqlx::test]` isolated DBs that all
/// share one workspace id, so a process-global cache keyed by workspace would
/// clobber across DBs under concurrent test threads. Always `false` in
/// production (the cache is invalidated via the notify_event poller instead).
pub static ASSET_PRODUCER_CACHE_DISABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Load (cached) the producer→writes map for a workspace. The single load
/// query replaces the per-completion producer lookup; once cached, every
/// completion in the workspace is served from memory until invalidation.
async fn workspace_producer_writes(
    db: &Pool<Postgres>,
    workspace_id: &str,
) -> Result<Arc<HashMap<String, Vec<(AssetKind, String)>>>> {
    let use_cache = !ASSET_PRODUCER_CACHE_DISABLED.load(std::sync::atomic::Ordering::Relaxed);
    if use_cache {
        if let Some(map) = ASSET_PRODUCER_WRITES_CACHE.get(workspace_id) {
            return Ok(map);
        }
    }
    let rows = sqlx::query!(
        r#"
        SELECT
            usage_path AS "usage_path!",
            kind AS "kind!: AssetKind",
            path AS "path!"
        FROM asset
        WHERE workspace_id = $1
          AND usage_kind = 'script'
          AND usage_access_type IN ('w', 'rw')
        "#,
        workspace_id,
    )
    .fetch_all(db)
    .await?;
    let mut map: HashMap<String, Vec<(AssetKind, String)>> = HashMap::new();
    for r in rows {
        map.entry(r.usage_path).or_default().push((r.kind, r.path));
    }
    let map = Arc::new(map);
    if use_cache {
        ASSET_PRODUCER_WRITES_CACHE.insert(workspace_id.to_string(), map.clone());
    }
    Ok(map)
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

async fn push_subscriber(
    db: &DB,
    producer: &MiniCompletedJob,
    subscriber_path: &str,
    asset_kind: AssetKind,
    asset_path: &str,
    producer_path: &str,
    chain: &[String],
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

    // Debounce / retry semantics are a `private` feature (see `cascade`).
    // OSS degrades both: debounce falls back to the subscriber's own
    // script-level settings, retry is never applied.
    let debouncing_settings = crate::cascade::cascade_debouncing_settings(
        subscriber_path,
        partition,
        debounce_s,
        script.runnable_settings.debouncing_settings,
    );

    // When the cascade declares a retry, hand `push` a one-step-flow request
    // carrying the policy + `language`; `push` materializes it into a native
    // retryable `Script` (not a flow), so a failed/recovered subscriber stays
    // eligible to trigger its own downstream. No retry = plain `ScriptHash`.
    let payload = if let Some(retry) = crate::cascade::cascade_retry(retry_count, retry_delay_s) {
        JobPayload::SingleStepFlow {
            path: subscriber_path.to_string(),
            hash: Some(hash),
            flow_version: None,
            language: Some(script.language),
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
        CHAIN_KEY: chain,
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
