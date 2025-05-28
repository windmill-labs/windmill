use axum::{http::Request, middleware::Next, response::Response, Router};

pub fn global_service() -> Router {
    crate::scim_ee::global_service()
}

pub async fn ee() -> String {
    crate::scim_ee::ee().await
}

pub async fn has_scim_token<B>(_request: Request<B>, _next: Next) -> Response {
    crate::scim_ee::has_scim_token(_request, _next).await
}