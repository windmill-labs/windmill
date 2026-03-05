#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::volumes_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(not(feature = "private"))]
#[allow(dead_code)]
pub fn agent_workspaced_service() -> Router {
    Router::new()
}
