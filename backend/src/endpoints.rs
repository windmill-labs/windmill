/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sql_builder::prelude::*;

use axum::{
    extract::{Extension, Host, Path, Query},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};

use crate::{
    audit::{audit_log, ActionKind},
    db::{UserDB, DB},
    error::{self, to_anyhow, Error, JsonResult, Result},
    jobs::RawCode,
    scripts::Schema,
    users::Authed,
    utils::{http_get_from_hub, list_elems_from_hub, Pagination, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_endpoints))
        .route("/create", post(create_endpoint))
        .route("/update/*path", post(update_endpoint))
        //.route("/archive/*path", post(archive_flow_by_path))
        //.route("/get/*path", get(get_flow_by_path))
        //.route("/exists/*path", get(exists_flow_by_path))
}

#[derive(FromRow, Serialize)]
pub struct Endpoint {
    pub workspace_id: String,
    pub path_prefix: String,
    pub summary: String,
    pub description: String,
    pub target_script: String,
    // TODO: Authenticated?
    pub edited_by: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
}

#[derive(FromRow, Deserialize)]
pub struct NewEndpoint {
    pub path_prefix: String,
    pub summary: String,
    pub description: String,
    pub target_script: String,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct ListEndpointQuery {
    pub path_start: Option<String>,
    pub path_exact: Option<String>,
    pub edited_by: Option<String>,
    pub show_archived: Option<bool>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
}

async fn list_endpoints(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListEndpointQuery>,
) -> JsonResult<Vec<Endpoint>> {
    let (per_page, offset) = crate::utils::paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("endpoint as o")
        .fields(&[
            "workspace_id",
            "path_prefix",
            "summary",
            "description",
            "created_by",
            "created_at",
            "archived",
            "deleted",
        ])
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .and_where("workspace_id = ? OR workspace_id = 'starter'".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if !lq.show_archived.unwrap_or(false) {
        sqlb.and_where_eq("archived", false);
    }
    if let Some(ps) = &lq.path_start {
        sqlb.and_where_like_left("path_prefix", "?".bind(ps));
    }
    if let Some(p) = &lq.path_exact {
        sqlb.and_where_eq("path_prefix", "?".bind(p));
    }
    //if let Some(cb) = &lq.edited_by {
    //    sqlb.and_where_eq("edited_by", "?".bind(cb));
    //}

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, Endpoint>(&sql).fetch_all(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(rows))
}


async fn create_endpoint(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(nf): Json<NewEndpoint>,
) -> Result<String> {
    // cron::Schedule::from_str(&ns.schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO endpoint (workspace_id, path_prefix, summary, description, target_script, created_by, created_at, archived, \
         deleted) VALUES ($1, $2, $3, $4, $5, $6, $7, false, false)",
        w_id,
        nf.path_prefix,
        nf.summary,
        nf.description,
        nf.target_script,
        &authed.username,
        &chrono::Utc::now(),
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "endpoints.create",
        ActionKind::Create,
        &w_id,
        Some(&nf.path_prefix.to_string()),
        Some(
            [Some(("endpoint", nf.path_prefix.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(nf.path_prefix.to_string())
}


async fn update_endpoint(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Json(nf): Json<NewEndpoint>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let flow_path = flow_path.to_path();

    let flow = sqlx::query_scalar!(
        "UPDATE endpoint SET path_prefix = $1, summary = $2, description = $3, target_script = $4, created_by = $5, \
         created_at = $6 WHERE path_prefix = $7 AND workspace_id = $8 RETURNING path_prefix",
        nf.path_prefix,
        nf.summary,
        nf.description,
        nf.target_script,
        &authed.username,
        &chrono::Utc::now(),
        flow_path,
        w_id,
    )
    .fetch_optional(&mut tx)
    .await?;
    crate::utils::not_found_if_none(flow, "Flow", flow_path)?;

    audit_log(
        &mut tx,
        &authed.username,
        "flows.update",
        ActionKind::Create,
        &w_id,
        Some(&nf.path_prefix.to_string()),
        Some(
            [Some(("flow", nf.path_prefix.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(nf.path_prefix.to_string())
}

/*async fn get_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Flow> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow_o = sqlx::query_as::<_, Flow>(
        "SELECT * FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id = 'starter')",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let flow = crate::utils::not_found_if_none(flow_o, "Flow", path)?;
    Ok(Json(flow))
}

async fn exists_flow_by_path(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id \
         = 'starter'))",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn archive_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "UPDATE flow SET archived = true WHERE path = $1 AND workspace_id = $2",
        path,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "flows.archive",
        ActionKind::Delete,
        &w_id,
        Some(path),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Flow {path} archived"))
}*/
