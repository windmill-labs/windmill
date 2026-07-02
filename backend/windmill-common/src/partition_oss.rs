//! OSS fallback for pipeline partition resolution.
//!
//! Partition resolution is a `private` feature (see `partition_ee`). In the
//! public build it is absent: `resolve_partition` yields no partition, so the
//! cascade runs partition-agnostically, and the args helpers degrade to a
//! plain (non-partition-preserving) write. The real implementation lives in
//! `windmill-ee-private`.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::value::RawValue;
use sqlx::types::Json;
use sqlx::PgExecutor;
use uuid::Uuid;
use windmill_parser::asset_parser::PartitionSpec;

use crate::error::{Error, Result};

/// Well-known arg key the resolved partition value is injected under.
pub const PARTITION_ARG: &str = "partition";

/// No-op: OSS never resolves a partition, so there is nothing to persist.
pub async fn set_resolved_partition<'e>(
    _executor: impl PgExecutor<'e>,
    _job_id: Uuid,
    _value: &str,
) -> Result<()> {
    Ok(())
}

/// Plain args replace — OSS has no resolved partition to carry forward.
pub async fn merge_args_preserving_partition<'e>(
    executor: impl PgExecutor<'e>,
    job_id: Uuid,
    new_args: HashMap<String, Box<RawValue>>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE v2_job SET args = $1, preprocessed = TRUE WHERE id = $2",
        Json(new_args) as Json<HashMap<String, Box<RawValue>>>,
        job_id,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// No partition in the OSS build.
pub fn resolve_partition(
    _spec: &PartitionSpec,
    _at: DateTime<Utc>,
    _payload: Option<&serde_json::Value>,
) -> Result<Option<String>> {
    Ok(None)
}

/// Range backfill (enumerating the partition worklist) is an enterprise
/// feature; single-partition runs with an explicit `partition` arg remain
/// available in OSS.
pub fn enumerate_time_partitions(
    _spec: &PartitionSpec,
    _from: NaiveDate,
    _to: NaiveDate,
) -> Result<Vec<String>> {
    Err(Error::BadRequest(
        "Backfilling a range of partitions is an enterprise feature".to_string(),
    ))
}
