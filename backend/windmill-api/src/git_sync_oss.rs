#[cfg(not(feature = "private"))]
use axum::routing::Router;

#[cfg(not(feature = "private"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new()
}
