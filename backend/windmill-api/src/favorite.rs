/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use windmill_common::error::Result;

use serde::{Deserialize, Serialize};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/star", post(star))
        .route("/unstar", post(unstar))
        .route("/list", get(list_favorites))
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[sqlx(type_name = "FAVORITE_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum FavoriteKind {
    Script,
    Flow,
    App,
    #[allow(non_camel_case_types)]
    Raw_App,
    Asset,
}
#[derive(Deserialize)]
pub struct Favorite {
    pub favorite_kind: FavoriteKind,
    pub path: String,
}

async fn star(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(Favorite { favorite_kind, path }): Json<Favorite>,
) -> Result<String> {
    sqlx::query!(
        "INSERT INTO favorite (workspace_id, usr, path, favorite_kind) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        &w_id,
        authed.username,
        path,
        favorite_kind as FavoriteKind,
    )
    .execute(&db)
    .await?;

    Ok(format!("Starred {}", path))
}

async fn unstar(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(Favorite { favorite_kind, path }): Json<Favorite>,
) -> Result<String> {
    sqlx::query!(
        "DELETE FROM favorite WHERE workspace_id = $1 AND usr = $2 AND path = $3 AND favorite_kind = $4",
        &w_id,
        authed.username,
        path,
        favorite_kind as FavoriteKind,
    )
    .execute(&db)
    .await?;

    Ok(format!("Unstarred {}", path))
}

#[derive(Serialize)]
pub struct FavoriteItem {
    pub path: String,
    pub favorite_kind: FavoriteKind,
}

async fn list_favorites(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<Json<Vec<FavoriteItem>>> {
    let favorites = sqlx::query_as!(
        FavoriteItem,
        "SELECT path, favorite_kind as \"favorite_kind: FavoriteKind\" FROM favorite WHERE workspace_id = $1 AND usr = $2 ORDER BY path",
        &w_id,
        authed.username,
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(favorites))
}
