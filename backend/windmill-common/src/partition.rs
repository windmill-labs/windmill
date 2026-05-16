//! Pipeline partition-value resolution.
//!
//! `// partitioned <kind> [opts]` declares that a pipeline script
//! materializes a partitioned output. The graph/parser keep the
//! `{partition}` token literal (lineage stays partition-agnostic); this
//! module turns a [`PartitionSpec`] + run context into the concrete value
//! the run uses. The value is then injected into the job's args and
//! propagated down the asset-trigger cascade (see `asset_dispatch`), so it
//! is resolved exactly once at the top of a chain.
//!
//! - time kinds (`daily`/`hourly`/`weekly`/`monthly`): the run's anchor
//!   instant localized to `tz` (default UTC), rendered with `format`
//!   (default per-kind), gated on `start` (older partitions not backfilled).
//! - `dynamic key=<path>`: extracted from the triggering payload via a
//!   minimal `$.a.b` JSON path (the realistic per-tenant/shard/event case).

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use windmill_parser::asset_parser::{PartitionKind, PartitionSpec};

use crate::error::{Error, Result};

/// Well-known arg key the resolved partition value is injected under (also
/// mirrored at `trigger.partition` for cascade propagation).
pub const PARTITION_ARG: &str = "partition";

fn default_format(kind: &PartitionKind) -> &'static str {
    match kind {
        PartitionKind::Hourly => "%Y-%m-%dT%H",
        PartitionKind::Weekly => "%G-W%V",
        PartitionKind::Monthly => "%Y-%m",
        // daily + dynamic fall back to a plain date when a format is wanted
        _ => "%Y-%m-%d",
    }
}

/// Resolve a time-based partition for `spec` at instant `at`.
///
/// Returns `Ok(None)` when the computed partition is strictly before
/// `spec.start` (anchor: older partitions are not backfilled). Errors on an
/// invalid `tz` or `start` rather than silently materializing the wrong
/// partition. `Dynamic` is rejected — use [`extract_dynamic_partition`].
pub fn resolve_time_partition(spec: &PartitionSpec, at: DateTime<Utc>) -> Result<Option<String>> {
    if matches!(spec.kind, PartitionKind::Dynamic { .. }) {
        return Err(Error::BadRequest(
            "resolve_time_partition called on a dynamic partition".to_string(),
        ));
    }
    let tz: Tz = match spec.tz.as_deref() {
        None => chrono_tz::UTC,
        Some(s) => s
            .parse()
            .map_err(|_| Error::BadRequest(format!("invalid partition tz: {s}")))?,
    };
    let local = at.with_timezone(&tz);
    let fmt = spec
        .format
        .as_deref()
        .unwrap_or_else(|| default_format(&spec.kind));
    if let Some(start) = spec.start.as_deref() {
        let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| {
            Error::BadRequest(format!(
                "invalid partition start (want YYYY-MM-DD): {start}"
            ))
        })?;
        if local.date_naive() < start_date {
            return Ok(None);
        }
    }
    Ok(Some(local.format(fmt).to_string()))
}

/// Extract a dynamic partition value from `payload` using a minimal
/// `$.a.b.c` dotted path (no brackets/wildcards/filters — that covers the
/// realistic per-tenant/shard/event keys). The leaf must be a string,
/// number or bool.
pub fn extract_dynamic_partition(key: &str, payload: &serde_json::Value) -> Result<String> {
    let path = key.strip_prefix("$").unwrap_or(key);
    let path = path.strip_prefix('.').unwrap_or(path);
    let mut cur = payload;
    if !path.is_empty() {
        for seg in path.split('.') {
            if seg.is_empty() || seg.contains(['[', ']', '*']) {
                return Err(Error::BadRequest(format!(
                    "unsupported dynamic partition key (only $.a.b): {key}"
                )));
            }
            cur = cur.get(seg).ok_or_else(|| {
                Error::BadRequest(format!("dynamic partition key not found in payload: {key}"))
            })?;
        }
    }
    match cur {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        _ => Err(Error::BadRequest(format!(
            "dynamic partition key {key} resolved to a non-scalar value"
        ))),
    }
}

/// Resolve `spec` for a run anchored at `at`, with `payload` available for
/// dynamic kinds. `Ok(None)` = before `start` (skip materialization).
pub fn resolve_partition(
    spec: &PartitionSpec,
    at: DateTime<Utc>,
    payload: Option<&serde_json::Value>,
) -> Result<Option<String>> {
    match &spec.kind {
        PartitionKind::Dynamic { key } => {
            let p = payload.ok_or_else(|| {
                Error::BadRequest("dynamic partition requires a triggering payload".to_string())
            })?;
            extract_dynamic_partition(key, p).map(Some)
        }
        _ => resolve_time_partition(spec, at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn spec(kind: PartitionKind) -> PartitionSpec {
        PartitionSpec { kind, tz: None, format: None, start: None }
    }
    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn daily_default_format_utc() {
        let r = resolve_partition(
            &spec(PartitionKind::Daily),
            at("2026-05-16T09:30:00Z"),
            None,
        );
        assert_eq!(r.unwrap(), Some("2026-05-16".to_string()));
    }

    #[test]
    fn hourly_and_monthly_defaults() {
        assert_eq!(
            resolve_partition(
                &spec(PartitionKind::Hourly),
                at("2026-05-16T09:30:00Z"),
                None
            )
            .unwrap(),
            Some("2026-05-16T09".to_string())
        );
        assert_eq!(
            resolve_partition(
                &spec(PartitionKind::Monthly),
                at("2026-05-16T09:30:00Z"),
                None
            )
            .unwrap(),
            Some("2026-05".to_string())
        );
    }

    #[test]
    fn tz_shifts_the_day_boundary() {
        // 00:30Z on the 16th is still the 15th in America/New_York.
        let s = PartitionSpec {
            kind: PartitionKind::Daily,
            tz: Some("America/New_York".to_string()),
            format: None,
            start: None,
        };
        assert_eq!(
            resolve_partition(&s, at("2026-05-16T00:30:00Z"), None).unwrap(),
            Some("2026-05-15".to_string())
        );
    }

    #[test]
    fn custom_format_is_honoured() {
        let s = PartitionSpec {
            kind: PartitionKind::Daily,
            tz: None,
            format: Some("%Y/%m/%d".to_string()),
            start: None,
        };
        assert_eq!(
            resolve_partition(&s, at("2026-05-16T09:30:00Z"), None).unwrap(),
            Some("2026/05/16".to_string())
        );
    }

    #[test]
    fn start_anchor_skips_older_partitions() {
        let s = PartitionSpec {
            kind: PartitionKind::Daily,
            tz: None,
            format: None,
            start: Some("2026-06-01".to_string()),
        };
        assert_eq!(
            resolve_partition(&s, at("2026-05-16T09:30:00Z"), None).unwrap(),
            None
        );
        assert_eq!(
            resolve_partition(&s, at("2026-06-02T00:00:00Z"), None).unwrap(),
            Some("2026-06-02".to_string())
        );
    }

    #[test]
    fn invalid_tz_and_start_error() {
        let bad_tz = PartitionSpec {
            kind: PartitionKind::Daily,
            tz: Some("Not/AZone".to_string()),
            format: None,
            start: None,
        };
        assert!(resolve_partition(&bad_tz, at("2026-05-16T00:00:00Z"), None).is_err());
        let bad_start = PartitionSpec {
            kind: PartitionKind::Daily,
            tz: None,
            format: None,
            start: Some("06/01/2026".to_string()),
        };
        assert!(resolve_partition(&bad_start, at("2026-05-16T00:00:00Z"), None).is_err());
    }

    #[test]
    fn dynamic_extracts_dotted_path() {
        let s = spec(PartitionKind::Dynamic { key: "$.tenant_id".to_string() });
        let payload = serde_json::json!({ "tenant_id": "acme", "other": 1 });
        assert_eq!(
            resolve_partition(&s, Utc.timestamp_opt(0, 0).unwrap(), Some(&payload)).unwrap(),
            Some("acme".to_string())
        );
        let nested = spec(PartitionKind::Dynamic { key: "$.a.b".to_string() });
        let p2 = serde_json::json!({ "a": { "b": 42 } });
        assert_eq!(
            resolve_partition(&nested, Utc.timestamp_opt(0, 0).unwrap(), Some(&p2)).unwrap(),
            Some("42".to_string())
        );
    }

    #[test]
    fn dynamic_errors_on_missing_key_or_no_payload() {
        let s = spec(PartitionKind::Dynamic { key: "$.tenant_id".to_string() });
        assert!(resolve_partition(&s, Utc.timestamp_opt(0, 0).unwrap(), None).is_err());
        let payload = serde_json::json!({ "nope": 1 });
        assert!(resolve_partition(&s, Utc.timestamp_opt(0, 0).unwrap(), Some(&payload)).is_err());
    }

    #[test]
    fn dynamic_rejects_unsupported_path_syntax() {
        assert!(extract_dynamic_partition("$.items[0]", &serde_json::json!({})).is_err());
    }
}
