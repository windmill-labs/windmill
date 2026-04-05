#![cfg(feature = "parquet")]
/*
 * Manual trigger for cleaning up expired log files from object storage.
 *
 * Mirrors the periodic cleanup done in backend/src/monitor.rs::delete_expired_items,
 * but runs on demand from the UI with progress reporting and uses
 * ObjectStore::delete_stream for batched S3 deletes (up to 1000 per request).
 */

use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use serde::Serialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use windmill_common::error::{self};
use windmill_common::tracing_init::{LOGS_SERVICE, TMP_WINDMILL_LOGS_SERVICE};
use windmill_common::worker::WINDMILL_DIR;
use windmill_common::{DB, JOB_RETENTION_SECS, SERVICE_LOG_RETENTION_SECS};

use windmill_object_store::object_store_reexports::{ObjectStore, Path as ObjectPath};

const SERVICE_LOG_BATCH: i64 = 2_000;
const JOB_BATCH: i64 = 1_000;

#[derive(Clone, Serialize)]
pub struct LogCleanupProgress {
    pub running: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub total_service: u64,
    pub processed_service: u64,
    pub total_jobs: u64,
    pub processed_jobs: u64,
    pub s3_deleted: u64,
    pub errors: u64,
    pub last_error: Option<String>,
}

lazy_static::lazy_static! {
    pub static ref LOG_CLEANUP_STATUS: Arc<RwLock<Option<LogCleanupProgress>>> =
        Arc::new(RwLock::new(None));
}

/// Attempt to mark the cleanup as running. Returns `Err` if it is already running.
pub async fn try_start() -> Result<(), &'static str> {
    let mut guard = LOG_CLEANUP_STATUS.write().await;
    if let Some(p) = guard.as_ref() {
        if p.running {
            return Err("Log cleanup is already running");
        }
    }
    *guard = Some(LogCleanupProgress {
        running: true,
        started_at: Utc::now(),
        finished_at: None,
        total_service: 0,
        processed_service: 0,
        total_jobs: 0,
        processed_jobs: 0,
        s3_deleted: 0,
        errors: 0,
        last_error: None,
    });
    Ok(())
}

async fn update<F: FnOnce(&mut LogCleanupProgress)>(f: F) {
    let mut guard = LOG_CLEANUP_STATUS.write().await;
    if let Some(p) = guard.as_mut() {
        f(p);
    }
}

async fn finish() {
    update(|p| {
        p.running = false;
        p.finished_at = Some(Utc::now());
    })
    .await;
}

async fn record_error(msg: String) {
    tracing::error!("log cleanup: {msg}");
    update(|p| {
        p.errors = p.errors.saturating_add(1);
        p.last_error = Some(msg);
    })
    .await;
}

/// Delete the given object paths from S3 in batches (uses ObjectStore::delete_stream
/// which on S3 issues a single DeleteObjects request per 1000 paths).
async fn s3_bulk_delete(
    store: &Arc<dyn ObjectStore>,
    paths: Vec<ObjectPath>,
) -> (u64 /* deleted */, u64 /* errors */) {
    if paths.is_empty() {
        return (0, 0);
    }
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

async fn cleanup_service_logs(db: &DB, store: &Arc<dyn ObjectStore>) -> error::Result<()> {
    // Count candidates upfront for progress reporting.
    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM log_file WHERE log_ts <= now() - ($1::bigint::text || ' s')::interval",
        SERVICE_LOG_RETENTION_SECS,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    update(|p| p.total_service = total as u64).await;

    if total <= 0 {
        return Ok(());
    }

    struct LogFileRow {
        file_path: String,
        hostname: String,
    }

    loop {
        // DELETE a batch of expired service log rows, returning their (file_path, hostname).
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

        // S3 delete (batched).
        let s3_paths: Vec<ObjectPath> = rel_paths
            .iter()
            .map(|p| ObjectPath::from(format!("{}{}", LOGS_SERVICE, p)))
            .collect();
        let (deleted, errors) = s3_bulk_delete(store, s3_paths).await;

        // Disk delete in parallel (best-effort).
        disk_bulk_delete(&*TMP_WINDMILL_LOGS_SERVICE, &rel_paths).await;

        update(|p| {
            p.processed_service = p.processed_service.saturating_add(batch_len);
            p.s3_deleted = p.s3_deleted.saturating_add(deleted);
            p.errors = p.errors.saturating_add(errors);
        })
        .await;
    }

    Ok(())
}

async fn cleanup_job_logs(db: &DB, store: &Arc<dyn ObjectStore>) -> error::Result<()> {
    let retention_secs = *JOB_RETENTION_SECS.read().await;
    if retention_secs <= 0 {
        return Ok(());
    }

    // Upper bound on the number of expired jobs.
    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM v2_job_completed
         WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval",
        retention_secs,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    update(|p| p.total_jobs = total as u64).await;

    if total <= 0 {
        return Ok(());
    }

    loop {
        let (deleted_count, rel_paths) =
            delete_expired_jobs_batch(db, retention_secs, JOB_BATCH).await?;

        if deleted_count == 0 {
            break;
        }

        // S3 delete (batched).
        let s3_paths: Vec<ObjectPath> = rel_paths
            .iter()
            .map(|p| ObjectPath::from(p.clone()))
            .collect();
        let (deleted, errors) = s3_bulk_delete(store, s3_paths).await;

        // Disk delete in parallel (best-effort).
        disk_bulk_delete(&*WINDMILL_DIR, &rel_paths).await;

        update(|p| {
            p.processed_jobs = p.processed_jobs.saturating_add(deleted_count as u64);
            p.s3_deleted = p.s3_deleted.saturating_add(deleted);
            p.errors = p.errors.saturating_add(errors);
        })
        .await;
    }

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

    let _ = sqlx::query!(
        "DELETE FROM job_stats WHERE job_id = ANY($1)",
        &deleted_jobs
    )
    .execute(&mut *tx)
    .await;

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

    let _ = sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted_jobs)
        .execute(&mut *tx)
        .await;

    let _ = sqlx::query!(
        "DELETE FROM job_result_stream_v2 WHERE job_id = ANY($1)",
        &deleted_jobs
    )
    .execute(&mut *tx)
    .await;

    tx.commit().await?;

    Ok((deleted_count, log_paths))
}

/// Spawn the cleanup task. Caller is responsible for ensuring only one runs at a time
/// (use `try_start` first).
pub fn spawn_cleanup(db: DB) {
    tokio::spawn(async move {
        let store = match windmill_object_store::get_object_store().await {
            Some(s) => s,
            None => {
                record_error("Object storage is not configured".to_string()).await;
                finish().await;
                return;
            }
        };

        if let Err(e) = cleanup_service_logs(&db, &store).await {
            record_error(format!("service logs phase failed: {e:#}")).await;
        }
        if let Err(e) = cleanup_job_logs(&db, &store).await {
            record_error(format!("job logs phase failed: {e:#}")).await;
        }

        finish().await;
    });
}
