use crate::auth::Tokened;
use crate::db::ApiAuthed;
use crate::HTTP_CLIENT;
use axum::{
    extract::{DefaultBodyLimit, FromRequestParts, Json, Path, Query, RawPathParams},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Deserializer, Serialize};
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
        .route(
            "/projects/{slug}/pipeline_recording",
            post(publish_pipeline_recording),
        )
        .route(
            "/projects/{slug}/logo",
            post(publish_project_logo).layer(DefaultBodyLimit::max(LOGO_BODY_LIMIT)),
        )
        .route("/resource_types", post(publish_resource_type))
        .route("/resources", post(publish_resources))
        .route("/triggers", post(publish_triggers))
        .route("/migrations", post(publish_migrations))
        .route("/projects/{slug}/export", get(get_project_export))
        .route("/projects/{slug}/submit", post(submit_project))
        .route("/project", get(get_project_by_source))
}

#[derive(Deserialize)]
struct HubScope {
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

/// A Hub project slug that is valid by construction: deserialization (from a
/// request body or a path segment) is the only way to obtain one and it runs
/// `validate_project_slug`, so no handler can forward or interpolate an
/// unvalidated slug into a Hub URL.
#[derive(Serialize)]
#[serde(transparent)]
struct ProjectSlug(String);

impl<'de> Deserialize<'de> for ProjectSlug {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        validate_project_slug(&s).map_err(serde::de::Error::custom)?;
        Ok(ProjectSlug(s))
    }
}

impl std::fmt::Display for ProjectSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The single gate every Hub endpoint goes through: an admin caller, their
/// token (the Hub authenticates it back against this instance's whoami), and
/// the validated `workspace_id:folder` source key scoping ownership Hub-side.
/// Handlers can only reach the Hub via this extractor's methods, so a new
/// endpoint cannot forget the admin check or folder validation.
///
/// A workspace can publish one Hub project per folder. The stable, never-mutated
/// link key is `workspace_id:folder_name` (folder name is the path segment and is
/// never renamed — only display_name changes). `:` is safe: neither workspace ids
/// nor folder names (alphanumeric, underscore, hyphen) contain it.
struct HubPublishCtx {
    source_id: Option<String>,
    token: String,
}

impl<S> FromRequestParts<S> for HubPublishCtx
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let authed = ApiAuthed::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let tokened = Tokened::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let params = RawPathParams::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let workspace = params
            .iter()
            .find(|(k, _)| *k == "workspace_id")
            .map(|(_, v)| v.to_owned());
        let Query(scope) = Query::<HubScope>::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let build = || -> Result<HubPublishCtx, Error> {
            require_admin(authed.is_admin, &authed.username)?;
            let workspace = workspace.ok_or_else(|| {
                Error::internal_err(
                    "hub publish route must be nested under /w/{workspace_id}".to_string(),
                )
            })?;
            let source_id = scope
                .folder
                .as_deref()
                .map(|f| source_key(&workspace, f))
                .transpose()?;
            Ok(HubPublishCtx { source_id, token: tokened.token })
        };
        build().map_err(IntoResponse::into_response)
    }
}

impl HubPublishCtx {
    fn require_source(&self) -> Result<&str, Error> {
        self.source_id
            .as_deref()
            .ok_or_else(|| Error::BadRequest("missing folder query param".to_string()))
    }

    async fn post<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<(StatusCode, String), Error> {
        forward_to_hub(path, self.require_source()?, &self.token, body).await
    }

    async fn get(&self, path: &str) -> Result<(StatusCode, String), Error> {
        get_from_hub(path, self.require_source()?, &self.token).await
    }

    /// GET without requiring a folder scope. Only for reads the Hub allows
    /// publicly (e.g. exporting an approved project); the empty source id makes
    /// the Hub skip the ownership match.
    async fn get_maybe_unscoped(&self, path: &str) -> Result<(StatusCode, String), Error> {
        get_from_hub(path, self.source_id.as_deref().unwrap_or(""), &self.token).await
    }
}

#[derive(Deserialize, Serialize)]
struct PublishDraftBody {
    slug: ProjectSlug,
    name: String,
    summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    readme: Option<String>,
}

async fn publish_draft(
    ctx: HubPublishCtx,
    Json(body): Json<PublishDraftBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post("/projects", &body).await
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
    project_slug: ProjectSlug,
}

async fn publish_script(
    ctx: HubPublishCtx,
    Json(body): Json<PublishScriptBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post("/scripts/add", &body).await
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
    project_slug: ProjectSlug,
}

async fn publish_flow(
    ctx: HubPublishCtx,
    Json(body): Json<PublishFlowBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post("/flows", &body).await
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
    project_slug: ProjectSlug,
}

async fn publish_app(
    ctx: HubPublishCtx,
    Json(body): Json<PublishAppBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post("/apps", &body).await
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
    project_slug: ProjectSlug,
}

async fn publish_raw_app(
    ctx: HubPublishCtx,
    Json(body): Json<PublishRawAppBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post("/raw_apps", &body).await
}

#[derive(Deserialize, Serialize)]
struct RawAppEmbedBody {
    // No skip_serializing_if: `null` must reach the Hub to clear the embed (unpublish).
    external_embed_url: Option<String>,
    project_slug: ProjectSlug,
}

async fn publish_raw_app_embed(
    ctx: HubPublishCtx,
    Path((_workspace, id)): Path<(String, i64)>,
    Json(body): Json<RawAppEmbedBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(&format!("/raw_apps/{}/embed", id), &body).await
}

#[derive(Deserialize, Serialize)]
struct RecordingBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
    project_slug: ProjectSlug,
}

async fn publish_script_recording(
    ctx: HubPublishCtx,
    Path((_workspace, ask_id)): Path<(String, i64)>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(&format!("/scripts/{}/recording", ask_id), &body)
        .await
}

async fn publish_flow_recording(
    ctx: HubPublishCtx,
    Path((_workspace, flow_id)): Path<(String, i64)>,
    Json(body): Json<RecordingBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(&format!("/flows/{}/recording", flow_id), &body)
        .await
}

// A data-pipeline recording is scoped to the whole project (a folder cascade),
// not a single Hub item, so the slug comes from the path (validated by
// construction) and only the opaque recording is forwarded.
#[derive(Deserialize, Serialize)]
struct PipelineRecordingBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    recording: Option<serde_json::Value>,
}

async fn publish_pipeline_recording(
    ctx: HubPublishCtx,
    Path((_workspace, slug)): Path<(String, ProjectSlug)>,
    Json(body): Json<PipelineRecordingBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(&format!("/projects/{}/pipeline_recording", slug), &body)
        .await
}

#[derive(Deserialize, Serialize)]
struct ProjectLogoInner {
    b64: String,
    mime: String,
}

// Custom project logo (png/svg, base64). Double-Option so a missing `logo`
// key is distinguishable from an explicit `logo: null` (which clears the
// logo on the Hub) — otherwise POSTing `{}` would silently delete it.
#[derive(Deserialize, Serialize)]
struct ProjectLogoBody {
    #[serde(default, deserialize_with = "deserialize_explicit")]
    logo: Option<Option<ProjectLogoInner>>,
}

fn deserialize_explicit<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<Option<ProjectLogoInner>>, D::Error> {
    Option::<ProjectLogoInner>::deserialize(d).map(Some)
}

// Mirrors the Hub's own limits so an oversized/invalid payload is rejected
// here instead of being deserialized, copied and forwarded first. The route
// also carries a DefaultBodyLimit sized for a max logo in base64 + JSON
// envelope, overriding the much larger global request limit.
const MAX_LOGO_BYTES: usize = 512 * 1024;
const LOGO_BODY_LIMIT: usize = MAX_LOGO_BYTES / 3 * 4 + 16 * 1024;
const ALLOWED_LOGO_MIMES: [&str; 2] = ["image/png", "image/svg+xml"];

fn validate_logo(inner: &ProjectLogoInner) -> Result<(), Error> {
    if !ALLOWED_LOGO_MIMES.contains(&inner.mime.as_str()) {
        return Err(Error::BadRequest(
            "logo mime must be image/png or image/svg+xml".to_string(),
        ));
    }
    let b = inner.b64.as_bytes();
    let padding = b.iter().rev().take_while(|&&c| c == b'=').count();
    let valid = !b.is_empty()
        && b.len() % 4 == 0
        && padding <= 2
        && b[..b.len() - padding]
            .iter()
            .all(|&c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/');
    if !valid {
        return Err(Error::BadRequest("invalid base64".to_string()));
    }
    if b.len() / 4 * 3 - padding > MAX_LOGO_BYTES {
        return Err(Error::BadRequest(format!(
            "logo too large (max {}KB)",
            MAX_LOGO_BYTES / 1024
        )));
    }
    Ok(())
}

async fn publish_project_logo(
    ctx: HubPublishCtx,
    Path((_workspace, slug)): Path<(String, ProjectSlug)>,
    Json(body): Json<ProjectLogoBody>,
) -> Result<impl IntoResponse, Error> {
    let Some(logo) = body.logo else {
        return Err(Error::BadRequest(
            "logo field is required: an object to set it, or null to clear it".to_string(),
        ));
    };
    if let Some(inner) = &logo {
        validate_logo(inner)?;
    }
    ctx.post(
        &format!("/projects/{}/logo", slug),
        &serde_json::json!({ "logo": logo }),
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
    project_slug: ProjectSlug,
}

async fn publish_resource_type(
    ctx: HubPublishCtx,
    Json(body): Json<PublishResourceTypeBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(
        &format!("/projects/{}/resource_types", body.project_slug),
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
    project_slug: ProjectSlug,
}

async fn publish_resources(
    ctx: HubPublishCtx,
    Json(body): Json<PublishResourcesBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(&format!("/projects/{}/resources", body.project_slug), &body)
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
    project_slug: ProjectSlug,
}

async fn publish_triggers(
    ctx: HubPublishCtx,
    Json(body): Json<PublishTriggersBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(&format!("/projects/{}/triggers", body.project_slug), &body)
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
    project_slug: ProjectSlug,
}

async fn publish_migrations(
    ctx: HubPublishCtx,
    Json(body): Json<PublishMigrationsBody>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(
        &format!("/projects/{}/migrations", body.project_slug),
        &body,
    )
    .await
}

// Export is owner-scoped only when re-exporting your own draft; approved
// projects are public, so the folder scope is optional here.
async fn get_project_export(
    ctx: HubPublishCtx,
    Path((_workspace, slug)): Path<(String, ProjectSlug)>,
) -> Result<impl IntoResponse, Error> {
    ctx.get_maybe_unscoped(&format!("/projects/{}/export", slug))
        .await
}

async fn get_project_by_source(ctx: HubPublishCtx) -> Result<impl IntoResponse, Error> {
    ctx.get("/projects/by_source").await
}

async fn submit_project(
    ctx: HubPublishCtx,
    Path((_workspace, slug)): Path<(String, ProjectSlug)>,
) -> Result<impl IntoResponse, Error> {
    ctx.post(
        &format!("/projects/{}/submit", slug),
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
