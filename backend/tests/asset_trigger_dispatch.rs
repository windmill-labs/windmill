//! End-to-end test for asset-trigger dispatch.
//!
//! Runs a real Bash producer through a worker, lets the
//! `result_processor` hook fire `dispatch_asset_triggers`, and then makes
//! several follow-up calls into `dispatch_asset_triggers` against the same
//! seeded graph to cover the eligibility branches (self-loop, skip arg,
//! cycle guard, flow subscriber, ineligible job kinds). Direct calls share
//! the same workspace so we exercise the real query paths against real
//! `asset` / `script_trigger` rows produced by deploy-equivalent seeding.

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::jobs::{JobKind, JobPayload};
use windmill_common::scripts::{ScriptHash, ScriptLang};
use windmill_queue::asset_dispatch::dispatch_asset_triggers;
use windmill_queue::cascade::reap_stale_join_slots;
use windmill_queue::MiniCompletedJob;
use windmill_test_utils::{initialize_tracing, ApiServer, RunJob};

const WS: &str = "test-workspace";
const PRODUCER: &str = "u/test-user/producer";
const SUB_S3: &str = "u/test-user/sub-s3";
const SUB_RES: &str = "u/test-user/sub-res";
const SUB_FLOW: &str = "u/test-user/sub-flow";

// ── Seeding helpers ───────────────────────────────────────────────────────

async fn seed_script(
    db: &Pool<Postgres>,
    path: &str,
    content: &str,
    language: &str,
) -> anyhow::Result<i64> {
    // Hash needs to be unique per (workspace, hash). Derive from path AND
    // content: the worker's script cache (`cache::script::fetch`) is keyed
    // by hash alone and is process-global, so tests running in the same
    // process that seed the same path with different content would poison
    // each other's cache if the hash came from the path only.
    let mut h = 0i64;
    for b in path.bytes().chain(content.bytes()) {
        h = h.wrapping_mul(31).wrapping_add(b as i64);
    }
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content,
                                created_by, language, tag, lock)
           VALUES ($1, $2, $3, '', '', $4, 'test-user', $5::script_lang, 'deno', '')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(WS)
    .bind(h)
    .bind(path)
    .bind(content)
    .bind(language)
    .execute(db)
    .await?;
    // These tests use #[sqlx::test] isolated DBs that share one workspace id and
    // reuse script paths, while the same path is seeded with different content
    // (hence different hashes) across tests. The process-global deployed-script
    // caches are keyed by (workspace, path)/(workspace, hash), so a concurrent
    // test resolves a path to a hash that lives in another test's DB and the
    // dispatch 404s. Disable them so every resolution reads the test's own DB.
    windmill_common::DEPLOYED_SCRIPT_CACHE_DISABLED
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(h)
}

async fn seed_asset_write(
    db: &Pool<Postgres>,
    producer_path: &str,
    kind: &str,
    asset_path: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind)
           VALUES ($1, $2, $3::asset_kind, 'w'::asset_access_type, $4, 'script'::asset_usage_kind)
           ON CONFLICT DO NOTHING"#,
    )
    .bind(WS)
    .bind(asset_path)
    .bind(kind)
    .bind(producer_path)
    .execute(db)
    .await?;
    // These tests use #[sqlx::test] isolated DBs that all share one workspace
    // id, so the process-global producer cache (keyed by workspace) would
    // clobber across DBs under concurrent test threads. Disable it so every
    // dispatch reads the test's own DB. (Production invalidates via the
    // notify_event poller instead.)
    windmill_queue::asset_dispatch::ASSET_PRODUCER_CACHE_DISABLED
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

async fn seed_subscription(
    db: &Pool<Postgres>,
    subscriber_path: &str,
    subscriber_kind: &str,
    trigger_ref: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script_trigger
             (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref)
           VALUES ($1, $2::asset_usage_kind, $3, 'asset'::script_trigger_kind, $4)"#,
    )
    .bind(WS)
    .bind(subscriber_kind)
    .bind(subscriber_path)
    .bind(trigger_ref)
    .execute(db)
    .await?;
    Ok(())
}

/// Insert a synthetic producer `v2_job` row used by the direct
/// `dispatch_asset_triggers` calls in the edge-case section. `args` lets the
/// test inject `_wmill_skip_asset_dispatch` or `trigger.chain` so the
/// dispatcher's arg-driven branches are exercised against real rows.
async fn seed_producer_job(db: &Pool<Postgres>, args: serde_json::Value) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO v2_job (id, workspace_id, kind, runnable_path, args, created_by,
                                permissioned_as, permissioned_as_email, tag, script_lang)
           VALUES ($1, $2, 'script'::job_kind, $3, $4, 'test-user',
                   'u/test-user', 'test@windmill.dev', 'deno', 'bash'::script_lang)"#,
        id,
        WS,
        PRODUCER,
        args,
    )
    .execute(db)
    .await?;
    Ok(id)
}

/// Like `seed_producer_job` but for an arbitrary runnable path (the
/// AND-join test needs two distinct producers).
async fn seed_producer_job_path(
    db: &Pool<Postgres>,
    path: &str,
    args: serde_json::Value,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO v2_job (id, workspace_id, kind, runnable_path, args, created_by,
                                permissioned_as, permissioned_as_email, tag, script_lang)
           VALUES ($1, $2, 'script'::job_kind, $3, $4, 'test-user',
                   'u/test-user', 'test@windmill.dev', 'deno', 'bash'::script_lang)"#,
        id,
        WS,
        path,
        args,
    )
    .execute(db)
    .await?;
    Ok(id)
}

/// Seed an asset subscription flagged as an AND join (`// trigger all`).
async fn seed_subscription_and(
    db: &Pool<Postgres>,
    subscriber_path: &str,
    trigger_ref: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script_trigger
             (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref, join_all)
           VALUES ($1, 'script'::asset_usage_kind, $2, 'asset'::script_trigger_kind, $3, TRUE)"#,
    )
    .bind(WS)
    .bind(subscriber_path)
    .bind(trigger_ref)
    .execute(db)
    .await?;
    Ok(())
}

/// Seed an asset subscription with an opt-in debounce window (seconds).
async fn seed_subscription_debounced(
    db: &Pool<Postgres>,
    subscriber_path: &str,
    trigger_ref: &str,
    debounce_s: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script_trigger
             (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref, debounce_s)
           VALUES ($1, 'script'::asset_usage_kind, $2, 'asset'::script_trigger_kind, $3, $4)"#,
    )
    .bind(WS)
    .bind(subscriber_path)
    .bind(trigger_ref)
    .bind(debounce_s)
    .execute(db)
    .await?;
    Ok(())
}

/// Seed an asset subscription with a `// retry <n> [<delay>]` policy.
async fn seed_subscription_with_retry(
    db: &Pool<Postgres>,
    subscriber_path: &str,
    trigger_ref: &str,
    retry_count: i16,
    retry_delay_s: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script_trigger
             (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref,
              retry_count, retry_delay_s)
           VALUES ($1, 'script'::asset_usage_kind, $2, 'asset'::script_trigger_kind, $3, $4, $5)"#,
    )
    .bind(WS)
    .bind(subscriber_path)
    .bind(trigger_ref)
    .bind(retry_count)
    .bind(retry_delay_s)
    .execute(db)
    .await?;
    Ok(())
}

fn make_mini(id: Uuid, runnable_path: &str) -> MiniCompletedJob {
    MiniCompletedJob {
        id,
        workspace_id: WS.to_string(),
        runnable_id: Some(ScriptHash(1)),
        scheduled_for: chrono::Utc::now(),
        parent_job: None,
        flow_innermost_root_job: None,
        runnable_path: Some(runnable_path.to_string()),
        kind: JobKind::Script,
        started_at: Some(chrono::Utc::now()),
        permissioned_as: "u/test-user".to_string(),
        created_by: "test-user".to_string(),
        script_lang: Some(ScriptLang::Bash),
        permissioned_as_email: "test@windmill.dev".to_string(),
        flow_step_id: None,
        trigger_kind: None,
        trigger: None,
        priority: None,
        concurrent_limit: None,
        tag: "deno".to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        runnable_settings_handle: None,
    }
}

/// Read `v2_job` rows that were created by asset dispatch (filtered by
/// `trigger_kind = 'asset'` so dep-jobs / other infra rows don't leak in).
async fn fetch_dispatched(
    db: &Pool<Postgres>,
) -> anyhow::Result<Vec<(String, Option<String>, Option<serde_json::Value>)>> {
    let rows = sqlx::query!(
        r#"SELECT runnable_path AS "runnable_path!", trigger,
                  args AS "args: sqlx::types::Json<serde_json::Value>"
             FROM v2_job
             WHERE workspace_id = $1 AND trigger_kind = 'asset'
             ORDER BY runnable_path"#,
        WS,
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.runnable_path, r.trigger, r.args.map(|j| j.0)))
        .collect())
}

async fn clear_dispatched(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!(
        "DELETE FROM v2_job WHERE workspace_id = $1 AND trigger_kind = 'asset'",
        WS,
    )
    .execute(db)
    .await?;
    Ok(())
}

// ── The test ─────────────────────────────────────────────────────────────

/// One end-to-end test that:
///   1. seeds a producer that writes two asset kinds (s3 + resource) with three
///      subscribers (two script subs + one flow sub that must be skipped),
///   2. runs the producer through a real worker and asserts the
///      `result_processor` hook fired and pushed the right jobs with the
///      right trigger metadata,
///   3. then drives `dispatch_asset_triggers` directly against the same
///      seeded graph to cover the arg-driven and eligibility branches that
///      can't be reached by varying the producer's runtime args alone:
///        - skip arg suppresses dispatch
///        - a subscriber already in the lineage is skipped (cycle guard)
///        - the lineage chain accumulates the producer path each hop
///        - self-loop subscriber is filtered
///        - producer with parent_job is ineligible
///        - producer with `Flow` kind is ineligible
#[sqlx::test(fixtures("base"))]
async fn end_to_end_asset_dispatch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // ── Seed the graph ──────────────────────────────────────────────────
    let producer_hash = seed_script(&db, PRODUCER, "echo producer", "bash").await?;
    seed_script(&db, SUB_S3, "echo s3-subscriber", "bash").await?;
    seed_script(&db, SUB_RES, "echo res-subscriber", "bash").await?;
    // Self-loop subscriber: same path as producer → must be filtered.
    seed_subscription(&db, PRODUCER, "script", "s3://f/blob").await?;
    // Flow subscriber on the same asset → V1 hard-filters runnable_kind='flow'.
    seed_subscription(&db, SUB_FLOW, "flow", "s3://f/blob").await?;
    // Legit subscribers.
    seed_subscription(&db, SUB_S3, "script", "s3://f/blob").await?;
    seed_subscription(&db, SUB_RES, "script", "$res:f/cfg").await?;
    // Two writes from one producer, distinct kinds.
    seed_asset_write(&db, PRODUCER, "s3object", "f/blob").await?;
    seed_asset_write(&db, PRODUCER, "resource", "f/cfg").await?;

    // ── 1. Real worker run: the hook must fire after producer success ───
    let job = JobPayload::ScriptHash {
        path: PRODUCER.to_string(),
        hash: ScriptHash(producer_hash),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language: ScriptLang::Bash,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        labels: None,
    };
    let completed = RunJob::from(job).run_until_complete(&db, false, port).await;
    assert!(
        completed.success,
        "producer must succeed for dispatch to fire"
    );

    let mut rows = fetch_dispatched(&db).await?;
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        rows.len(),
        2,
        "expected dispatch to the two legit script subscribers (flow sub filtered, self-loop filtered)"
    );
    let by_path: std::collections::HashMap<_, _> = rows.iter().map(|r| (r.0.as_str(), r)).collect();

    let s3_row = by_path
        .get(SUB_S3)
        .expect("s3 subscriber should have a job");
    let s3_trig = s3_row.2.as_ref().unwrap().get("trigger").unwrap();
    assert_eq!(s3_trig["kind"], "asset");
    assert_eq!(s3_trig["asset_kind"], "s3object");
    assert_eq!(s3_trig["asset_path"], "f/blob");
    assert_eq!(s3_trig["producer_path"], PRODUCER);
    assert_eq!(
        s3_trig["chain"],
        json!([PRODUCER]),
        "lineage starts with the producer on the first hop"
    );
    assert_eq!(s3_row.1.as_deref(), Some(PRODUCER));

    let res_row = by_path
        .get(SUB_RES)
        .expect("resource subscriber should have a job");
    let res_trig = res_row.2.as_ref().unwrap().get("trigger").unwrap();
    assert_eq!(res_trig["asset_kind"], "resource");
    assert_eq!(res_trig["asset_path"], "f/cfg");

    // ── 2. Direct calls to dispatch_asset_triggers for arg / eligibility
    //      branches that can't be reached via the runtime path ──────────
    clear_dispatched(&db).await?;

    // skip arg suppresses dispatch
    let id = seed_producer_job(&db, json!({ "_wmill_skip_asset_dispatch": true })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(id, PRODUCER)).await;
    assert_eq!(r.dispatched.len(), 0, "skip arg suppressed dispatch");

    // cycle guard: a subscriber already in the lineage is skipped, but its
    // siblings still dispatch (only the cyclic edge is cut).
    let id = seed_producer_job(&db, json!({ "trigger": { "chain": [SUB_S3] } })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(id, PRODUCER)).await;
    assert_eq!(
        r.dispatched.len(),
        1,
        "cyclic subscriber (already in lineage) skipped; sibling still dispatched"
    );

    // lineage accumulates: a fresh producer extends the chain with its own path
    clear_dispatched(&db).await?;
    let id = seed_producer_job(&db, json!({ "trigger": { "chain": ["f/upstream"] } })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(id, PRODUCER)).await;
    assert_eq!(r.dispatched.len(), 2);
    let rows = fetch_dispatched(&db).await?;
    for row in &rows {
        assert_eq!(
            row.2.as_ref().unwrap()["trigger"]["chain"],
            json!(["f/upstream", PRODUCER]),
            "lineage accumulates the producer path"
        );
    }

    // flow step (carries flow_step_id) is ineligible
    clear_dispatched(&db).await?;
    let id = seed_producer_job(&db, json!({})).await?;
    let mut mini = make_mini(id, PRODUCER);
    mini.flow_step_id = Some("a".to_string());
    let r = dispatch_asset_triggers(&db, &mini).await;
    assert_eq!(r.dispatched.len(), 0, "flow step producer ineligible");

    // A native retry attempt has a native_retry_attempt marker — it stays eligible,
    // so a subscriber that recovers on retry still cascades.
    clear_dispatched(&db).await?;
    let id = seed_producer_job(&db, json!({})).await?;
    sqlx::query("INSERT INTO native_retry_attempt (job_id, attempt) VALUES ($1, 1)")
        .bind(id)
        .execute(&db)
        .await?;
    let mut mini = make_mini(id, PRODUCER);
    mini.parent_job = Some(Uuid::new_v4());
    let r = dispatch_asset_triggers(&db, &mini).await;
    assert_eq!(
        r.dispatched.len(),
        2,
        "native retry attempt (marked) still dispatches"
    );

    // A parented Script child WITHOUT the marker — a schedule handler or a WAC
    // inline child (which re-runs the same runnable) — must NOT cascade.
    clear_dispatched(&db).await?;
    let id = seed_producer_job(&db, json!({})).await?;
    let mut mini = make_mini(id, PRODUCER);
    mini.parent_job = Some(Uuid::new_v4()); // no marker => not a retry
    let r = dispatch_asset_triggers(&db, &mini).await;
    assert_eq!(
        r.dispatched.len(),
        0,
        "parented child without the marker (handler/WAC inline) does not cascade"
    );

    // producer with kind=Flow is ineligible
    let id = seed_producer_job(&db, json!({})).await?;
    let mut mini = make_mini(id, PRODUCER);
    mini.kind = JobKind::Flow;
    let r = dispatch_asset_triggers(&db, &mini).await;
    assert_eq!(r.dispatched.len(), 0, "flow producer ineligible");

    // Sanity: the eligible direct call (clean producer, no args) still fires —
    // proves the assertions above are negative cases, not a broken setup.
    let id = seed_producer_job(&db, json!({})).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(id, PRODUCER)).await;
    assert_eq!(r.dispatched.len(), 2, "clean direct call still dispatches");

    Ok(())
}

/// Stage C: a `// partitioned dynamic` producer run through a real worker
/// must (1) resolve the partition off its triggering payload at execution
/// time, (2) persist it back into its own `v2_job.args` so the cascade
/// reads it, and (3) propagate the same value into the dispatched
/// subscriber's args + `trigger.partition`.
#[sqlx::test(fixtures("base"))]
async fn partition_dynamic_resolved_persisted_and_propagated(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Producer declares a dynamic partition keyed off the run payload.
    // (Bash uses `#` comments — the annotation parser accepts `#`/`--`/`//`.)
    let producer_hash = seed_script(
        &db,
        PRODUCER,
        "# pipeline\n# partitioned dynamic key=\"$.tenant_id\"\necho producer",
        "bash",
    )
    .await?;
    seed_script(&db, SUB_S3, "echo s3-subscriber", "bash").await?;
    seed_asset_write(&db, PRODUCER, "s3object", "f/blob").await?;
    seed_subscription(&db, SUB_S3, "script", "s3://f/blob").await?;

    let job = JobPayload::ScriptHash {
        path: PRODUCER.to_string(),
        hash: ScriptHash(producer_hash),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language: ScriptLang::Bash,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        labels: None,
    };
    let completed = RunJob::from(job)
        .arg("tenant_id", json!("acme"))
        .run_until_complete(&db, false, port)
        .await;
    assert!(completed.success, "partitioned producer must succeed");

    // (2) resolved value persisted back into the producer's own args.
    let prod = sqlx::query!(
        r#"SELECT args AS "args: sqlx::types::Json<serde_json::Value>"
             FROM v2_job
             WHERE workspace_id = $1 AND runnable_path = $2 AND trigger_kind IS NULL"#,
        WS,
        PRODUCER,
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        prod.args.unwrap().0["partition"],
        json!("acme"),
        "Stage C must persist the resolved partition into v2_job.args"
    );

    // (3) propagated into the dispatched subscriber.
    let rows = fetch_dispatched(&db).await?;
    let sub = rows
        .iter()
        .find(|r| r.0 == SUB_S3)
        .expect("subscriber must be dispatched");
    let args = sub.2.as_ref().unwrap();
    assert_eq!(
        args["partition"],
        json!("acme"),
        "subscriber gets top-level partition arg"
    );
    assert_eq!(
        args["trigger"]["partition"],
        json!("acme"),
        "subscriber gets trigger.partition"
    );

    Ok(())
}

/// Stage D: an AND-join subscriber (`// trigger all`) with two
/// partition-bearing inputs must NOT dispatch until both inputs have
/// arrived for the *same* partition; then it fires exactly once. Slots
/// are per-partition and cleared on fire (re-accumulate, no double-fire).
#[sqlx::test(fixtures("base"))]
async fn and_join_waits_for_all_partition_inputs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    const PROD_A: &str = "u/test-user/prod-a";
    const PROD_B: &str = "u/test-user/prod-b";
    const SUB_J: &str = "u/test-user/sub-join";

    seed_script(&db, SUB_J, "echo join-subscriber", "bash").await?;
    // Two partition-bearing producers, one input each (literal token form).
    seed_asset_write(&db, PROD_A, "s3object", "lake/{partition}/a").await?;
    seed_asset_write(&db, PROD_B, "s3object", "lake/{partition}/b").await?;
    seed_subscription_and(&db, SUB_J, "s3://lake/{partition}/a").await?;
    seed_subscription_and(&db, SUB_J, "s3://lake/{partition}/b").await?;

    // Input A for partition "acme" → slot 1/2, must NOT dispatch.
    let a_acme = seed_producer_job_path(&db, PROD_A, json!({ "partition": "acme" })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(a_acme, PROD_A)).await;
    assert!(
        r.dispatched.is_empty(),
        "AND join must wait: only 1 of 2 inputs present"
    );
    assert!(
        fetch_dispatched(&db).await?.is_empty(),
        "no subscriber job pushed yet"
    );

    // A different partition for input B must open its OWN slot, not
    // complete acme's.
    let b_globex = seed_producer_job_path(&db, PROD_B, json!({ "partition": "globex" })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(b_globex, PROD_B)).await;
    assert!(
        r.dispatched.is_empty(),
        "different partition opens a separate slot, does not complete acme"
    );

    // Input B for "acme" → acme slot now 2/2 → dispatch exactly once.
    let b_acme = seed_producer_job_path(&db, PROD_B, json!({ "partition": "acme" })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(b_acme, PROD_B)).await;
    assert_eq!(r.dispatched.len(), 1, "AND join fires once both inputs in");

    let rows = fetch_dispatched(&db).await?;
    assert_eq!(rows.len(), 1);
    let sub = &rows[0];
    assert_eq!(sub.0, SUB_J);
    let args = sub.2.as_ref().unwrap();
    assert_eq!(args["partition"], json!("acme"));
    assert_eq!(args["trigger"]["partition"], json!("acme"));

    // Slot cleared on fire: re-arrival of A/acme alone is 1/2 again, no
    // double-fire.
    clear_dispatched(&db).await?;
    let a_acme2 = seed_producer_job_path(&db, PROD_A, json!({ "partition": "acme" })).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(a_acme2, PROD_A)).await;
    assert!(
        r.dispatched.is_empty(),
        "slot was cleared on fire; single input must not re-fire"
    );

    Ok(())
}

/// Stage E3: a subscriber whose edge has a debounce window gets real
/// DebouncingSettings (delay + a (subscriber, partition) key) on the
/// dispatched job; an undebounced subscriber on the same asset gets none
/// (fan-out, unchanged). Asserts the wiring fetch→push→payload→handle;
/// the actual window-collapse is the queue subsystem's own concern.
#[sqlx::test(fixtures("base"))]
async fn debounce_setting_applied_to_dispatched_subscriber(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    seed_script(&db, SUB_S3, "echo debounced", "bash").await?;
    seed_script(&db, SUB_RES, "echo plain", "bash").await?;
    seed_asset_write(&db, PRODUCER, "s3object", "f/blob").await?;
    seed_subscription_debounced(&db, SUB_S3, "s3://f/blob", 30).await?;
    seed_subscription(&db, SUB_RES, "script", "s3://f/blob").await?;

    let id = seed_producer_job(&db, json!({})).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(id, PRODUCER)).await;
    assert_eq!(r.dispatched.len(), 2, "both subscribers dispatched");

    // Resolve the persisted debounce window straight from this test's own
    // (isolated) DB by walking the handle chain
    // v2_job_queue.runnable_settings_handle → runnable_settings.debouncing_settings
    // → debouncing_settings. Reading the rows directly rather than through
    // `prefetch_cached_from_handle` keeps the assertion off the process-global
    // runnable-settings cache (and its tempdir-backed file I/O), which is
    // shared by every test running concurrently in this binary — a needless
    // cross-test coupling for what is purely a "was the handle wired through to
    // the queued job" check. An undebounced subscriber has a NULL handle, so
    // the inner joins yield no row → (None, None).
    async fn debounce_of(
        db: &Pool<Postgres>,
        path: &str,
    ) -> anyhow::Result<(Option<i32>, Option<String>)> {
        use sqlx::Row;
        let row = sqlx::query(
            r#"SELECT ds.debounce_delay_s, ds.debounce_key
                 FROM v2_job j
                 JOIN v2_job_queue q ON q.id = j.id
                 JOIN runnable_settings rs ON rs.hash = q.runnable_settings_handle
                 JOIN debouncing_settings ds ON ds.hash = rs.debouncing_settings
                WHERE j.workspace_id = $1 AND j.runnable_path = $2
                  AND j.trigger_kind = 'asset'"#,
        )
        .bind(WS)
        .bind(path)
        .fetch_optional(db)
        .await?;
        Ok(match row {
            Some(r) => (
                r.try_get::<Option<i32>, _>("debounce_delay_s")?,
                r.try_get::<Option<String>, _>("debounce_key")?,
            ),
            None => (None, None),
        })
    }

    let (deb_delay, deb_key) = debounce_of(&db, SUB_S3).await?;
    assert_eq!(deb_delay, Some(30), "debounced edge → 30s window");
    assert!(
        deb_key
            .as_deref()
            .is_some_and(|k| k.starts_with("asset-cascade:")),
        "debounce key is scoped to the (subscriber, partition) cascade slot, got {deb_key:?}"
    );

    let (plain_delay, _) = debounce_of(&db, SUB_RES).await?;
    assert_eq!(
        plain_delay, None,
        "undebounced edge → no debounce (fan-out)"
    );

    Ok(())
}

/// `// retry <n> [<delay>]` opts the subscriber into native retry: the
/// dispatcher pushes a real `JobKind::Script` (not a one-step flow) carrying
/// the policy in its `runnable_settings_handle`, so a failed subscriber re-runs
/// natively and stays eligible to trigger its own downstream. Subscribers
/// without retry are pushed as a plain `Script` with no settings handle.
#[sqlx::test(fixtures("base"))]
async fn retry_setting_dispatches_subscriber_as_native_script(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    seed_script(&db, SUB_S3, "echo retrying", "bash").await?;
    seed_script(&db, SUB_RES, "echo plain", "bash").await?;
    seed_asset_write(&db, PRODUCER, "s3object", "f/blob").await?;
    // Retry policy on the s3 edge; res edge stays vanilla so we also assert the
    // "no retry" path carries no settings handle.
    seed_subscription_with_retry(&db, SUB_S3, "s3://f/blob", 3, 5).await?;
    seed_subscription(&db, SUB_RES, "script", "s3://f/blob").await?;

    let id = seed_producer_job(&db, json!({})).await?;
    let r = dispatch_asset_triggers(&db, &make_mini(id, PRODUCER)).await;
    assert_eq!(r.dispatched.len(), 2, "both subscribers dispatched");

    let rows: Vec<(String, String, Option<i64>)> = sqlx::query!(
        r#"SELECT j.runnable_path AS "runnable_path!", j.kind::text AS "kind!",
                  q.runnable_settings_handle
             FROM v2_job j JOIN v2_job_queue q ON q.id = j.id
             WHERE j.workspace_id = $1 AND j.trigger_kind = 'asset'
             ORDER BY j.runnable_path"#,
        WS,
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| (r.runnable_path, r.kind, r.runnable_settings_handle))
    .collect();

    let s3 = rows
        .iter()
        .find(|(p, _, _)| p == SUB_S3)
        .expect("s3 dispatched");
    let res = rows
        .iter()
        .find(|(p, _, _)| p == SUB_RES)
        .expect("res dispatched");

    // Native retry: a real Script (not a SingleStepFlow), with the policy in the
    // runnable_settings_handle.
    assert_eq!(
        s3.1, "script",
        "retry subscriber dispatched as a native Script"
    );
    assert!(
        s3.2.is_some(),
        "retry subscriber carries a runnable_settings_handle (the retry policy)"
    );
    assert_eq!(
        res.1, "script",
        "no-retry subscriber stays a plain ScriptHash push"
    );
    assert!(
        res.2.is_none(),
        "no-retry subscriber carries no settings handle"
    );

    Ok(())
}

/// Regression for the AND-join check-then-act race: when a subscriber's
/// last partition-bearing inputs complete concurrently (different workers
/// finishing different upstream producers at once), the barrier must
/// still fire the subscriber exactly once for the partition. Fires all N
/// producers' dispatch simultaneously (a barrier releases them together)
/// and asserts a single dispatch and a cleared slot. This invariant holds
/// for the transactional, advisory-locked gate regardless of interleaving;
/// a regression to a non-atomic check-then-act fails it.
#[sqlx::test(fixtures("base"))]
async fn and_join_fires_once_under_concurrent_completion(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    const SUB_J: &str = "u/test-user/sub-join-conc";
    const N: usize = 5;

    seed_script(&db, SUB_J, "echo join", "bash").await?;
    let mut producers = Vec::new();
    for i in 0..N {
        let prod = format!("u/test-user/prod-conc-{i}");
        seed_asset_write(&db, &prod, "s3object", &format!("lake/{{partition}}/i{i}")).await?;
        seed_subscription_and(&db, SUB_J, &format!("s3://lake/{{partition}}/i{i}")).await?;
        let id = seed_producer_job_path(&db, &prod, json!({ "partition": "acme" })).await?;
        producers.push((id, prod));
    }

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(N));
    let mut set = tokio::task::JoinSet::new();
    for (id, prod) in producers {
        let db = db.clone();
        let barrier = barrier.clone();
        set.spawn(async move {
            barrier.wait().await;
            dispatch_asset_triggers(&db, &make_mini(id, &prod))
                .await
                .dispatched
                .len()
        });
    }
    let mut total = 0usize;
    while let Some(r) = set.join_next().await {
        total += r?;
    }

    assert_eq!(
        total, 1,
        "AND join must dispatch the subscriber exactly once under concurrent completion"
    );
    let fires = fetch_dispatched(&db)
        .await?
        .iter()
        .filter(|r| r.0 == SUB_J)
        .count();
    assert_eq!(fires, 1, "exactly one subscriber job pushed");

    let leftover = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
           FROM join_pending_inputs
           WHERE workspace_id = $1 AND subscriber_path = $2"#,
        WS,
        SUB_J,
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(leftover, 0, "join slot cleared after fire");

    Ok(())
}

/// Fuller pipeline: a partitioned chain that fans in through an AND-join
/// and then fans out over several more hops. Asserts the resolved
/// partition propagates unchanged at every hop, the chain depth
/// increments per hop, the AND barrier fires once, and a different
/// partition opens an independent slot (no cross-partition bleed) across
/// the whole multi-hop graph.
///
/// Shape: A,B (partitioned producers) ─┐
///                                     ├─▶ J (// trigger all) ─▶ C ─▶ D
///        A,B ─────────────────────────┘
#[sqlx::test(fixtures("base"))]
async fn fuller_partitioned_join_multihop_pipeline(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    const PA: &str = "u/test-user/p-a";
    const PB: &str = "u/test-user/p-b";
    const JN: &str = "u/test-user/p-join";
    const CN: &str = "u/test-user/p-c";
    const DN: &str = "u/test-user/p-d";

    for p in [JN, CN, DN] {
        seed_script(&db, p, "echo step", "bash").await?;
    }
    seed_asset_write(&db, PA, "s3object", "lake/{partition}/a").await?;
    seed_asset_write(&db, PB, "s3object", "lake/{partition}/b").await?;
    seed_asset_write(&db, JN, "s3object", "lake/{partition}/j").await?;
    seed_asset_write(&db, CN, "s3object", "lake/{partition}/c").await?;
    // J is an AND join over both partition-bearing inputs.
    seed_subscription_and(&db, JN, "s3://lake/{partition}/a").await?;
    seed_subscription_and(&db, JN, "s3://lake/{partition}/b").await?;
    seed_subscription(&db, CN, "script", "s3://lake/{partition}/j").await?;
    seed_subscription(&db, DN, "script", "s3://lake/{partition}/c").await?;

    // Helper: assert exactly one dispatch to `path` carrying partition
    // `part`, with a cascade lineage of `chain_len` producer paths.
    async fn assert_hop(
        db: &Pool<Postgres>,
        path: &str,
        part: &str,
        chain_len: usize,
    ) -> anyhow::Result<()> {
        let rows = fetch_dispatched(db).await?;
        let hits: Vec<_> = rows.iter().filter(|r| r.0 == path).collect();
        assert_eq!(hits.len(), 1, "expected exactly one dispatch to {path}");
        let args = hits[0].2.as_ref().unwrap();
        assert_eq!(args["partition"], json!(part), "{path} top-level partition");
        assert_eq!(
            args["trigger"]["partition"],
            json!(part),
            "{path} trigger.partition"
        );
        assert_eq!(
            args["trigger"]["chain"].as_array().map(|c| c.len()),
            Some(chain_len),
            "{path} lineage length"
        );
        Ok(())
    }

    // day1: A arrives → J waits (1/2 partition-bearing inputs).
    let pa = seed_producer_job_path(
        &db,
        PA,
        json!({ "partition": "day1", "trigger": { "chain": ["s0"] } }),
    )
    .await?;
    let r = dispatch_asset_triggers(&db, &make_mini(pa, PA)).await;
    assert!(r.dispatched.is_empty(), "J must wait: only A present");
    assert!(fetch_dispatched(&db).await?.is_empty());

    // day1: B arrives → J fires once for day1 at depth 2.
    let pb = seed_producer_job_path(
        &db,
        PB,
        json!({ "partition": "day1", "trigger": { "chain": ["s0"] } }),
    )
    .await?;
    let r = dispatch_asset_triggers(&db, &make_mini(pb, PB)).await;
    assert_eq!(r.dispatched.len(), 1, "J fires once when both inputs in");
    assert_hop(&db, JN, "day1", 2).await?;
    clear_dispatched(&db).await?;

    // J completes for day1 → C runs for day1 at depth 3.
    let jn = seed_producer_job_path(
        &db,
        JN,
        json!({ "partition": "day1", "trigger": { "chain": ["s0", PB] } }),
    )
    .await?;
    let r = dispatch_asset_triggers(&db, &make_mini(jn, JN)).await;
    assert_eq!(r.dispatched.len(), 1);
    assert_hop(&db, CN, "day1", 3).await?;
    clear_dispatched(&db).await?;

    // C completes for day1 → D (leaf) runs for day1 at depth 4.
    let cn = seed_producer_job_path(
        &db,
        CN,
        json!({ "partition": "day1", "trigger": { "chain": ["s0", PB, JN] } }),
    )
    .await?;
    let r = dispatch_asset_triggers(&db, &make_mini(cn, CN)).await;
    assert_eq!(r.dispatched.len(), 1);
    assert_hop(&db, DN, "day1", 4).await?;
    clear_dispatched(&db).await?;

    // A different partition opens an independent J slot — no bleed from
    // the completed day1 run.
    let pa2 = seed_producer_job_path(
        &db,
        PA,
        json!({ "partition": "day2", "trigger": { "chain": ["s0"] } }),
    )
    .await?;
    let r = dispatch_asset_triggers(&db, &make_mini(pa2, PA)).await;
    assert!(
        r.dispatched.is_empty(),
        "day2 is a separate slot; J must not fire from day1's completion"
    );

    Ok(())
}

/// The TTL reaper deletes abandoned AND-join slots, but keyed on the
/// slot's MOST RECENT row: a slot still receiving input (newest row
/// fresh) is never reaped even if it also has rows older than the TTL.
/// This per-slot (not per-row) property is the correctness point — it
/// prevents corrupting a join whose inputs trickle in slowly.
#[sqlx::test(fixtures("base"))]
async fn reaper_clears_only_stale_join_slots(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Seed a join_pending_inputs row with an explicit age (days old).
    async fn seed_slot_row(
        db: &Pool<Postgres>,
        sub: &str,
        part: &str,
        tref: &str,
        age_days: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO join_pending_inputs
                 (workspace_id, subscriber_path, partition, trigger_ref, received_at)
               VALUES ($1, $2, $3, $4, now() - ($5::bigint::text || ' d')::interval)"#,
        )
        .bind(WS)
        .bind(sub)
        .bind(part)
        .bind(tref)
        .bind(age_days)
        .execute(db)
        .await?;
        Ok(())
    }
    async fn slot_count(db: &Pool<Postgres>, sub: &str) -> anyhow::Result<i64> {
        Ok(sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM join_pending_inputs
               WHERE workspace_id = $1 AND subscriber_path = $2"#,
            WS,
            sub,
        )
        .fetch_one(db)
        .await?)
    }

    // Stale: every row older than the 60d TTL → reaped.
    seed_slot_row(
        &db,
        "u/test-user/sub-stale",
        "p1",
        "s3://x/{partition}/a",
        61,
    )
    .await?;
    seed_slot_row(
        &db,
        "u/test-user/sub-stale",
        "p1",
        "s3://x/{partition}/b",
        90,
    )
    .await?;
    // Fresh: recent → kept.
    seed_slot_row(
        &db,
        "u/test-user/sub-fresh",
        "p1",
        "s3://y/{partition}/a",
        0,
    )
    .await?;
    // Mixed: one ancient row + one fresh row in the SAME slot. max(received_at)
    // is fresh, so the whole slot must be kept (the correctness property).
    seed_slot_row(
        &db,
        "u/test-user/sub-mixed",
        "p1",
        "s3://z/{partition}/a",
        120,
    )
    .await?;
    seed_slot_row(
        &db,
        "u/test-user/sub-mixed",
        "p1",
        "s3://z/{partition}/b",
        0,
    )
    .await?;

    reap_stale_join_slots(&db).await?;

    assert_eq!(
        slot_count(&db, "u/test-user/sub-stale").await?,
        0,
        "stale slot reaped"
    );
    assert_eq!(
        slot_count(&db, "u/test-user/sub-fresh").await?,
        1,
        "fresh slot kept"
    );
    assert_eq!(
        slot_count(&db, "u/test-user/sub-mixed").await?,
        2,
        "slot with a recent row must be kept entirely (per-slot, not per-row)"
    );

    Ok(())
}
