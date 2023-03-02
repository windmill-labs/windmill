/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post, put},
    Json, Router,
};
use hyper::{HeaderMap, StatusCode};
use serde::Deserialize;
use windmill_common::{
    error::{JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};

use crate::{
    db::{UserDB, DB},
    jobs::add_include_headers,
    users::Authed,
};

const KEEP_LAST: i64 = 8;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/*path", put(new_payload))
        .route("/*path", get(get_payload))
}

pub fn global_service() -> Router {
    Router::new().route("/*path", post(update_payload))
}

pub async fn new_payload(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<StatusCode> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "
       INSERT INTO capture
                   (workspace_id, path, created_by)
            VALUES ($1, $2, $3)
       ON CONFLICT (workspace_id, path)
     DO UPDATE SET created_at = now()
        ",
        &w_id,
        &path.to_path(),
        &authed.username,
    )
    .execute(&mut tx)
    .await?;

    /* Retain only KEEP_LAST most recent captures by this user in this workspace. */
    sqlx::query!(
        "
   DELETE FROM capture
         WHERE workspace_id = $1
           AND created_by = $2
           AND created_at <=
              ( SELECT created_at
                  FROM capture
                 WHERE workspace_id = $1
                   AND created_by = $2
              ORDER BY created_at DESC
                OFFSET $3
                 LIMIT 1 )
        ",
        &w_id,
        &authed.username,
        KEEP_LAST,
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Clone)]
pub struct IncludeHeaderQuery {
    include_header: Option<String>,
}

pub async fn update_payload(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(run_query): Query<IncludeHeaderQuery>,
    headers: HeaderMap,
    Json(args): Json<Option<serde_json::Map<String, serde_json::Value>>>,
) -> Result<StatusCode> {
    let mut tx = db.begin().await?;

    let args = add_include_headers(&run_query.include_header, headers, args.unwrap_or_default());
    sqlx::query!(
        "
       UPDATE capture
          SET payload = $3
        WHERE workspace_id = $1
          AND path = $2
        ",
        &w_id,
        &path.to_path(),
        serde_json::json!(args),
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_payload(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<serde_json::Value> {
    let mut tx = user_db.begin(&authed).await?;

    let payload = sqlx::query_scalar!(
        "
       SELECT payload
         FROM capture
        WHERE workspace_id = $1
          AND path = $2
        ",
        &w_id,
        &path.to_path(),
    )
    .fetch_optional(&mut tx)
    .await?;

    tx.commit().await?;

    not_found_if_none(payload, "capture", path.to_path()).map(axum::Json)
}
