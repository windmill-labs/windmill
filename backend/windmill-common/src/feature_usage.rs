/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 */

//! Backend writer for the anonymous `feature_usage` counters.
//!
//! Counts are accumulated in memory and flushed in batches, so an instrumented
//! call site costs a hash lookup rather than a query — some of them sit on the
//! job push path. Only aggregated counts ever leave the instance: never a
//! workspace path, a prompt, code, or anything identifying a user.
//!
//! The frontend twin is `frontend/src/lib/utils/featureUsage.ts`, which posts to
//! `/w/{workspace}/workspaces/log_feature_usage`. Both validate against
//! [`FEATURE_USAGE_KINDS`] below, so the registry of what may be recorded lives
//! in one place.

use std::collections::HashMap;
use std::sync::Mutex;

use sqlx::{Pool, Postgres};

/// Only registered (feature, kind) actions are accepted, so telemetry stays
/// limited to predefined feature actions. Keys are shape-checked
/// (identifier-like, no spaces) rather than pinned to value sets: they come from
/// our own code (modes, tab/draft kinds, tool names, provider:model) and pinning
/// every value was not worth the maintenance.
pub const FEATURE_USAGE_KINDS: &[(&str, &str)] = &[
    ("ai_session", "created"),
    ("ai_session", "message"),
    ("ai_session", "autonomy"),
    ("ai_session", "tab"),
    ("ai_session", "tokens"),
    ("ai_session", "deployed"),
    ("ai_session", "archived"),
    ("ai_session", "deleted"),
    ("ai_session", "beta_optout"),
    ("ai_session", "beta_optin"),
    ("ai_chat", "message"),
    ("ai_chat", "model"),
    ("ai_chat", "tool"),
    ("flow_editor", "panel_placement"),
    ("flow_run", "tab"),
    ("flow_step", "pinned"),
    ("trigger", "fired"),
    ("command_script", "invoked"),
    ("hub_script", "picked"),
    ("hub_script", "considered_ai"),
];

pub fn is_identifier_shaped(s: &str, max_len: usize) -> bool {
    !s.is_empty()
        && s.len() <= max_len
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '/'))
}

pub const MAX_KEY_LEN: usize = 100;
pub const MAX_ENTITY_ID_LEN: usize = 50;

/// Distinct keys held per action between flushes. A call site that keys on
/// something unexpectedly high-cardinality would otherwise grow this map without
/// bound; past the cap new keys are dropped rather than accumulated.
const MAX_PENDING_KEYS_PER_ACTION: usize = 2_000;

// Nested rather than keyed by one `(feature, kind, key)` tuple so an already
// counted key is looked up by `&str`: the job push path reaches this on every
// externally triggered job and should not allocate to find its counter.
lazy_static::lazy_static! {
    static ref PENDING: Mutex<HashMap<(&'static str, &'static str), HashMap<String, i64>>> =
        Mutex::new(HashMap::new());
}

/// Record one anonymous feature-usage event. Fire-and-forget: increments an
/// in-memory counter, flushed later by [`flush_feature_usage`].
///
/// `feature` and `kind` are `&'static str` so a call site cannot pass a computed
/// pair that is missing from [`FEATURE_USAGE_KINDS`] — unregistered pairs and
/// malformed keys are dropped here rather than reaching the database.
pub fn log_feature_usage(feature: &'static str, kind: &'static str, key: &str) {
    if !FEATURE_USAGE_KINDS.contains(&(feature, kind)) {
        return;
    }
    if !key.is_empty() && !is_identifier_shaped(key, MAX_KEY_LEN) {
        return;
    }
    // A poisoned lock means a previous holder panicked mid-update; telemetry is
    // never worth propagating that, so drop the event.
    let Ok(mut pending) = PENDING.lock() else {
        return;
    };
    let counters = pending.entry((feature, kind)).or_default();
    match counters.get_mut(key) {
        Some(value) => *value += 1,
        None => {
            if counters.len() >= MAX_PENDING_KEYS_PER_ACTION {
                return;
            }
            counters.insert(key.to_string(), 1);
        }
    }
}

/// Drain the accumulator into `feature_usage`. Called from the monitor loop.
pub async fn flush_feature_usage(db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let drained: Vec<((&'static str, &'static str), HashMap<String, i64>)> = {
        let Ok(mut pending) = PENDING.lock() else {
            return Ok(());
        };
        if pending.is_empty() {
            return Ok(());
        }
        pending.drain().collect()
    };

    // Sorted, not in HashMap order: every server and worker flushes the same few
    // rows on the same cadence, and two batches upserting them in opposite orders
    // deadlock. Postgres aborts one, and its counts are already drained from
    // memory, so they would be lost rather than retried.
    let mut rows: Vec<(&'static str, &'static str, String, i64)> = drained
        .into_iter()
        .flat_map(|((feature, kind), counters)| {
            counters
                .into_iter()
                .map(move |(key, value)| (feature, kind, key, value))
        })
        .collect();
    rows.sort_unstable_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));

    let mut features = Vec::with_capacity(rows.len());
    let mut kinds = Vec::with_capacity(rows.len());
    let mut keys = Vec::with_capacity(rows.len());
    let mut values = Vec::with_capacity(rows.len());
    for (feature, kind, key, value) in rows {
        features.push(feature.to_string());
        kinds.push(kind.to_string());
        keys.push(key);
        values.push(value);
    }
    if features.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO feature_usage (feature, kind, key, value)
         SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::bigint[])
         ON CONFLICT (feature, kind, key, entity_id, day)
         DO UPDATE SET value = feature_usage.value + EXCLUDED.value, updated_at = now()",
        &features,
        &kinds,
        &keys,
        &values
    )
    .execute(db)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_of(feature: &str, kind: &str, key: &str) -> i64 {
        PENDING
            .lock()
            .unwrap()
            .get(&(feature, kind))
            .and_then(|c| c.get(key))
            .copied()
            .unwrap_or(0)
    }

    #[test]
    fn only_registered_actions_with_a_well_shaped_key_are_recorded() {
        // Unique keys: the accumulator is a process-wide global shared with any
        // other test that logs.
        log_feature_usage("trigger", "fired", "registered_ok_key");
        log_feature_usage("trigger", "fired", "registered_ok_key");
        assert_eq!(count_of("trigger", "fired", "registered_ok_key"), 2);

        // Unregistered pair: nothing an unreviewed call site emits reaches the table.
        log_feature_usage("not_a_feature", "nope", "unregistered_key");
        assert_eq!(count_of("not_a_feature", "nope", "unregistered_key"), 0);

        // A key that is not identifier-shaped could carry arbitrary text.
        log_feature_usage("trigger", "fired", "has spaces and <html>");
        assert_eq!(count_of("trigger", "fired", "has spaces and <html>"), 0);
    }
}
