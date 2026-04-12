//! OSS stubs for the fork review requests feature. The EE build overrides
//! these via `fork_reviews_ee.rs` when the `private` feature is active.
//!
//! These helpers are called from fork item-change hooks in CE code (e.g.
//! script/flow update endpoints) so the obsolescence call site compiles on
//! OSS even though the EE feature is what actually persists state.

#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::fork_reviews_ee::*;

#[cfg(not(feature = "private"))]
use crate::{db::DB, error::Result};

/// Mark every anchored comment on the currently open review request for the
/// given fork workspace that pins to `(anchor_kind, anchor_path)` as obsolete.
///
/// OSS stub: no-op. The EE build writes to `workspace_fork_review_comment`.
#[cfg(not(feature = "private"))]
pub async fn mark_anchor_obsolete(
    _db: &DB,
    _fork_workspace_id: &str,
    _anchor_kind: &str,
    _anchor_path: &str,
) -> Result<()> {
    Ok(())
}

/// Close the currently open review request for `(source_workspace_id,
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
