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

/// Payload meaning "drop every entry". A change to the shape of the tree invalidates an unbounded
/// set of descendants, which is not worth enumerating; a change to what a single id denotes is.
const CLEAR_ALL: &str = "*";

/// Drop the lineage-derived caches of one workspace id in THIS process only: the tag workspace
/// here, and the root workspace a job reads as `WM_ROOT_WORKSPACE`. Both answer a walk up the
/// ancestor chain, so every mutation invalidating one invalidates the other, and both are swept
/// together rather than through parallel call sites. Resolution walks ancestors, so a mutation
/// also invalidates every descendant; sweep them here as well. Replicas need a broadcast to match:
/// [`notify_fork_lineage_reset`] when a subtree moved (attach, detach, archive, rename, a delete
/// that orphans), or [`notify_fork_lineage_change`] when a single id changed what it denotes.
pub fn invalidate_fork_parent_cache(workspace_id: &str) {
    FORK_PARENT_CACHE.remove(workspace_id);
    windmill_common::workspaces::invalidate_root_workspace_cache(workspace_id);
}

/// Apply a broadcast lineage change to this process's caches.
pub fn apply_fork_lineage_change(payload: &str) {
    if payload == CLEAR_ALL {
        FORK_PARENT_CACHE.clear();
        windmill_common::workspaces::clear_root_workspace_cache();
    } else {
        FORK_PARENT_CACHE.remove(payload);
        windmill_common::workspaces::invalidate_root_workspace_cache(payload);
    }
}

/// Broadcast that one workspace id now denotes something else, so every process drops just that
/// entry. Ids are reclaimable, so a deleted fork's mapping must not outlive it: the id can be
/// claimed again under a different parent well inside the cache TTL.
pub async fn notify_fork_lineage_change<'e>(
    executor: impl sqlx::Executor<'e, Database = Postgres>,
    workspace_id: &str,
) -> Result<(), sqlx::Error> {
    broadcast(executor, workspace_id).await
}

/// Broadcast that the shape of the tree changed, so every process drops every entry. Used where a
/// mutation moves an unbounded set of descendants; these are rare admin actions.
pub async fn notify_fork_lineage_reset<'e>(
    executor: impl sqlx::Executor<'e, Database = Postgres>,
) -> Result<(), sqlx::Error> {
    broadcast(executor, CLEAR_ALL).await
}

/// Called after the mutation commits, alongside the local invalidation: a lost event only means
/// replicas wait out the cache TTL, which is what they did before the broadcast existed.
async fn broadcast<'e>(
    executor: impl sqlx::Executor<'e, Database = Postgres>,
    payload: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO notify_event (channel, payload) VALUES ($1, $2)")
        .bind(FORK_LINEAGE_CHANGE_CHANNEL)
        .bind(payload)
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
        // No row means the workspace does not exist yet, or its chain is broken: both are "we
        // cannot resolve this", so fall back to its own id for this call WITHOUT caching. Caching
        // it would pin a fork to its own unserved id for the whole TTL — a rename resolves the new
        // id before the row lands, and every job pushed until the TTL expires would queue.
        Ok(None) => {
            return TagWorkspace {
                id: workspace_id.to_string(),
                is_fork: workspace_id.starts_with(WM_FORK_PREFIX),
            }
        }
        Ok(Some(id)) => {
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
