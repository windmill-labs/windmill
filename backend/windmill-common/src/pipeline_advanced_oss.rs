//! OSS fallback: pipeline freshness/SLA enforcement and partition backfills
//! are enterprise features; their implementations live in windmill-ee-private
//! (see `pipeline_advanced_ee`). In the public build the entry points report
//! that the enterprise edition is required.

use crate::error::Error;

pub fn freshness_enforcement_todo() -> Error {
    Error::internal_err(
        "Pipeline freshness/SLA enforcement requires the enterprise edition".to_string(),
    )
}

pub fn backfill_todo() -> Error {
    Error::internal_err("Pipeline partition backfill requires the enterprise edition".to_string())
}

/// Whether `// data_test` probes run inside the materialization write
/// transaction (write-audit-publish: a violation aborts the transaction, so a
/// failing slice is never published). Enterprise-only; the public build keeps
/// the dbt-like commit-then-test behavior — a failed test still fails the run
/// and stops the cascade, but the written slice stays live.
pub fn transactional_data_tests() -> bool {
    false
}
