use chrono::{DateTime, Utc};
use uuid::Uuid;
use windmill_common::DB;

#[allow(dead_code)]
pub(crate) async fn update_concurrency_counter(
    _db: &DB,
    _job_id: &Uuid,
    _job_concurrency_key: String,
    _jobs_uuids_init_json_value: serde_json::Value,
    _pulled_job_id: String,
    _job_custom_concurrency_time_window_s: i32,
    _limit: i32,
) -> anyhow::Result<(bool, Option<DateTime<Utc>>)> {
    Ok((true, None))
}
