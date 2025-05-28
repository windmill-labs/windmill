use axum::Router;

pub fn workspaced_service() -> Router {
    crate::indexer_ee::workspaced_service()
}

pub fn global_service() -> Router {
    crate::indexer_ee::global_service()
}