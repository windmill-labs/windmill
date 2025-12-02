use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::Deserialize;
use windmill_common::{
    cache::workspace_dependencies::EXISTS_CACHE_TIMEOUT,
    error::{self, JsonResult},
    scripts::ScriptLang,
    users::username_to_permissioned_as,
    utils::require_admin,
    workspace_dependencies::WorkspaceDependencies,
    DB,
};
use windmill_worker::{
    scoped_dependency_map, trigger_dependents_to_recompute_dependencies,
    workspace_dependencies::NewWorkspaceDependencies,
};

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
    Ok((
        StatusCode::CREATED,
        format!(
            "{}",
            nwd.create(
                &authed.email,
                &authed.username,
                &username_to_permissioned_as(&authed.username),
                &db
            )
            .await?
        ),
    ))
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
    WorkspaceDependencies::archive(params.name.clone(), language, &w_id, db).await?;

    if params.name.is_none() {
        tracing::debug!(
            workspace_id = %w_id,
            ?language,
            "waiting for cache timeout after archiving unnamed workspace dependencies"
        );
        // for context read [[NewWorkspaceDependencies::create]]
        tokio::time::sleep(EXISTS_CACHE_TIMEOUT).await;
    }

    trigger_dependents_to_recompute_dependencies(
        &w_id,
        scoped_dependency_map::ScopedDependencyMap::get_dependents(
            WorkspaceDependencies::to_path(&params.name, language)?.as_str(),
            &w_id,
            db,
        )
        .await?,
        None,
        None,
        &authed.email,
        &authed.username,
        &username_to_permissioned_as(&authed.username),
        db,
        vec![],
    )
    .await
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
    WorkspaceDependencies::delete(params.name.clone(), language, &w_id, db).await?;

    if params.name.is_none() {
        tracing::debug!(
            workspace_id = %w_id,
            ?language,
            "waiting for cache timeout after deleting unnamed workspace dependencies"
        );
        // for context read [[NewWorkspaceDependencies::create]]
        tokio::time::sleep(EXISTS_CACHE_TIMEOUT).await;
    }

    trigger_dependents_to_recompute_dependencies(
        &w_id,
        scoped_dependency_map::ScopedDependencyMap::get_dependents(
            WorkspaceDependencies::to_path(&params.name, language)?.as_str(),
            &w_id,
            db,
        )
        .await?,
        None,
        None,
        &authed.email,
        &authed.username,
        &username_to_permissioned_as(&authed.username),
        db,
        vec![],
    )
    .await
}
