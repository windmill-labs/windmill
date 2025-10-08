use axum::{extract::Path, routing::get, Extension, Json, Router};
use reqwest::Client;
use serde::Deserialize;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    DB,
};

use crate::{
    db::ApiAuthed,
    native_triggers::nextcloud::{NextCloudEventType, NextCloudResource},
    resources::try_get_resource_from_db_as,
};

#[derive(Deserialize)]
struct EventsQuery {
    resource_path: String,
}

pub async fn get_available_events(auth: &NextCloudResource) -> Result<Vec<NextCloudEventType>> {
    let response = Client::new()
        .get(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/events",
            auth.base_url
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to get NextCloud events: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "NextCloud API error ({}): {}",
            status, body
        )));
    }

    #[derive(Deserialize)]
    struct OcsResponse {
        ocs: OcsData,
    }

    #[derive(Deserialize)]
    struct OcsData {
        data: Vec<NextCloudEventType>,
    }

    let ocs_response: OcsResponse = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    Ok(ocs_response.ocs.data)
}

async fn list_available_events(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Path(resource_path): Path<String>,
) -> JsonResult<Vec<NextCloudEventType>> {
    let auth = try_get_resource_from_db_as::<NextCloudResource>(
        &authed,
        Some(user_db),
        &db,
        &resource_path,
        &workspace_id,
    )
    .await?;

    let events = get_available_events(&auth).await?;
    Ok(Json(events))
}

pub fn nextcloud_routes() -> Router {
    Router::new().route("/events/:workspace_id/*resource_path", get(list_available_events))
}
