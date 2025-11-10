use axum::{extract::Path, routing::post, Extension, Json, Router};
use http::StatusCode;
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    raw_requirements::{RawRequirements, ViewableRawRequirements},
    utils::StripPath,
    DB,
};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create))
        .route("/list", post(list))
        .route("/archive/p/*path", post(archive))
}

#[axum::debug_handler]
async fn create(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Json(nrr): Json<ViewableRawRequirements>,
) -> error::Result<(StatusCode, String)> {
    // TODO: Check that it is an admin
    Ok((StatusCode::CREATED, format!("{}", nrr.create(&db).await?)))
}

#[axum::debug_handler]
async fn list(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ViewableRawRequirements>> {
    // TODO: Check that it is an admin
    Ok(Json(ViewableRawRequirements::list(&w_id, &db).await?))
}

#[axum::debug_handler]
async fn archive(
    // authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<()> {
    // TODO: Check that it is an admin
    RawRequirements::archive(path.to_path(), &w_id, &db).await
}
