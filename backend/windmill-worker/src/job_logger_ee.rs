#[cfg(feature = "private")]
use crate::job_logger_ee;

use std::io;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use uuid::Uuid;
use windmill_common::DB;

use crate::job_logger::CompactLogs;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub(crate) async fn s3_storage(
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    logs: &str,
    total_size: Arc<AtomicU32>,
    worker_name: &str,
) {
    #[cfg(feature = "private")]
    {
        job_logger_ee::s3_storage(job_id, w_id, db, logs, total_size, worker_name).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (w_id, db, logs, total_size, worker_name); // job_id is used
        tracing::info!("Logs length of {job_id} has exceeded a threshold. Implementation to store excess on s3 in not OSS");
    }
}

#[allow(dead_code)]
pub(crate) async fn default_disk_log_storage(
    job_id: &Uuid,
    w_id: &str,
    db: &DB,
    logs: &str,
    total_size: Arc<AtomicU32>,
    compact_kind: CompactLogs,
    worker_name: &str,
) {
    #[cfg(feature = "private")]
    {
        job_logger_ee::default_disk_log_storage(job_id, w_id, db, logs, total_size, compact_kind, worker_name).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (w_id, db, logs, total_size, compact_kind, worker_name); // job_id is used
        tracing::info!("Logs length of {job_id} has exceeded a threshold. Implementation to store excess on disk in not OSS");
    }
}

pub(crate) fn process_streaming_log_lines(
    r: Result<Option<String>, io::Error>,
    stderr: bool,
    job_id: &Uuid,
) -> Option<Result<String, io::Error>> {
    #[cfg(feature = "private")]
    {
        return job_logger_ee::process_streaming_log_lines(r, stderr, job_id);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (stderr, job_id);
        r.transpose()
    }
}
