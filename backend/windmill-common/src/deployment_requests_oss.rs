//! OSS stubs for the fork deployment requests feature. The EE build
//! overrides these via `deployment_requests_ee.rs` when the `private`
//! feature is active.
//!
//! These helpers are called from fork item-change hooks in EE code (see
//! `windmill-git-sync/src/git_sync_ee.rs::tally_deployed_object_changes`).
//! The stub exists so code paths that wire the hook compile on OSS even
//! though the obsolescence behavior itself is EE-only.
//!
//! On OSS the call is a no-op, so anchored-comment obsolescence after an
//! item change only actually fires when compiled with `--features private`.
//! The `fork_deployment_requests` integration test simulates the EE effect
//! via a raw SQL UPDATE to keep the lifecycle test meaningful on CE builds.

#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::deployment_requests_ee::*;

#[cfg(not(feature = "private"))]
use crate::{db::DB, error::Result};

/// Mark every anchored comment on the currently open deployment request
/// for the given fork workspace that pins to `(anchor_kind, anchor_path)`
/// as obsolete.
///
/// OSS stub: no-op. The EE build writes to
/// `workspace_fork_deployment_request_comment`.
#[cfg(not(feature = "private"))]
pub async fn mark_anchor_obsolete(
    _db: &DB,
    _fork_workspace_id: &str,
    _anchor_kind: &str,
    _anchor_path: &str,
) -> Result<()> {
    Ok(())
}

/// Close the currently open deployment request for `(source_workspace_id,
/// fork_workspace_id)` with `closed_reason = 'merged'` and mark all its
/// comments obsolete. Called from the frontend deploy success handler.
///
/// OSS stub: no-op.
#[cfg(not(feature = "private"))]
pub async fn close_open_request_on_merge(
    _db: &DB,
    _source_workspace_id: &str,
    _fork_workspace_id: &str,
) -> Result<()> {
    Ok(())
}
