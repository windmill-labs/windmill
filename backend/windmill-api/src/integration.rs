use crate::{db::DB, HTTP_CLIENT};
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use windmill_common::{error::Error, utils::query_elems_from_hub, HUB_BASE_URL};

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_integrations))
        .route("/hub/{app}/meta", get(get_hub_integration_meta))
}

#[derive(serde::Deserialize)]
struct ListHubIntegrationsQuery {
    kind: Option<String>,
}
async fn list_hub_integrations(
    Query(query): Query<ListHubIntegrationsQuery>,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    let mut query_params = vec![];

    if let Some(kind) = query.kind {
        query_params.push(("kind", kind));
    }

    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/integrations/list", **HUB_BASE_URL.load()),
        Some(query_params),
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, response))
}

/// Everything a caller needs to write code against one integration: its resource
/// types, the provider knowledge the content repo authored, and facts derived from
/// the shipped scripts. A hub older than the endpoint answers 404, which passes
/// through as-is.
async fn get_hub_integration_meta(
    Path(app): Path<String>,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/integrations/{}/meta", **HUB_BASE_URL.load(), app),
        None,
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, response))
}
