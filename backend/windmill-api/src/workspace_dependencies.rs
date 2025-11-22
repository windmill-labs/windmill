use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::Deserialize;
use tracing::{debug, error, info, instrument};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    scripts::ScriptLang,
    utils::StripPath,
    workspace_dependencies::WorkspaceDependencies,
    DB,
};
use windmill_worker::workspace_dependencies::NewWorkspaceDependencies;

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create))
        .route("/list", get(list))
        .route("/archive/:language", post(archive))
        .route("/get_latest/:language", get(get_latest))
        // TODO: We should really not delete it, archiving it would be better.
        .route("/delete/:language", post(delete))
}

#[axum::debug_handler]
async fn create(
    authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Json(nwd): Json<NewWorkspaceDependencies>,
) -> error::Result<(StatusCode, String)> {
    tracing::info!(workspace_id = %nwd.workspace_id, name = ?nwd.name, language = ?nwd.language, "create workspace dependencies");
    // TODO: Check that it is an admin
    Ok((StatusCode::CREATED, format!("{}", nwd.create(&db).await?)))
}

#[axum::debug_handler]
async fn list(
    authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WorkspaceDependencies>> {
    tracing::info!(workspace_id = %w_id, "list workspace dependencies");
    // TODO: Check that it is an admin
    Ok(Json(WorkspaceDependencies::list(&w_id, &db).await?))
}

#[derive(Deserialize)]
struct NameQuery {
    name: Option<String>,
}

#[axum::debug_handler]
async fn get_latest(
    authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language)): Path<(String, ScriptLang)>,
    Query(params): Query<NameQuery>,
) -> JsonResult<Option<WorkspaceDependencies>> {
    tracing::info!(workspace_id = %w_id, language = ?language, name = ?params.name, "get latest workspace dependencies");
    // TODO: Check that it is an admin
    Ok(Json(
        WorkspaceDependencies::get_latest(params.name, language, &w_id, db.into()).await?,
    ))
}

#[axum::debug_handler]
async fn archive(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language)): Path<(String, ScriptLang)>,
    Query(params): Query<NameQuery>,
) -> error::Result<()> {
    tracing::info!(workspace_id = %w_id, language = ?language, name = ?params.name, "archive workspace dependencies");
    // TODO: Check that it is an admin
    WorkspaceDependencies::archive(params.name, language, &w_id, &db).await
}

#[axum::debug_handler]
async fn delete(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language)): Path<(String, ScriptLang)>,
    Query(params): Query<NameQuery>,
) -> error::Result<()> {
    tracing::info!(workspace_id = %w_id, language = ?language, name = ?params.name, "delete workspace dependencies");
    // TODO: Check that it is an admin
    WorkspaceDependencies::delete(params.name, language, &w_id, &db).await
}
