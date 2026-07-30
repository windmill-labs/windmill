use sqlx::{Pool, Postgres};
use windmill_common::worker::{
    DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES, FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX,
};
use windmill_common::workspaces::WM_FORK_PREFIX;

const FORK_PARENT_CACHE_TTL_SECS: u64 = 300;

#[derive(Clone)]
struct TagWorkspace {
    /// The workspace id embedded in this workspace's tags.
    id: String,
    /// Whether the `-fork` suffix may apply: the workspace shares an ancestor's id, or carries the
    /// generated prefix itself.
    is_fork: bool,
}

lazy_static::lazy_static! {
    // Cache of workspace id -> (resolved tag workspace, cached_at).
    // The resolution depends on the whole ancestor chain, so attach/detach of a dev workspace
    // invalidates the mutated workspace AND its descendants; a parentless result is still cached
    // briefly so non-forks (the common case) do not hammer the DB.
    static ref FORK_PARENT_CACHE: quick_cache::sync::Cache<String, (TagWorkspace, std::time::Instant)> =
        quick_cache::sync::Cache::new(500);
}

/// Channel on which a lineage change is broadcast to every server and worker process.
pub const FORK_LINEAGE_CHANGE_CHANNEL: &str = "notify_fork_lineage_change";

/// Drop the cached tag workspace for a workspace. Call after mutating `parent_workspace_id` or
/// `is_dev_workspace` (attaching/detaching a dev workspace) so job tags resolve against the new
/// lineage immediately instead of after the cache TTL. Resolution walks ancestors, so a workspace's
/// descendants must be invalidated too. This only reaches the calling process; pair it with
/// [`notify_fork_lineage_change`] so replicas do not keep serving the old answer.
pub fn invalidate_fork_parent_cache(workspace_id: &str) {
    FORK_PARENT_CACHE.remove(workspace_id);
}

/// Drop every cached tag workspace. Used by the lineage-change listener: resolution depends on a
/// whole ancestor chain, so a mutation invalidates an unbounded set of descendants and clearing all
/// of it costs one lookup per active workspace on the next push. Lineage changes are rare admin
/// actions, so that is cheaper than broadcasting an id per affected workspace and safer than
/// trying to enumerate them.
pub fn clear_fork_parent_cache() {
    FORK_PARENT_CACHE.clear();
}

/// Broadcast a lineage change so every process drops its cached tag workspaces. Called after the
/// mutation commits, alongside the local invalidation: a lost event only means replicas wait out
/// the cache TTL, which is what they did before the broadcast existed.
pub async fn notify_fork_lineage_change<'e>(
    executor: impl sqlx::Executor<'e, Database = Postgres>,
    workspace_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO notify_event (channel, payload) VALUES ($1, $2)")
        .bind(FORK_LINEAGE_CHANGE_CHANNEL)
        .bind(workspace_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// The workspace id embedded in a job's tags, for both the default `{lang}-{ws}` tags and explicit
/// tags containing `$workspace`.
///
/// Resolves to the nearest ancestor (possibly the workspace itself) whose id an admin would
/// actually provision workers for. An ephemeral fork is skipped: forks are created and destroyed
/// continuously under generated `wm-fork-*` ids, so a fork-scoped tag would never be served. A dev
/// workspace stops the walk and keeps its own id — it is long-lived and may have had its own
/// workers before it was attached — unless it too carries the generated prefix.
///
/// Unauthenticated helper: reads workspace hierarchy for any `workspace_id`, so callers must
/// already be authorized for that workspace (or run in trusted server-side code).
pub async fn tag_workspace_id(workspace_id: &str, db: &Pool<Postgres>) -> String {
    lookup_tag_workspace(workspace_id, db).await.id
}

/// Returns `Some(effective_workspace_tag_id)` if jobs of `workspace_id` should use workspace-
/// specific default tags. Returns `None` when default (non-workspaced) tags should be used.
///
/// The id is [`tag_workspace_id`], optionally suffixed with `-fork` (controlled by the
/// `FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX` instance setting) so admins can route fork jobs to
/// dedicated workers.
pub async fn per_workspace_tag(workspace_id: &str, db: &Pool<Postgres>) -> Option<String> {
    // Fast path: global toggle off -> no workspacing at all.
    if !DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    let resolved = lookup_tag_workspace(workspace_id, db).await;

    // Whitelist check is against the resolved id so that including a workspace in the whitelist
    // transparently covers every fork that borrows its id.
    let per_workspace_workspaces = DEFAULT_TAGS_WORKSPACES.load();
    let whitelisted = per_workspace_workspaces.is_none()
        || (**per_workspace_workspaces)
            .as_ref()
            .unwrap()
            .contains(&resolved.id);

    if !whitelisted {
        return None;
    }

    // Only meaningful for workspaces sharing an ancestor's id: the suffix is what separates their
    // jobs from that ancestor's own (e.g. `python3-{parent_id}-fork`).
    let append_fork_suffix = resolved.is_fork
        && FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX.load(std::sync::atomic::Ordering::Relaxed);

    Some(if append_fork_suffix {
        format!("{}-fork", resolved.id)
    } else {
        resolved.id
    })
}

/// Backed by a short-TTL cache to avoid a DB round-trip per job push. A transient DB error resolves
/// to the workspace's own id for this call but is NOT cached, so the next push retries instead of
/// misrouting for the whole TTL.
async fn lookup_tag_workspace(workspace_id: &str, db: &Pool<Postgres>) -> TagWorkspace {
    if let Some((resolved, cached_at)) = FORK_PARENT_CACHE.get(workspace_id) {
        if cached_at.elapsed().as_secs() < FORK_PARENT_CACHE_TTL_SECS {
            return resolved;
        }
    }

    // Walk up to the first ancestor that keeps its own id: a root, or a dev workspace that is not
    // itself under a generated fork id. The depth bound mirrors the other chain walkers as a
    // cycle-safety backstop. A `wm-fork-` workspace orphaned by `ON DELETE SET NULL` has no parent
    // left to borrow from, so it ends the walk on itself.
    let resolved = match sqlx::query_scalar!(
        r#"
            WITH RECURSIVE chain AS (
                SELECT id, parent_workspace_id, is_dev_workspace, 0 AS depth
                FROM workspace WHERE id = $1
                UNION ALL
                SELECT w.id, w.parent_workspace_id, w.is_dev_workspace, chain.depth + 1
                FROM workspace w
                JOIN chain ON w.id = chain.parent_workspace_id
                WHERE chain.depth < 20
            )
            SELECT id AS "id!" FROM chain
            WHERE parent_workspace_id IS NULL
               OR (is_dev_workspace AND id NOT LIKE 'wm-fork-%')
            ORDER BY depth LIMIT 1
        "#,
        workspace_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(row) => {
            let id = row.unwrap_or_else(|| workspace_id.to_string());
            let is_fork = id != workspace_id || workspace_id.starts_with(WM_FORK_PREFIX);
            TagWorkspace { id, is_fork }
        }
        Err(e) => {
            tracing::warn!("failed to resolve tag workspace for {workspace_id}: {e:#}");
            return TagWorkspace {
                id: workspace_id.to_string(),
                is_fork: workspace_id.starts_with(WM_FORK_PREFIX),
            };
        }
    };

    FORK_PARENT_CACHE.insert(
        workspace_id.to_string(),
        (resolved.clone(), std::time::Instant::now()),
    );
    resolved
}
