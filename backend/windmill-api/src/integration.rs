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

/// Axum percent-decodes a path parameter, so an interpolated slug carrying `..`, `?`
/// or `#` re-targets the proxied GET at another path on the hub origin — with the
/// instance's hub credentials attached. Slugs are `[A-Za-z0-9_-]`; reject the rest.
fn is_hub_integration_slug(app: &str) -> bool {
    !app.is_empty()
        && app.len() <= 64
        && app
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Everything a caller needs to write code against one integration: its resource
/// types, the provider knowledge the content repo authored, and facts derived from
/// the shipped scripts. A hub older than the endpoint answers 404, which passes
/// through as-is.
async fn get_hub_integration_meta(
    Path(app): Path<String>,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    if !is_hub_integration_slug(&app) {
        return Err(Error::BadRequest(format!(
            "Not a valid integration name: {app}"
        )));
    }
    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/integrations/{}/meta", **HUB_BASE_URL.load(), app),
        None,
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, response))
}

#[cfg(test)]
mod tests {
    use super::is_hub_integration_slug;

    #[test]
    fn rejects_slugs_that_would_re_target_the_proxied_request() {
        assert!(is_hub_integration_slug("confluence"));
        assert!(is_hub_integration_slug("aws-ses"));
        assert!(is_hub_integration_slug("bamboo_hr"));
        assert!(is_hub_integration_slug("RSS"));

        for escape in [
            "../../scripts/top",
            "confluence?foo=bar",
            "confluence#frag",
            "confluence/meta",
            "",
        ] {
            assert!(!is_hub_integration_slug(escape), "accepted {escape}");
        }
    }
}
