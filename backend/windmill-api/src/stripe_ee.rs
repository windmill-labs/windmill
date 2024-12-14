#[cfg(feature = "stripe")]
use axum::Router;

#[cfg(feature = "stripe")]
pub fn add_stripe_routes(router: Router) -> Router {
    return router;
}
