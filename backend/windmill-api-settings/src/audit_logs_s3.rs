#![cfg(feature = "parquet")]
//! Read-only status for the audit-log → object-store exporter.
//!
//! The exporter (backend/src/monitor.rs) persists its cursor + progress in the
//! `background_task_state` row named
//! `windmill_common::global_settings::AUDIT_LOGS_S3_EXPORT_TASK`. Any API
//! replica can serve this status (it just reads that row), giving operators an
//! overview of how far the export has progressed — in particular the latest
//! audit timestamp actually uploaded.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::background_task;
use windmill_common::error::{self};
use windmill_common::global_settings::AUDIT_LOGS_S3_EXPORT_TASK;
use windmill_common::DB;

#[derive(Serialize)]
pub struct AuditLogsS3ExportStatus {
    /// xid cursor: rows of transactions below this have been exported.
    pub last_xmin: i64,
    /// Partition-pruning floor (the epoch sentinel while still bootstrapping).
    pub last_ts: Option<DateTime<Utc>>,
    /// True until the initial post-enable backlog has been fully drained.
    pub bootstrapping: bool,
    /// The latest audit-row timestamp actually written to object storage so
    /// far (monotonic) — the "how current is the mirror" figure.
    pub last_exported_audit_ts: Option<DateTime<Utc>>,
    /// When the exporter last completed a run.
    pub last_run_at: Option<DateTime<Utc>>,
    /// Number of audit rows uploaded in that last run.
    pub last_run_exported: i64,
    /// When the cursor row was last written (heartbeat).
    pub updated_at: DateTime<Utc>,
    /// Instance that last advanced the cursor.
    pub owner: Option<String>,
}

/// Current export status, or `None` if the feature was never enabled (no row).
pub async fn get_status(db: &DB) -> error::Result<Option<AuditLogsS3ExportStatus>> {
    let Some(row) = background_task::get(db, AUDIT_LOGS_S3_EXPORT_TASK).await? else {
        return Ok(None);
    };
    let v = &row.value;
    let parse_dt = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
    };
    let last_ts = parse_dt("last_ts");
    let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let bootstrapping = last_ts.map(|t| t <= epoch).unwrap_or(true);
    Ok(Some(AuditLogsS3ExportStatus {
        last_xmin: v.get("last_xmin").and_then(|x| x.as_i64()).unwrap_or(0),
        last_ts,
        bootstrapping,
        last_exported_audit_ts: parse_dt("last_exported_audit_ts"),
        last_run_at: parse_dt("last_run_at"),
        last_run_exported: v
            .get("last_run_exported")
            .and_then(|x| x.as_i64())
            .unwrap_or(0),
        updated_at: row.updated_at,
        owner: row.owner,
    }))
}
