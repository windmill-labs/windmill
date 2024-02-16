/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use crate::{
    db::{ApiAuthed, DB},
    users::require_owner_of_path,
    webhook_util::{WebhookMessage, WebhookShared},
};
use axum::{
    body,
    extract::{Extension, Json, Path, Query},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use bytes::Bytes;
use hyper::{header, StatusCode};
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::FromRow;
use std::str;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
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
        .route("/exists/*path", get(exists_app))
        .route("/update/*path", post(update_app))
        .route("/delete/*path", delete(delete_app))
        .route("/create", post(create_app))
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

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<String>,
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

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
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

    Ok(res
        .body(body::boxed(body::Full::from(Bytes::from(app))))
        .unwrap())
}

async fn create_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(app): Json<CreateApp>,
) -> Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;
    if &app.path == "" {
        return Err(Error::BadRequest("App path cannot be empty".to_string()));
    }

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        app.path,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "App with path {} already exists",
            &app.path
        )));
    }

    sqlx::query!(
        "INSERT INTO raw_app
            (workspace_id, path, summary, extra_perms, data)
            VALUES ($1, $2, $3, '{}', $4)",
        w_id,
        app.path,
        app.summary,
        app.value,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "apps.create",
        ActionKind::Create,
        &w_id,
        Some(&app.path),
        None,
    )
    .await?;

    tx.commit().await?;
    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: app.path.clone() },
    );

    Ok((StatusCode::CREATED, app.path))
}

async fn delete_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM raw_app WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed.username,
        "apps.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;
    webhook.send_message(
        w_id.clone().clone(),
        WebhookMessage::DeleteApp { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("app {} deleted", path))
}

async fn update_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(app): Json<EditApp>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();

    let mut tx = user_db.begin(&authed).await?;
    let mut sqlb = SqlBuilder::update_table("raw_app");
    sqlb.and_where_eq("path", "?".bind(&path));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    let npath = &app.path;
    if npath.is_some() || app.summary.is_some() {
        if let Some(npath) = npath {
            if npath != path {
                require_owner_of_path(&authed, path)?;

                let exists = sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
                    npath,
                    w_id
                )
                .fetch_one(&mut *tx)
                .await?
                .unwrap_or(false);

                if exists {
                    return Err(Error::BadRequest(format!(
                        "App with path {} already exists",
                        npath
                    )));
                }
            }
            sqlb.set_str("path", npath);
        }

        if let Some(nsummary) = &app.summary {
            sqlb.set_str("summary", nsummary);
        }
    }

    if let Some(value) = &app.value {
        sqlb.set_str("data", value);
        sqlb.set("version", "version + 1");
    }

    sqlb.returning("path");

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut *tx).await?;
    not_found_if_none(npath_o, "Raw App", path)?;

    let npath = app.path.clone().unwrap_or_else(|| path.to_owned());
    audit_log(
        &mut *tx,
        &authed.username,
        "apps.update",
        ActionKind::Update,
        &w_id,
        Some(&path),
        None,
    )
    .await?;
    tx.commit().await?;
    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateApp {
            workspace: w_id,
            old_path: path.to_owned(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("app {} updated (npath: {:?})", path, npath))
}

async fn exists_app(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM raw_app WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}
