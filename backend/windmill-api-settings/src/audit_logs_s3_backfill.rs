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
//! as `audit_backfill_<min_id>.ndjson`, with the exact same row shape, so a
//! consumer reads them uniformly. Re-running the same window is deterministic
//! (audit history is append-only), so it overwrites the same objects rather than
//! duplicating. A window that overlaps already-exported steady-state rows simply
//! re-emits them under a different key; consumers dedupe by `id`.
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
    // declares completion once the scan runs dry. But a row's `timestamp` is its
    // inserting transaction's `xact_start`, so a transaction that started inside
    // `[from, to)` yet commits after the scan has passed that timestamp — or any row
    // committed when `to` is in the future — would be silently omitted from a
    // "completed" backfill. Require `to` to be at or before the oldest in-flight
    // `xact_start`: everything strictly older than the oldest running transaction is
    // already committed and stable. Same trustworthy gating as the exporter's floor
    // (a restricted stats role or a prepared 2PC txn makes the value unsafe); fall back
    // to a 7-day-old cutoff then.
    let settled_cutoff: Option<DateTime<Utc>> = sqlx::query_scalar!(
        r#"SELECT CASE WHEN (current_setting('is_superuser') = 'on'
                              OR pg_has_role(current_user, 'pg_read_all_stats', 'USAGE'))
                          AND NOT EXISTS (SELECT 1 FROM pg_prepared_xacts)
                     THEN (SELECT min(xact_start) FROM pg_stat_activity WHERE xact_start IS NOT NULL)
                     ELSE NULL END AS "cutoff?""#
    )
    .fetch_one(db)
    .await?;
    let settled_cutoff = settled_cutoff.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    if to > settled_cutoff {
        return Err(error::Error::BadRequest(format!(
            "audit backfill: `to` ({to}) must be at or before {settled_cutoff}, the point up to \
             which audit rows are guaranteed settled; choose an earlier upper bound. (Granting \
             pg_read_all_stats to the windmill DB user tightens this cutoff from a 7-day margin to \
             the live oldest-transaction boundary.)"
        )));
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
            PAGE_ROWS
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
                "{LOGS_AUDIT}dt={day}/audit_backfill_{min_id}.ndjson"
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
        if (rows.len() as i64) < PAGE_ROWS {
            break;
        }
    }

    Ok(())
}
