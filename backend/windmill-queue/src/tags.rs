use windmill_common::worker::{DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES};

pub async fn per_workspace_tag(workspace_id: &str) -> bool {
    let per_workspace_workspaces = DEFAULT_TAGS_WORKSPACES.read().await;
    DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed)
        && (per_workspace_workspaces.is_none()
            || per_workspace_workspaces
                .as_ref()
                .unwrap()
                .contains(&workspace_id.to_string()))
}
