//! OSS fallback: scheduled ducklake maintenance (snapshot expiry, adjacent-file
//! compaction, orphaned-file cleanup via managed per-lake schedules) is an
//! enterprise feature; the implementation lives in windmill-ee-private
//! (see `ducklake_maintenance_ee`). In the public build the entry points report
//! that the enterprise edition is required.

use std::collections::HashMap;

use sqlx::{Postgres, Transaction};
use windmill_common::{
    error::{Error, Result},
    workspaces::{Ducklake, DucklakeMaintenance},
    DB,
};

/// Reconcile the managed `f/ducklake_maintenance/<lake>` schedule rows with the
/// (already validated) ducklake settings, inside the caller's transaction.
pub async fn sync_ducklake_maintenance_schedules<'c>(
    _db: &DB,
    tx: Transaction<'c, Postgres>,
    _w_id: &str,
    ducklakes: &HashMap<String, Ducklake>,
    _edited_by: &str,
    _email: &str,
) -> Result<Transaction<'c, Postgres>> {
    if ducklakes
        .values()
        .any(|dl| dl.maintenance.as_ref().is_some_and(|m| m.enabled))
    {
        return Err(Error::BadRequest(
            "Ducklake scheduled maintenance is only available in the enterprise edition"
                .to_string(),
        ));
    }
    Ok(tx)
}

/// Generate the DuckDB maintenance script for one lake.
// NotFound (not a generic error) so a schedule row left over from an
// enterprise period is auto-disabled with `schedule.error` recorded by the
// post-completion scheduler, instead of grinding through completion retries.
pub fn build_ducklake_maintenance_sql(_lake: &str, _m: &DucklakeMaintenance) -> Result<String> {
    Err(Error::NotFound(
        "Ducklake scheduled maintenance is only available in the enterprise edition".to_string(),
    ))
}
