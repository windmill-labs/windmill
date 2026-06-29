//! OSS fallback for the asset-trigger cascade's AND-join / debounce / retry.
//!
//! The richer cascade semantics are a `private` feature (see `cascade_ee`).
//! In the public build they are absent and the cascade degrades to a plain
//! OR fan-out: every join always fires immediately, no slots are recorded or
//! reaped, debounce falls back to the subscriber's own script-level settings,
//! and retry is never applied (the subscriber pushes as a bare `ScriptHash`).
//! The real implementation lives in `windmill-ee-private`.

use windmill_common::error::Result;
use windmill_common::flows::Retry;
use windmill_common::runnable_settings::DebouncingSettings;
use windmill_common::DB;

/// Outcome of evaluating an AND-join barrier for one (subscriber, input).
/// Kept identical to the EE definition so the core matcher in
/// `asset_dispatch` compiles against either build.
pub enum JoinDecision {
    /// Input does not advance the join (recorded as Skipped with `reason`).
    Skip(&'static str),
    /// Join advanced but is not yet complete (recorded as JoinPending).
    Pending { received: i32, required: i32 },
    /// All required inputs are present — push the subscriber.
    Fire,
}

/// OSS has no AND-join: every input fires immediately (plain OR fan-out).
pub async fn handle_join(
    _db: &DB,
    _workspace_id: &str,
    _sub_path: &str,
    _trigger_ref: &str,
    _partition: Option<&str>,
) -> Result<JoinDecision> {
    Ok(JoinDecision::Fire)
}

/// Unused in OSS (no slots are ever recorded), kept for API parity.
pub const JOIN_SLOT_TTL_SECS: i64 = 60 * 24 * 60 * 60; // 60 days

/// No-op: OSS never records join slots, so there is nothing to reap.
pub async fn reap_stale_join_slots(_db: &DB) -> Result<()> {
    Ok(())
}

/// No per-edge debounce in OSS — always defer to the subscriber's own
/// script-level debounce settings.
pub fn cascade_debouncing_settings(
    _subscriber_path: &str,
    _partition: Option<&str>,
    _debounce_s: Option<i32>,
    fallback: DebouncingSettings,
) -> DebouncingSettings {
    fallback
}

/// No per-edge retry in OSS — the subscriber pushes as a bare `ScriptHash`.
pub fn cascade_retry(_retry_count: Option<i16>, _retry_delay_s: Option<i32>) -> Option<Retry> {
    None
}
