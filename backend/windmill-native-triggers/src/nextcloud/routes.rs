use std::{collections::HashMap, sync::Arc};

use axum::{extract::Path, routing::get, Extension, Json, Router};
use http::Method;
use windmill_common::{
    error::{Error, JsonResult},
    DB,
};

use crate::{
    get_workspace_integration,
    nextcloud::{NextCloudEventType, OcsResponse},
    External, ServiceName,
};

async fn list_available_events<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<NextCloudEventType>> {
    let integration = get_workspace_integration(&db, &workspace_id, ServiceName::Nextcloud).await?;

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
            &db,
            Some(headers),
            None,
        )
        .await?;

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
