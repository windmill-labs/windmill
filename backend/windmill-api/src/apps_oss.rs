use axum::Router;

pub fn global_unauthed_service() -> Router {
    crate::apps_ee::global_unauthed_service()
}