use std::collections::HashMap;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use crate::{db::UserDB, users::Authed};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
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
    pub id: i64,
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub versions: Vec<i64>,
    pub execution_mode: ExecutionMode,
    pub extra_perms: serde_json::Value,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct AppVersion {
    pub id: i64,
    pub flow_id: Uuid,
    pub value: serde_json::Value,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct AppWithLastVersion {
    pub id: Uuid,
    pub summary: String,
    pub version_id: Uuid,
    pub value: serde_json::Value,
    pub policy: Policy,
    pub execution_mode: ExecutionMode,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct TriggerablePolicy {
    pub path: String,
    pub static_fields: Map<String, Value>,
    pub is_flow: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Policy {
    pub paths: HashMap<String, TriggerablePolicy>,
}

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub execution_mode: ExecutionMode,
    pub value: serde_json::Value,
    pub policy: Policy,
}

#[derive(Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<serde_json::Value>,
    pub policy: Option<Policy>,
    pub execution_mode: Option<ExecutionMode>,
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
        "SELECT id, workspace_id, path, summary, versions, execution_mode as \"execution_mode: _\", extra_perms from app WHERE path = $1 AND workspace_id = $2",
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

    let id = sqlx::query_scalar!(
        "INSERT INTO app
            (workspace_id, path, summary, policy, execution_mode)
            VALUES ($1, $2, $3, $4, $5) RETURNING id",
        w_id,
        app.path,
        app.summary,
        json!(app.policy),
        app.execution_mode: ExecutionMode
    )
    .fetch_one(&mut tx)
    .await?;

    let v_id = sqlx::query_scalar!(
        "INSERT INTO app_version
            (flow_id, value, created_by)
            VALUES ($1, $2, $3) RETURNING id",
        id,
        app.value,
        authed.username,
    )
    .fetch_one(&mut tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET versions = array_append(versions, $1) WHERE id = $2",
        v_id,
        id
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
