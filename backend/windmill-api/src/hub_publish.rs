use crate::HTTP_CLIENT;
use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use windmill_common::{error::Error, HUB_BASE_URL};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/publish_draft", post(publish_draft))
        .route("/scripts", post(publish_script))
        .route("/flows", post(publish_flow))
        .route("/apps", post(publish_app))
        .route("/raw_apps", post(publish_raw_app))
        .route(
            "/scripts/{ask_id}/recording",
            post(publish_script_recording),
        )
        .route("/flows/{flow_id}/recording", post(publish_flow_recording))
        .route("/resource_types", post(publish_resource_type))
        .route("/resources", post(publish_resources))
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
struct HubWorkspaceBody {
    slug: String,
    name: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    readme: Option<String>,
}

async fn publish_draft(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishDraftBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub(
        "/workspaces",
        &HubWorkspaceBody {
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
    workspace_slug: String,
}

async fn publish_script(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishScriptBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub("/scripts/add", &body).await
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
    workspace_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
}

async fn publish_flow(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishFlowBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub("/flows", &body).await
}

#[derive(Deserialize, Serialize)]
struct PublishAppBody {
    app: serde_json::Value,
    apps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    summary: String,
    workspace_slug: String,
}

async fn publish_app(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishAppBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub("/apps", &body).await
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
    workspace_slug: String,
}

async fn publish_raw_app(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishRawAppBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub("/raw_apps", &body).await
}

#[derive(Deserialize, Serialize)]
struct RecordingBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
    workspace_slug: String,
}

async fn publish_script_recording(
    Path((_workspace, ask_id)): Path<(String, i64)>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub(&format!("/scripts/{}/recording", ask_id), &body).await
}

async fn publish_flow_recording(
    Path((_workspace, flow_id)): Path<(String, i64)>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub(&format!("/flows/{}/recording", flow_id), &body).await
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
    workspace_slug: String,
}

async fn publish_resource_type(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishResourceTypeBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub(
        &format!("/workspaces/{}/resource_types", body.workspace_slug),
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
    workspace_slug: String,
}

async fn publish_resources(
    Path(_workspace): Path<String>,
    Json(body): Json<PublishResourcesBody>,
) -> Result<impl IntoResponse, Error> {
    forward_to_hub(
        &format!("/workspaces/{}/resources", body.workspace_slug),
        &body,
    )
    .await
}

async fn forward_to_hub<T: Serialize>(path: &str, body: &T) -> Result<(StatusCode, String), Error> {
    let hub_token = std::env::var("HUB_DEV_TOKEN")
        .map_err(|_| Error::BadRequest("HUB_DEV_TOKEN not set".into()))?;

    let url = format!("{}{}", **HUB_BASE_URL.load(), path);

    let res = HTTP_CLIENT
        .post(&url)
        .bearer_auth(&hub_token)
        .json(body)
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
