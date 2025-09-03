#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::s3_proxy_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn workspaced_unauthed_service() -> Router {
    Router::new()
}
