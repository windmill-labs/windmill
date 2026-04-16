use sqlx::{Pool, Postgres};
use windmill_common::worker::{
    DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES, FORK_WORKSPACE_TAG_USE_PARENT,
};
use windmill_common::workspaces::WM_FORK_PREFIX;

const FORK_PARENT_CACHE_TTL_SECS: u64 = 300;

lazy_static::lazy_static! {
    // Cache of fork workspace id -> (parent_workspace_id, cached_at).
    // `parent_workspace_id` is essentially immutable once a fork is created, so a multi-minute TTL
    // is safe. `None` means the lookup found no parent (or the DB call failed); we still cache it
    // briefly so that forks missing a parent do not hammer the DB.
    static ref FORK_PARENT_CACHE: quick_cache::sync::Cache<String, (Option<String>, std::time::Instant)> =
        quick_cache::sync::Cache::new(500);
}

/// Returns `Some(effective_workspace_id)` if jobs of `workspace_id` should use workspace-specific
/// tags, where `effective_workspace_id` is the ID to embed in the tag (possibly the parent for
/// forks). Returns `None` when default (non-workspaced) tags should be used.
pub async fn per_workspace_tag(workspace_id: &str, db: &Pool<Postgres>) -> Option<String> {
    // Fast path: global toggle off -> no workspacing at all.
    if !DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    // Resolve the effective workspace ID. For forks configured to inherit the parent's tags, look
    // up the parent. Regular workspaces avoid both the cache and the DB hit.
    let effective_ws_id: String = if workspace_id.starts_with(WM_FORK_PREFIX)
        && FORK_WORKSPACE_TAG_USE_PARENT.load(std::sync::atomic::Ordering::Relaxed)
    {
        lookup_fork_parent(workspace_id, db)
            .await
            .unwrap_or_else(|| workspace_id.to_string()) // no parent found -> fall back to fork's own id
    } else {
        workspace_id.to_string()
    };

    let per_workspace_workspaces = DEFAULT_TAGS_WORKSPACES.load();
    if per_workspace_workspaces.is_none()
        || (**per_workspace_workspaces)
            .as_ref()
            .unwrap()
            .contains(&effective_ws_id)
    {
        Some(effective_ws_id)
    } else {
        None
    }
}

/// Returns the parent workspace id for a fork, or `None` if the fork has no parent set (or the
/// DB lookup failed). Backed by a short-TTL cache to avoid a DB round-trip per job push.
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
        Ok(Some(Some(parent))) => Some(parent),
        _ => None,
    };

    FORK_PARENT_CACHE.insert(
        fork_id.to_string(),
        (parent.clone(), std::time::Instant::now()),
    );
    parent
}
