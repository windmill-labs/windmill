//! Detects changes to a workspace's scripts and flows for `subscriptions/listen`.
//!
//! Each process polls a fingerprint of every workspace one of its listeners watches,
//! so a change made on any replica, through any path (UI, CLI, git sync, a user
//! rename), reaches every listener within one poll interval with no cross-replica
//! signalling.

use crate::server::backend::McpBackend;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, Notify};

const POLL_INTERVAL: Duration = Duration::from_secs(30);

struct Watcher {
    changes: broadcast::Sender<()>,
    poll_now: Arc<Notify>,
}

#[derive(Default)]
pub(crate) struct ListChanges {
    watchers: Mutex<HashMap<String, Watcher>>,
}

impl ListChanges {
    /// Subscribe to `workspace_id`'s changes, starting its poller if this is the
    /// first listener for it in this process.
    pub(crate) fn subscribe<B: McpBackend>(
        self: &Arc<Self>,
        backend: &Arc<B>,
        workspace_id: &str,
    ) -> broadcast::Receiver<()> {
        let mut watchers = self.watchers.lock().unwrap();
        if let Some(watcher) = watchers.get(workspace_id) {
            return watcher.changes.subscribe();
        }
        let (changes, rx) = broadcast::channel(16);
        let poll_now = Arc::new(Notify::new());
        watchers.insert(
            workspace_id.to_string(),
            Watcher { changes: changes.clone(), poll_now: poll_now.clone() },
        );
        tokio::spawn(self.clone().poll(
            backend.clone(),
            workspace_id.to_string(),
            changes,
            poll_now,
        ));
        rx
    }

    /// Poll `workspace_id` now rather than at its next interval, after this process
    /// changed it: the client that made the change is the one waiting on it. Polling
    /// rather than signalling directly keeps the poller's baseline current, so the
    /// same change is not reported again on the next tick.
    pub(crate) fn poll_now(&self, workspace_id: &str) {
        if let Some(watcher) = self.watchers.lock().unwrap().get(workspace_id) {
            watcher.poll_now.notify_one();
        }
    }

    async fn poll<B: McpBackend>(
        self: Arc<Self>,
        backend: Arc<B>,
        workspace_id: String,
        changes: broadcast::Sender<()>,
        poll_now: Arc<Notify>,
    ) {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last = None;
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = poll_now.notified() => {}
            }
            {
                // Checked under the lock `subscribe` takes, so a listener cannot join a
                // watcher that is about to stop.
                let mut watchers = self.watchers.lock().unwrap();
                if changes.receiver_count() == 0 {
                    watchers.remove(&workspace_id);
                    return;
                }
            }
            match backend.runnable_list_fingerprint(&workspace_id).await {
                Ok(fingerprint) => {
                    // The first fingerprint also notifies: it is taken after the listener
                    // subscribed, possibly after its client listed tools, so a change landing
                    // in between would otherwise become the baseline and never be announced.
                    if last.as_ref() != Some(&fingerprint) {
                        let _ = changes.send(());
                    }
                    last = Some(fingerprint);
                }
                Err(e) => tracing::warn!(
                    "MCP tool-list poll failed for workspace {workspace_id}: {}",
                    e.message
                ),
            }
        }
    }
}
