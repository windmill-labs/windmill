//! OSS fallback: pipeline partition backfills are an enterprise feature;
//! their implementations live in windmill-ee-private (see
//! `pipeline_advanced_ee`). In the public build the entry points report that
//! the enterprise edition is required. (Passive freshness monitoring — the
//! fresh/stale badge on the asset graph — is CE and lives in the assets API,
//! so it has no entry point here.)

use crate::error::Error;

pub fn backfill_todo() -> Error {
    Error::internal_err("Pipeline partition backfill requires the enterprise edition".to_string())
}
