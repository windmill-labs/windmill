use uuid::Uuid;
use windmill_common::DB;

pub(crate) async fn update_concurrency_counter(
    _db: &DB,
    _job_id: &Uuid,
    _job_concurrency_key: String,
    _jobs_uuids_init_json_value: serde_json::Value,
    _pulled_job_id: String,
    _completed_count: i32,
    _limit: i32,
) -> anyhow::Result<bool> {
    tracing::error!("Concurrency limits are not OSS");
    Ok(true)
}
