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

use object_store::path::Path as OPath;
#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;

// use serde::{Deserialize, Serialize};
use windmill_common::error::{to_anyhow, Error, JsonResult, Result};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/exists/:id", get(exists_codebase))
        .route("/upload/:id", post(upload_codebase))
}
// async fn get_codebase(
//     authed: ApiAuthed,
//     Extension(db): Extension<DB>,
//     Path(w_id, id): Path<(String, String)>,
//     Json(Favorite { favorite_kind, path }): Json<Favorite>,
// ) -> Result<String> {
//     sqlx::query!(
//         "INSERT INTO favorite (workspace_id, usr, path, favorite_kind) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
//         &w_id,
//         authed.username,
//         path,
//         favorite_kind as FavoriteKind,
//     )
//     .execute(&db)
//     .await?;

//     Ok(format!("Starred {}", path))
// }

async fn exists_codebase(Path((w_id, id)): Path<(String, String)>) -> JsonResult<bool> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
        let url = os
            .get(&OPath::from(codebase_id_to_path(&w_id, &id)))
            .await
            .map_err(to_anyhow);
        return Ok(Json(url.is_ok()));
    } else {
        return Err(Error::BadRequest(format!(
            "Object store not configured in the instance settings",
        )));
    }

    return Err(Error::BadRequest(format!("Upgrade to EE to use codebases")));
}

async fn upload_codebase(
    Path((w_id, id)): Path<(String, String)>,
    request: axum::extract::Request,
) -> Result<String> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
        let path = codebase_id_to_path(&w_id, &id);
        crate::job_helpers_ee::upload_file_internal(os, &path, request).await?;
        return Ok(format!("Uploaded codebase {}", id));
    } else {
        return Err(Error::BadRequest(format!(
            "Object store not configured in the instance settings",
        )));
    }

    return Err(Error::BadRequest(format!("Upgrade to EE to use codebases")));
}
