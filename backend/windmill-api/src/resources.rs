/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::{
    db::{ApiAuthed, DB},
    users::{maybe_refresh_folders, require_owner_of_path, Tokened},
    utils::check_scopes,
    webhook_util::{WebhookMessage, WebhookShared},
};
use axum::{
    body::Body,
    extract::{Extension, Path, Query},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use hyper::{header, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use sql_builder::{bind::Bind, quote, SqlBuilder};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use windmill_audit::audit_oss::{audit_log, AuditAuthor};
use windmill_audit::ActionKind;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, paginate, require_admin, Pagination, StripPath},
    variables,
    worker::CLOUD_HOSTED,
};
use windmill_tool_macros::windmill_tool;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_resources))
        .route("/list_search", get(list_search_resources))
        .route("/list_names/:type", get(list_names))
        .route("/get/*path", get(get_resource))
        .route("/exists/*path", get(exists_resource))
        .route("/get_value/*path", get(get_resource_value))
        .route(
            "/get_value_interpolated/*path",
            get(get_resource_value_interpolated),
        )
        .route("/update/*path", post(update_resource))
        .route("/update_value/*path", post(update_resource_value))
        .route("/delete/*path", delete(delete_resource))
        .route("/create", post(create_resource))
        .route("/type/list", get(list_resource_types))
        .route("/type/listnames", get(list_resource_types_names))
        .route("/type/get/:name", get(get_resource_type))
        .route("/type/exists/:name", get(exists_resource_type))
        .route("/type/update/:name", post(update_resource_type))
        .route("/type/delete/:name", delete(delete_resource_type))
        .route(
            "/file_resource_type_to_file_ext_map",
            get(file_resource_ext_to_resource_type),
        )
        .route("/type/create", post(create_resource_type))
}

pub fn public_service() -> Router {
    Router::new().route("/custom_component/:name", get(custom_component))
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct ResourceType {
    pub workspace_id: String,
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
    pub format_extension: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateResourceType {
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
    pub format_extension: Option<String>,
}

#[derive(Deserialize)]
pub struct EditResourceType {
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct Resource {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub resource_type: String,
    pub extra_perms: serde_json::Value,
    pub created_by: Option<String>,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct ListableResource {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub resource_type: String,
    pub extra_perms: serde_json::Value,
    pub created_by: Option<String>,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_linked: Option<bool>,
    pub is_refreshed: Option<bool>,
    pub is_oauth: Option<bool>,
    pub is_expired: Option<bool>,
    pub refresh_error: Option<String>,
    pub account: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateResource {
    pub path: String,
    pub value: Option<Box<RawValue>>,
    pub description: Option<String>,
    pub resource_type: String,
}
#[derive(Deserialize)]
struct EditResource {
    path: Option<String>,
    description: Option<String>,
    value: Option<Box<RawValue>>,
}

#[derive(Deserialize)]
pub struct ListResourceQuery {
    resource_type: Option<String>,
    resource_type_exclude: Option<String>,
    path_start: Option<String>,
}

#[derive(Serialize, FromRow)]
pub struct NamePath {
    name: String,
    path: String,
}
async fn list_names(
    authed: ApiAuthed,
    Path((w_id, rt)): Path<(String, String)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<NamePath>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT value->>'name' as name, path from resource WHERE resource_type = $1 AND workspace_id = $2",
        rt,
        &w_id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter_map(|x| x.name.map(|name| NamePath { name, path: x.path }))
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize, FromRow)]
pub struct SearchResource {
    path: String,
    value: serde_json::Value,
}
async fn list_search_resources(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<SearchResource>> {
    let mut tx = user_db.begin(&authed).await?;
    #[cfg(feature = "enterprise")]
    let n = 1000;

    #[cfg(not(feature = "enterprise"))]
    let n = 3;

    let rows = sqlx::query_as!(
        SearchResource,
        "SELECT path, value from resource WHERE workspace_id = $1 LIMIT $2",
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

#[windmill_tool(
    name = "list_resources",
    description = "List resources in a workspace",
    method = "GET", 
    path = "/list"
)]
async fn list_resources_tool(args: serde_json::Value) -> serde_json::Value {
    // For now, return a simple mock response
    // In step 4 we'll implement the actual call forwarding
    serde_json::json!({
        "message": "list_resources tool called",
        "args": args
    })
}

async fn list_resources(
    authed: ApiAuthed,
    Query(lq): Query<ListResourceQuery>,
    Query(pagination): Query<Pagination>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ListableResource>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("resource")
        .fields(&[
            "resource.workspace_id",
            "resource.path",
            "null::JSONB as value",
            "resource.description",
            "resource_type",
            "resource.extra_perms",
            "(now() > account.expires_at) as is_expired",
            "variable.path IS NOT NULL as is_linked",
            "account.refresh_token != '' as is_refreshed",
            "variable.is_oauth",
            "variable.account",
            "account.refresh_error",
            "resource.created_by",
            "resource.edited_at",
        ])
        .left()
        .join("variable")
        .on("variable.path = resource.path AND variable.workspace_id = resource.workspace_id")
        .left()
        .join("account")
        .on("variable.account = account.id AND account.workspace_id = variable.workspace_id")
        .order_by("path", true)
        .and_where("resource.workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if let Some(rt) = &lq.resource_type {
        let resource_type_filters = rt.split(',').collect::<Vec<&str>>();
        if resource_type_filters.len() == 1 {
            sqlb.and_where_eq("resource_type", "?".bind(rt));
        } else {
            let mut list = Vec::new();
            for rt in resource_type_filters {
                let quoted_value = quote(rt);
                list.push(quoted_value);
            }
            sqlb.and_where_in("resource_type", list.as_slice());
        }
    }
    if let Some(rt) = &lq.resource_type_exclude {
        for rt in rt.split(',') {
            sqlb.and_where_ne("resource_type", "?".bind(&rt));
        }
    }

    if let Some(path_start) = &lq.path_start {
        sqlb.and_where_like_left("resource.path", path_start);
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableResource>(&sql)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_resource(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<ListableResource> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let resource_o = sqlx::query_as!(
        ListableResource,
        "SELECT resource.*, (now() > account.expires_at) as is_expired, account.refresh_token != '' as is_refreshed,
        account.refresh_error,
        variable.path IS NOT NULL as is_linked,
        variable.is_oauth as \"is_oauth?\",
        variable.account
        FROM resource
        LEFT JOIN variable ON variable.path = resource.path AND variable.workspace_id = $2
        LEFT JOIN account ON variable.account = account.id AND account.workspace_id = $2
        WHERE resource.path = $1 AND resource.workspace_id = $2",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if resource_o.is_none() {
        explain_resource_perm_error(&path, &w_id, &db, &authed).await?;
    }
    let resource = not_found_if_none(resource_o, "Resource", path)?;
    Ok(Json(resource))
}

async fn exists_resource(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn get_resource_value(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let value_o = sqlx::query_scalar!(
        "SELECT value from resource WHERE path = $1 AND workspace_id = $2",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;
    if value_o.is_none() {
        explain_resource_perm_error(&path, &w_id, &db, &authed).await?;
    }

    let value = not_found_if_none(value_o, "Resource", path)?;
    Ok(Json(value))
}

async fn explain_resource_perm_error(
    path: &str,
    w_id: &str,
    db: &sqlx::Pool<Postgres>,
    authed: &ApiAuthed,
) -> windmill_common::error::Result<()> {
    let extra_perms = sqlx::query_scalar!(
        "SELECT extra_perms from resource WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Resource {} not found", path)))?;
    if path.starts_with("f/") {
        let folder = path.split("/").nth(1).ok_or_else(|| {
            Error::BadRequest(format!(
                "path {} should have at least 2 components separated by /",
                path
            ))
        })?;
        let folder_extra_perms = sqlx::query_scalar!(
            "SELECT extra_perms from folder WHERE name = $1 AND workspace_id = $2",
            folder,
            w_id
        )
        .fetch_optional(db)
        .await?;
        return Err(Error::NotAuthorized(format!(
            "Resource exists but you don't have access to it:\nresource perms: {}\nfolder perms: {}\nauthed as: {authed:?}",
            serde_json::to_string_pretty(&extra_perms).unwrap_or_default(), serde_json::to_string_pretty(&folder_extra_perms).unwrap_or_default()
        )));
    } else {
        return Err(Error::NotAuthorized(format!(
            "Resource exists but you don't have access to it:\nresource perms: {}\nauthed as: {authed:?}",
            serde_json::to_string_pretty(&extra_perms).unwrap_or_default()
        )));
    }
}

async fn custom_component(
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<Response> {
    let cc_o = sqlx::query_scalar!(
        "SELECT value->>'js' FROM resource
        WHERE path = $1 AND workspace_id = $2",
        format!("f/app_custom/{name}"),
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();

    let cc = not_found_if_none(cc_o, "Custom Component", name)?;
    let res = Response::builder().header(header::CONTENT_TYPE, "text/javascript");

    Ok(res.body(Body::from(cc)).unwrap())
}

#[derive(Deserialize)]
struct JobInfo {
    job_id: Option<Uuid>,
}
async fn get_resource_value_interpolated(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(job_info): Query<JobInfo>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;

    return get_resource_value_interpolated_internal(
        &authed,
        Some(user_db),
        &db,
        w_id.as_str(),
        path,
        job_info.job_id,
        token.as_str(),
    )
    .await
    .map(|success| Json(success));
}

use async_recursion::async_recursion;
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

pub async fn get_resource_value_interpolated_internal(
    authed: &ApiAuthed,
    user_db: Option<UserDB>, // if none, no permission will be checked to access the resource
    db: &DB,
    workspace: &str,
    path: &str,
    job_id: Option<Uuid>,
    token: &str,
) -> Result<Option<serde_json::Value>> {
    let mut tx = authed_transaction_or_default(authed, user_db.clone(), db).await?;

    let value_o = sqlx::query_scalar!(
        "SELECT value from resource WHERE path = $1 AND workspace_id = $2",
        path,
        workspace
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if value_o.is_none() {
        explain_resource_perm_error(path, workspace, db, &authed).await?;
    }

    let value = not_found_if_none(value_o, "Resource", path)?;
    if let Some(value) = value {
        Ok(Some(
            transform_json_value(
                authed,
                user_db.clone(),
                db,
                workspace,
                value,
                &job_id,
                token,
            )
            .await?,
        ))
    } else {
        Ok(None)
    }
}

#[async_recursion]
pub async fn transform_json_value<'c>(
    authed: &ApiAuthed,
    user_db: Option<UserDB>, // if none, no permission will be checked to access the resources/variables
    db: &DB,
    workspace: &str,
    v: Value,
    job_id: &Option<Uuid>,
    token: &str,
) -> Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();
            let tx: Transaction<'_, Postgres> =
                authed_transaction_or_default(authed, user_db.clone(), db).await?;

            let v = crate::variables::get_value_internal(
                tx,
                db,
                workspace,
                path,
                &user_db
                    .clone()
                    .map(|_| authed.into())
                    .unwrap_or(AuditAuthor {
                        email: "backend".to_string(),
                        username: "backend".to_string(),
                        username_override: None,
                        token_prefix: None,
                    }),
            )
            .await?;
            Ok(Value::String(v))
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();
            if path.split("/").count() < 2 {
                return Err(Error::internal_err(format!(
                    "Invalid resource path: {path}"
                )));
            }
            let mut tx: Transaction<'_, Postgres> =
                authed_transaction_or_default(authed, user_db.clone(), db).await?;
            let v = sqlx::query_scalar!(
                "SELECT value from resource WHERE path = $1 AND workspace_id = $2",
                path,
                &workspace
            )
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;
            let v = not_found_if_none(v, "Resource", path)?;
            if let Some(v) = v {
                transform_json_value(authed, user_db.clone(), db, workspace, v, job_id, token).await
            } else {
                Ok(Value::Null)
            }
        }
        Value::String(y) if y.starts_with("$") && job_id.is_some() => {
            let mut tx = authed_transaction_or_default(authed, user_db.clone(), db).await?;
            let job_id = job_id.unwrap();
            let job = sqlx::query!(
                "SELECT
                    email AS \"email!\",
                    created_by AS \"created_by!\",
                    parent_job, permissioned_as AS \"permissioned_as!\",
                    script_path, schedule_path, flow_step_id, root_job,
                    scheduled_for AS \"scheduled_for!: chrono::DateTime<chrono::Utc>\"
                FROM v2_as_queue WHERE id = $1 AND workspace_id = $2",
                job_id,
                workspace
            )
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;

            let job = not_found_if_none(job, "Job", job_id.to_string())?;

            let flow_path = if let Some(uuid) = job.parent_job {
                let mut tx: Transaction<'_, Postgres> =
                    authed_transaction_or_default(authed, user_db.clone(), db).await?;
                let p = sqlx::query_scalar!("SELECT runnable_path FROM v2_job WHERE id = $1", uuid)
                    .fetch_optional(&mut *tx)
                    .await?
                    .flatten();
                tx.commit().await?;
                p
            } else {
                None
            };

            let variables = variables::get_reserved_variables(
                &db.into(),
                workspace,
                token,
                &job.email,
                &job.created_by,
                &job_id.to_string(),
                &job.permissioned_as,
                job.script_path.clone(),
                job.parent_job.map(|x| x.to_string()),
                flow_path,
                job.schedule_path.clone(),
                job.flow_step_id.clone(),
                job.root_job.map(|x| x.to_string()),
                Some(job.scheduled_for.clone()),
            )
            .await;

            let name = y.strip_prefix("$").unwrap();

            let value = variables
                .iter()
                .find(|x| x.name == name)
                .map(|x| x.value.clone())
                .unwrap_or_else(|| y);
            Ok(serde_json::json!(value))
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                let v =
                    transform_json_value(authed, user_db.clone(), db, workspace, b, job_id, token)
                        .await?;
                m.insert(a.clone(), v);
            }
            Ok(Value::Object(m))
        }
        a @ _ => Ok(a),
    }
}

async fn authed_transaction_or_default<'c>(
    authed: &ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
) -> sqlx::error::Result<Transaction<'c, Postgres>> {
    if let Some(user_db) = user_db {
        user_db.begin(authed).await
    } else {
        db.clone().begin().await
    }
}

async fn check_path_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!(
            "Resource {} already exists",
            path
        )));
    }
    return Ok(());
}

#[derive(Deserialize)]
struct CreateResourceQuery {
    update_if_exists: Option<bool>,
}
async fn create_resource(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Query(q): Query<CreateResourceQuery>,
    Json(resource): Json<CreateResource>,
) -> Result<(StatusCode, String)> {
    check_scopes(&authed, || format!("resources:write:{}", resource.path))?;
    if *CLOUD_HOSTED {
        let nb_resources = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM resource WHERE workspace_id = $1",
            &w_id
        )
        .fetch_one(&db)
        .await?;
        if nb_resources.unwrap_or(0) >= 10000 {
            return Err(Error::BadRequest(
                    "You have reached the maximum number of resources (10000) on cloud. Contact support@windmill.dev to increase the limit"
                        .to_string(),
                ));
        }
    }
    let authed = maybe_refresh_folders(&resource.path, &w_id, authed, &db).await;

    let mut tx = user_db.begin(&authed).await?;

    let update_if_exists = q.update_if_exists.unwrap_or(false);
    if !update_if_exists {
        check_path_conflict(&mut tx, &w_id, &resource.path).await?;
    }

    let res_value = resource.value.unwrap_or_default();
    let raw_json = sqlx::types::Json(res_value.as_ref());

    sqlx::query!(
        "INSERT INTO resource
            (workspace_id, path, value, description, resource_type, created_by, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6, now()) ON CONFLICT (workspace_id, path)
            DO UPDATE SET value = $3, description = $4, resource_type = $5, edited_at = now()",
        w_id,
        resource.path,
        raw_json as sqlx::types::Json<&RawValue>,
        resource.description,
        resource.resource_type,
        authed.username
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "resources.create",
        ActionKind::Create,
        &w_id,
        Some(&resource.path),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Resource { path: resource.path.clone(), parent_path: None },
        Some(format!("Resource '{}' created", resource.path.clone())),
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateResource { workspace: w_id, path: resource.path.clone() },
    );

    Ok((
        StatusCode::CREATED,
        format!("resource {} created", resource.path),
    ))
}

async fn delete_resource(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:write:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM resource WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM variable WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "resources.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Resource { path: path.to_string(), parent_path: Some(path.to_string()) },
        Some(format!("Resource '{}' deleted", path)),
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteResource { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("resource {} deleted", path))
}

async fn update_resource(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditResource>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();
    check_scopes(&authed, || format!("resources:write:{}", path))?;

    let mut sqlb = SqlBuilder::update_table("resource");
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

    sqlb.set_str("edited_at", "now()");

    sqlb.returning("path");
    let authed = maybe_refresh_folders(path, &w_id, authed, &db).await;

    let mut tx = user_db.begin(&authed).await?;

    if let Some(npath) = ns.path {
        if npath != path {
            check_path_conflict(&mut tx, &w_id, &npath).await?;

            require_owner_of_path(&authed, path)?;

            sqlx::query!(
                "UPDATE variable SET path = $1 WHERE path = $2 AND workspace_id = $3",
                npath,
                path,
                w_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut *tx).await?;

    let npath = not_found_if_none(npath_o, "Resource", path)?;

    audit_log(
        &mut *tx,
        &authed,
        "resources.update",
        ActionKind::Update,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Resource { path: npath.to_string(), parent_path: Some(path.to_string()) },
        Some(format!("Resource '{}' updated", npath)),
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateResource {
            workspace: w_id,
            old_path: path.to_owned(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("resource {} updated (npath: {:?})", path, npath))
}

#[derive(FromRow, Serialize, Deserialize)]
struct UpdateResource {
    value: Option<serde_json::Value>,
}

async fn update_resource_value(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(nv): Json<UpdateResource>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:write:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "UPDATE resource SET value = $1, edited_at = now() WHERE path = $2 AND workspace_id = $3",
        nv.value,
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "resources.update",
        ActionKind::Update,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Resource { path: path.to_string(), parent_path: Some(path.to_string()) },
        None,
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateResource {
            workspace: w_id,
            old_path: path.to_owned(),
            new_path: path.to_owned(),
        },
    );

    Ok(format!("value of resource {} updated", path))
}

async fn file_resource_ext_to_resource_type(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<HashMap<String, String>> {
    #[derive(Serialize, sqlx::FromRow)]
    struct LocalFileResourceExtension {
        name: String,
        format_extension: Option<String>,
    }

    let r = sqlx::query_as!(LocalFileResourceExtension, "
        SELECT name, format_extension FROM resource_type WHERE format_extension IS NOT NULL AND (workspace_id = $1 OR workspace_id = 'admins')", w_id)
        .fetch_all(&db)
        .await?;

    let hashmap: HashMap<String, String> = r
        .into_iter()
        .filter_map(|entry| {
            if let Some(format_extension) = entry.format_extension {
                Some((entry.name, format_extension))
            } else {
                None
            }
        })
        .collect();

    Ok(Json(hashmap))
}

async fn list_resource_types(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ResourceType>> {
    let rows = sqlx::query_as!(
        ResourceType,
        "SELECT * from resource_type WHERE (workspace_id = $1 OR workspace_id = 'admins') ORDER \
         BY name",
        &w_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

async fn list_resource_types_names(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let rows = sqlx::query_scalar!(
        "SELECT name from resource_type WHERE (workspace_id = $1 OR workspace_id = 'admins') \
         ORDER BY name",
        &w_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

async fn get_resource_type(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<ResourceType> {
    let mut tx = user_db.begin(&authed).await?;

    let resource_type_o = sqlx::query_as!(
        ResourceType,
        "SELECT * from resource_type WHERE name = $1 AND (workspace_id = $2 OR workspace_id = 'admins')",
        &name,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let resource_type = not_found_if_none(resource_type_o, "ResourceType", name)?;
    Ok(Json(resource_type))
}

async fn exists_resource_type(
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource_type WHERE name = $1 AND (workspace_id = $2 OR workspace_id = 'admins'))",
        name,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn create_resource_type(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(resource_type): Json<CreateResourceType>,
) -> Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;

    check_rt_path_conflict(&mut tx, &w_id, &resource_type.name).await?;

    sqlx::query!(
        "INSERT INTO resource_type
            (workspace_id, name, schema, description, created_by, format_extension, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())",
        w_id,
        resource_type.name,
        resource_type.schema,
        resource_type.description,
        authed.username,
        resource_type.format_extension,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "resource_types.create",
        ActionKind::Create,
        &w_id,
        Some(&resource_type.name),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::ResourceType { path: resource_type.name.clone() },
        Some(format!(
            "Resource Type '{}' created",
            resource_type.name.clone()
        )),
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateResourceType { name: resource_type.name.clone() },
    );

    Ok((
        StatusCode::CREATED,
        format!("resource_type {} created", resource_type.name),
    ))
}

async fn check_rt_path_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    name: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource_type WHERE name = $1 AND workspace_id = $2)",
        name,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!(
            "Resource type {} already exists",
            name
        )));
    }
    return Ok(());
}

async fn delete_resource_type(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM resource_type WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "resource_types.delete",
        ActionKind::Delete,
        &w_id,
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::ResourceType { path: name.clone() },
        None,
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteResourceType { name: name.clone() },
    );

    Ok(format!("resource_type {} deleted", name))
}

async fn update_resource_type(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
    Json(ns): Json<EditResourceType>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let mut sqlb = SqlBuilder::update_table("resource_type");
    sqlb.and_where_eq("name", "?".bind(&name));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));
    if let Some(nschema) = ns.schema {
        sqlb.set_str("schema", nschema);
    }
    if let Some(ndesc) = ns.description {
        sqlb.set_str("description", ndesc);
    }
    sqlb.set_str("edited_at", "now()");
    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query(&sql).execute(&mut *tx).await?;
    audit_log(
        &mut *tx,
        &authed,
        "resource_types.update",
        ActionKind::Update,
        &w_id,
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::ResourceType { path: name.clone() },
        None,
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateResourceType { name: name.clone() },
    );

    Ok(format!("resource_type {} updated", name))
}

#[cfg(any(
    feature = "http_trigger",
    feature = "postgres_trigger",
    feature = "mqtt_trigger",
    all(
        feature = "enterprise",
        any(feature = "sqs_trigger", feature = "gcp_trigger")
    )
))]
pub async fn try_get_resource_from_db_as<T>(
    authed: &ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    resource_path: &str,
    w_id: &str,
) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let resource = get_resource_value_interpolated_internal(
        &authed,
        user_db,
        &db,
        &w_id,
        &resource_path,
        None,
        "",
    )
    .await?;

    let resource = match resource {
        Some(resource) => serde_json::from_value::<T>(resource)
            .map_err(|e| Error::SerdeJson { error: e, location: "resources.rs".to_string() })?,
        None => {
            return {
                Err(Error::NotFound(format!(
                    "resource at path :{} do not exist",
                    &resource_path
                )))
            }
        }
    };
}