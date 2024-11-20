/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path},
    routing::{get, post, put},
    Router,
};
use hyper::StatusCode;
use sqlx::types::Json;
use windmill_common::{
    db::UserDB,
    error::{JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};
use windmill_queue::{PushArgs, PushArgsOwned};

use crate::db::{ApiAuthed, DB};

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
    authed: ApiAuthed,
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
    .execute(&mut *tx)
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
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::CREATED)
}

pub async fn update_payload(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    args: PushArgsOwned,
) -> Result<StatusCode> {
    let mut tx = db.begin().await?;

    sqlx::query!(
        "
       UPDATE capture
          SET payload = $3
        WHERE workspace_id = $1
          AND path = $2
        ",
        &w_id,
        &path.to_path(),
        Json(PushArgs { args: &args.args, extra: args.extra }) as Json<PushArgs>,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(sqlx::FromRow)]
struct Payload {
    payload: sqlx::types::Json<Box<serde_json::value::RawValue>>,
}
pub async fn get_payload(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Box<serde_json::value::RawValue>> {
    let mut tx = user_db.begin(&authed).await?;

    let payload = sqlx::query_as::<_, Payload>(
        "
       SELECT payload
         FROM capture
        WHERE workspace_id = $1
          AND path = $2
        ",
    )
    .bind(&w_id)
    .bind(&path.to_path())
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    not_found_if_none(payload.map(|x| x.payload.0), "capture", path.to_path()).map(axum::Json)
}
