use axum::Router;

pub fn workspaced_service() -> Router {
    crate::git_sync_ee::workspaced_service()
}

pub fn global_service() -> Router {
    crate::git_sync_ee::global_service()
}