use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::Deserialize;
use windmill_common::{
    error::{self, JsonResult},
    scripts::ScriptLang,
    users::username_to_permissioned_as,
    utils::require_admin,
    workspace_dependencies::WorkspaceDependencies,
    DB,
};
use windmill_dep_map::workspace_dependencies::{
    trigger_dependents_to_recompute_dependencies_in_the_background, NewWorkspaceDependencies,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create))
        .route("/list", get(list))
        .route("/archive/:language", post(archive))
        .route("/get_latest/:language", get(get_latest))
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
    require_admin(authed.is_admin, &authed.username)?;

    let dep_path = WorkspaceDependencies::to_path(&nwd.name, nwd.language)?;
    let w_id = nwd.workspace_id.clone();
    let email = authed.email.clone();
    let username = authed.username.clone();

    let id = nwd
        .create(
            (
                authed.email,
                username_to_permissioned_as(&authed.username),
                authed.username,
            ),
            db.clone(),
        )
        .await?;

    handle_deployment_metadata(
        &email,
        &username,
        &db,
        &w_id,
        DeployedObject::WorkspaceDependencies { path: dep_path },
        None,
        true,
        None,
    )
    .await?;

    Ok((StatusCode::CREATED, format!("{}", id)))
}

#[axum::debug_handler]
async fn list(
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WorkspaceDependencies>> {
    tracing::info!(workspace_id = %w_id, "list workspace dependencies");
    Ok(Json(WorkspaceDependencies::list(&w_id, &db).await?))
}

#[derive(Deserialize)]
pub(super) struct NameQuery {
    name: Option<String>,
}

#[axum::debug_handler]
pub(super) async fn get_latest(
    Extension(db): Extension<DB>,
    Path((w_id, language)): Path<(String, ScriptLang)>,
    Query(params): Query<NameQuery>,
) -> JsonResult<Option<WorkspaceDependencies>> {
    tracing::info!(workspace_id = %w_id, language = ?language, name = ?params.name, "get latest workspace dependencies");
    Ok(Json(
        WorkspaceDependencies::get_latest(params.name, language, &w_id, db.into()).await?,
    ))
}

#[axum::debug_handler]
async fn archive(
    authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language)): Path<(String, ScriptLang)>,
    Query(params): Query<NameQuery>,
) -> error::Result<()> {
    tracing::info!(workspace_id = %w_id, language = ?language, name = ?params.name, "archive workspace dependencies");
    require_admin(authed.is_admin, &authed.username)?;
    let db = &db;
    let dep_path = WorkspaceDependencies::to_path(&params.name, language)?;
    WorkspaceDependencies::archive(params.name.clone(), language, &w_id, db).await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        db,
        &w_id,
        DeployedObject::WorkspaceDependencies { path: dep_path.clone() },
        None,
        true,
        None,
    )
    .await?;

    trigger_dependents_to_recompute_dependencies_in_the_background(
        params.name.is_none(),
        w_id,
        language,
        (
            authed.email,
            username_to_permissioned_as(&authed.username),
            authed.username,
        ),
        dep_path,
        db.clone(),
    )
    .await;
    Ok(())
}

#[axum::debug_handler]
async fn delete(
    authed: ApiAuthed,
    // Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, language)): Path<(String, ScriptLang)>,
    Query(params): Query<NameQuery>,
) -> error::Result<()> {
    tracing::info!(workspace_id = %w_id, language = ?language, name = ?params.name, "delete workspace dependencies");
    require_admin(authed.is_admin, &authed.username)?;
    let db = &db;
    let dep_path = WorkspaceDependencies::to_path(&params.name, language)?;
    WorkspaceDependencies::delete(params.name.clone(), language, &w_id, db).await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        db,
        &w_id,
        DeployedObject::WorkspaceDependencies { path: dep_path.clone() },
        None,
        true,
        None,
    )
    .await?;

    trigger_dependents_to_recompute_dependencies_in_the_background(
        params.name.is_none(),
        w_id,
        language,
        (
            authed.email,
            username_to_permissioned_as(&authed.username),
            authed.username,
        ),
        dep_path,
        db.clone(),
    )
    .await;
    Ok(())
}
