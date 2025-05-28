use axum::Router;

pub fn global_service() -> Router {
    crate::oidc_ee::global_service()
}

pub fn workspaced_service() -> Router {
    crate::oidc_ee::workspaced_service()
}