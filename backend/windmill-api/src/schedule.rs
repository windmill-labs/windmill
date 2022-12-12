/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::Authed,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::DateTime;
use windmill_common::{
    error::{JsonResult, Result},
    utils::{not_found_if_none, Pagination, StripPath},
};
use windmill_queue::{
    self,
    schedule::{EditSchedule, NewSchedule, PreviewPayload, Schedule, SetEnabled},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_schedule))
        .route("/get/*path", get(get_schedule))
        .route("/exists/*path", get(exists_schedule))
        .route("/create", post(create_schedule))
        .route("/update/*path", post(edit_schedule))
        .route("/delete/*path", delete(delete_schedule))
        .route("/setenabled/*path", post(set_enabled))
}

pub fn global_service() -> Router {
    Router::new().route("/preview", post(preview_schedule))
}

async fn create_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewSchedule>,
) -> Result<String> {
    let tx = user_db.begin(&authed).await?;
    let res =
        windmill_queue::schedule::create_schedule(tx, w_id, ns, &authed.username, &authed.email)
            .await?;
    Ok(res)
}

async fn edit_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(es): Json<EditSchedule>,
) -> Result<String> {
    let tx = user_db.begin(&authed).await?;
    let res = windmill_queue::schedule::edit_schedule(tx, w_id, path, es, &authed.username).await?;
    Ok(res)
}

async fn list_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<Schedule>> {
    let tx = user_db.begin(&authed).await?;
    let res = windmill_queue::schedule::list_schedule(tx, w_id, pagination).await?;
    Ok(Json(res))
}

async fn get_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Schedule> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let schedule_o = windmill_queue::schedule::get_schedule_opt(&mut tx, &w_id, path).await?;
    let schedule = not_found_if_none(schedule_o, "Schedule", path)?;
    tx.commit().await?;
    Ok(Json(schedule))
}

async fn exists_schedule(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let mut tx = db.begin().await?;
    let res = windmill_queue::schedule::exists_schedule(&mut tx, w_id, path).await?;
    tx.commit().await?;
    Ok(Json(res))
}

pub async fn preview_schedule(
    Json(payload): Json<PreviewPayload>,
) -> JsonResult<Vec<DateTime<chrono::Utc>>> {
    Ok(Json(windmill_queue::schedule::preview_schedule(payload)?))
}

pub async fn set_enabled(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> Result<String> {
    let tx = user_db.begin(&authed).await?;
    let res = windmill_queue::schedule::set_enabled(
        tx,
        w_id,
        path,
        payload,
        &authed.username,
        &authed.email,
    )
    .await?;
    Ok(res)
}

async fn delete_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let tx = user_db.begin(&authed).await?;
    let res = windmill_queue::schedule::delete_schedule(tx, w_id, path, &authed.username).await?;
    Ok(res)
}
