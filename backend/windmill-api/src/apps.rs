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
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Uuid, FromRow};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_apps))
        .route("/get/*path", get(get_app))
        .route("/update/*path", post(update_app))
        .route("/delete/*path", delete(delete_app))
        .route("/create", post(create_app))
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[sqlx(type_name = "EXECUTION_MODE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase"))]
pub enum ExecutionMode {
    Anonymous,
    Publisher,
    Viewer,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct App {
    pub id: Uuid,
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub value: serde_json::Value,
    pub policy: serde_json::Value,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub execution_mode: ExecutionMode,
    pub extra_perms: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct TriggerablePolicy {
    pub path: String,
    pub staticFields: serde_json::Value,
    pub is_flow: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Policy {
    pub paths: Map<String, TriggerablePolicy>,
}

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub execution_mode: ExecutionMode,
    pub value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct EditApp {
    pub summary: String,
    pub value: serde_json::Value,
}

async fn list_apps(
    authed: Authed,
    Query(pagination): Query<Pagination>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<App>> {
    let (per_page, offset) = paginate(pagination);

    let sqlb = SqlBuilder::select_from("app")
        .fields(&[
            "id",
            "workspace_id",
            "path",
            "summary",
            "versions",
            "execution_mode",
            "extra_perms",
        ])
        .order_by("path", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();
    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, App>(&sql).fetch_all(&mut tx).await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_app(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<App> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as!(
        App,
        "SELECT * from app WHERE path = $1 AND workspace_id = $2",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    Ok(Json(app))
}

async fn create_app(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(app): Json<CreateApp>,
) -> Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO app
            (workspace_id, path, value, description, app_type, is_oauth)
            VALUES ($1, $2, $3, $4, $5, $6)",
        w_id,
        app.path,
        app.value,
        app.description,
        app.app_type,
        app.is_oauth.unwrap_or(false)
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "apps.create",
        ActionKind::Create,
        &w_id,
        Some(&app.path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("app {} created", app.path)))
}

async fn delete_app(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM app WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "apps.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("app {} deleted", path))
}

async fn update_app(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditApp>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();

    let mut sqlb = SqlBuilder::update_table("app");
    sqlb.and_where_eq("path", "?".bind(&path));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(npath) = &ns.path {
        sqlb.set_str("path", npath);
    }
    if let Some(nvalue) = ns.value {
        sqlb.set_str("value", nvalue.to_string());
    }
    if let Some(ndesc) = ns.description {
        sqlb.set_str("description", ndesc);
    }

    sqlb.returning("path");

    let mut tx = user_db.begin(&authed).await?;

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut tx).await?;

    let npath = not_found_if_none(npath_o, "App", path)?;

    audit_log(
        &mut tx,
        &authed.username,
        "apps.update",
        ActionKind::Update,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("app {} updated (npath: {:?})", path, npath))
}
