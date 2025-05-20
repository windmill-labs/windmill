#[cfg(feature = "private")]
use crate::apps_ee;

use axum::Router;

pub fn global_unauthed_service() -> Router {
    #[cfg(feature = "private")]
    {
        return apps_ee::global_unauthed_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}
