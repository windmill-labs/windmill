use sqlx::{Pool, Postgres};
use windmill_common::worker::{
    DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES, FORK_WORKSPACE_TAG_USE_PARENT,
};

/// Returns `Some(effective_workspace_id)` if jobs of `workspace_id` should use workspace-specific
/// tags, where `effective_workspace_id` is the ID to embed in the tag (possibly the parent for
/// forks). Returns `None` when default (non-workspaced) tags should be used.
pub async fn per_workspace_tag(workspace_id: &str, db: &Pool<Postgres>) -> Option<String> {
    // Fast path: global toggle off -> no workspacing at all.
    if !DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    // Resolve the effective workspace ID. For forks configured to inherit the parent's tags, look
    // up the parent. This DB hit only occurs for forks, so it is not on the hot path for regular
    // workspaces.
    // TODO(perf): if this becomes a bottleneck, add a TTL cache of fork-id -> parent-id.
    let effective_ws_id: String = if workspace_id.starts_with("wm-fork-")
        && FORK_WORKSPACE_TAG_USE_PARENT.load(std::sync::atomic::Ordering::Relaxed)
    {
        match sqlx::query_scalar!(
            "SELECT parent_workspace_id FROM workspace WHERE id = $1",
            workspace_id
        )
        .fetch_optional(db)
        .await
        {
            Ok(Some(Some(parent))) => parent,
            _ => workspace_id.to_string(), // no parent found -> fall back to fork's own id
        }
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
