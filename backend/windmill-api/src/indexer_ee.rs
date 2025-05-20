#[cfg(feature = "private")]
use crate::indexer_ee;

use axum::Router;

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return indexer_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

pub fn global_service() -> Router {
    #[cfg(feature = "private")]
    {
        return indexer_ee::global_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}
