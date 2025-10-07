use axum::{extract::{Path, Query}, routing::get, Extension, Json, Router};
use serde::Deserialize;
use windmill_common::{
    error::{Error, JsonResult},
    DB,
};

use crate::{
    db::ApiAuthed,
    native_triggers::nextcloud::{client, NextCloudClient, NextCloudEventType},
};

#[derive(Deserialize)]
struct EventsQuery {
    resource_path: String,
}

async fn list_available_events(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> JsonResult<Vec<NextCloudEventType>> {
    // Fetch the resource
    let auth = crate::resources::get_resource_value_interpolated_internal(
        &authed,
        None,
        &db,
        &workspace_id,
        &query.resource_path,
        None,
        "",
        false,
    )
    .await?
    .ok_or_else(|| Error::NotFound(format!("Resource not found: {}", query.resource_path)))?;

    let auth: client::NextCloudResource = serde_json::from_value(auth)
        .map_err(|e| Error::BadConfig(format!("Invalid NextCloud resource format: {}", e)))?;

    let events = NextCloudClient::get_available_events(&auth).await?;
    Ok(Json(events))
}

pub fn nextcloud_routes() -> Router {
    Router::new().route("/events", get(list_available_events))
}
