#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::inkeep_ee::*;

#[cfg(not(feature = "private"))]
use axum::{routing::post, Router};

#[cfg(not(feature = "private"))]
use windmill_common::error::Error;

#[cfg(not(feature = "private"))]
pub fn global_service() -> Router {
    Router::new().route("/", post(inkeep_not_available))
}

#[cfg(not(feature = "private"))]
async fn inkeep_not_available() -> windmill_common::error::Result<()> {
    Err(Error::Generic(
        http::StatusCode::FORBIDDEN,
        "Inkeep AI documentation assistant is only available in Windmill Enterprise Edition"
            .to_string(),
    ))
}
