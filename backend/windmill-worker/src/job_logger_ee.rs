use std::io;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use uuid::Uuid;
use windmill_common::DB;

use crate::job_logger::CompactLogs;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub(crate) async fn s3_storage(_job_id: Uuid, _w_id: &String, _db: &sqlx::Pool<sqlx::Postgres>, _logs: &String, _total_size: &Arc<AtomicU32>, _worker_name: &String) {
    tracing::info!("Logs length of {job_id} has exceeded a threshold. Implementation to store excess on s3 in not OSS");
}

pub(crate) async fn default_disk_log_storage(
    job_id: Uuid,
    _w_id: &str,
    _db: &DB,
    _nlogs: String,
    _total_size: Arc<AtomicU32>,
    _compact_kind: CompactLogs,
    _worker_name: &str,
) {
    tracing::info!("Logs length of {job_id} has exceeded a threshold. Implementation to store excess on disk in not OSS");
}


pub(crate) fn process_streaming_log_lines(r: Result<Option<String>, io::Error>, _stderr: bool) -> Option<Result<String, io::Error>> {
    r.transpose()
}