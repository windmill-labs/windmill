/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::db::ApiAuthed;
use crate::triggers::{
    get_triggers_count_internal, list_tokens_internal, TriggersCount, TruncatedTokenWithEmail,
};
use crate::utils::{RunnableKind, WithStarredInfoQuery};
use crate::{
    db::DB,
    schedule::clear_schedule,
    users::{maybe_refresh_folders, require_owner_of_path},
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
use axum::response::IntoResponse;
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};

use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sql_builder::prelude::*;
use sqlx::{FromRow, Postgres, Transaction};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::utils::query_elems_from_hub;
use windmill_common::worker::to_raw_value;
use windmill_common::HUB_BASE_URL;
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, Error, JsonResult, Result},
    flows::{Flow, FlowWithStarred, ListFlowQuery, ListableFlow, NewFlow},
    jobs::JobPayload,
    schedule::Schedule,
    scripts::Schema,
    utils::{http_get_from_hub, not_found_if_none, paginate, Pagination, StripPath},
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, schedule::push_scheduled_job, PushIsolationLevel};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_flows))
        .route("/list_search", get(list_search_flows))
        .route("/create", post(create_flow))
        .route("/update/*path", post(update_flow))
        .route("/archive/*path", post(archive_flow_by_path))
        .route("/delete/*path", delete(delete_flow_by_path))
        .route("/get_triggers_count/*path", get(get_triggers_count))
        .route("/list_tokens/*path", get(list_tokens))
        .route("/get/*path", get(get_flow_by_path))
        .route("/deployment_status/p/*path", get(get_deployment_status))
        .route("/get/draft/*path", get(get_flow_by_path_w_draft))
        .route("/exists/*path", get(exists_flow_by_path))
        .route("/list_paths", get(list_paths))
        .route("/history/p/*path", get(get_flow_history))
        .route("/get_latest_version/*path", get(get_latest_version))
        .route(
            "/list_paths_from_workspace_runnable/:runnable_kind/*path",
            get(list_paths_from_workspace_runnable),
        )
        .route(
            "/history_update/v/:version/p/*path",
            post(update_flow_history),
        )
        .route("/get/v/:version/p/*path", get(get_flow_version))
        .route(
            "/toggle_workspace_error_handler/*path",
            post(toggle_workspace_error_handler),
        )
}

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_flows))
        .route("/hub/get/:id", get(get_hub_flow_by_id))
}

#[derive(Serialize, FromRow)]
pub struct SearchFlow {
    path: String,
    value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
}
async fn list_search_flows(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<SearchFlow>> {
    #[cfg(feature = "enterprise")]
    let n = 1000;

    #[cfg(not(feature = "enterprise"))]
    let n = 3;
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_as::<_, SearchFlow>(
        "SELECT flow.path, flow_version.value
        FROM flow 
        LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
        WHERE flow.workspace_id = $1 LIMIT $2",
    )
    .bind(&w_id)
    .bind(n)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_flows(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListFlowQuery>,
) -> JsonResult<Vec<ListableFlow>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("flow as o")
        .fields(&[
            "o.workspace_id",
            "o.path",
            "summary",
            "description",
            "fv.created_by as edited_by",
            "fv.created_at as edited_at",
            "archived",
            "extra_perms",
            "favorite.path IS NOT NULL as starred",
            "draft.path IS NOT NULL as has_draft",
            "draft_only",
            "ws_error_handler_muted"
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'flow' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .left()
        .join("draft")
        .on(
            "draft.path = o.path AND draft.workspace_id = o.workspace_id AND draft.typ = 'flow'"
        )
        .left()
        .join("flow_version fv")
        .on(
            "fv.id = o.versions[array_upper(o.versions, 1)]"
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("fv.created_at", lq.order_desc.unwrap_or(true))
        .and_where("o.workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    sqlb.and_where_eq("archived", lq.show_archived.unwrap_or(false));

    if let Some(ps) = &lq.path_start {
        sqlb.and_where_like_left("o.path", ps);
    }
    if let Some(p) = &lq.path_exact {
        sqlb.and_where_eq("o.path", "?".bind(p));
    }
    if let Some(cb) = &lq.edited_by {
        sqlb.and_where_eq("fv.created_by", "?".bind(cb));
    }
    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    if !lq.include_draft_only.unwrap_or(false) || authed.is_operator {
        sqlb.and_where("o.draft_only IS NOT TRUE");
    }

    if lq.with_deployment_msg.unwrap_or(false) {
        sqlb.join("deployment_metadata dm")
            .left()
            .on("dm.flow_version = o.versions[array_upper(o.versions, 1)]")
            .fields(&["dm.deployment_msg"]);
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableFlow>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_hub_flows(Extension(db): Extension<DB>) -> impl IntoResponse {
    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!(
            "{}/searchFlowData?approved=true",
            *HUB_BASE_URL.read().await
        ),
        None,
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, response))
}

async fn list_paths(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;

    let flows = sqlx::query_scalar!(
        "SELECT distinct(path) FROM flow WHERE  workspace_id = $1",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(flows))
}

pub async fn get_hub_flow_by_id(
    Path(id): Path<i32>,
    Extension(db): Extension<DB>,
) -> JsonResult<Box<serde_json::value::RawValue>> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("{}/flows/{}/json", *HUB_BASE_URL.read().await, id),
        false,
        None,
        Some(&db),
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
}

#[derive(Deserialize)]
pub struct ToggleWorkspaceErrorHandler {
    #[cfg(feature = "enterprise")]
    pub muted: Option<bool>,
}

#[cfg(not(feature = "enterprise"))]
async fn toggle_workspace_error_handler(
    _authed: ApiAuthed,
    Extension(_user_db): Extension<UserDB>,
    Path((_w_id, _path)): Path<(String, StripPath)>,
    Json(_req): Json<ToggleWorkspaceErrorHandler>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Muting the error handler for certain flow is only available in enterprise version"
            .to_string(),
    ));
}

#[cfg(feature = "enterprise")]
async fn toggle_workspace_error_handler(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(req): Json<ToggleWorkspaceErrorHandler>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let error_handler_maybe: Option<String> = sqlx::query_scalar!(
        "SELECT error_handler FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(None);

    return match error_handler_maybe {
        Some(_) => {
            sqlx::query_scalar!(
                "UPDATE flow SET ws_error_handler_muted = $3 WHERE path = $1 AND workspace_id = $2",
                path.to_path(),
                w_id,
                req.muted,
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok("".to_string())
        }
        None => {
            tx.commit().await?;
            Err(Error::ExecutionErr(
                "Workspace error handler needs to be defined".to_string(),
            ))
        }
    };
}

async fn check_path_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!("Flow {} already exists", path)));
    }
    return Ok(());
}

async fn list_paths_from_workspace_runnable(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;
    let runnables = sqlx::query_scalar!(
        r#"SELECT f.path
            FROM workspace_runnable_dependencies wru 
            JOIN flow f
                ON wru.flow_path = f.path AND wru.workspace_id = f.workspace_id
            WHERE wru.runnable_path = $1 AND wru.runnable_is_flow = $2 AND wru.workspace_id = $3"#,
        path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(runnables))
}

async fn create_flow(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(nf): Json<NewFlow>,
) -> Result<(StatusCode, String)> {
    #[cfg(not(feature = "enterprise"))]
    if nf
        .value
        .get("ws_error_handler_muted")
        .map(|val| val.as_bool().unwrap_or(false))
        .is_some_and(|val| val)
    {
        return Err(Error::BadRequest(
            "Muting the error handler for certain flow is only available in enterprise version"
                .to_string(),
        ));
    }

    // cron::Schedule::from_str(&ns.schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;
    let authed = maybe_refresh_folders(&nf.path, &w_id, authed, &db).await;

    let mut tx = user_db.clone().begin(&authed).await?;

    check_path_conflict(&mut tx, &w_id, &nf.path).await?;
    check_schedule_conflict(&mut tx, &w_id, &nf.path).await?;

    let schema_str = nf.schema.and_then(|x| serde_json::to_string(&x.0).ok());

    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, \
         dependency_job, lock_error_logs, draft_only, tag, dedicated_worker, visible_to_runner_only, on_behalf_of_email, value, schema, edited_by, edited_at) 
         VALUES ($1, $2, $3, $4, NULL, '', $5, $6, $7, $8, $9, $10, $11::text::json, $12, now())",
        w_id,
        nf.path,
        nf.summary,
        nf.description.unwrap_or_else(String::new),
        nf.draft_only,
        nf.tag,
        nf.dedicated_worker,
        nf.visible_to_runner_only.unwrap_or(false),
        if nf.on_behalf_of_email.is_some() {
            Some(&authed.email)
        } else {
            None
        },
        nf.value,
        schema_str,
        &authed.username,
    )
    .execute(&mut *tx)
    .await?;

    let version = sqlx::query_scalar!(
        "INSERT INTO flow_version (workspace_id, path, value, schema, created_by) 
        VALUES ($1, $2, $3, $4::text::json, $5)
        RETURNING id",
        w_id,
        nf.path,
        nf.value,
        schema_str,
        &authed.username,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE flow SET versions = array_append(versions, $1) WHERE path = $2 AND workspace_id = $3",
        version,
        nf.path,
        w_id
    ).execute(&mut *tx).await?;

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'flow'",
        nf.path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "flows.create",
        ActionKind::Create,
        &w_id,
        Some(&nf.path.to_string()),
        Some(
            [Some(("flow", nf.path.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = nf.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }

    let tx = PushIsolationLevel::Transaction(tx);
    let (dependency_job_uuid, mut new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::FlowDependencies {
            path: nf.path.clone(),
            dedicated_worker: nf.dedicated_worker,
            version: version,
        },
        windmill_queue::PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        nf.tag,
        None,
        None,
        None,
        Some(&authed.clone().into()),
    )
    .await?;

    sqlx::query!(
        "UPDATE flow SET dependency_job = $1 WHERE path = $2 AND workspace_id = $3",
        dependency_job_uuid,
        nf.path,
        w_id
    )
    .execute(&mut *new_tx)
    .await?;

    new_tx.commit().await?;
    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateFlow { workspace: w_id.clone(), path: nf.path.clone() },
    );

    Ok((StatusCode::CREATED, nf.path.to_string()))
}

async fn check_schedule_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> error::Result<()> {
    let exists_flow = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2 AND path != \
         script_path)",
        path,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if exists_flow {
        return Err(error::Error::BadConfig(format!(
            "A flow cannot have the same path as a schedule if the schedule does not trigger that \
             same flow: {path}",
        )));
    };
    Ok(())
}

pub async fn require_is_writer(authed: &ApiAuthed, path: &str, w_id: &str, db: DB) -> Result<()> {
    return crate::users::require_is_writer(
        authed,
        path,
        w_id,
        db,
        "SELECT extra_perms FROM flow WHERE path = $1 AND workspace_id = $2",
        "flow",
    )
    .await;
}

#[derive(Serialize)]
pub struct FlowVersion {
    pub id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
}

async fn get_flow_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<FlowVersion>> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flows = sqlx::query_as!(
        FlowVersion,
        "SELECT flow_version.id, flow_version.created_at, deployment_metadata.deployment_msg FROM flow_version 
        LEFT JOIN deployment_metadata ON flow_version.id = deployment_metadata.flow_version
        WHERE flow_version.path = $1 AND flow_version.workspace_id = $2 
        ORDER BY flow_version.created_at DESC",
        path,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(flows))
}

async fn get_latest_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<FlowVersion>> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let version = sqlx::query_as!(
        FlowVersion,
        "SELECT flow_version.id, flow_version.created_at, deployment_metadata.deployment_msg FROM flow_version 
        LEFT JOIN deployment_metadata ON flow_version.id = deployment_metadata.flow_version
        WHERE flow_version.path = $1 AND flow_version.workspace_id = $2 
        ORDER BY flow_version.created_at DESC",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(version))
}

async fn get_flow_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, version, path)): Path<(String, i64, StripPath)>,
) -> JsonResult<Flow> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow = sqlx::query_as::<_, Flow>(
        "SELECT flow.workspace_id, flow.path, flow.summary, flow.description, flow.archived, flow.extra_perms, flow.draft_only, flow.dedicated_worker, flow.tag, flow.ws_error_handler_muted, flow.timeout, flow.visible_to_runner_only, flow.on_behalf_of_email, flow_version.schema, flow_version.value, flow_version.created_at as edited_at, flow_version.created_by as edited_by
        FROM flow
        LEFT JOIN flow_version ON flow_version.path = flow.path AND flow_version.workspace_id = flow.workspace_id
        WHERE flow.path = $1 AND flow.workspace_id = $2 AND flow_version.id = $3",
    )
    .bind(path)
    .bind(w_id)
    .bind(version)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    let flow = not_found_if_none(flow, "Flow version", version.to_string())?;

    Ok(Json(flow))
}

#[derive(Deserialize)]
pub struct FlowHistoryUpdate {
    pub deployment_msg: String,
}

async fn update_flow_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, version, path)): Path<(String, i64, StripPath)>,
    Json(history_update): Json<FlowHistoryUpdate>,
) -> Result<()> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let path_o = sqlx::query_scalar!(
        "SELECT flow.path FROM flow
        LEFT JOIN flow_version
            ON flow_version.path = flow.path AND flow_version.workspace_id = flow.workspace_id
        WHERE flow.path = $1 AND flow.workspace_id = $2 AND flow_version.id = $3",
        path,
        w_id,
        version
    )
    .fetch_optional(&mut *tx)
    .await?;

    if path_o.is_none() {
        tx.commit().await?;
        return Err(Error::NotFound(
            format!("Flow version {version} for path {path} not found").to_string(),
        ));
    }

    sqlx::query!(
        "INSERT INTO deployment_metadata (workspace_id, path, flow_version, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path, flow_version) WHERE flow_version IS NOT NULL DO UPDATE SET deployment_msg = $4",
        w_id,
        path_o.unwrap(),
        version,
        history_update.deployment_msg,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(());
}

async fn update_flow(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Json(nf): Json<NewFlow>,
) -> Result<String> {
    #[cfg(not(feature = "enterprise"))]
    if nf
        .value
        .get("ws_error_handler_muted")
        .map(|val| val.as_bool().unwrap_or(false))
        .is_some_and(|val| val)
    {
        return Err(Error::BadRequest(
            "Muting the error handler for certain flow is only available in enterprise version"
                .to_string(),
        ));
    }

    let flow_path = flow_path.to_path();
    let authed = maybe_refresh_folders(&flow_path, &w_id, authed, &db).await;

    let mut tx = user_db.clone().begin(&authed).await?;

    check_schedule_conflict(&mut tx, &w_id, flow_path).await?;

    let schema = nf.schema.map(|x| x.0);
    let old_dep_job = sqlx::query_scalar!(
        "SELECT dependency_job FROM flow WHERE path = $1 AND workspace_id = $2",
        flow_path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let old_dep_job = not_found_if_none(old_dep_job, "Flow", flow_path)?;

    let is_new_path = nf.path != flow_path;

    let schema_str = schema.and_then(|x| serde_json::to_string(&x).ok());

    sqlx::query!(
        "UPDATE flow SET path = $1, summary = $2, description = $3,\
        dependency_job = NULL, lock_error_logs = '', draft_only = NULL, tag = $4, dedicated_worker = $5, visible_to_runner_only = $6, on_behalf_of_email = $7, \
        value = $8, schema = $9::text::json, edited_by = $10, edited_at = now()
        WHERE path = $11 AND workspace_id = $12",
        if is_new_path { flow_path } else { &nf.path }, // if new path, do not rename directly (to avoid flow_version foreign key constraint)
        nf.summary,
        nf.description.unwrap_or_else(String::new),
        nf.tag,
        nf.dedicated_worker,
        nf.visible_to_runner_only.unwrap_or(false),
        if nf.on_behalf_of_email.is_some() {
            Some(&authed.email)
        } else {
            None
        },
        nf.value,
        schema_str,
        &authed.username,
        flow_path,
        w_id,
    )
    .execute(&mut *tx)
    .await.map_err(|e| error::Error::internal_err(format!("Error updating flow due to flow update: {e:#}")))?;

    if is_new_path {
        // if new path, must clone flow to new path and delete old flow for flow_version foreign key constraint
        sqlx::query!(
            "INSERT INTO flow 
                (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at) 
            SELECT workspace_id, $1, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
                FROM flow
                WHERE path = $2 AND workspace_id = $3",
            nf.path,
            flow_path,
            w_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error::Error::internal_err(format!("Error updating flow due to create new flow: {e:#}"))
        })?;

        sqlx::query!(
            "UPDATE flow_version SET path = $1 WHERE path = $2 AND workspace_id = $3",
            nf.path,
            flow_path,
            w_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error::Error::internal_err(format!(
                "Error updating flow due to updating flow history path: {e:#}"
            ))
        })?;

        sqlx::query!(
            "DELETE FROM flow WHERE path = $1 AND workspace_id = $2",
            flow_path,
            w_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error::Error::internal_err(format!(
                "Error updating flow due to deleting old flow: {e:#}"
            ))
        })?;

        sqlx::query!(
            "UPDATE capture_config SET path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS TRUE",
            nf.path,
            flow_path,
            w_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE capture SET path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS TRUE",
            nf.path,
            flow_path,
            w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    let version = sqlx::query_scalar!(
        "INSERT INTO flow_version (workspace_id, path, value, schema, created_by) VALUES ($1, $2, $3, $4::text::json, $5) RETURNING id",
        w_id,
        nf.path,
        nf.value,
        schema_str,
        &authed.username,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        error::Error::internal_err(format!(
            "Error updating flow due to flow history insert: {e:#}"
        ))
    })?;

    sqlx::query!(
        "UPDATE flow SET versions = array_append(versions, $1) WHERE path = $2 AND workspace_id = $3",
        version, nf.path, w_id
    ).execute(&mut *tx).await?;

    if is_new_path {
        check_schedule_conflict(&mut tx, &w_id, &nf.path).await?;

        if !authed.is_admin {
            require_owner_of_path(&authed, flow_path)?;
        }
    }

    let mut schedulables: Vec<Schedule> = sqlx::query_as::<_, Schedule>(
            "UPDATE schedule SET script_path = $1 WHERE script_path = $2 AND path != $2 AND workspace_id = $3 AND is_flow IS true RETURNING *")
            .bind(&nf.path)
            .bind(&flow_path)
            .bind(&w_id)
        .fetch_all(&mut *tx)
        .await.map_err(|e| error::Error::internal_err(format!("Error updating flow due to related schedules update: {e:#}")))?;

    let schedule = sqlx::query_as::<_, Schedule>(
        "UPDATE schedule SET path = $1, script_path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS true RETURNING *")
        .bind(&nf.path)
        .bind(&flow_path)
        .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await.map_err(|e| error::Error::internal_err(format!("Error updating flow due to related schedule update: {e:#}")))?;

    if let Some(schedule) = schedule {
        clear_schedule(&mut tx, &flow_path, &w_id).await?;
        schedulables.push(schedule);
    }

    for schedule in schedulables.into_iter() {
        clear_schedule(&mut tx, &schedule.path, &w_id).await?;

        if schedule.enabled {
            tx = push_scheduled_job(&db, tx, &schedule, None).await?;
        }
    }

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'flow'",
        flow_path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "flows.update",
        ActionKind::Create,
        &w_id,
        Some(&nf.path.to_string()),
        Some(
            [Some(("flow", nf.path.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateFlow {
            workspace: w_id.clone(),
            old_path: flow_path.to_owned(),
            new_path: nf.path.clone(),
        },
    );

    let tx = PushIsolationLevel::Transaction(tx);

    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = nf.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }
    args.insert("parent_path".to_string(), to_raw_value(&flow_path));

    let (dependency_job_uuid, mut new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::FlowDependencies {
            path: nf.path.clone(),
            dedicated_worker: nf.dedicated_worker,
            version: version,
        },
        windmill_queue::PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&authed.clone().into()),
    )
    .await?;
    sqlx::query!(
        "UPDATE flow SET dependency_job = $1 WHERE path = $2 AND workspace_id = $3",
        dependency_job_uuid,
        nf.path,
        w_id
    )
    .execute(&mut *new_tx)
    .await
    .map_err(|e| {
        error::Error::internal_err(format!(
            "Error updating flow due to updating dependency job field: {e:#}"
        ))
    })?;
    if let Some(old_dep_job) = old_dep_job {
        sqlx::query!(
            "UPDATE v2_job_queue SET
                canceled_by = $2,
                canceled_reason = 're-deployment'
            WHERE id = $1",
            old_dep_job,
            &authed.username
        )
        .execute(&mut *new_tx)
        .await
        .map_err(|e| {
            error::Error::internal_err(format!(
                "Error updating flow due to cancelling dependency job: {e:#}"
            ))
        })?;
    }

    new_tx.commit().await?;

    Ok(nf.path.to_string())
}

async fn get_triggers_count(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<TriggersCount> {
    let path = path.to_path();
    get_triggers_count_internal(&db, &w_id, &path, true).await
}

async fn list_tokens(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<TruncatedTokenWithEmail>> {
    let path = path.to_path();
    list_tokens_internal(&db, &w_id, &path, true).await
}

#[derive(FromRow, Serialize)]
struct DeploymentStatus {
    lock_error_logs: Option<String>,
}
async fn get_deployment_status(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<DeploymentStatus> {
    let path = path.to_path();
    let mut tx = db.begin().await?;
    let status_o: Option<DeploymentStatus> = sqlx::query_as!(
        DeploymentStatus,
        "SELECT lock_error_logs FROM flow WHERE path = $1 AND workspace_id = $2",
        path,
        w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let status = not_found_if_none(status_o, "DeploymentStatus", path)?;

    tx.commit().await?;
    Ok(Json(status))
}

async fn get_flow_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<WithStarredInfoQuery>,
) -> JsonResult<FlowWithStarred> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow_o = if query.with_starred_info.unwrap_or(false) {
        sqlx::query_as::<_, FlowWithStarred>(
            "SELECT flow.workspace_id, flow.path, flow.lock_error_logs, flow.summary, flow.description, flow.archived, flow.extra_perms, flow.draft_only, flow.dedicated_worker, flow.tag, flow.ws_error_handler_muted, flow.timeout, flow.visible_to_runner_only, flow.on_behalf_of_email, flow_version.schema, flow_version.value, flow_version.created_at as edited_at, flow_version.created_by as edited_by, favorite.path IS NOT NULL as starred
            FROM flow
            LEFT JOIN favorite
            ON favorite.favorite_kind = 'flow' 
                AND favorite.workspace_id = flow.workspace_id 
                AND favorite.path = flow.path 
                AND favorite.usr = $3
            LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
            WHERE flow.path = $1 AND flow.workspace_id = $2"
        )
            .bind(path)
            .bind(w_id)
            .bind(&authed.username)
            .fetch_optional(&mut *tx)
            .await?
    } else {
        sqlx::query_as::<_, FlowWithStarred>(
            "SELECT flow.workspace_id, flow.path, flow.lock_error_logs, flow.summary, flow.description, flow.archived, flow.extra_perms, flow.draft_only, flow.dedicated_worker, flow.tag, flow.ws_error_handler_muted, flow.timeout, flow.visible_to_runner_only, flow.on_behalf_of_email, flow_version.schema, flow_version.value, flow_version.created_at as edited_at, flow_version.created_by as edited_by, NULL as starred
            FROM flow
            LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
            WHERE flow.path = $1 AND flow.workspace_id = $2"
        )
            .bind(path)
            .bind(w_id)
            .fetch_optional(&mut *tx)
            .await?
    };
    tx.commit().await?;

    let flow = not_found_if_none(flow_o, "Flow", path)?;
    Ok(Json(flow))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FlowWDraft {
    pub path: String,
    pub summary: String,
    pub description: String,
    pub schema: Option<Schema>,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_to_runner_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_behalf_of_email: Option<String>,
}

async fn get_flow_by_path_w_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<FlowWDraft> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow_o = sqlx::query_as::<_, FlowWDraft>(
        "SELECT flow.path, flow.summary, flow,description, flow_version.schema, flow_version.value, flow.extra_perms, flow.draft_only, flow.ws_error_handler_muted, flow.dedicated_worker, draft.value as draft, flow.tag, flow.visible_to_runner_only, flow.on_behalf_of_email
         FROM flow
        LEFT JOIN draft
            ON flow.path = draft.path AND draft.workspace_id = $2 AND draft.typ = 'flow' 
        LEFT JOIN flow_version 
            ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
        WHERE flow.path = $1 AND flow.workspace_id = $2",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let flow = not_found_if_none(flow_o, "Flow", path)?;
    Ok(Json(flow))
}

async fn exists_flow_by_path(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

#[derive(Deserialize)]
struct Archived {
    archived: Option<bool>,
}

async fn archive_flow_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(archived): Json<Archived>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "UPDATE flow SET archived = $1 WHERE path = $2 AND workspace_id = $3",
        archived.archived.unwrap_or(true),
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "flows.archive",
        ActionKind::Delete,
        &w_id,
        Some(path),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Flow {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        },
        Some(format!(
            "Flow '{}' {}",
            path,
            if archived.archived.unwrap_or(true) {
                "archived"
            } else {
                "unarchived"
            }
        )),
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::ArchiveFlow { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("Flow {path} archived"))
}

#[derive(Deserialize)]
struct DeleteFlowQuery {
    keep_captures: Option<bool>,
}

async fn delete_flow_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<DeleteFlowQuery>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'flow'",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM flow WHERE path = $1 AND workspace_id = $2",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    if !query.keep_captures.unwrap_or(false) {
        sqlx::query!(
            "DELETE FROM capture_config WHERE path = $1 AND workspace_id = $2 AND is_flow IS TRUE",
            path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM capture WHERE path = $1 AND workspace_id = $2 AND is_flow IS TRUE",
            path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "flows.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Flow {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        },
        Some(format!("Flow '{}' deleted", path)),
        true,
    )
    .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE path = $1 AND workspace_id = $2 AND script_hash IS NULL and app_version IS NULL",
        path,
        w_id
    )
    .execute(&db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "error deleting deployment metadata for script with path {path} in workspace {w_id}: {e:#}"
        ))
    })?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteFlow { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("Flow {path} deleted"))
}

#[cfg(test)]
mod tests {

    use std::{collections::HashMap, time::Duration};

    use windmill_common::{
        flows::{
            ConstantDelay, ExponentialDelay, FlowModule, FlowModuleValue, FlowValue,
            InputTransform, Retry, StopAfterIf,
        },
        scripts,
    };

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn flowmodule_serde() {
        let fv = FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    value: windmill_common::worker::to_raw_value(&FlowModuleValue::Script {
                        path: "test".to_string(),
                        input_transforms: [(
                            "test".to_string(),
                            InputTransform::Static {
                                value: windmill_common::worker::to_raw_value(&"test2".to_string()),
                            },
                        )]
                        .into(),
                        hash: None,
                        tag_override: None,
                        is_trigger: None,
                    }),
                    stop_after_if: None,
                    stop_after_all_iters_if: None,
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: windmill_common::worker::to_raw_value(&FlowModuleValue::RawScript {
                        input_transforms: HashMap::new(),
                        content: "test".to_string(),
                        language: scripts::ScriptLang::Deno,
                        path: None,
                        lock: None,
                        tag: None,
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        is_trigger: None,
                    }),
                    stop_after_if: Some(StopAfterIf {
                        expr: "foo = 'bar'".to_string(),
                        ..Default::default()
                    }),
                    stop_after_all_iters_if: None,
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
                FlowModule {
                    id: "c".to_string(),
                    value: windmill_common::worker::to_raw_value(&FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static {
                            value: windmill_common::worker::to_raw_value(&[1, 2, 3]),
                        },
                        modules: vec![],
                        modules_node: None,
                        skip_failures: true,
                        parallel: false,
                        parallelism: None,
                    }),
                    stop_after_if: Some(StopAfterIf {
                        expr: "previous.isEmpty()".to_string(),
                        ..Default::default()
                    }),
                    stop_after_all_iters_if: None,
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
            ],
            failure_module: Some(Box::new(FlowModule {
                id: "d".to_string(),
                value: FlowModuleValue::Script {
                    path: "test".to_string(),
                    input_transforms: HashMap::new(),
                    hash: None,
                    tag_override: None,
                    is_trigger: None,
                }
                .into(),
                stop_after_if: Some(StopAfterIf {
                    expr: "previous.isEmpty()".to_string(),
                    ..Default::default()
                }),
                stop_after_all_iters_if: None,
                summary: None,
                suspend: Default::default(),
                retry: None,
                sleep: None,
                cache_ttl: None,
                mock: None,
                timeout: None,
                priority: None,
                delete_after_use: None,
                continue_on_error: None,
                skip_if: None,
            })),
            preprocessor_module: None,
            same_worker: false,
            concurrent_limit: None,
            concurrency_time_window_s: None,
            skip_expr: None,
            cache_ttl: None,
            priority: None,
            early_return: None,
            concurrency_key: None,
        };
        let expect = serde_json::json!({
          "modules": [
            {
              "id": "a",
              "value": {
                "input_transforms": {
                    "test": {
                      "type": "static",
                      "value": "test2"
                    }
                  },
                "type": "script",
                "path": "test",
              },
            },
            {
              "id": "b",
              "value": {
                "input_transforms": {},
                "type": "rawscript",
                "content": "test",
                "language": "deno"
              },
              "stop_after_if": {
                  "expr": "foo = 'bar'",
                  "skip_if_stopped": false,
                  "error_message": null
              }
            },
            {
              "id": "c",
              "value": {
                "type": "forloopflow",
                "iterator": {
                  "type": "static",
                  "value": [
                    1,
                    2,
                    3
                  ]
                },
                "parallel": false,
                "skip_failures": true,
                "modules": []
              },
              "stop_after_if": {
                  "expr": "previous.isEmpty()",
                  "skip_if_stopped": false,
                  "error_message": null
              }
            }
          ],
          "failure_module": {
            "id": "d",
            "value": {
              "input_transforms": {},
              "type": "script",
              "path": "test",
            },
            "stop_after_if": {
                "expr": "previous.isEmpty()",
                "skip_if_stopped": false,
                "error_message": null
            }
          },
        });
        assert_eq!(dbg!(serde_json::json!(fv)), dbg!(expect));
    }

    #[test]
    fn retry_serde() {
        assert_eq!(Retry::default(), serde_json::from_str(r#"{}"#).unwrap());

        assert_eq!(
            Retry::default(),
            serde_json::from_str(
                r#"
                {
                  "constant": {
                    "seconds": 0
                  },
                  "exponential": {
                    "multiplier": 1,
                    "seconds": 0
                  }
                }
                "#
            )
            .unwrap()
        );

        assert_eq!(
            Retry {
                constant: Default::default(),
                exponential: ExponentialDelay {
                    attempts: 0,
                    multiplier: 1,
                    seconds: 123,
                    random_factor: None
                }
            },
            serde_json::from_str(
                r#"
                {
                  "constant": {},
                  "exponential": { "seconds": 123 }
                }
                "#
            )
            .unwrap()
        );
    }

    #[test]
    fn retry_exponential() {
        let retry = Retry {
            constant: ConstantDelay::default(),
            exponential: ExponentialDelay {
                attempts: 3,
                multiplier: 4,
                seconds: 3,
                random_factor: None,
            },
        };
        assert_eq!(
            vec![
                Some(12 * SECOND),
                Some(36 * SECOND),
                Some(108 * SECOND),
                None
            ],
            (0..4)
                .map(|previous_attempts| retry.interval(previous_attempts, false))
                .collect::<Vec<_>>()
        );

        assert_eq!(Some(108 * SECOND), retry.max_interval());
    }

    #[test]
    fn retry_both() {
        let retry = Retry {
            constant: ConstantDelay { attempts: 2, seconds: 4 },
            exponential: ExponentialDelay {
                attempts: 2,
                multiplier: 1,
                seconds: 3,
                random_factor: None,
            },
        };
        assert_eq!(
            vec![
                Some(4 * SECOND),
                Some(4 * SECOND),
                Some(27 * SECOND),
                Some(81 * SECOND),
                None,
            ],
            (0..5)
                .map(|previous_attempts| retry.interval(previous_attempts, false))
                .collect::<Vec<_>>()
        );

        assert_eq!(Some(81 * SECOND), retry.max_interval());
    }
}
