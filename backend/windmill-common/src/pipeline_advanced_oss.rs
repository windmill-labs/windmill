//! OSS fallback: pipeline partition backfills are an enterprise feature;
//! their implementations live in windmill-ee-private (see
//! `pipeline_advanced_ee`). In the public build the entry points report that
//! the enterprise edition is required. (Freshness lives elsewhere: the
//! fresh/stale badge is CE in the assets API, the active watchdog is
//! windmill-queue's `freshness_watchdog`.)

use crate::error::Error;

pub fn backfill_todo() -> Error {
    Error::internal_err("Pipeline partition backfill requires the enterprise edition".to_string())
}

/// Write-audit-publish guard for `// data_test` (enterprise): the statement
/// placed inside the materialization write transaction that aborts it on any
/// test violation, so a failing slice is never published. The implementation
/// lives in `pipeline_advanced_ee`; the public build generates no guard, which
/// keeps the dbt-like commit-then-test behavior — a failed test still fails
/// the run and stops the cascade, but the written slice stays live.
pub fn data_test_guard_sql(
    _asset_path: &str,
    _checks: &[windmill_parser::sql_materialize::DataTestCheck],
) -> Option<String> {
    None
}
