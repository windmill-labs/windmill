use chrono::{DateTime, Utc};

pub(crate) async fn update_concurrency_counter(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _w_id: &str,
    _job_id: uuid::Uuid,
    _concurrency_key: &str,
    _cancel: bool,
) -> anyhow::Result<(bool, Option<DateTime<Utc>>)> {
    crate::jobs_ee::update_concurrency_counter(_tx, _w_id, _job_id, _concurrency_key, _cancel).await
}