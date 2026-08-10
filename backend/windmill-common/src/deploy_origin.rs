/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where a deploy event came from, for the fork tally (`workspace_diff` — see the
//! `workspace_diff_last_event` migration for what the tally does with it).
//!
//! A client that applies a state computed elsewhere (a git-sync pull, a
//! workspace-to-workspace deploy) says so with [`DEPLOY_ORIGIN_HEADER`]; the API
//! scopes it for the request and the tally records it alongside the counters.
//! Anything unmarked is [`DeployOrigin::Authored`].
//!
//! The flag is evidence for a UI offer, never authority. Claiming `sync` cannot
//! make the merge propose a removal — that needs `authored` — but it is not inert
//! either: `compare_workspaces` excludes a parent-only `sync` row from both sides
//! of its `all_ahead_items_visible` comparison, so stamping `sync` on a row the
//! caller cannot see suppresses the "changes not visible to your user" warning
//! that `authored` would raise. That flag is a visibility guarantee rather than an
//! authorization boundary (each item is re-authorized at deploy time), and this
//! matches how an untallied row already behaves, so it is bounded — but do not
//! read the header as unable to affect anything.

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

/// Run `f` with `origin` as the origin of every deploy event it causes. Entered
/// for every request, marked or not, so that being in scope at all means "this
/// task is serving the write it is about to report" — see [`current`].
///
/// Task-locals do not cross `tokio::spawn`, and the tally is spawned: read
/// [`current`] on the request task and carry the value into the spawned future.
pub async fn scope<F: std::future::Future>(origin: DeployOrigin, f: F) -> F::Output {
    REQUEST_DEPLOY_ORIGIN.scope(origin, f).await
}

/// The origin in scope, or `None` where there is no scope to read.
///
/// `None` is what a worker gets. A dependency job reports a deploy that
/// committed before the job was even queued and finishes whenever lock
/// generation finishes, so its reading of the workspace describes whoever wrote
/// last, not its own event. Nothing outside a request can answer for that, so
/// nothing outside a request is asked to.
pub fn current() -> Option<DeployOrigin> {
    REQUEST_DEPLOY_ORIGIN.try_with(|origin| *origin).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole worker side of the tally rests on this: a deploy reported from
    /// anywhere but the task that served it records no evidence, without each
    /// call site remembering to say so.
    #[tokio::test]
    async fn origin_is_absent_outside_a_request() {
        assert_eq!(current(), None);
        assert_eq!(
            scope(DeployOrigin::Authored, async { current() }).await,
            Some(DeployOrigin::Authored)
        );
        // Not through a spawn, which is why `handle_deployment_metadata` reads it
        // before spawning the tally rather than inside.
        let spawned = scope(DeployOrigin::Sync, async {
            tokio::spawn(async { current() }).await.unwrap()
        })
        .await;
        assert_eq!(spawned, None);
    }
}
