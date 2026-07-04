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

/// Assemble a materialization plan into the statement list the DuckDB executor
/// runs. The public build executes the plan verbatim — dbt-like
/// commit-then-test: a failing `// data_test` still fails the run and stops
/// the cascade, but the written slice stays live. The enterprise
/// implementation (`pipeline_advanced_ee`) instead restructures the plan into
/// write-audit-publish, where a failing test rolls the whole write back before
/// anything is published.
pub fn finalize_materialize_query(
    plan: windmill_parser::sql_materialize::MaterializePlan,
    _asset_path: &str,
) -> Vec<String> {
    plan.stmts.into_iter().map(|s| s.sql).collect()
}
