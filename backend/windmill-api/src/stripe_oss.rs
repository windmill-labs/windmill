#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::stripe_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;

#[cfg(not(feature = "private"))]
pub fn add_stripe_routes(router: Router) -> Router {
    return router;
}
