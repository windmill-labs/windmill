use axum::{
    extract::Path,
    routing::{get, post},
    Extension, Json, Router,
};
use http::StatusCode;
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
        .route("/archive/:language/:name", post(archive))
        .route("/get-latest/:language/:name", get(get_latest))
        // TODO: We should really not delete it, archiving it would be better.
        .route("/delete/:language/:name", post(delete))
}

#[axum::debug_handler]
async fn create(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Json(nwr): Json<NewWorkspaceDependencies>,
) -> error::Result<(StatusCode, String)> {
    // TODO: Check that it is an admin
    Ok((StatusCode::CREATED, format!("{}", nwr.create(&db).await?)))
}

#[axum::debug_handler]
async fn list(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WorkspaceDependencies>> {
    // TODO: Check that it is an admin
    Ok(Json(WorkspaceDependencies::list(&w_id, &db).await?))
}

#[axum::debug_handler]
async fn get_latest(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language, name)): Path<(String, ScriptLang, Option<String>)>,
) -> JsonResult<WorkspaceDependencies> {
    // TODO: Check that it is an admin
    Ok(Json(
        WorkspaceDependencies::get_latest(name, language, &w_id, db.into()).await?,
    ))
}

#[axum::debug_handler]
async fn archive(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language, name)): Path<(String, ScriptLang, Option<String>)>,
) -> error::Result<()> {
    // TODO: Check that it is an admin
    WorkspaceDependencies::archive(name, language, &w_id, &db).await
}

#[axum::debug_handler]
async fn delete(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language, name)): Path<(String, ScriptLang, Option<String>)>,
) -> error::Result<()> {
    // TODO: Check that it is an admin
    WorkspaceDependencies::delete(name, language, &w_id, &db).await
}
