//! OSS fallback for anonymous feature-usage collection.
//!
//! Collection is a `private` feature (see `feature_usage_ee`). The public build
//! never sends a stats payload (`stats_oss`), so counting anything would only
//! write rows nothing reads: every entry point here is inert, and the
//! `log_feature_usage` endpoint accepts its posts without recording them.

use sqlx::{Pool, Postgres};

/// No action is recordable in the public build.
pub fn is_recordable_event(
    _feature: &str,
    _kind: &str,
    _key: &str,
    _entity_id: &str,
) -> bool {
    false
}

/// No-op: nothing is counted in the public build.
pub fn log_feature_usage(_feature: &'static str, _kind: &'static str, _key: &str) {}

/// Nothing accumulates, so there is nothing to flush.
pub async fn flush_feature_usage(_db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    Ok(())
}
