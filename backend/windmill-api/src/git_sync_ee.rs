#[cfg(feature = "private")]
use crate::git_sync_ee;

use axum::routing::Router;

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return git_sync_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

pub fn global_service() -> Router {
    #[cfg(feature = "private")]
    {
        return git_sync_ee::global_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}
