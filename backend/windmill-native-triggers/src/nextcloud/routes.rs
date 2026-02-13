use std::{collections::HashMap, sync::Arc};

use axum::{extract::Path, routing::get, Extension, Json, Router};
use http::Method;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
    DB,
};

use crate::{
    get_workspace_integration,
    nextcloud::{NextCloudEventType, OcsResponse},
    External, ServiceName,
};
use windmill_api_auth::ApiAuthed;

async fn list_available_events<T: External>(
    authed: ApiAuthed,
    Extension(handler): Extension<Arc<T>>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<NextCloudEventType>> {
    let mut tx = user_db.clone().begin(&authed).await?;
    let integration =
        get_workspace_integration(&mut *tx, &workspace_id, ServiceName::Nextcloud).await?;

    let base_url = integration
        .oauth_data
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let url = format!(
        "{}/ocs/v2.php/apps/integration_windmill/api/v1/list/events",
        base_url,
    );

    let mut headers = HashMap::new();
    headers.insert("OCS-APIRequest".to_string(), "true".to_string());

    let ocs_response = handler
        .http_client_request::<OcsResponse, ()>(
            &url,
            Method::GET,
            &workspace_id,
            &mut *tx,
            &db,
            Some(headers),
            None,
        )
        .await?;
    tx.commit().await?;

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
