use sqlx::{Pool, Postgres};
use windmill_common::worker::{
    DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES, FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX,
};
use windmill_common::workspaces::WM_FORK_PREFIX;

const FORK_PARENT_CACHE_TTL_SECS: u64 = 300;

lazy_static::lazy_static! {
    // Cache of fork workspace id -> (parent_workspace_id, cached_at).
    // `parent_workspace_id` is stable for the lifetime of a fork EXCEPT across attach/detach of a
    // dev workspace, which set/keep it; those paths call `invalidate_fork_parent_cache` so routing
    // doesn't lag. `None` means the lookup found no parent (or the DB call failed); we still cache
    // it briefly so that forks missing a parent do not hammer the DB.
    static ref FORK_PARENT_CACHE: quick_cache::sync::Cache<String, (Option<String>, std::time::Instant)> =
        quick_cache::sync::Cache::new(500);
}

/// Drop the cached fork->parent mapping for a workspace. Call after mutating `parent_workspace_id`
/// (attaching/detaching a dev workspace) so per-workspace job tags resolve to the new parent
/// immediately instead of after the cache TTL.
pub fn invalidate_fork_parent_cache(workspace_id: &str) {
    FORK_PARENT_CACHE.remove(workspace_id);
}

/// Returns `Some(effective_workspace_tag_id)` if jobs of `workspace_id` should use workspace-
/// specific tags, where `effective_workspace_tag_id` is the string embedded in the tag. For forks,
/// this is always the parent workspace id, optionally suffixed with `-fork` (controlled by the
/// `FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX` instance setting) so admins can route fork jobs to
/// dedicated workers. Returns `None` when default (non-workspaced) tags should be used.
pub async fn per_workspace_tag(workspace_id: &str, db: &Pool<Postgres>) -> Option<String> {
    // Fast path: global toggle off -> no workspacing at all.
    if !DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    // Resolve to the parent workspace id when the workspace is a fork or dev workspace (both set
    // parent_workspace_id). The lookup caches its `None` result, so non-forks stay cheap after warmup
    // (and the common case is already short-circuited by the global toggle above).
    let parent = lookup_fork_parent(workspace_id, db).await;
    // A `wm-fork-` workspace can outlive its parent (the FK is `ON DELETE SET NULL`), so keep
    // treating the prefix as fork-ness for the `-fork` suffix even when the parent link is gone.
    let is_fork = parent.is_some() || workspace_id.starts_with(WM_FORK_PREFIX);
    let effective_ws_id: String = parent.unwrap_or_else(|| workspace_id.to_string());

    // Whitelist check is against the resolved (parent) id so that including a parent in the
    // whitelist transparently covers all of its forks.
    let per_workspace_workspaces = DEFAULT_TAGS_WORKSPACES.load();
    let whitelisted = per_workspace_workspaces.is_none()
        || (**per_workspace_workspaces)
            .as_ref()
            .unwrap()
            .contains(&effective_ws_id);

    if !whitelisted {
        return None;
    }

    // For forks, optionally append a `-fork` suffix so all forks of a parent share a common
    // dedicated tag (e.g. `python3-{parent_id}-fork`).
    let append_fork_suffix =
        is_fork && FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX.load(std::sync::atomic::Ordering::Relaxed);

    Some(if append_fork_suffix {
        format!("{}-fork", effective_ws_id)
    } else {
        effective_ws_id
    })
}

/// Returns the parent workspace id for a fork, or `None` if the fork has no parent set. Backed by a
/// short-TTL cache to avoid a DB round-trip per job push. A transient DB error returns `None` for
/// this call but is NOT cached, so the next push retries instead of misrouting a (prefix-less) dev
/// workspace's jobs for the whole TTL.
async fn lookup_fork_parent(fork_id: &str, db: &Pool<Postgres>) -> Option<String> {
    if let Some((parent, cached_at)) = FORK_PARENT_CACHE.get(fork_id) {
        if cached_at.elapsed().as_secs() < FORK_PARENT_CACHE_TTL_SECS {
            return parent;
        }
    }

    let parent = match sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1",
        fork_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(opt) => opt.flatten(),
        Err(e) => {
            tracing::warn!("failed to look up fork parent for {fork_id}: {e:#}");
            return None;
        }
    };

    FORK_PARENT_CACHE.insert(
        fork_id.to_string(),
        (parent.clone(), std::time::Instant::now()),
    );
    parent
}
