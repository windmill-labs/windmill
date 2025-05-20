#[cfg(feature = "private")]
use crate::jobs_ee;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use windmill_common::DB;

#[allow(dead_code)]
pub(crate) async fn update_concurrency_counter(
    db: &DB,
    job_id: &Uuid,
    job_concurrency_key: String,
    jobs_uuids_init_json_value: serde_json::Value,
    pulled_job_id: String,
    job_custom_concurrency_time_window_s: i32,
    limit: i32,
) -> anyhow::Result<(bool, Option<DateTime<Utc>>)> {
    #[cfg(feature = "private")]
    {
        return jobs_ee::update_concurrency_counter(db, job_id, job_concurrency_key, jobs_uuids_init_json_value, pulled_job_id, job_custom_concurrency_time_window_s, limit).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, job_id, job_concurrency_key, jobs_uuids_init_json_value, pulled_job_id, job_custom_concurrency_time_window_s, limit);
        Ok((true, None))
    }
}
