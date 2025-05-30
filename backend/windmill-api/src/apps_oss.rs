#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn global_unauthed_service() -> Router {
    Router::new()
}
