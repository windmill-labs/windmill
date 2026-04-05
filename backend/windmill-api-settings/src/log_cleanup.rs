#![cfg(feature = "parquet")]
/*
 * Manual trigger for cleaning up expired log files from object storage.
 *
 * Mirrors the periodic cleanup done in backend/src/monitor.rs::delete_expired_items,
 * but runs on demand from the UI with progress reporting and uses
 * ObjectStore::delete_stream for batched S3 deletes (up to 1000 per request).
 *
 * Note: unlike the periodic cleanup (which only hits S3 when MONITOR_LOGS_ON_OBJECT_STORE
 * is enabled), this manual path ALWAYS issues S3 deletes. That is intentional: operators
 * who previously ran with the setting OFF may have orphan log files in their bucket and
 * need a way to reclaim that space. Do not add a MONITOR_LOGS_ON_OBJECT_STORE guard here
 * without first considering that use case.
 *
 * Progress state is persisted in the `background_task_state` table (see
 * windmill-common/src/background_task.rs) so that any API server replica can serve
 * the status endpoint — not just the one that happened to receive the POST.
 */

use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use windmill_common::background_task;
use windmill_common::error::{self};
use windmill_common::tracing_init::{LOGS_SERVICE, TMP_WINDMILL_LOGS_SERVICE};
use windmill_common::worker::WINDMILL_DIR;
use windmill_common::{DB, INSTANCE_NAME, JOB_RETENTION_SECS, SERVICE_LOG_RETENTION_SECS};

use windmill_object_store::object_store_reexports::{ObjectStore, Path as ObjectPath};

pub const TASK_NAME: &str = "log_cleanup";

const SERVICE_LOG_BATCH: i64 = 2_000;
const JOB_BATCH: i64 = 1_000;
/// Number of S3 paths to accumulate before issuing a batched DeleteObjects /
/// v2_job membership check during the orphan scan.
const ORPHAN_BATCH: usize = 1_000;
/// Flush orphan_scanned to the DB every N inspected objects. Per-object
/// writes would turn a TB-bucket scan into millions of DB round-trips.
const ORPHAN_SCAN_FLUSH_TICK: u64 = 1_000;

#[derive(Clone, Serialize, Deserialize)]
pub struct LogCleanupProgress {
    pub running: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Human-readable description of the current phase.
    pub phase: String,
    pub total_service: u64,
    pub processed_service: u64,
    pub total_jobs: u64,
    pub processed_jobs: u64,
    pub s3_deleted: u64,
    /// Number of S3 objects inspected during the orphan scan phase.
    pub orphans_scanned: u64,
    /// Number of orphan S3 objects deleted (no corresponding DB row).
    pub orphans_deleted: u64,
    pub errors: u64,
    pub last_error: Option<String>,
}

impl LogCleanupProgress {
    fn new_running() -> Self {
        Self {
            running: true,
            started_at: Utc::now(),
            finished_at: None,
            phase: "starting".to_string(),
            total_service: 0,
            processed_service: 0,
            total_jobs: 0,
            processed_jobs: 0,
            s3_deleted: 0,
            orphans_scanned: 0,
            orphans_deleted: 0,
            errors: 0,
            last_error: None,
        }
    }
}

/// Per-task mutable state shared across the async helpers. Holds the DB handle
/// and owner string so every mutation can be persisted atomically.
struct Session {
    db: DB,
    owner: String,
    progress: RwLock<LogCleanupProgress>,
}

impl Session {
    async fn update<F: FnOnce(&mut LogCleanupProgress)>(&self, f: F) {
        let snapshot = {
            let mut p = self.progress.write().await;
            f(&mut p);
            p.clone()
        };
        if let Err(e) =
            background_task::update_state(&self.db, TASK_NAME, &self.owner, &snapshot).await
        {
            tracing::warn!("log cleanup: failed to persist progress: {e:#}");
        }
    }

    async fn set_phase(&self, phase: &str) {
        self.update(|p| p.phase = phase.to_string()).await;
    }

    async fn record_error(&self, msg: String) {
        tracing::error!("log cleanup: {msg}");
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
        if let Err(e) = background_task::release(&self.db, TASK_NAME, &self.owner, &snapshot).await
        {
            tracing::warn!("log cleanup: failed to release lease: {e:#}");
        }
    }
}

/// Try to atomically claim the cleanup lease. Returns Ok on success, or
/// Err if another server/process already holds a fresh lease.
pub async fn try_start(db: &DB) -> error::Result<()> {
    let claimed = background_task::try_claim(
        db,
        TASK_NAME,
        &*INSTANCE_NAME,
        &LogCleanupProgress::new_running(),
    )
    .await?;
    if !claimed {
        return Err(error::Error::BadRequest(
            "Log cleanup is already running".to_string(),
        ));
    }
    Ok(())
}

/// Fetch the current status from the DB. Any API server can call this.
pub async fn get_status(db: &DB) -> error::Result<Option<LogCleanupProgress>> {
    let row = background_task::get(db, TASK_NAME).await?;
    let Some(r) = row else { return Ok(None) };
    match serde_json::from_value::<LogCleanupProgress>(r.value) {
        Ok(mut p) => {
            // background_task::get collapses `running` to false when the
            // heartbeat is stale — mirror that into the returned struct.
            p.running = r.running;
            Ok(Some(p))
        }
        Err(e) => Err(error::Error::internal_err(format!(
            "deserialize log cleanup progress: {e:#}"
        ))),
    }
}

/// Delete the given object paths from S3 in batches (uses ObjectStore::delete_stream
/// which on S3 issues a single DeleteObjects request per 1000 paths).
async fn s3_bulk_delete(
    store: &Arc<dyn ObjectStore>,
    paths: Vec<ObjectPath>,
) -> (u64 /* deleted */, u64 /* errors */) {
    let stream = futures::stream::iter(paths.into_iter().map(Ok)).boxed();
    let mut deleted = 0u64;
    let mut errors = 0u64;
    let mut res = store.delete_stream(stream);
    while let Some(r) = res.next().await {
        match r {
            Ok(_) => deleted += 1,
            Err(e) => {
                errors += 1;
                tracing::warn!("log cleanup: failed to delete object: {e:#}");
            }
        }
    }
    (deleted, errors)
}

/// Delete the given relative paths from the local filesystem under `base_dir`.
async fn disk_bulk_delete(base_dir: &str, rel_paths: &[String]) {
    let futs = rel_paths.iter().map(|p| async move {
        let full = std::path::Path::new(base_dir).join(p);
        if tokio::fs::metadata(&full).await.is_ok() {
            if let Err(e) = tokio::fs::remove_file(&full).await {
                tracing::warn!(
                    "log cleanup: failed to delete {}: {e}",
                    full.to_string_lossy()
                );
            }
        }
    });
    futures::future::join_all(futs).await;
}

async fn cleanup_service_logs(
    session: &Session,
    db: &DB,
    store: &Arc<dyn ObjectStore>,
) -> error::Result<()> {
    // Count candidates upfront for progress reporting.
    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM log_file WHERE log_ts <= now() - ($1::bigint::text || ' s')::interval",
        SERVICE_LOG_RETENTION_SECS,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    session.update(|p| p.total_service = total as u64).await;

    if total <= 0 {
        return Ok(());
    }

    struct LogFileRow {
        file_path: String,
        hostname: String,
    }

    loop {
        let rows = sqlx::query_as!(
            LogFileRow,
            "DELETE FROM log_file WHERE (file_path, hostname) IN (
                 SELECT file_path, hostname FROM log_file
                 WHERE log_ts <= now() - ($1::bigint::text || ' s')::interval
                 LIMIT $2
             ) RETURNING file_path, hostname",
            SERVICE_LOG_RETENTION_SECS,
            SERVICE_LOG_BATCH,
        )
        .fetch_all(db)
        .await?;

        if rows.is_empty() {
            break;
        }

        let rel_paths: Vec<String> = rows
            .iter()
            .map(|r| format!("{}/{}", r.hostname, r.file_path))
            .collect();
        let batch_len = rel_paths.len() as u64;

        let s3_paths: Vec<ObjectPath> = rel_paths
            .iter()
            .map(|p| ObjectPath::from(format!("{}{}", LOGS_SERVICE, p)))
            .collect();
        let (deleted, errors) = s3_bulk_delete(store, s3_paths).await;
        disk_bulk_delete(&*TMP_WINDMILL_LOGS_SERVICE, &rel_paths).await;

        session
            .update(|p| {
                p.processed_service = p.processed_service.saturating_add(batch_len);
                if p.processed_service > p.total_service {
                    p.total_service = p.processed_service;
                }
                p.s3_deleted = p.s3_deleted.saturating_add(deleted);
                p.errors = p.errors.saturating_add(errors);
            })
            .await;
    }

    // Collapse the total to what we actually processed.
    session
        .update(|p| p.total_service = p.processed_service)
        .await;

    Ok(())
}

async fn cleanup_job_logs(
    session: &Session,
    db: &DB,
    store: &Arc<dyn ObjectStore>,
) -> error::Result<()> {
    let retention_secs = *JOB_RETENTION_SECS.read().await;
    if retention_secs <= 0 {
        return Ok(());
    }

    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM v2_job_completed
         WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval",
        retention_secs,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    session.update(|p| p.total_jobs = total as u64).await;

    if total <= 0 {
        return Ok(());
    }

    loop {
        let (deleted_count, rel_paths) =
            delete_expired_jobs_batch(db, retention_secs, JOB_BATCH).await?;

        if deleted_count == 0 {
            break;
        }

        let s3_paths: Vec<ObjectPath> = rel_paths
            .iter()
            .map(|p| ObjectPath::from(p.clone()))
            .collect();
        let (deleted, errors) = s3_bulk_delete(store, s3_paths).await;
        disk_bulk_delete(&*WINDMILL_DIR, &rel_paths).await;

        session
            .update(|p| {
                p.processed_jobs = p.processed_jobs.saturating_add(deleted_count as u64);
                if p.processed_jobs > p.total_jobs {
                    p.total_jobs = p.processed_jobs;
                }
                p.s3_deleted = p.s3_deleted.saturating_add(deleted);
                p.errors = p.errors.saturating_add(errors);
            })
            .await;
    }

    // Collapse the total to what we actually processed — the upfront count
    // includes jobs whose root is still active (protected from deletion), so
    // without this the progress bar would get stuck at e.g. 3/44.
    session.update(|p| p.total_jobs = p.processed_jobs).await;

    Ok(())
}

/// Mirrors backend/src/monitor.rs::delete_expired_jobs_batch but returns the
/// log paths instead of deleting them from storage itself, so the caller can
/// issue a single batched S3 delete across many batches via delete_stream.
async fn delete_expired_jobs_batch(
    db: &DB,
    job_retention_secs: i64,
    batch_size: i64,
) -> error::Result<(usize, Vec<String>)> {
    let mut tx = db.begin().await?;

    let active_root_job_ids: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT q.id FROM v2_job_queue q
         JOIN v2_job j ON j.id = q.id
         WHERE j.parent_job IS NULL
           AND j.created_at <= now() - ($1::bigint::text || ' s')::interval",
        job_retention_secs
    )
    .fetch_all(&mut *tx)
    .await?;

    let deleted_jobs: Vec<Uuid> = sqlx::query_scalar!(
        "DELETE FROM v2_job_completed
         WHERE id IN (
             SELECT jc.id FROM v2_job_completed jc
             LEFT JOIN v2_job j ON j.id = jc.id
             WHERE jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
               AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) != ALL($3)
             ORDER BY jc.completed_at ASC
             LIMIT $2
             FOR UPDATE OF jc SKIP LOCKED
         )
         RETURNING id",
        job_retention_secs,
        batch_size,
        &active_root_job_ids
    )
    .fetch_all(&mut *tx)
    .await?;

    let deleted_count = deleted_jobs.len();
    if deleted_count == 0 {
        tx.commit().await?;
        return Ok((0, Vec::new()));
    }

    if let Err(e) = sqlx::query!(
        "DELETE FROM job_stats WHERE job_id = ANY($1)",
        &deleted_jobs
    )
    .execute(&mut *tx)
    .await
    {
        tracing::error!("log cleanup: error deleting job stats: {e:?}");
    }

    let log_paths: Vec<String> = match sqlx::query_scalar!(
        "DELETE FROM job_logs WHERE job_id = ANY($1) RETURNING log_file_index",
        &deleted_jobs
    )
    .fetch_all(&mut *tx)
    .await
    {
        Ok(log_file_index) => log_file_index
            .into_iter()
            .filter_map(|opt| opt)
            .flat_map(|inner_vec| inner_vec.into_iter())
            .collect(),
        Err(e) => {
            tracing::error!("log cleanup: error deleting job logs: {e:?}");
            Vec::new()
        }
    };

    if let Err(e) = sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted_jobs)
        .execute(&mut *tx)
        .await
    {
        tracing::error!("log cleanup: error deleting job: {e:?}");
    }

    if let Err(e) = sqlx::query!(
        "DELETE FROM job_result_stream_v2 WHERE job_id = ANY($1)",
        &deleted_jobs
    )
    .execute(&mut *tx)
    .await
    {
        tracing::error!("log cleanup: error deleting job result stream: {e:?}");
    }

    tx.commit().await?;

    Ok((deleted_count, log_paths))
}

/// Scan S3 under the `logs/` prefix for orphan log files and delete them.
///
/// An orphan is an S3 object that is older than retention and has no corresponding
/// live job / service-log row in the DB. This is how we reclaim space from:
///   - operators who previously ran with MONITOR_LOGS_ON_OBJECT_STORE off (so DB
///     cleanup skipped S3 and left files behind when jobs were deleted)
///   - previous failed batched S3 deletes that left stragglers
///
/// Job log paths look like `logs/<uuid>/<ts>_<size>.txt` — we parse the uuid and
/// protect any object whose job_id still exists in `v2_job` (both queued and
/// completed jobs, so active flows keep their logs until their root completes
/// and regular DB-driven cleanup catches them).
///
/// Service log paths look like `logs/services/<hostname>/<file>` — we cannot
/// reliably map them to `log_file` rows (file_path format is rotation-dependent)
/// so we rely on `last_modified` alone, which after the DB phase completed
/// already reflects true orphans.
async fn cleanup_s3_orphans(
    session: &Session,
    db: &DB,
    store: &Arc<dyn ObjectStore>,
) -> error::Result<()> {
    let job_retention_secs = *JOB_RETENTION_SECS.read().await;
    let now = Utc::now();
    // Service logs always have a retention (hardcoded SERVICE_LOG_RETENTION_SECS),
    // so we scan for service-log orphans regardless of JOB_RETENTION_SECS. Job-log
    // orphans, by contrast, can only be considered expired relative to
    // JOB_RETENTION_SECS; when that is disabled we skip the job branch entirely.
    let service_cutoff = now - chrono::Duration::seconds(SERVICE_LOG_RETENTION_SECS);
    let job_cutoff = if job_retention_secs > 0 {
        Some(now - chrono::Duration::seconds(job_retention_secs))
    } else {
        None
    };

    let logs_prefix = ObjectPath::from("logs/");
    let mut stream = store.list(Some(&logs_prefix));

    let mut service_batch: Vec<ObjectPath> = Vec::with_capacity(ORPHAN_BATCH);
    let mut job_batch: Vec<(ObjectPath, Uuid)> = Vec::with_capacity(ORPHAN_BATCH);
    let mut scanned_since_flush: u64 = 0;

    while let Some(item) = stream.next().await {
        let meta = match item {
            Ok(m) => m,
            Err(e) => {
                session.record_error(format!("list objects: {e:#}")).await;
                // Listing error aborts (continuation token is gone); the next
                // manual run will pick up where we left off.
                break;
            }
        };

        scanned_since_flush += 1;
        if scanned_since_flush >= ORPHAN_SCAN_FLUSH_TICK {
            let delta = scanned_since_flush;
            scanned_since_flush = 0;
            session
                .update(|p| p.orphans_scanned = p.orphans_scanned.saturating_add(delta))
                .await;
        }

        let path_str = meta.location.as_ref();
        let rest = match path_str.strip_prefix("logs/") {
            Some(r) => r,
            None => continue,
        };

        if let Some(_after_services) = rest.strip_prefix("services/") {
            if meta.last_modified < service_cutoff {
                service_batch.push(meta.location);
                if service_batch.len() >= ORPHAN_BATCH {
                    flush_service_orphans(session, store, &mut service_batch).await;
                }
            }
        } else if let Some(job_cutoff) = job_cutoff {
            // logs/<uuid>/... — parse uuid segment.
            let first_seg = rest.split('/').next().unwrap_or("");
            let job_id = match Uuid::parse_str(first_seg) {
                Ok(u) => u,
                Err(_) => continue,
            };
            if meta.last_modified < job_cutoff {
                job_batch.push((meta.location, job_id));
                if job_batch.len() >= ORPHAN_BATCH {
                    flush_job_orphans(session, db, store, &mut job_batch).await;
                }
            }
        }
    }

    // Flush remaining progress counter and any residual delete batches.
    if scanned_since_flush > 0 {
        let delta = scanned_since_flush;
        session
            .update(|p| p.orphans_scanned = p.orphans_scanned.saturating_add(delta))
            .await;
    }
    if !service_batch.is_empty() {
        flush_service_orphans(session, store, &mut service_batch).await;
    }
    if !job_batch.is_empty() {
        flush_job_orphans(session, db, store, &mut job_batch).await;
    }

    Ok(())
}

async fn flush_service_orphans(
    session: &Session,
    store: &Arc<dyn ObjectStore>,
    batch: &mut Vec<ObjectPath>,
) {
    let paths = std::mem::take(batch);
    let (deleted, errors) = s3_bulk_delete(store, paths).await;
    session
        .update(|p| {
            p.orphans_deleted = p.orphans_deleted.saturating_add(deleted);
            p.s3_deleted = p.s3_deleted.saturating_add(deleted);
            p.errors = p.errors.saturating_add(errors);
        })
        .await;
}

async fn flush_job_orphans(
    session: &Session,
    db: &DB,
    store: &Arc<dyn ObjectStore>,
    batch: &mut Vec<(ObjectPath, Uuid)>,
) {
    let taken = std::mem::take(batch);

    let ids: Vec<Uuid> = {
        let mut seen = std::collections::HashSet::with_capacity(taken.len());
        taken
            .iter()
            .filter_map(|(_, id)| if seen.insert(*id) { Some(*id) } else { None })
            .collect()
    };

    let protected: std::collections::HashSet<Uuid> =
        match sqlx::query_scalar!("SELECT id FROM v2_job WHERE id = ANY($1)", &ids)
            .fetch_all(db)
            .await
        {
            Ok(rows) => rows.into_iter().collect(),
            Err(e) => {
                session
                    .record_error(format!("orphan membership check failed: {e:#}"))
                    .await;
                return;
            }
        };

    let to_delete: Vec<ObjectPath> = taken
        .into_iter()
        .filter_map(|(path, id)| {
            if protected.contains(&id) {
                None
            } else {
                Some(path)
            }
        })
        .collect();

    if to_delete.is_empty() {
        return;
    }

    let (deleted, errors) = s3_bulk_delete(store, to_delete).await;
    session
        .update(|p| {
            p.orphans_deleted = p.orphans_deleted.saturating_add(deleted);
            p.s3_deleted = p.s3_deleted.saturating_add(deleted);
            p.errors = p.errors.saturating_add(errors);
        })
        .await;
}

/// Spawn the cleanup task. Caller is responsible for ensuring only one runs at a time
/// (use `try_start` first).
pub fn spawn_cleanup(db: DB) {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    tokio::spawn(async move {
        let session = Arc::new(Session {
            db: db.clone(),
            owner: INSTANCE_NAME.clone(),
            progress: RwLock::new(LogCleanupProgress::new_running()),
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
            s.set_phase("service logs (db)").await;
            if let Err(e) = cleanup_service_logs(&s, &db, &store).await {
                s.record_error(format!("service logs phase failed: {e:#}"))
                    .await;
            }
            s.set_phase("job logs (db)").await;
            if let Err(e) = cleanup_job_logs(&s, &db, &store).await {
                s.record_error(format!("job logs phase failed: {e:#}"))
                    .await;
            }
            s.set_phase("orphan S3 scan").await;
            if let Err(e) = cleanup_s3_orphans(&s, &db, &store).await {
                s.record_error(format!("orphan scan phase failed: {e:#}"))
                    .await;
            }
        };

        // catch_unwind so a panic inside the cleanup can't leave the lease held forever.
        if let Err(panic) = AssertUnwindSafe(task).catch_unwind().await {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            session
                .record_error(format!("cleanup task panicked: {msg}"))
                .await;
        }

        session.release().await;
    });
}
