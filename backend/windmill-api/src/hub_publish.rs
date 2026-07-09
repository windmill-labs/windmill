use crate::auth::Tokened;
use crate::db::ApiAuthed;
use crate::HTTP_CLIENT;
use axum::{
    extract::{Json, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use windmill_common::{
    error::{to_anyhow, Error},
    utils::require_admin,
    HUB_BASE_URL,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/publish_draft", post(publish_draft))
        .route("/scripts", post(publish_script))
        .route("/flows", post(publish_flow))
        .route("/apps", post(publish_app))
        .route("/raw_apps", post(publish_raw_app))
        .route("/raw_apps/{id}/embed", post(publish_raw_app_embed))
        .route(
            "/scripts/{ask_id}/recording",
            post(publish_script_recording),
        )
        .route("/flows/{flow_id}/recording", post(publish_flow_recording))
        .route("/resource_types", post(publish_resource_type))
        .route("/resources", post(publish_resources))
        .route("/triggers", post(publish_triggers))
        .route("/migrations", post(publish_migrations))
        .route("/projects/{slug}/export", get(get_project_export))
        .route("/projects/{slug}/submit", post(submit_project))
        .route("/project", get(get_project_by_source))
}

// A workspace can publish one Hub project per folder. The stable, never-mutated
// link key is `workspace_id:folder_name` (folder name is the path segment and is
// never renamed — only display_name changes). `:` is safe: neither workspace ids
// nor folder names (alphanumeric, underscore, hyphen) contain it.
#[derive(Deserialize)]
struct HubScope {
    folder: String,
}

// Export is owner-scoped only when re-exporting your own draft; approved projects
// are public, so folder is optional there.
#[derive(Deserialize)]
struct HubScopeOpt {
    folder: Option<String>,
}

fn validate_folder(folder: &str) -> Result<(), Error> {
    let ok = !folder.is_empty()
        && folder.len() <= 255
        && folder
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(Error::BadRequest(format!("invalid folder: {folder}")))
    }
}

fn source_key(workspace: &str, folder: &str) -> Result<String, Error> {
    validate_folder(folder)?;
    Ok(format!("{workspace}:{folder}"))
}

fn validate_project_slug(slug: &str) -> Result<(), Error> {
    let ok = slug.len() >= 3
        && slug.len() <= 50
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(Error::BadRequest(format!("invalid project slug: {slug}")))
    }
}

#[derive(Deserialize)]
struct PublishDraftBody {
    slug: String,
    name: String,
    summary: String,
    #[serde(default)]
    readme: Option<String>,
}

#[derive(Serialize)]
struct HubProjectBody {
    slug: String,
    name: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    readme: Option<String>,
}

async fn publish_draft(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishDraftBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        "/projects",
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &HubProjectBody {
            slug: body.slug,
            name: body.name,
            summary: body.summary,
            readme: body.readme,
        },
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishScriptBody {
    summary: String,
    app: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    content: String,
    language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lockfile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    project_slug: String,
}

async fn publish_script(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishScriptBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        "/scripts/add",
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishFlowInner {
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct PublishFlowBody {
    flow: PublishFlowInner,
    apps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    project_slug: String,
}

async fn publish_flow(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishFlowBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        "/flows",
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishAppBody {
    app: serde_json::Value,
    apps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    project_slug: String,
}

async fn publish_app(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishAppBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        "/apps",
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishRawAppBody {
    raw: String,
    apps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    project_slug: String,
}

async fn publish_raw_app(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishRawAppBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        "/raw_apps",
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct RawAppEmbedBody {
    // No skip_serializing_if: `null` must reach the Hub to clear the embed (unpublish).
    external_embed_url: Option<String>,
    project_slug: String,
}

async fn publish_raw_app_embed(
    authed: ApiAuthed,
    tokened: Tokened,
    Path((workspace, id)): Path<(String, i64)>,
    Query(scope): Query<HubScope>,
    Json(body): Json<RawAppEmbedBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        &format!("/raw_apps/{}/embed", id),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct RecordingBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
    project_slug: String,
}

async fn publish_script_recording(
    authed: ApiAuthed,
    tokened: Tokened,
    Path((workspace, ask_id)): Path<(String, i64)>,
    Query(scope): Query<HubScope>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        &format!("/scripts/{}/recording", ask_id),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

async fn publish_flow_recording(
    authed: ApiAuthed,
    tokened: Tokened,
    Path((workspace, flow_id)): Path<(String, i64)>,
    Query(scope): Query<HubScope>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        &format!("/flows/{}/recording", flow_id),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishResourceTypeBody {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    project_slug: String,
}

async fn publish_resource_type(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishResourceTypeBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    validate_project_slug(&body.project_slug)?;
    forward_to_hub(
        &format!("/projects/{}/resource_types", body.project_slug),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishResourceBody {
    path: String,
    resource_type: String,
}

#[derive(Deserialize, Serialize)]
struct PublishResourcesBody {
    resources: Vec<PublishResourceBody>,
    project_slug: String,
}

async fn publish_resources(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishResourcesBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    validate_project_slug(&body.project_slug)?;
    forward_to_hub(
        &format!("/projects/{}/resources", body.project_slug),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishTriggerBody {
    path: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    script_ask_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flow_id: Option<i64>,
}

#[derive(Deserialize, Serialize)]
struct PublishTriggersBody {
    triggers: Vec<PublishTriggerBody>,
    project_slug: String,
}

async fn publish_triggers(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishTriggersBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    validate_project_slug(&body.project_slug)?;
    forward_to_hub(
        &format!("/projects/{}/triggers", body.project_slug),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

// One best-effort data table migration attached to a project (per data table).
#[derive(Deserialize, Serialize)]
struct PublishMigrationBody {
    datatable_name: String,
    sql: String,
    #[serde(default)]
    sql_down: String,
    enabled: bool,
}

#[derive(Deserialize, Serialize)]
struct PublishMigrationsBody {
    migrations: Vec<PublishMigrationBody>,
    project_slug: String,
}

async fn publish_migrations(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
    Json(body): Json<PublishMigrationsBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    validate_project_slug(&body.project_slug)?;
    forward_to_hub(
        &format!("/projects/{}/migrations", body.project_slug),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &body,
    )
    .await
}

async fn get_project_export(
    authed: ApiAuthed,
    tokened: Tokened,
    Path((workspace, slug)): Path<(String, String)>,
    Query(scope): Query<HubScopeOpt>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    validate_project_slug(&slug)?;
    let source = match scope.folder {
        Some(folder) => source_key(&workspace, &folder)?,
        None => String::new(),
    };
    get_from_hub(
        &format!("/projects/{}/export", slug),
        &source,
        &tokened.token,
    )
    .await
}

async fn get_project_by_source(
    authed: ApiAuthed,
    tokened: Tokened,
    Path(workspace): Path<String>,
    Query(scope): Query<HubScope>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    get_from_hub(
        "/projects/by_source",
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
    )
    .await
}

async fn submit_project(
    authed: ApiAuthed,
    tokened: Tokened,
    Path((workspace, slug)): Path<(String, String)>,
    Query(scope): Query<HubScope>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    validate_project_slug(&slug)?;
    forward_to_hub(
        &format!("/projects/{}/submit", slug),
        &source_key(&workspace, &scope.folder)?,
        &tokened.token,
        &serde_json::json!({}),
    )
    .await
}

// The Hub has no auth of its own: it validates bearer tokens by calling this
// instance's /api/users/whoami. Forwarding the caller's own token logs them in
// on the Hub as themselves (account auto-created on first use).
async fn get_from_hub(
    path: &str,
    source_id: &str,
    token: &str,
) -> Result<(StatusCode, String), Error> {
    let url = format!("{}{}", **HUB_BASE_URL.load(), path);

    let res = HTTP_CLIENT
        .get(&url)
        .query(&[("source_id", source_id)])
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("hub request failed: {e}")))?;

    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let text = res
        .text()
        .await
        .map_err(|e| Error::InternalErr(format!("hub response read failed: {e}")))?;

    Ok((status, text))
}

async fn forward_to_hub<T: Serialize>(
    path: &str,
    source_id: &str,
    token: &str,
    body: &T,
) -> Result<(StatusCode, String), Error> {
    let url = format!("{}{}", **HUB_BASE_URL.load(), path);

    let mut payload = serde_json::to_value(body).map_err(to_anyhow)?;
    let obj = payload
        .as_object_mut()
        .ok_or_else(|| Error::internal_err("hub publish body must be a JSON object".to_string()))?;
    obj.insert(
        "source_id".to_string(),
        serde_json::Value::String(source_id.to_string()),
    );

    let res = HTTP_CLIENT
        .post(&url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("hub request failed: {e}")))?;

    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let text = res
        .text()
        .await
        .map_err(|e| Error::InternalErr(format!("hub response read failed: {e}")))?;

    Ok((status, text))
}
