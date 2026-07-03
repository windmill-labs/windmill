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
