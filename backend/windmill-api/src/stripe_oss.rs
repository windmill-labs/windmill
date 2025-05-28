use axum::Router;

pub fn add_stripe_routes(router: Router) -> Router {
    crate::stripe_ee::add_stripe_routes(router)
}