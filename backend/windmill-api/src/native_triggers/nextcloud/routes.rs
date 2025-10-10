use axum::{extract::Path, routing::get, Extension, Json, Router};
use reqwest::Client;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    DB,
};

use crate::{
    db::ApiAuthed,
    native_triggers::nextcloud::{NextCloudEventType, NextCloudResource, OcsResponse},
    resources::try_get_resource_from_db_as,
};

pub async fn get_available_events(auth: &NextCloudResource) -> Result<Vec<NextCloudEventType>> {
    let response = Client::new()
        .get(format!(
            "{}/ocs/v2.php/apps/integration_windmill/api/v1/list/events",
            &auth.base_url,
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .header("accept", "application/json")
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

    let ocs_response = response
        .json::<OcsResponse>()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    let events = serde_json::from_str(&ocs_response.ocs.data)
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud events data: {}", e)))?;

    Ok(events)
}

async fn list_available_events(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, resource_path)): Path<(String, String)>,
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
    Router::new().route("/events/*resource_path", get(list_available_events))
}
