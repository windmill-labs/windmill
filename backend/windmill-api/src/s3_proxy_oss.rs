#[cfg(all(feature = "private", feature = "parquet"))]
#[allow(unused)]
pub use crate::s3_proxy_ee::*;

#[cfg(not(all(feature = "private", feature = "parquet")))]
pub fn workspaced_unauthed_service() -> axum::Router {
    axum::Router::new()
}
