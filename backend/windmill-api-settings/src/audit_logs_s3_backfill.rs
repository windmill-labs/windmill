#![cfg(feature = "parquet")]
//! Opt-in historical backfill of audit logs to the instance object store.
//!
//! The steady-state exporter (the EE `export_audit_logs_to_object_store`) cursors
//! on transaction xmin and, by design, only exports rows committed *after* the
//! feature was enabled — it never rescans history (an `age(xmin)` predicate is
//! unindexable, so scanning the whole partitioned table can't survive a
//! `statement_timeout`). This module covers the complementary need: exporting a
//! chosen historical `[from, to)` window (e.g. the gap left while the export was
//! disabled) on demand.
//!
//! It is safe to run on a large table because it scans strictly by `timestamp`
//! (the partition key — pruned and indexed) in bounded keyset pages, so every
//! query touches at most one page worth of rows and survives a statement timeout.
//! It does not touch the xmin cursor / checkpoint at all.
//!
//! Objects are written next to the steady-state ones under `logs/audit/dt=<day>/`
//! as `audit_backfill_<window>_<min_id>.ndjson`, with the exact same row shape, so
//! a consumer reads them uniformly. The key includes the requested window so two
//! different backfill ranges never overwrite each other (a per-page `min_id` alone
//! is not unique across windows). Re-running the *same* window is deterministic
//! (audit history is append-only), so it overwrites the same objects rather than
//! duplicating. A window that overlaps already-exported steady-state rows simply
//! re-emits them under a different key; consumers dedupe by `id`.
//!
//! Scope: like the steady-state export, this reads only `audit_partitioned`. The
//! pre-partitioning `audit` table is intentionally not exported; a window that
//! overlaps any legacy `audit` row is rejected (see [`try_start`]) so a backfill
//! never silently reports success while omitting them.
//!
//! Progress is persisted in `background_task_state` (name [`TASK_NAME`]) so any
//! API replica can serve the status endpoint, mirroring `log_cleanup`.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::background_task;
use windmill_common::error::{self};
use windmill_common::tracing_init::LOGS_AUDIT;
use windmill_common::{DB, INSTANCE_NAME};

use windmill_object_store::object_store_reexports::{ObjectStore, Path as ObjectPath};

pub const TASK_NAME: &str = "audit_logs_s3_backfill";

/// Rows fetched per keyset page. Bounds each query so it stays well under any
/// `statement_timeout` even on a busy partition, and bounds peak memory (one
/// page of ndjson is buffered before the day-grouped PUTs).
const PAGE_ROWS: i64 = 10_000;

/// Test-only override for [`PAGE_ROWS`] (0 = use the default), so a test can force
/// multi-page / page-spanning-day keyset behaviour with only a handful of rows.
#[cfg(test)]
static PAGE_ROWS_OVERRIDE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn page_rows() -> i64 {
    #[cfg(test)]
    {
        match PAGE_ROWS_OVERRIDE.load(std::sync::atomic::Ordering::Relaxed) {
            0 => PAGE_ROWS,
            n => n,
        }
    }
    #[cfg(not(test))]
    {
        PAGE_ROWS
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuditBackfillProgress {
    pub running: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Human-readable description of the current phase.
    pub phase: String,
    /// Inclusive lower / exclusive upper bound of the window being exported.
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// Audit rows written to object storage so far.
    pub rows_written: u64,
    /// Object PUTs issued so far (one per day per page).
    pub objects_written: u64,
    /// Keyset cursor: the timestamp of the last row exported (how far the
    /// backfill has progressed through the window).
    pub last_ts: Option<DateTime<Utc>>,
    pub errors: u64,
    pub last_error: Option<String>,
}

impl AuditBackfillProgress {
    fn new_running(from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        Self {
            running: true,
            started_at: Utc::now(),
            finished_at: None,
            phase: "starting".to_string(),
            from,
            to,
            rows_written: 0,
            objects_written: 0,
            last_ts: None,
            errors: 0,
            last_error: None,
        }
    }
}

struct Session {
    db: DB,
    owner: String,
    progress: RwLock<AuditBackfillProgress>,
}

impl Session {
    async fn update<F: FnOnce(&mut AuditBackfillProgress)>(&self, f: F) {
        let snapshot = {
            let mut p = self.progress.write().await;
            f(&mut p);
            p.clone()
        };
        if let Err(e) =
            background_task::update_state(&self.db, TASK_NAME, &self.owner, &snapshot).await
        {
            tracing::warn!("audit backfill: failed to persist progress: {e:#}");
        }
    }

    async fn record_error(&self, msg: String) {
        tracing::error!("audit backfill: {msg}");
        self.update(|p| {
            p.errors = p.errors.saturating_add(1);
            p.last_error = Some(msg);
        })
        .await;
    }

    async fn release(&self) {
        let snapshot = {
            let mut p = self.progress.write().await;
            p.running = false;
            p.finished_at = Some(Utc::now());
            p.phase = "done".to_string();
            p.clone()
        };
        tracing::info!(
            "audit backfill finished: {} row(s) in {} object(s) for [{}, {}), {} error(s)",
            snapshot.rows_written,
            snapshot.objects_written,
            snapshot.from,
            snapshot.to,
            snapshot.errors
        );
        if let Err(e) = background_task::release(&self.db, TASK_NAME, &self.owner, &snapshot).await
        {
            tracing::warn!("audit backfill: failed to release lease: {e:#}");
        }
    }
}

#[derive(Deserialize)]
pub struct BackfillRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// Atomically claim the backfill lease, or error if one is already running.
pub async fn try_start(db: &DB, from: DateTime<Utc>, to: DateTime<Utc>) -> error::Result<()> {
    if from >= to {
        return Err(error::Error::BadRequest(
            "audit backfill: `from` must be strictly before `to`".to_string(),
        ));
    }
    // The backfill keyset-pages by `(timestamp, id)` over rows visible at scan time and
    // declares the window fully exported once the scan runs dry. But a row's `timestamp`
    // is its inserting transaction's `xact_start`, so a transaction that started inside
    // `[from, to)` yet commits after the scan has passed that timestamp — or any row
    // committed when `to` is in the future — would be silently omitted. Require `to` to
    // be at or before the oldest in-flight `xact_start`: everything strictly older than
    // the oldest running transaction is already committed and stable.
    //
    // That bound is only sound when we can see every xmin-holding transaction. A role
    // without pg_read_all_stats/superuser sees only its own sessions, and a prepared
    // (2PC) transaction is invisible to pg_stat_activity — in either case an old
    // transaction could still commit rows inside an accepted window after our scan ends.
    // Since a backfill asserts completeness, we REJECT in those cases (the function
    // returns NULL). (The continuous exporter, which only claims bounded lag, keeps the
    // 7-day fallback instead.) The probe lives in the `audit_logs_s3_oldest_inflight_ts()`
    // SQL function (migration 20260626132251) so its `pg_has_role`/`pg_authid` read is
    // wrapped in a subtransaction EXCEPTION: managed providers (e.g. Cloud SQL) forbid
    // reading pg_authid from an elevated context, which would otherwise surface here as
    // an opaque error instead of NULL → the actionable rejection below.
    let settled_cutoff: Option<DateTime<Utc>> =
        sqlx::query_scalar!(r#"SELECT audit_logs_s3_oldest_inflight_ts() AS "cutoff?""#)
            .fetch_one(db)
            .await?;
    let Some(settled_cutoff) = settled_cutoff else {
        return Err(error::Error::BadRequest(
            "audit backfill: cannot determine a trustworthy settled-time boundary, so completeness \
             can't be guaranteed. The windmill DB role needs pg_read_all_stats (or superuser) and \
             there must be no prepared (2PC) transactions in progress — otherwise an old or \
             invisible transaction could later commit audit rows inside the requested window and \
             the backfill would miss them. Grant the privilege / resolve prepared transactions and \
             retry."
                .to_string(),
        ));
    };
    if to > settled_cutoff {
        return Err(error::Error::BadRequest(format!(
            "audit backfill: `to` ({to}) must be at or before {settled_cutoff}, the newest point \
             guaranteed settled (the oldest in-flight transaction's start); choose an earlier \
             upper bound."
        )));
    }
    // The backfill (like the steady-state export) reads only `audit_partitioned`. Audit
    // history from before partitioning was introduced lives in the legacy `audit` table
    // and is intentionally not exported. If the requested window overlaps any legacy row,
    // reject — otherwise a "completed" backfill would silently omit them. Checking the
    // legacy table directly (rather than min(audit_partitioned)) also covers an upgraded
    // instance whose `audit_partitioned` is still empty, where a min() guard would no-op.
    // Non-macro query: no compile-time-checked entry needed.
    let overlaps_legacy: bool = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM audit WHERE timestamp >= $1 AND timestamp < $2)",
    )
    .bind(from)
    .bind(to)
    .fetch_one(db)
    .await?;
    if overlaps_legacy {
        return Err(error::Error::BadRequest(
            "audit backfill: the requested window overlaps rows in the legacy (pre-partitioning) \
             `audit` table, which is not exported to object storage. Restrict the window to the \
             partitioned era (after audit-log partitioning was introduced)."
                .to_string(),
        ));
    }
    let claimed = background_task::try_claim(
        db,
        TASK_NAME,
        &*INSTANCE_NAME,
        &AuditBackfillProgress::new_running(from, to),
    )
    .await?;
    if !claimed {
        return Err(error::Error::BadRequest(
            "An audit log backfill is already running".to_string(),
        ));
    }
    Ok(())
}

/// Fetch the current backfill status. Any API server can call this.
pub async fn get_status(db: &DB) -> error::Result<Option<AuditBackfillProgress>> {
    let Some(r) = background_task::get(db, TASK_NAME).await? else {
        return Ok(None);
    };
    match serde_json::from_value::<AuditBackfillProgress>(r.value) {
        Ok(mut p) => {
            // get() collapses `running` to false when the heartbeat is stale.
            p.running = r.running;
            Ok(Some(p))
        }
        Err(e) => Err(error::Error::internal_err(format!(
            "deserialize audit backfill progress: {e:#}"
        ))),
    }
}

pub fn spawn_backfill(db: DB, from: DateTime<Utc>, to: DateTime<Utc>) {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    tokio::spawn(async move {
        let session = Arc::new(Session {
            db: db.clone(),
            owner: INSTANCE_NAME.clone(),
            progress: RwLock::new(AuditBackfillProgress::new_running(from, to)),
        });

        let s = session.clone();
        let task = async move {
            let store = match windmill_object_store::get_object_store().await {
                Some(st) => st,
                None => {
                    s.record_error("Object storage is not configured".to_string())
                        .await;
                    return;
                }
            };
            if let Err(e) = run_backfill(&s, &db, &store, from, to).await {
                s.record_error(format!("backfill failed: {e:#}")).await;
            }
        };

        // catch_unwind so a panic can't leave the lease held forever.
        if let Err(panic) = AssertUnwindSafe(task).catch_unwind().await {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            session
                .record_error(format!("backfill task panicked: {msg}"))
                .await;
        }

        session.release().await;
    });
}

/// Export `[from, to)` in keyset pages ordered by `(timestamp, id)`. Each page is
/// a bounded, partition-pruned scan; rows are grouped by UTC day and written one
/// object per day per page.
async fn run_backfill(
    session: &Session,
    db: &DB,
    store: &Arc<dyn ObjectStore>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> error::Result<()> {
    session.update(|p| p.phase = "exporting".to_string()).await;

    // Keyset cursor over (timestamp, id). `id` starts below any real value so the
    // first page includes rows at exactly `from`.
    let mut cursor_ts = from;
    let mut cursor_id: i64 = -1;
    let page_rows = page_rows();
    // Namespace object keys by the requested window. The per-page `min_id` alone is not
    // unique across runs: a narrower, overlapping backfill can start a day's page at the
    // same first row (same `min_id`) but contain fewer rows, and `put` would overwrite a
    // broader run's object — silently dropping the rows only that object held. Including
    // the window makes different ranges write disjoint keys (same window re-runs stay
    // idempotent); consumers already dedupe overlapping rows by `id`.
    let window_key = format!("{}_{}", from.timestamp_millis(), to.timestamp_millis());

    loop {
        let rows = sqlx::query!(
            r#"SELECT to_char(timestamp AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS "day!",
                      id AS "id!",
                      timestamp AS "ts!",
                      row_to_json(r)::text AS "line!"
               FROM (
                   SELECT workspace_id, id, timestamp, username, operation,
                          action_kind::text AS action_kind, resource, parameters, email, span
                   FROM audit_partitioned
                   WHERE timestamp >= $1 AND timestamp < $2
                     AND (timestamp, id) > ($3, $4)
                   ORDER BY timestamp, id
                   LIMIT $5
               ) r
               ORDER BY timestamp, id"#,
            from,
            to,
            cursor_ts,
            cursor_id,
            page_rows
        )
        .fetch_all(db)
        .await?;

        if rows.is_empty() {
            break;
        }

        // Group this page's ndjson lines by day, preserving (timestamp, id) order,
        // and track the min id per day for a deterministic, collision-free key.
        let mut by_day: Vec<(String, i64, String)> = Vec::new(); // (day, min_id, ndjson)
        for row in &rows {
            match by_day.last_mut() {
                Some((day, _min_id, acc)) if *day == row.day => {
                    acc.push('\n');
                    acc.push_str(&row.line);
                }
                _ => by_day.push((row.day.clone(), row.id, row.line.clone())),
            }
        }

        for (day, min_id, ndjson) in &by_day {
            let object_path = ObjectPath::from(format!(
                "{LOGS_AUDIT}dt={day}/audit_backfill_{window_key}_{min_id}.ndjson"
            ));
            store
                .put(&object_path, ndjson.clone().into_bytes().into())
                .await
                .map_err(|e| error::Error::internal_err(format!("upload {object_path}: {e:#}")))?;
            let n = ndjson.lines().count() as u64;
            // Persist progress (and refresh the lease heartbeat) after every object,
            // not just once the page completes: a stale heartbeat lets another replica
            // re-claim the lease and run a concurrent backfill, so the gap between
            // heartbeats must stay well under STALE_HEARTBEAT_SECS even if a page's
            // uploads are slow.
            session
                .update(|p| {
                    p.rows_written = p.rows_written.saturating_add(n);
                    p.objects_written = p.objects_written.saturating_add(1);
                })
                .await;
        }

        // Advance the keyset cursor past the last row of this page.
        let last = rows.last().expect("page is non-empty");
        cursor_ts = last.ts;
        cursor_id = last.id;

        let new_last_ts = last.ts;
        session.update(|p| p.last_ts = Some(new_last_ts)).await;

        // A short page means the window is exhausted.
        if (rows.len() as i64) < page_rows {
            break;
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "parquet"))]
mod tests {
    use super::*;
    use futures::stream::StreamExt;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use windmill_object_store::object_store_reexports::{InMemory, ObjectStore, Path as OsPath};

    /// A private, per-test object store. `run_backfill` takes the store as a parameter,
    /// so tests use a local one and never touch the process-global `OBJECT_STORE_SETTINGS`
    /// (which would otherwise race across the parallel test runner).
    fn local_store() -> (Arc<InMemory>, Arc<dyn ObjectStore>) {
        let store = Arc::new(InMemory::new());
        let dynstore: Arc<dyn ObjectStore> = store.clone();
        (store, dynstore)
    }

    /// Serializes the tests that touch the `PAGE_ROWS_OVERRIDE` process global (read
    /// inside `run_backfill`) so they can't observe each other's value under the parallel
    /// runner.
    static PAGE_OVERRIDE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// Resets `PAGE_ROWS_OVERRIDE` on drop so a failing assertion can't leak a non-default
    /// page size into another test.
    struct ResetPageOverride;
    impl Drop for ResetPageOverride {
        fn drop(&mut self) {
            PAGE_ROWS_OVERRIDE.store(0, Ordering::Relaxed);
        }
    }

    /// Insert an audit row `days` days in the past (creating the daily partition if
    /// needed). The row's `timestamp` defaults to that point, landing it in the
    /// matching partition.
    async fn insert_audit_days_ago(db: &DB, operation: &str, days: i64) -> i64 {
        sqlx::query(&format!(
            "DO $$ DECLARE d date := current_date - {days}; BEGIN \
             EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF audit_partitioned \
             FOR VALUES FROM (%L) TO (%L)', 'audit_'||to_char(d,'YYYYMMDD'), d, d + 1); END $$;"
        ))
        .execute(db)
        .await
        .ok();
        sqlx::query_scalar::<_, i64>(&format!(
            "INSERT INTO audit_partitioned
                 (workspace_id, username, operation, action_kind, parameters, timestamp)
             VALUES ('test-ws','tester',$1,'create'::action_kind,'{{}}'::jsonb,
                     now() - interval '{days} days')
             RETURNING id"
        ))
        .bind(operation)
        .fetch_one(db)
        .await
        .expect("insert audit row")
    }

    /// Insert an audit row at an exact timestamp (creating the daily partition if
    /// needed), for tests that need distinct in-day timestamps.
    async fn insert_audit_at(db: &DB, operation: &str, ts: DateTime<Utc>) -> i64 {
        sqlx::query(&format!(
            "DO $$ DECLARE d date := '{}'; BEGIN \
             EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF audit_partitioned \
             FOR VALUES FROM (%L) TO (%L)', 'audit_'||to_char(d,'YYYYMMDD'), d, d + 1); END $$;",
            ts.format("%Y-%m-%d")
        ))
        .execute(db)
        .await
        .ok();
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO audit_partitioned
                 (workspace_id, username, operation, action_kind, parameters, timestamp)
             VALUES ('test-ws','tester',$1,'create'::action_kind,'{}'::jsonb,$2)
             RETURNING id",
        )
        .bind(operation)
        .bind(ts)
        .fetch_one(db)
        .await
        .expect("insert audit row")
    }

    /// All ids across every `audit_backfill_*.ndjson` object, and the set of object
    /// paths (to assert pagination/day keying).
    async fn backfilled(store: &InMemory) -> (Vec<i64>, Vec<String>) {
        let prefix = OsPath::from("logs/audit");
        let metas = store
            .list(Some(&prefix))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|m| m.expect("list object"))
            .collect::<Vec<_>>();
        let mut ids = Vec::new();
        let mut paths = Vec::new();
        for meta in metas {
            paths.push(meta.location.to_string());
            let bytes = store
                .get(&meta.location)
                .await
                .expect("get object")
                .bytes()
                .await
                .expect("read bytes");
            for line in std::str::from_utf8(&bytes).unwrap().lines() {
                if line.is_empty() {
                    continue;
                }
                let v: serde_json::Value = serde_json::from_str(line).expect("valid ndjson");
                ids.push(v.get("id").and_then(|x| x.as_i64()).expect("row has id"));
            }
        }
        ids.sort();
        paths.sort();
        (ids, paths)
    }

    fn session(db: &DB, from: DateTime<Utc>, to: DateTime<Utc>) -> Session {
        Session {
            db: db.clone(),
            owner: INSTANCE_NAME.clone(),
            progress: RwLock::new(AuditBackfillProgress::new_running(from, to)),
        }
    }

    // End-to-end backfill: a settled multi-day window is exported in bounded keyset
    // pages (forced to 2 rows/page) — every in-window row lands exactly once, rows
    // outside [from,to) are excluded, a day that spans a page boundary produces more
    // than one object, and re-running is idempotent (same keys overwritten, no dupes).
    #[sqlx::test(migrations = "../migrations")]
    async fn backfill_exports_window_in_pages(db: DB) -> anyhow::Result<()> {
        let _serial = PAGE_OVERRIDE_LOCK.lock().await;
        let _reset = ResetPageOverride; // restores the page override even on panic
        let (store, dyn_store) = local_store();
        // Force multi-page / page-spanning-day keyset behaviour with a handful of rows.
        PAGE_ROWS_OVERRIDE.store(2, Ordering::Relaxed);

        // In window [now-6d, now-2d): days 5, 4, 3 ago.
        let mut want = Vec::new();
        for i in 0..3 {
            want.push(insert_audit_days_ago(&db, &format!("bf.d5.{i}"), 5).await);
        }
        for i in 0..2 {
            want.push(insert_audit_days_ago(&db, &format!("bf.d4.{i}"), 4).await);
        }
        for i in 0..2 {
            want.push(insert_audit_days_ago(&db, &format!("bf.d3.{i}"), 3).await);
        }
        want.sort();
        // Out of window: before `from` and at/after `to`.
        let before = insert_audit_days_ago(&db, "bf.before", 7).await;
        let after = insert_audit_days_ago(&db, "bf.after", 1).await;

        let from = Utc::now() - chrono::Duration::days(6);
        let to = Utc::now() - chrono::Duration::days(2);

        let s = session(&db, from, to);
        run_backfill(&s, &db, &dyn_store, from, to).await?;

        let (ids, paths) = backfilled(&store).await;
        assert_eq!(ids, want, "exactly the in-window rows, each once: {ids:?}");
        assert!(
            !ids.contains(&before) && !ids.contains(&after),
            "rows outside [from,to) must not be exported"
        );
        // 3 rows on the day-5 partition at a 2-row page size => that day spans pages,
        // so it yields >1 object — proving keyset paging across a day boundary.
        let day5_objects = paths
            .iter()
            .filter(|p| p.contains("audit_backfill_"))
            .count();
        assert!(
            day5_objects >= 4,
            "expected multiple paged objects (incl. a split day), got {paths:?}"
        );
        {
            let p = s.progress.read().await;
            assert_eq!(p.rows_written, want.len() as u64, "progress row count");
        }

        // Idempotent re-run: deterministic keys are overwritten, never duplicated.
        let s2 = session(&db, from, to);
        run_backfill(&s2, &db, &dyn_store, from, to).await?;
        let (ids2, _) = backfilled(&store).await;
        assert_eq!(ids2, want, "re-run stays exactly once per row: {ids2:?}");

        Ok(())
    }

    // A narrower backfill overlapping a broader one must not overwrite (and drop rows
    // from) the broader run's object: the object key includes the window. The two
    // windows share a day and the same first row (so the same `min_id`), but the
    // narrower one holds fewer rows.
    #[sqlx::test(migrations = "../migrations")]
    async fn backfill_window_in_key_prevents_overwrite(db: DB) -> anyhow::Result<()> {
        // Hold the lock so no concurrent test's PAGE_ROWS_OVERRIDE is observed; this test
        // wants the default (large) page size so each day is one object per window.
        let _serial = PAGE_OVERRIDE_LOCK.lock().await;
        let (store, dyn_store) = local_store();

        // Four rows on the same day at distinct times.
        let base = Utc::now() - chrono::Duration::days(5);
        let r0 = insert_audit_at(&db, "ov.0", base).await;
        let r1 = insert_audit_at(&db, "ov.1", base + chrono::Duration::seconds(10)).await;
        let r2 = insert_audit_at(&db, "ov.2", base + chrono::Duration::seconds(20)).await;
        let r3 = insert_audit_at(&db, "ov.3", base + chrono::Duration::seconds(30)).await;

        // Broad run covers all four (one object for the day, keyed by r0).
        let a_from = base - chrono::Duration::seconds(1);
        let a_to = base + chrono::Duration::seconds(31);
        run_backfill(&session(&db, a_from, a_to), &db, &dyn_store, a_from, a_to).await?;

        // Narrow run starts at the same first row (same min_id) but holds only r0, r1.
        let b_from = base - chrono::Duration::seconds(1);
        let b_to = base + chrono::Duration::seconds(15);
        run_backfill(&session(&db, b_from, b_to), &db, &dyn_store, b_from, b_to).await?;

        let (ids, _) = backfilled(&store).await;
        for id in [r0, r1, r2, r3] {
            assert!(
                ids.contains(&id),
                "row {id} lost — a narrower overlapping window overwrote the broader run's \
                 object: {ids:?}"
            );
        }
        Ok(())
    }

    // The endpoint rejects a window whose upper bound is not yet settled (a row's
    // timestamp is its txn's xact_start, so a future/live `to` could miss late
    // commits), but accepts a window safely in the past.
    #[sqlx::test(migrations = "../migrations")]
    async fn backfill_rejects_unstable_window(db: DB) -> anyhow::Result<()> {
        let future = Utc::now() + chrono::Duration::days(1);
        let past_from = Utc::now() - chrono::Duration::days(2);
        let err = try_start(&db, past_from, future).await.unwrap_err();
        assert!(
            matches!(err, error::Error::BadRequest(_)),
            "a future `to` must be rejected as unstable, got {err:?}"
        );

        // A window fully in the settled past is accepted.
        let from = Utc::now() - chrono::Duration::days(3);
        let to = Utc::now() - chrono::Duration::days(2);
        try_start(&db, from, to)
            .await
            .expect("a settled past window is accepted");
        Ok(())
    }

    /// Insert a row into the legacy (non-partitioned) `audit` table at an exact time.
    async fn insert_legacy_audit_at(db: &DB, operation: &str, ts: DateTime<Utc>) {
        sqlx::query(
            "INSERT INTO audit (workspace_id, username, operation, action_kind, parameters, timestamp)
             VALUES ('test-ws','tester',$1,'create'::action_kind,'{}'::jsonb,$2)",
        )
        .bind(operation)
        .bind(ts)
        .execute(db)
        .await
        .expect("insert legacy audit row");
    }

    // A window overlapping rows in the legacy (non-partitioned) `audit` table is rejected:
    // those rows are not exported, so the backfill must not report success while silently
    // omitting them. Covers the empty-`audit_partitioned` case (a min(partitioned) guard
    // would no-op there).
    #[sqlx::test(migrations = "../migrations")]
    async fn backfill_rejects_window_overlapping_legacy(db: DB) -> anyhow::Result<()> {
        // A legacy row ~5 days ago, and no partitioned rows at all.
        insert_legacy_audit_at(&db, "legacy.row", Utc::now() - chrono::Duration::days(5)).await;

        // A window covering it is rejected.
        let from = Utc::now() - chrono::Duration::days(6);
        let to = Utc::now() - chrono::Duration::days(2);
        let err = try_start(&db, from, to).await.unwrap_err();
        assert!(
            matches!(err, error::Error::BadRequest(_)),
            "a window overlapping legacy audit rows must be rejected, got {err:?}"
        );

        // A window clear of any legacy row is accepted.
        let from_ok = Utc::now() - chrono::Duration::days(2);
        let to_ok = Utc::now() - chrono::Duration::days(1);
        try_start(&db, from_ok, to_ok)
            .await
            .expect("a window with no legacy overlap is accepted");
        Ok(())
    }
}
