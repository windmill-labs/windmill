use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use http::Method;
use serde::{Deserialize, Serialize};
use windmill_common::{error::JsonResult, DB};

use crate::{get_workspace_integration, External, ServiceName};

use super::Google;

fn escape_drive_query(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleCalendarEntry {
    pub id: String,
    pub summary: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarListResponse {
    #[serde(default)]
    items: Vec<GoogleCalendarListItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarListItem {
    id: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleDriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(default)]
    pub is_folder: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleDriveFilesResponse {
    pub files: Vec<GoogleDriveFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveApiResponse {
    #[serde(default)]
    files: Vec<DriveApiFile>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveApiFile {
    id: String,
    name: String,
    mime_type: String,
}

#[derive(Debug, Deserialize)]
pub struct DriveFilesQuery {
    pub q: Option<String>,
    pub parent_id: Option<String>,
    pub page_token: Option<String>,
    #[serde(default)]
    pub shared_with_me: bool,
}

async fn list_calendars(
    Extension(handler): Extension<Arc<Google>>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<GoogleCalendarEntry>> {
    get_workspace_integration(&db, &workspace_id, ServiceName::Google).await?;

    let url = format!(
        "{}/users/me/calendarList",
        super::endpoints::CALENDAR_API_BASE
    );

    let response: GoogleCalendarListResponse = handler
        .http_client_request::<_, ()>(&url, Method::GET, &workspace_id, &db, None, None)
        .await?;

    let calendars = response
        .items
        .into_iter()
        .map(|item| GoogleCalendarEntry {
            id: item.id,
            summary: item.summary,
            primary: item.primary,
        })
        .collect();

    Ok(Json(calendars))
}

async fn list_drive_files(
    Extension(handler): Extension<Arc<Google>>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<DriveFilesQuery>,
) -> JsonResult<GoogleDriveFilesResponse> {
    get_workspace_integration(&db, &workspace_id, ServiceName::Google).await?;

    let drive_query = if query.shared_with_me {
        "sharedWithMe = true and trashed = false".to_string()
    } else if let Some(ref parent_id) = query.parent_id {
        format!(
            "'{}' in parents and trashed = false",
            escape_drive_query(parent_id)
        )
    } else if let Some(ref search) = query.q {
        format!(
            "name contains '{}' and trashed = false",
            escape_drive_query(search)
        )
    } else {
        "'root' in parents and trashed = false".to_string()
    };

    let mut url = format!(
        "{}/files?q={}&fields=files(id,name,mimeType),nextPageToken&pageSize=50&orderBy=folder,name&supportsAllDrives=true&includeItemsFromAllDrives=true",
        super::endpoints::DRIVE_API_BASE,
        urlencoding::encode(&drive_query)
    );

    if let Some(ref page_token) = query.page_token {
        url.push_str(&format!("&pageToken={}", urlencoding::encode(page_token)));
    }

    let response: DriveApiResponse = handler
        .http_client_request::<_, ()>(&url, Method::GET, &workspace_id, &db, None, None)
        .await?;

    let files = response
        .files
        .into_iter()
        .map(|f| {
            let is_folder = f.mime_type == "application/vnd.google-apps.folder";
            GoogleDriveFile { id: f.id, name: f.name, mime_type: f.mime_type, is_folder }
        })
        .collect();

    Ok(Json(GoogleDriveFilesResponse {
        files,
        next_page_token: response.next_page_token,
    }))
}

#[derive(Debug, Serialize)]
pub struct SharedDriveEntry {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedDrivesApiResponse {
    #[serde(default)]
    drives: Vec<SharedDriveApiEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedDriveApiEntry {
    id: String,
    name: String,
}

async fn list_shared_drives(
    Extension(handler): Extension<Arc<Google>>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<SharedDriveEntry>> {
    get_workspace_integration(&db, &workspace_id, ServiceName::Google).await?;

    let url = format!(
        "{}/drives?pageSize=100&fields=drives(id,name)",
        super::endpoints::DRIVE_API_BASE
    );

    let response: SharedDrivesApiResponse = handler
        .http_client_request::<_, ()>(&url, Method::GET, &workspace_id, &db, None, None)
        .await?;

    let drives = response
        .drives
        .into_iter()
        .map(|d| SharedDriveEntry { id: d.id, name: d.name })
        .collect();

    Ok(Json(drives))
}

pub fn google_routes(service: Google) -> Router {
    let service = Arc::new(service);
    Router::new()
        .route("/calendars", get(list_calendars))
        .route("/drive/files", get(list_drive_files))
        .route("/drive/shared_drives", get(list_shared_drives))
        .layer(Extension(service))
}
