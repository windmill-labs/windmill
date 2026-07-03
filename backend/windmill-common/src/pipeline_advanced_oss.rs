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
