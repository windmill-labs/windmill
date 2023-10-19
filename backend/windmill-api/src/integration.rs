use crate::{db::ApiAuthed, HTTP_CLIENT};
use axum::{body::StreamBody, extract::Query, response::IntoResponse, routing::get, Router};
use windmill_common::{error::Error, utils::query_elems_from_hub};

pub fn global_service() -> Router {
    Router::new().route("/hub/list", get(list_hub_integrations))
}

#[derive(serde::Deserialize)]
struct ListHubIntegrationsQuery {
    kind: Option<String>,
}
async fn list_hub_integrations(
    ApiAuthed { email, .. }: ApiAuthed,
    Query(query): Query<ListHubIntegrationsQuery>,
) -> impl IntoResponse {
    let mut query_params = vec![];

    if let Some(kind) = query.kind {
        query_params.push(("kind", kind));
    }

    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        "https://hub.windmill.dev/integrations/list",
        &email,
        Some(query_params),
    )
    .await?;
    Ok::<_, Error>((
        status_code,
        headers,
        StreamBody::new(response.bytes_stream()),
    ))
}
