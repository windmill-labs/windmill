#[cfg(feature = "private")]
use crate::stripe_ee;

use axum::Router;

pub fn add_stripe_routes(router: Router) -> Router {
    #[cfg(feature = "private")]
    {
        return stripe_ee::add_stripe_routes(router);
    }
    #[cfg(not(feature = "private"))]
    {
        return router;
    }
}
