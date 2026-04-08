#![cfg(feature = "parquet")]
/*
 * Background computation of object-storage usage aggregated by top-level folder.
 *
 * On large buckets (millions of objects, TB of data) `list` is an O(N) operation:
 * the object store returns 1000 objects per API call, so summing sizes under
 * "logs/" can take several minutes. That is far longer than most reverse proxies
 * allow in front of a single HTTP request (typically 30-60s), so we run the
 * computation in a background tokio task and expose it via a status endpoint
 * that the frontend polls.
 *
 * Progress is persisted in the `background_task_state` table so the status
 * endpoint works across API-server replicas (the task runs on whichever
 * server received the POST; any server can serve the GET).
 */

use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::background_task;
use windmill_common::error;
use windmill_common::{DB, INSTANCE_NAME};

use windmill_object_store::object_store_reexports::ObjectStore;

pub const TASK_NAME: &str = "storage_usage";

#[derive(Clone, Serialize, Deserialize)]
pub struct FolderUsage {
    pub prefix: String,
    pub size: u64,
    /// True if listing this prefix errored mid-stream; `size` is then a lower
    /// bound, not the full total. UI surfaces this so operators don't
    /// misinterpret the partial sum as the real folder size.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub partial: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StorageUsageProgress {
    pub running: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub current_prefix: Option<String>,
    pub scanned_objects: u64,
    pub folders: Vec<FolderUsage>,
    pub error: Option<String>,
}

impl StorageUsageProgress {
    fn new_running() -> Self {
        Self {
            running: true,
            started_at: Utc::now(),
            finished_at: None,
            current_prefix: None,
            scanned_objects: 0,
            folders: Vec::new(),
            error: None,
        }
    }
}

struct Session {
    db: DB,
    owner: String,
    progress: RwLock<StorageUsageProgress>,
}

impl Session {
    async fn update<F: FnOnce(&mut StorageUsageProgress)>(&self, f: F) {
        let snapshot = {
            let mut p = self.progress.write().await;
            f(&mut p);
            p.clone()
        };
        if let Err(e) =
            background_task::update_state(&self.db, TASK_NAME, &self.owner, &snapshot).await
        {
            tracing::warn!("storage usage: failed to persist progress: {e:#}");
        }
    }

    async fn release(&self) {
        let snapshot = {
            let mut p = self.progress.write().await;
            p.running = false;
            p.finished_at = Some(Utc::now());
            p.clone()
        };
        if let Err(e) = background_task::release(&self.db, TASK_NAME, &self.owner, &snapshot).await
        {
            tracing::warn!("storage usage: failed to release lease: {e:#}");
        }
    }
}

pub async fn try_start(db: &DB) -> error::Result<()> {
    let claimed = background_task::try_claim(
        db,
        TASK_NAME,
        &*INSTANCE_NAME,
        &StorageUsageProgress::new_running(),
    )
    .await?;
    if !claimed {
        return Err(error::Error::BadRequest(
            "Storage usage computation is already running".to_string(),
        ));
    }
    Ok(())
}

pub async fn get_status(db: &DB) -> error::Result<Option<StorageUsageProgress>> {
    let row = background_task::get(db, TASK_NAME).await?;
    let Some(r) = row else { return Ok(None) };
    match serde_json::from_value::<StorageUsageProgress>(r.value) {
        Ok(mut p) => {
            p.running = r.running;
            Ok(Some(p))
        }
        Err(e) => Err(error::Error::internal_err(format!(
            "deserialize storage usage progress: {e:#}"
        ))),
    }
}

async fn compute_inner(session: &Session, store: Arc<dyn ObjectStore>) -> error::Result<()> {
    let list_result = store
        .list_with_delimiter(None)
        .await
        .map_err(|e| error::Error::internal_err(format!("list_with_delimiter at root: {e:#}")))?;

    let root_size: u64 = list_result.objects.iter().map(|o| o.size).sum();
    if root_size > 0 {
        let root_count = list_result.objects.len() as u64;
        session
            .update(|p| {
                p.folders.push(FolderUsage {
                    prefix: "(root files)".to_string(),
                    size: root_size,
                    partial: false,
                });
                p.scanned_objects = p.scanned_objects.saturating_add(root_count);
            })
            .await;
    }

    /// Keep heartbeat well below STALE_HEARTBEAT_SECS/2 so a slow S3 LIST
    /// (large bucket, rate-limited) can't let another replica reclaim.
    const HEARTBEAT_SECS: u64 = 30;
    const PROGRESS_TICK: u64 = 1_000;

    for prefix in list_result.common_prefixes {
        let prefix_str = prefix.to_string();
        session
            .update(|p| p.current_prefix = Some(prefix_str.clone()))
            .await;

        let mut total_size: u64 = 0;
        let mut local_count: u64 = 0;
        let mut partial = false;
        let mut last_heartbeat = std::time::Instant::now();
        let mut stream = store.list(Some(&prefix));

        while let Some(item) = stream.next().await {
            match item {
                Ok(meta) => {
                    total_size += meta.size;
                    local_count += 1;
                    if local_count % PROGRESS_TICK == 0
                        || last_heartbeat.elapsed()
                            >= std::time::Duration::from_secs(HEARTBEAT_SECS)
                    {
                        last_heartbeat = std::time::Instant::now();
                        session
                            .update(|p| {
                                p.scanned_objects = p.scanned_objects.saturating_add(PROGRESS_TICK)
                            })
                            .await;
                    }
                }
                Err(e) => {
                    tracing::warn!("storage usage: error listing {prefix_str}: {e:#}");
                    partial = true;
                    break;
                }
            }
        }

        let remainder = local_count % PROGRESS_TICK;
        if remainder > 0 {
            session
                .update(|p| p.scanned_objects = p.scanned_objects.saturating_add(remainder))
                .await;
        }

        session
            .update(|p| {
                p.folders
                    .push(FolderUsage { prefix: prefix_str, size: total_size, partial });
                // Keep folders sorted by size so UI stays stable as we stream results.
                p.folders.sort_by(|a, b| b.size.cmp(&a.size));
            })
            .await;
    }

    session.update(|p| p.current_prefix = None).await;

    Ok(())
}

pub fn spawn_compute(db: DB) {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    tokio::spawn(async move {
        let session = Arc::new(Session {
            db: db.clone(),
            owner: INSTANCE_NAME.clone(),
            progress: RwLock::new(StorageUsageProgress::new_running()),
        });

        let s = session.clone();
        let task = async move {
            let store = match windmill_object_store::get_object_store().await {
                Some(st) => st,
                None => {
                    s.update(|p| p.error = Some("Object storage is not configured".to_string()))
                        .await;
                    return;
                }
            };
            if let Err(e) = compute_inner(&s, store).await {
                s.update(|p| p.error = Some(format!("{e:#}"))).await;
            }
        };

        if let Err(panic) = AssertUnwindSafe(task).catch_unwind().await {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            session
                .update(|p| p.error = Some(format!("task panicked: {msg}")))
                .await;
        }

        session.release().await;
    });
}
