/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use crate::{db::ApiAuthed, utils::check_scopes};
use axum::{
    body::Body,
    extract::{Extension, Json, Path, Query},
    response::Response,
    routing::get,
    Router,
};
use hyper::header;
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::FromRow;
use windmill_common::{
    apps::ListAppQuery,
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_apps))
        .route("/get_data/:version/*path", get(get_data))
}

#[derive(FromRow, Deserialize, Serialize)]
pub struct ListableApp {
    pub path: String,
    pub workspace_id: String,
    pub summary: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: serde_json::Value,
    pub starred: bool,
    pub version: i32,
}

async fn list_apps(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListAppQuery>,
) -> JsonResult<Vec<ListableApp>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("raw_app as app")
        .fields(&[
            "app.workspace_id",
            "app.path",
            "app.edited_at",
            "app.summary",
            "app.extra_perms",
            "app.version",
            "favorite.path IS NOT NULL as starred",
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'raw_app' AND favorite.workspace_id = app.workspace_id AND favorite.path = app.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("app.edited_at", true)
        .and_where("app.workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    if let Some(path_start) = &lq.path_start {
        sqlb.and_where_like_left("app.path", path_start);
    }

    if let Some(path_exact) = &lq.path_exact {
        sqlb.and_where_eq("app.path", "?".bind(path_exact));
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableApp>(&sql)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_data(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, _version, path)): Path<(String, u16, StripPath)>,
) -> Result<Response> {
    let path = path.to_path();
    check_scopes(&authed, || format!("raw_apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_scalar!(
        "SELECT data FROM raw_app
        WHERE path = $1 AND workspace_id = $2",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    let res = Response::builder().header(header::CONTENT_TYPE, "text/javascript");

    Ok(res.body(Body::from(app)).unwrap())
}
