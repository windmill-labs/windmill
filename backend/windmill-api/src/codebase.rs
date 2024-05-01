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
    routing::post,
    Json, Router,
};
use windmill_common::error::Result;

use serde::{Deserialize, Serialize};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/get/:id", get(get_codebase))
        .route("/create", post(create))
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
