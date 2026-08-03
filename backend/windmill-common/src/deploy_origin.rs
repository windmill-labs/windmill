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

/// Job arg carrying the origin of the deploy a dependency job was queued for, so
/// its tally can report the request's origin instead of guessing. Absent on a job
/// queued before this existed, which reads as [`TallyEvidence::Unknown`].
pub const DEPLOY_ORIGIN_ARG: &str = "__wm_deploy_origin";

/// How much of a deploy event the tallying task can answer for.
///
/// The two capabilities come apart. A dependency job knows the origin its request
/// declared and which path the deploy vacated — both are facts of the event it
/// was queued for. What it cannot answer is what the path holds *now*: it runs
/// whenever lock generation finishes, and by then someone else may have deleted
/// the item, which the tally would otherwise probe and file under this deploy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyEvidence {
    /// The task that served the write, so the current state is still its own.
    Served(DeployOrigin),
    /// A deploy that committed before this task ran.
    Deferred(DeployOrigin),
    /// Nothing known: no request served it and no origin came with it.
    Unknown,
}

impl TallyEvidence {
    pub fn origin(&self) -> Option<DeployOrigin> {
        match self {
            TallyEvidence::Served(o) | TallyEvidence::Deferred(o) => Some(*o),
            TallyEvidence::Unknown => None,
        }
    }

    /// Whether the path's current contents describe this event's outcome.
    pub fn can_probe(&self) -> bool {
        matches!(self, TallyEvidence::Served(_))
    }
}

tokio::task_local! {
    static DEPLOY_EVIDENCE: TallyEvidence;
}

/// Run `f` with `evidence` describing every deploy event it causes. The API
/// enters [`TallyEvidence::Served`] for every request, marked or not; a worker
/// replaying a request's deploy enters [`TallyEvidence::Deferred`].
///
/// Task-locals do not cross `tokio::spawn`, and the tally is spawned: read
/// [`current`] on the scoped task and carry the value into the spawned future.
pub async fn scope<F: std::future::Future>(evidence: TallyEvidence, f: F) -> F::Output {
    DEPLOY_EVIDENCE.scope(evidence, f).await
}

/// The evidence in scope, [`TallyEvidence::Unknown`] where there is no scope.
pub fn current() -> TallyEvidence {
    DEPLOY_EVIDENCE
        .try_with(|evidence| *evidence)
        .unwrap_or(TallyEvidence::Unknown)
}

/// Carry this deploy's origin into the dependency job it queues, whose tally is
/// the only one a lock-generating deploy gets. Call from the request task, which
/// is the last place that knows it. A no-op off one, leaving the job unable to
/// claim anything.
pub fn stamp_origin_arg(
    args: &mut std::collections::HashMap<String, Box<serde_json::value::RawValue>>,
) {
    if let Some(origin) = current().origin() {
        args.insert(
            DEPLOY_ORIGIN_ARG.to_string(),
            crate::worker::to_raw_value(&origin.as_str()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole worker side of the tally rests on this: nothing that did not
    /// serve the write may be probed, without each call site remembering to say
    /// so, and a deferred deploy still carries the origin it was queued with.
    #[tokio::test]
    async fn evidence_tracks_who_is_reporting() {
        assert_eq!(current(), TallyEvidence::Unknown);
        assert!(!TallyEvidence::Unknown.can_probe());
        assert!(!TallyEvidence::Deferred(DeployOrigin::Sync).can_probe());
        assert_eq!(
            TallyEvidence::Deferred(DeployOrigin::Sync).origin(),
            Some(DeployOrigin::Sync)
        );

        let served = TallyEvidence::Served(DeployOrigin::Authored);
        assert_eq!(scope(served, async { current() }).await, served);
        assert!(served.can_probe());

        // Not through a spawn, which is why `handle_deployment_metadata` reads it
        // before spawning the tally rather than inside.
        let spawned = scope(served, async {
            tokio::spawn(async { current() }).await.unwrap()
        })
        .await;
        assert_eq!(spawned, TallyEvidence::Unknown);
    }
}
