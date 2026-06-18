#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::jobs_ee::*;

#[cfg(not(feature = "private"))]
use chrono::{DateTime, Utc};
#[cfg(not(feature = "private"))]
use uuid::Uuid;
#[cfg(not(feature = "private"))]
use windmill_common::DB;

#[cfg(not(feature = "private"))]
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
