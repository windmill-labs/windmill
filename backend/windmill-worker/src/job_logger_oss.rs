use std::io;

pub(crate) async fn s3_storage(
    _value: &str,
    _job_id: &uuid::Uuid,
    _w_id: &str,
    _db: &windmill_common::DB,
    _offset: usize,
) {
    crate::job_logger_ee::s3_storage(_value, _job_id, _w_id, _db, _offset).await
}

pub(crate) async fn default_disk_log_storage(
    _value: &str,
    _job_id: &uuid::Uuid,
    _w_id: &str,
    _offset: usize,
) {
    crate::job_logger_ee::default_disk_log_storage(_value, _job_id, _w_id, _offset).await
}

pub(crate) fn process_streaming_log_lines(
    _line: &str,
    _job_id: &uuid::Uuid,
    _w_id: &str,
    _logs: &[String],
    _offset: usize,
) -> Option<Result<String, io::Error>> {
    crate::job_logger_ee::process_streaming_log_lines(_line, _job_id, _w_id, _logs, _offset)
}