use crate::db::ApiAuthed;
use crate::HTTP_CLIENT;
use axum::{
    extract::{Json, Path},
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
        .route("/projects/{slug}/export", get(get_project_export))
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
    Path(workspace): Path<String>,
    Json(body): Json<PublishDraftBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        "/projects",
        &workspace,
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
    project_slug: String,
}

async fn publish_script(
    authed: ApiAuthed,
    Path(workspace): Path<String>,
    Json(body): Json<PublishScriptBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub("/scripts/add", &workspace, &body).await
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
    project_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
}

async fn publish_flow(
    authed: ApiAuthed,
    Path(workspace): Path<String>,
    Json(body): Json<PublishFlowBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub("/flows", &workspace, &body).await
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
    project_slug: String,
}

async fn publish_app(
    authed: ApiAuthed,
    Path(workspace): Path<String>,
    Json(body): Json<PublishAppBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub("/apps", &workspace, &body).await
}

#[derive(Deserialize, Serialize)]
struct PublishRawAppBody {
    raw: String,
    apps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_embed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    project_slug: String,
}

async fn publish_raw_app(
    authed: ApiAuthed,
    Path(workspace): Path<String>,
    Json(body): Json<PublishRawAppBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub("/raw_apps", &workspace, &body).await
}

#[derive(Deserialize, Serialize)]
struct RawAppEmbedBody {
    // No skip_serializing_if: `null` must reach the Hub to clear the embed (unpublish).
    external_embed_url: Option<String>,
    project_slug: String,
}

async fn publish_raw_app_embed(
    authed: ApiAuthed,
    Path((workspace, id)): Path<(String, i64)>,
    Json(body): Json<RawAppEmbedBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(&format!("/raw_apps/{}/embed", id), &workspace, &body).await
}

#[derive(Deserialize, Serialize)]
struct RecordingBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
    project_slug: String,
}

async fn publish_script_recording(
    authed: ApiAuthed,
    Path((workspace, ask_id)): Path<(String, i64)>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(&format!("/scripts/{}/recording", ask_id), &workspace, &body).await
}

async fn publish_flow_recording(
    authed: ApiAuthed,
    Path((workspace, flow_id)): Path<(String, i64)>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(&format!("/flows/{}/recording", flow_id), &workspace, &body).await
}

#[derive(Deserialize, Serialize)]
struct PublishResourceTypeBody {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app: Option<String>,
    project_slug: String,
}

async fn publish_resource_type(
    authed: ApiAuthed,
    Path(workspace): Path<String>,
    Json(body): Json<PublishResourceTypeBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        &format!("/projects/{}/resource_types", body.project_slug),
        &workspace,
        &body,
    )
    .await
}

#[derive(Deserialize, Serialize)]
struct PublishResourceBody {
    path: String,
    resource_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct PublishResourcesBody {
    resources: Vec<PublishResourceBody>,
    project_slug: String,
}

async fn publish_resources(
    authed: ApiAuthed,
    Path(workspace): Path<String>,
    Json(body): Json<PublishResourcesBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        &format!("/projects/{}/resources", body.project_slug),
        &workspace,
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
    Path(workspace): Path<String>,
    Json(body): Json<PublishTriggersBody>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    forward_to_hub(
        &format!("/projects/{}/triggers", body.project_slug),
        &workspace,
        &body,
    )
    .await
}

async fn get_project_export(
    authed: ApiAuthed,
    Path((workspace, slug)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    require_admin(authed.is_admin, &authed.username)?;
    get_from_hub(&format!("/projects/{}/export", slug), &workspace).await
}

async fn get_from_hub(path: &str, source_id: &str) -> Result<(StatusCode, String), Error> {
    let hub_token = std::env::var("HUB_DEV_TOKEN")
        .map_err(|_| Error::BadRequest("HUB_DEV_TOKEN not set".into()))?;

    let url = format!("{}{}", **HUB_BASE_URL.load(), path);

    let res = HTTP_CLIENT
        .get(&url)
        .query(&[("source_id", source_id)])
        .bearer_auth(&hub_token)
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
    body: &T,
) -> Result<(StatusCode, String), Error> {
    let hub_token = std::env::var("HUB_DEV_TOKEN")
        .map_err(|_| Error::BadRequest("HUB_DEV_TOKEN not set".into()))?;

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
        .bearer_auth(&hub_token)
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
