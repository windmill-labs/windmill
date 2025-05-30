#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::apps_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn global_unauthed_service() -> Router {
    Router::new()
}
