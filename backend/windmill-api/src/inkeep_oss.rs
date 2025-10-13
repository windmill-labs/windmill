#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::inkeep_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new()
}
