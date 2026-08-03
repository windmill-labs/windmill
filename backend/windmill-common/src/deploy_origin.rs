/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where a deploy event came from, for the fork tally (`workspace_diff`).
//!
//! A write applied by a sync leaves the same trace as one a person made, which
//! is what makes a parent-only fork row undecidable. Clients that apply a state
//! computed elsewhere (a git-sync pull, a workspace-to-workspace deploy) say so
//! with [`DEPLOY_ORIGIN_HEADER`]; the API scopes it for the request and the tally
//! records it alongside the counters.
//!
//! The flag is evidence for a UI offer, never authority: claiming `sync` only
//! makes the tally *more* conservative (a sync-origin deletion is never offered
//! as a removal to merge), and `authored` is what any unmarked request gets.

/// Request header a sync client sets to mark its writes as applied rather than
/// authored. Only `sync` is meaningful; any other value reads as authored.
pub const DEPLOY_ORIGIN_HEADER: &str = "x-windmill-deploy-origin";

/// Who caused a deploy event at a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployOrigin {
    /// Written in this workspace by whoever made the request.
    Authored,
    /// Applied to this workspace by a sync (git-sync pull, cross-workspace deploy).
    Sync,
}

impl DeployOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeployOrigin::Authored => "authored",
            DeployOrigin::Sync => "sync",
        }
    }

    pub fn from_header_value(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("sync") {
            DeployOrigin::Sync
        } else {
            DeployOrigin::Authored
        }
    }
}

/// What a deploy event did to the path it is recorded against.
///
/// Create and update are one value: at the point the tally runs, the write has
/// already committed and no item kind carries a signal that separates them.
/// Existence *after* the event is what the merge direction needs, and the
/// comparison recomputes that per side (`exists_in_source` / `exists_in_fork`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployEventKind {
    /// The path holds an item after the event.
    Write,
    /// The path holds no item after the event.
    Delete,
    /// The path was vacated by a rename to another path.
    RenameFrom,
}

impl DeployEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeployEventKind::Write => "write",
            DeployEventKind::Delete => "delete",
            DeployEventKind::RenameFrom => "rename_from",
        }
    }
}

tokio::task_local! {
    static REQUEST_DEPLOY_ORIGIN: DeployOrigin;
}

/// Run `f` with `origin` as the origin of every deploy event it causes.
///
/// Task-locals do not cross `tokio::spawn`, and the tally is spawned: read
/// [`current`] on the request task and carry the value into the spawned future.
pub async fn scope<F: std::future::Future>(origin: DeployOrigin, f: F) -> F::Output {
    REQUEST_DEPLOY_ORIGIN.scope(origin, f).await
}

/// The origin in scope, defaulting to [`DeployOrigin::Authored`] outside a
/// request (worker-side deploys, tests) and for any request that did not mark
/// itself.
pub fn current() -> DeployOrigin {
    REQUEST_DEPLOY_ORIGIN
        .try_with(|origin| *origin)
        .unwrap_or(DeployOrigin::Authored)
}
