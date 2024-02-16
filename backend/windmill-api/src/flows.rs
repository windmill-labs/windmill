/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::db::ApiAuthed;
use crate::{
    db::DB,
    schedule::clear_schedule,
    users::{maybe_refresh_folders, require_owner_of_path},
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
use axum::body::StreamBody;
use axum::response::IntoResponse;
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};

use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sql_builder::prelude::*;
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::utils::query_elems_from_hub;
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, Error, JsonResult, Result},
    flows::{Flow, ListFlowQuery, ListableFlow, NewFlow},
    jobs::JobPayload,
    schedule::Schedule,
    scripts::Schema,
    utils::{http_get_from_hub, not_found_if_none, paginate, Pagination, StripPath},
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, schedule::push_scheduled_job, PushIsolationLevel, QueueTransaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_flows))
        .route("/list_search", get(list_search_flows))
        .route("/create", post(create_flow))
        .route("/update/*path", post(update_flow))
        .route("/archive/*path", post(archive_flow_by_path))
        .route("/delete/*path", delete(delete_flow_by_path))
        .route("/get/*path", get(get_flow_by_path))
        .route("/get/draft/*path", get(get_flow_by_path_w_draft))
        .route("/exists/*path", get(exists_flow_by_path))
        .route("/list_paths", get(list_paths))
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
    value: serde_json::Value,
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

    let rows = sqlx::query_as!(
        SearchFlow,
        "SELECT path, value from flow WHERE workspace_id = $1 LIMIT $2",
        &w_id,
        n
    )
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
            "edited_by",
            "edited_at",
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
        .order_desc("favorite.path IS NOT NULL")
        .order_by("edited_at", lq.order_desc.unwrap_or(true))
        .and_where("o.workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    sqlb.and_where_eq("archived", lq.show_archived.unwrap_or(false));

    if let Some(ps) = &lq.path_start {
        sqlb.and_where_like_left("path", "?".bind(ps));
    }
    if let Some(p) = &lq.path_exact {
        sqlb.and_where_eq("path", "?".bind(p));
    }
    if let Some(cb) = &lq.edited_by {
        sqlb.and_where_eq("edited_by", "?".bind(cb));
    }
    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
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
        "https://hub.windmill.dev/searchFlowData?approved=true",
        None,
        &db,
    )
    .await?;
    Ok::<_, Error>((
        status_code,
        headers,
        StreamBody::new(response.bytes_stream()),
    ))
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
) -> JsonResult<serde_json::Value> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("https://hub.windmill.dev/flows/{id}/json"),
        false,
        None,
        &db,
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
}

#[derive(Deserialize)]
pub struct ToggleWorkspaceErrorHandler {
    pub muted: Option<bool>,
}
async fn toggle_workspace_error_handler(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(req): Json<ToggleWorkspaceErrorHandler>,
) -> Result<String> {
    #[cfg(not(feature = "enterprise"))]
    {
        return Err(Error::BadRequest(
            "Muting the error handler for certain flow is only available in enterprise version"
                .to_string(),
        ));
    }

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

async fn create_flow(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
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

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();

    check_path_conflict(tx.transaction_mut(), &w_id, &nf.path).await?;
    check_schedule_conflict(tx.transaction_mut(), &w_id, &nf.path).await?;

    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, \
         schema, dependency_job, draft_only, tag, dedicated_worker) VALUES ($1, $2, $3, $4, $5, $6, now(), $7::text::json, NULL, $8, $9, $10)",
        w_id,
        nf.path,
        nf.summary,
        nf.description.unwrap_or_else(String::new),
        nf.value,
        &authed.username,
        nf.schema.and_then(|x| serde_json::to_string(&x.0).ok()),
        nf.draft_only,
        nf.tag,
        nf.dedicated_worker
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'flow'",
        nf.path,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
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

    let mut args: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(dm) = nf.deployment_message {
        args.insert("deployment_message".to_string(), json!(dm));
    }

    let tx = PushIsolationLevel::Transaction(tx);
    let (dependency_job_uuid, mut new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::FlowDependencies {
            path: nf.path.clone(),
            dedicated_worker: nf.dedicated_worker,
        },
        args,
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
    )
    .await?;

    sqlx::query!(
        "UPDATE flow SET dependency_job = $1 WHERE path = $2 AND workspace_id = $3",
        dependency_job_uuid,
        nf.path,
        w_id
    )
    .execute(&mut new_tx)
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

async fn update_flow(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
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

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();

    check_schedule_conflict(tx.transaction_mut(), &w_id, flow_path).await?;

    let schema = nf.schema.map(|x| x.0);
    let old_dep_job = sqlx::query_scalar!(
        "SELECT dependency_job FROM flow WHERE path = $1 AND workspace_id = $2",
        flow_path,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    let old_dep_job = not_found_if_none(old_dep_job, "Flow", flow_path)?;
    sqlx::query!(
        "UPDATE flow SET path = $1, summary = $2, description = $3, value = $4, edited_by = $5, \
         edited_at = now(), schema = $6::text::json, dependency_job = NULL, draft_only = NULL, tag = $9, dedicated_worker = $10
        WHERE path = $7 AND workspace_id = $8",
        nf.path,
        nf.summary,
        nf.description.unwrap_or_else(String::new),
        nf.value,
        &authed.username,
        schema.and_then(|x| serde_json::to_string(&x).ok()),
        flow_path,
        w_id,
        nf.tag,
        nf.dedicated_worker
    )
    .execute(&mut tx)
    .await?;

    if nf.path != flow_path {
        check_schedule_conflict(tx.transaction_mut(), &w_id, &nf.path).await?;

        if !authed.is_admin {
            require_owner_of_path(&authed, flow_path)?;
        }
    }

    let mut schedulables: Vec<Schedule> = sqlx::query_as!(
        Schedule,
            "UPDATE schedule SET script_path = $1 WHERE script_path = $2 AND path != $2 AND workspace_id = $3 AND is_flow IS true RETURNING *",
            nf.path,
            flow_path,
            w_id,
        )
        .fetch_all(&mut tx)
        .await?;

    let schedule = sqlx::query_as!(Schedule,
        "UPDATE schedule SET path = $1, script_path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS true RETURNING *",
        nf.path,
        flow_path,
        w_id,
    )
    .fetch_optional(&mut tx)
    .await?;

    if let Some(schedule) = schedule {
        clear_schedule(tx.transaction_mut(), &flow_path, &w_id).await?;
        schedulables.push(schedule);
    }

    for schedule in schedulables.into_iter() {
        clear_schedule(tx.transaction_mut(), &schedule.path, &w_id).await?;

        if schedule.enabled {
            tx = push_scheduled_job(&db, tx, schedule).await?;
        }
    }

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'flow'",
        flow_path,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
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

    let mut args: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(dm) = nf.deployment_message {
        args.insert("deployment_message".to_string(), json!(dm));
    }
    args.insert("parent_path".to_string(), json!(flow_path));

    let (dependency_job_uuid, mut new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::FlowDependencies {
            path: nf.path.clone(),
            dedicated_worker: nf.dedicated_worker,
        },
        args,
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
    )
    .await?;
    sqlx::query!(
        "UPDATE flow SET dependency_job = $1 WHERE path = $2 AND workspace_id = $3",
        dependency_job_uuid,
        nf.path,
        w_id
    )
    .execute(&mut new_tx)
    .await?;
    if let Some(old_dep_job) = old_dep_job {
        sqlx::query!(
            "UPDATE queue SET canceled = true WHERE id = $1",
            old_dep_job
        )
        .execute(&mut new_tx)
        .await?;
    }

    new_tx.commit().await?;

    Ok(nf.path.to_string())
}

async fn get_flow_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Flow> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow_o =
        sqlx::query_as::<_, Flow>("SELECT * FROM flow WHERE path = $1 AND workspace_id = $2")
            .bind(path)
            .bind(w_id)
            .fetch_optional(&mut *tx)
            .await?;
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
    pub value: serde_json::Value,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_worker: Option<bool>,
}

async fn get_flow_by_path_w_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<FlowWDraft> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow_o = sqlx::query_as::<_, FlowWDraft>(
        "SELECT flow.path, flow.summary, flow,description, flow.schema, flow.value, flow.extra_perms, flow.draft_only, flow.ws_error_handler_muted, flow.dedicated_worker, draft.value as draft, flow.tag
         FROM flow
        LEFT JOIN draft ON 
        flow.path = draft.path AND draft.workspace_id = $2 AND draft.typ = 'flow' 
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
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
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
        &authed.username,
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
        DeployedObject::Flow { path: path.to_string(), parent_path: Some(path.to_string()) },
        Some(format!(
            "Flow '{}' {}",
            path,
            if archived.archived.unwrap_or(true) {
                "archived"
            } else {
                "unarchived"
            }
        )),
        rsmq,
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::ArchiveFlow { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("Flow {path} archived"))
}

async fn delete_flow_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
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

    audit_log(
        &mut *tx,
        &authed.username,
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
        DeployedObject::Flow { path: path.to_string(), parent_path: Some(path.to_string()) },
        Some(format!("Flow '{}' deleted", path)),
        rsmq,
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
        Error::InternalErr(format!(
            "error deleting deployment metadata for script with path {path} in workspace {w_id}: {e}"
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
                    value: FlowModuleValue::Script {
                        path: "test".to_string(),
                        input_transforms: [(
                            "test".to_string(),
                            InputTransform::Static { value: serde_json::json!("test2") },
                        )]
                        .into(),
                        hash: None,
                        tag_override: None,
                    },
                    stop_after_if: None,
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: HashMap::new(),
                        content: "test".to_string(),
                        language: scripts::ScriptLang::Deno,
                        path: None,
                        lock: None,
                        tag: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                    },
                    stop_after_if: Some(StopAfterIf {
                        expr: "foo = 'bar'".to_string(),
                        skip_if_stopped: false,
                    }),
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                },
                FlowModule {
                    id: "c".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static { value: serde_json::json!([1, 2, 3]) },
                        modules: vec![],
                        skip_failures: true,
                        parallel: false,
                        parallelism: None,
                    },
                    stop_after_if: Some(StopAfterIf {
                        expr: "previous.isEmpty()".to_string(),
                        skip_if_stopped: false,
                    }),
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                },
            ],
            failure_module: Some(FlowModule {
                id: "d".to_string(),
                value: FlowModuleValue::Script {
                    path: "test".to_string(),
                    input_transforms: HashMap::new(),
                    hash: None,
                    tag_override: None,
                },
                stop_after_if: Some(StopAfterIf {
                    expr: "previous.isEmpty()".to_string(),
                    skip_if_stopped: false,
                }),
                summary: None,
                suspend: Default::default(),
                retry: None,
                sleep: None,
                cache_ttl: None,
                mock: None,
                timeout: None,
                priority: None,
                delete_after_use: None,
            }),
            same_worker: false,
            concurrent_limit: None,
            concurrency_time_window_s: None,
            skip_expr: None,
            cache_ttl: None,
            priority: None,
            early_return: None,
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
                "tag_override": Option::<String>::None,
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
                  "skip_if_stopped": false
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
              }
            }
          ],
          "failure_module": {
            "id": "d",
            "value": {
              "input_transforms": {},
              "type": "script",
              "path": "test",
              "tag_override": Option::<String>::None,
            },
            "stop_after_if": {
                "expr": "previous.isEmpty()",
                "skip_if_stopped": false
            }
          }
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
                .map(|previous_attempts| retry.interval(previous_attempts))
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
                .map(|previous_attempts| retry.interval(previous_attempts))
                .collect::<Vec<_>>()
        );

        assert_eq!(Some(81 * SECOND), retry.max_interval());
    }
}
