use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use http::Method;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
    DB,
};

use windmill_api_auth::ApiAuthed;

use crate::{
    decrypt_oauth_data, map_external_error,
    nextcloud::{NextCloudEventType, OcsResponse},
    picker_connection, ConnectionQuery, External, OAuthConfig, ServiceName,
};

async fn list_available_events<T: External>(
    authed: ApiAuthed,
    Extension(handler): Extension<Arc<T>>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ConnectionQuery>,
) -> JsonResult<Vec<NextCloudEventType>> {
    let connection_path = picker_connection(
        &authed,
        &db,
        user_db,
        &workspace_id,
        ServiceName::Nextcloud,
        query.connection_path.as_deref(),
    )
    .await?;
    let connection: OAuthConfig = decrypt_oauth_data(
        &db,
        &workspace_id,
        ServiceName::Nextcloud,
        Some(&connection_path),
    )
    .await?;

    let url = format!(
        "{}/ocs/v2.php/apps/integration_windmill/api/v1/list/events",
        connection.base_url,
    );

    let mut headers = HashMap::new();
    headers.insert("OCS-APIRequest".to_string(), "true".to_string());

    let ocs_response = handler
        .http_client_request::<OcsResponse, ()>(
            &url,
            Method::GET,
            &workspace_id,
            &connection_path,
            &db,
            Some(headers),
            None,
        )
        .await
        .map_err(map_external_error)?;

    let events = serde_json::from_str(&ocs_response.ocs.data)
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud events data: {}", e)))?;

    Ok(Json(events))
}

pub fn nextcloud_routes<T: External>(service: T) -> Router {
    let service = Arc::new(service);
    Router::new()
        .route("/events", get(list_available_events::<T>))
        .layer(Extension(service))
}
