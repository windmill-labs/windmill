//! OSS fallback: scheduled ducklake maintenance (snapshot expiry, adjacent-file
//! compaction, orphaned-file cleanup via managed per-lake schedules) is an
//! enterprise feature; the implementation lives in windmill-ee-private
//! (see `ducklake_maintenance_ee`). In the public build the entry points report
//! that the enterprise edition is required.

use std::collections::HashMap;

use sqlx::{Postgres, Transaction};
use windmill_common::{
    error::{Error, Result},
    jobs::JobPayload,
    schedule::Schedule,
    workspaces::Ducklake,
    DB,
};

/// Reconcile the managed `f/ducklake_maintenance/<lake>` schedule rows with
/// the ducklake settings, inside the caller's transaction.
///
/// Not an authorization boundary: it mutates schedule rows for `w_id` on
/// behalf of `edited_by`/`email`, so the caller MUST already have enforced
/// workspace-admin on `w_id` and that the identity is the authenticated
/// caller's (as `edit_ducklake_config` does via `require_admin`).
// Only newly-enabled maintenance is rejected: a config that already had it
// enabled (e.g. an enterprise license lapsed) must not make every unrelated
// ducklake settings save fail, and the admin must be able to save it off.
pub async fn sync_ducklake_maintenance_schedules<'c>(
    _db: &DB,
    tx: Transaction<'c, Postgres>,
    _w_id: &str,
    ducklakes: &HashMap<String, Ducklake>,
    previous: &HashMap<String, Ducklake>,
    _edited_by: &str,
    _email: &str,
) -> Result<Transaction<'c, Postgres>> {
    let enabled = |dl: &Ducklake| -> bool { dl.maintenance.as_ref().is_some_and(|m| m.enabled) };
    if ducklakes
        .iter()
        .any(|(name, dl)| enabled(dl) && !previous.get(name).is_some_and(|prev| enabled(prev)))
    {
        return Err(Error::BadRequest(
            "Ducklake scheduled maintenance is only available in the enterprise edition"
                .to_string(),
        ));
    }
    Ok(tx)
}

/// Build the job payload for one occurrence of a managed maintenance schedule
/// (`push_scheduled_job` calls this for reserved-prefix schedule paths).
/// Returns `(payload, tag, timeout, on_behalf_of_email, created_by)`.
// NotFound (not a generic error) so a schedule row left over from an
// enterprise period is auto-disabled with `schedule.error` recorded by the
// post-completion scheduler, instead of grinding through completion retries.
pub async fn build_maintenance_schedule_payload<'c>(
    _tx: &mut Transaction<'c, Postgres>,
    _schedule: &Schedule,
) -> Result<(
    JobPayload,
    Option<String>,
    Option<i32>,
    Option<String>,
    String,
)> {
    Err(Error::NotFound(
        "Ducklake scheduled maintenance is only available in the enterprise edition".to_string(),
    ))
}
