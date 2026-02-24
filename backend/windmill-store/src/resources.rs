/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use windmill_api_auth::{
    check_scopes, maybe_refresh_folders, require_owner_of_path, require_super_admin, ApiAuthed,
    Tokened,
};
use windmill_common::db::DB;
use windmill_common::workspaces::{check_user_against_rule, ProtectionRuleKind, RuleCheckResult};

use crate::secret_backend_ext::rename_vault_secret;
use crate::var_resource_cache::{cache_resource, get_cached_resource};
use windmill_common::utils::BulkDeleteRequest;
use windmill_common::webhook::{WebhookMessage, WebhookShared};

use axum::{
    body::Body,
    extract::{Extension, Path, Query},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use futures::future::try_join_all;
use hyper::{header, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use sql_builder::{bind::Bind, quote, SqlBuilder};
use sqlx::{Acquire, FromRow, Postgres, Transaction};
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::{
    db::{DbWithOptAuthed, UserDB},
    error::{self, Error, JsonResult, Result},
    get_database_url,
    utils::{
        get_custom_pg_instance_password, not_found_if_none, paginate, require_admin, Pagination,
        StripPath,
    },
    variables,
    worker::{CLOUD_HOSTED, TMP_DIR},
    PgDatabase,
};

use async_recursion::async_recursion;
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

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
        .route("/delete_bulk", delete(delete_resources_bulk))
        .route("/create", post(create_resource))
        .route("/git_commit_hash/*path", get(get_git_commit_hash))
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
    pub is_fileset: bool,
}

#[derive(Deserialize)]
pub struct CreateResourceType {
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
    pub format_extension: Option<String>,
    pub is_fileset: Option<bool>,
}

#[derive(Deserialize)]
pub struct EditResourceType {
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_fileset: Option<bool>,
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
    pub resource_type: Option<String>,
    pub resource_type_exclude: Option<String>,
    pub path_start: Option<String>,
    pub path: Option<String>,
    pub description: Option<String>,
    // filter by matching a subset of the value using base64 encoded json subset
    pub value: Option<String>,
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

    if let Some(path) = &lq.path {
        sqlb.and_where_eq("resource.path", "?".bind(path));
    }

    if let Some(description) = &lq.description {
        sqlb.and_where("resource.description ILIKE ?".bind(&format!("%{}%", description)));
    }

    if let Some(value) = &lq.value {
        sqlb.and_where("resource.value @> ?".bind(&value.replace("'", "''")));
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

pub async fn explain_resource_perm_error(
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
pub struct JobInfo {
    pub job_id: Option<Uuid>,
    pub allow_cache: Option<bool>,
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

    let db_with_opt_authed =
        DbWithOptAuthed::from_authed(&authed, db.clone(), Some(user_db.clone()));
    get_resource_value_interpolated_internal(
        &db_with_opt_authed,
        w_id.as_str(),
        path,
        job_info.job_id,
        Some(token.as_str()),
        job_info.allow_cache.unwrap_or(false),
    )
    .await
    .map(|success| Json(success))
}

pub async fn get_resource_value_interpolated_internal<'a>(
    db_with_opt_authed: &'a DbWithOptAuthed<'a, ApiAuthed>,
    workspace: &str,
    path: &str,
    job_id: Option<Uuid>,
    token_for_context: Option<&str>,
    allow_cache: bool,
) -> Result<Option<serde_json::Value>> {
    // This is a special syntax to help debugging custom instance databases
    if let Some(dbname) = path.strip_prefix("CUSTOM_INSTANCE_DB/") {
        let db = db_with_opt_authed.db();
        require_super_admin(db_with_opt_authed.db(), &db_with_opt_authed.email()).await?;
        let mut pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
        pg_creds.dbname = dbname.to_string();
        pg_creds.password = Some(get_custom_pg_instance_password(&db).await?);
        pg_creds.user = Some("custom_instance_user".to_string());
        let pg_creds = serde_json::to_value(&pg_creds)
            .map_err(|e| Error::internal_err(format!("Error serializing pg creds: {}", e)))?;
        return Ok(Some(pg_creds));
    }

    if allow_cache {
        if let Some(cached_value) = get_cached_resource(&workspace, &path) {
            return Ok(Some(cached_value));
        }
    }
    use sqlx::Acquire;
    let mut tx = db_with_opt_authed.begin().await?;

    let value_o = sqlx::query_scalar!(
        "SELECT value from resource WHERE path = $1 AND workspace_id = $2",
        path,
        workspace
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if value_o.is_none() {
        if let Some(authed) = db_with_opt_authed.authed() {
            let db = db_with_opt_authed.db();
            explain_resource_perm_error(path, workspace, db, authed).await?;
        }
    }

    let value = not_found_if_none(value_o, "Resource", path)?;
    if let Some(value) = value {
        let r = transform_json_value(
            &db_with_opt_authed,
            workspace,
            value,
            &job_id,
            token_for_context,
            0,
        )
        .await?;
        if allow_cache {
            cache_resource(&workspace, &path, r.clone());
        }
        Ok(Some(r))
    } else {
        Ok(None)
    }
}

#[async_recursion]
pub async fn transform_json_value(
    db_with_opt_authed: &DbWithOptAuthed<ApiAuthed>,
    workspace: &str,
    v: Value,
    job_id: &Option<Uuid>,
    token: Option<&str>,
    depth: u8,
) -> Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();

            let v =
                crate::variables::get_value_internal(&db_with_opt_authed, workspace, path, false)
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
            let mut tx: Transaction<'_, Postgres> = db_with_opt_authed.begin().await?;
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
                transform_json_value(db_with_opt_authed, workspace, v, job_id, token, depth + 1)
                    .await
            } else {
                Ok(Value::Null)
            }
        }
        Value::String(y) if y.starts_with("$") && job_id.is_some() => {
            let mut tx = db_with_opt_authed.begin().await?;
            let job_id = job_id.unwrap();
            let job = sqlx::query!(
                "SELECT
                    v2_job.permissioned_as_email,
                    v2_job.created_by,
                    v2_job.parent_job,
                    v2_job.permissioned_as,
                    v2_job.runnable_path,
                    CASE WHEN v2_job.trigger_kind = 'schedule'::job_trigger_kind THEN v2_job.trigger END AS schedule_path,
                    v2_job.flow_step_id,
                    v2_job.flow_innermost_root_job,
                    v2_job.root_job,
                    v2_job_queue.scheduled_for AS \"scheduled_for: chrono::DateTime<chrono::Utc>\"
                FROM v2_job INNER JOIN v2_job_queue ON v2_job.id = v2_job_queue.id
                WHERE v2_job.id = $1 AND v2_job.workspace_id = $2",
                job_id,
                workspace
            )
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;

            let job = not_found_if_none(job, "Job", job_id.to_string())?;

            let flow_path = if let Some(uuid) = job.parent_job {
                let mut tx: Transaction<'_, Postgres> = db_with_opt_authed.begin().await?;
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
                &db_with_opt_authed.db().into(),
                workspace,
                token.unwrap_or_else(|| "no_token_available"),
                &job.permissioned_as_email,
                &job.created_by,
                &job_id.to_string(),
                &job.permissioned_as,
                job.runnable_path.clone(),
                job.parent_job.map(|x| x.to_string()),
                flow_path,
                job.schedule_path.clone(),
                job.flow_step_id.clone(),
                job.flow_innermost_root_job.map(|x| x.to_string()),
                job.root_job.map(|x| x.to_string()),
                Some(job.scheduled_for.clone()),
                None,
                None,
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
        Value::Array(mut arr) if depth <= 2 && arr.len() <= 1000 => {
            for i in 0..arr.len() {
                let val = std::mem::take(&mut arr[i]);
                arr[i] = transform_json_value(
                    db_with_opt_authed,
                    workspace,
                    val,
                    job_id,
                    token,
                    depth + 1,
                )
                .await?;
            }
            Ok(Value::Array(arr))
        }
        Value::Array(arr) => {
            if arr.len() > 1000 {
                tracing::warn!(
                    "Array with {} items exceeds 1000 item limit for variable/resource resolution, skipping",
                    arr.len()
                );
            }
            Ok(Value::Array(arr))
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                let v = transform_json_value(
                    db_with_opt_authed,
                    workspace,
                    b,
                    job_id,
                    token,
                    depth + 1,
                )
                .await?;
                m.insert(a.clone(), v);
            }
            Ok(Value::Object(m))
        }
        a @ _ => Ok(a),
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
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

    if resource.path.starts_with("f/app_themes/") {
        sqlx::query!(
            "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by, edited_at) VALUES ($1, 'app_themes', 'App Themes', ARRAY[]::TEXT[], '{\"g/all\": false}', $2, now()) ON CONFLICT DO NOTHING",
            w_id,
            authed.username,
        )
        .execute(&db)
        .await?;
    } else if resource.path.starts_with("f/app_custom/") {
        sqlx::query!(
            "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by, edited_at) VALUES ($1, 'app_custom', 'App Custom Components', ARRAY[]::TEXT[], '{\"g/all\": false}', $2, now()) ON CONFLICT DO NOTHING",
            w_id,
            authed.username,
        )
        .execute(&db)
        .await?;
    } else if resource.path.starts_with("f/app_groups/") {
        sqlx::query!(
            "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by, edited_at) VALUES ($1, 'app_groups', 'App Groups', ARRAY[]::TEXT[], '{\"g/all\": false}', $2, now()) ON CONFLICT DO NOTHING",
            w_id,
            authed.username,
        )
        .execute(&db)
        .await?;
    }
    sqlx::query!(
        "INSERT INTO resource
            (workspace_id, path, value, description, resource_type, created_by, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6, now()) ON CONFLICT (workspace_id, path)
            DO UPDATE SET value = EXCLUDED.value, description = EXCLUDED.description, resource_type = EXCLUDED.resource_type, edited_at = now()",
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
        None,
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    let mut tx = user_db.begin(&authed).await?;

    let deleted_path = sqlx::query_scalar!(
        "DELETE FROM resource WHERE path = $1 AND workspace_id = $2 RETURNING path",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    not_found_if_none(deleted_path, "Resource", &path)?;
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
        None,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteResource { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("resource {} deleted", path))
}

async fn delete_resources_bulk(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(request): Json<BulkDeleteRequest>,
) -> JsonResult<Vec<String>> {
    for path in &request.paths {
        check_scopes(&authed, || format!("resources:write:{}", path))?;
    }

    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = user_db.begin(&authed).await?;

    let deleted_paths = sqlx::query_scalar!(
        "DELETE FROM resource WHERE path = ANY($1) AND workspace_id = $2 RETURNING path",
        &request.paths,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "resources.delete_bulk",
        ActionKind::Delete,
        &w_id,
        Some(&deleted_paths.join(", ")),
        None,
    )
    .await?;

    tx.commit().await?;

    try_join_all(deleted_paths.iter().map(|path| {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            DeployedObject::Resource {
                path: path.to_string(),
                parent_path: Some(path.to_string()),
            },
            Some(format!("Resource '{}' deleted", path)),
            true,
            None,
        )
    }))
    .await?;

    for path in &deleted_paths {
        webhook.send_message(
            w_id.clone(),
            WebhookMessage::DeleteResource { workspace: w_id.clone(), path: path.to_owned() },
        );
    }

    Ok(Json(deleted_paths))
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

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

    if let Some(npath) = ns.path.clone() {
        if npath != path {
            check_path_conflict(&mut tx, &w_id, &npath).await?;

            require_owner_of_path(&authed, path)?;

            // Handle Vault secret rename if the linked variable is a Vault-stored secret
            let linked_var = sqlx::query!(
                "SELECT value, is_secret FROM variable WHERE path = $1 AND workspace_id = $2",
                path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(var) = linked_var {
                if var.is_secret {
                    // Check if this is a Vault-stored secret and rename it
                    if let Some(new_value) =
                        rename_vault_secret(&db, &w_id, path, &npath, &var.value).await?
                    {
                        // Update the variable's value to point to the new Vault path
                        sqlx::query!(
                            "UPDATE variable SET value = $1 WHERE path = $2 AND workspace_id = $3",
                            new_value,
                            path,
                            w_id
                        )
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }

            sqlx::query!(
                "UPDATE variable SET path = $1 WHERE path = $2 AND workspace_id = $3",
                npath,
                path,
                w_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "UPDATE workspace_integrations SET resource_path = $1 WHERE workspace_id = $2 AND resource_path = $3",
                npath,
                w_id,
                path
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

    // Detect if this was a rename operation
    let old_path_if_renamed = if npath != path { Some(path) } else { None };

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Resource { path: npath.to_string(), parent_path: Some(path.to_string()) },
        Some(format!("Resource '{}' updated", npath)),
        true,
        old_path_if_renamed,
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
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
        None,
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

#[derive(Serialize)]
pub struct FileResourceTypeInfo {
    pub format_extension: Option<String>,
    pub is_fileset: bool,
}

async fn file_resource_ext_to_resource_type(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<HashMap<String, FileResourceTypeInfo>> {
    #[derive(sqlx::FromRow)]
    struct LocalFileResourceExtension {
        name: String,
        format_extension: Option<String>,
        is_fileset: bool,
    }

    let r = sqlx::query_as!(LocalFileResourceExtension, "
        SELECT name, format_extension, is_fileset FROM resource_type WHERE (format_extension IS NOT NULL OR is_fileset = true) AND (workspace_id = $1 OR workspace_id = 'admins')", w_id)
        .fetch_all(&db)
        .await?;

    let hashmap: HashMap<String, FileResourceTypeInfo> = r
        .into_iter()
        .map(|entry| {
            (
                entry.name,
                FileResourceTypeInfo {
                    format_extension: entry.format_extension,
                    is_fileset: entry.is_fileset,
                },
            )
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = user_db.begin(&authed).await?;

    check_rt_path_conflict(&mut tx, &w_id, &resource_type.name).await?;

    let is_fileset = resource_type.is_fileset.unwrap_or(false);

    if is_fileset && resource_type.format_extension.is_some() {
        return Err(Error::BadRequest(
            "A fileset resource type cannot have a format_extension".to_string(),
        ));
    }

    sqlx::query!(
        "INSERT INTO resource_type
            (workspace_id, name, schema, description, created_by, format_extension, is_fileset, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
        w_id,
        resource_type.name,
        resource_type.schema,
        resource_type.description,
        authed.username,
        resource_type.format_extension,
        is_fileset,
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
        None,
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = user_db.begin(&authed).await?;

    let deleted_name = sqlx::query_scalar!(
        "DELETE FROM resource_type WHERE name = $1 AND workspace_id = $2 RETURNING name",
        name,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    not_found_if_none(deleted_name, "ResourceType", &name)?;

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
        None,
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
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut sqlb = SqlBuilder::update_table("resource_type");
    sqlb.and_where_eq("name", "?".bind(&name));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));
    if let Some(nschema) = ns.schema {
        sqlb.set_str("schema", nschema);
    }
    if let Some(ndesc) = ns.description {
        sqlb.set_str("description", ndesc);
    }
    if let Some(is_fileset) = ns.is_fileset {
        sqlb.set("is_fileset", if is_fileset { "TRUE" } else { "FALSE" });
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
        None,
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
        any(
            feature = "sqs_trigger",
            feature = "gcp_trigger",
            feature = "kafka",
            feature = "nats"
        )
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
        &DbWithOptAuthed::from_authed(authed, db.clone(), user_db),
        &w_id,
        &resource_path,
        None,
        None,
        false,
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

    Ok(resource)
}

#[derive(Deserialize, Serialize)]
struct GitRepositoryResource {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
}

/// Validates a git URL to prevent git option injection attacks.
/// Git URLs starting with '-' could be interpreted as command-line options.
fn validate_git_url(url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() {
        return Err(Error::BadRequest("Git URL cannot be empty".to_string()));
    }
    if url.starts_with('-') {
        return Err(Error::BadRequest(
            "Git URL cannot start with '-' (potential option injection)".to_string(),
        ));
    }
    // Block other potentially dangerous patterns
    if url.contains('\0') || url.contains('\n') || url.contains('\r') {
        return Err(Error::BadRequest(
            "Git URL contains invalid characters".to_string(),
        ));
    }
    Ok(())
}

/// Validates a git branch/ref name to prevent injection attacks.
fn validate_git_ref(ref_name: &str) -> Result<()> {
    let ref_name = ref_name.trim();
    if ref_name.is_empty() {
        return Err(Error::BadRequest("Git ref cannot be empty".to_string()));
    }
    if ref_name.starts_with('-') {
        return Err(Error::BadRequest(
            "Git ref cannot start with '-' (potential option injection)".to_string(),
        ));
    }
    // Git ref names have specific rules - block dangerous characters
    if ref_name.contains('\0')
        || ref_name.contains('\n')
        || ref_name.contains('\r')
        || ref_name.contains("..")
        || ref_name.contains("@{")
        || ref_name.ends_with('.')
        || ref_name.ends_with('/')
        || ref_name.contains("//")
    {
        return Err(Error::BadRequest(
            "Git ref contains invalid characters or patterns".to_string(),
        ));
    }
    Ok(())
}

#[derive(Serialize)]
struct GitCommitHashResponse {
    commit_hash: String,
}

#[derive(Deserialize)]
struct GitCommitHashQuery {
    git_ssh_identity: Option<String>,
}

async fn get_git_commit_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<GitCommitHashQuery>,
) -> JsonResult<GitCommitHashResponse> {
    let path = path.to_path();

    check_scopes(&authed, || format!("resources:read:{}", path))?;

    let db_with_opt_authed =
        DbWithOptAuthed::from_authed(&authed, db.clone(), Some(user_db.clone()));
    let git_repo_resource_value = get_resource_value_interpolated_internal(
        &db_with_opt_authed,
        &w_id,
        path,
        None,
        None,
        false,
    )
    .await
    .map_err(|e| Error::NotFound(format!("Access to resource {} denied: ({e})", path)))?;

    let git_resource: GitRepositoryResource = match git_repo_resource_value {
        Some(value) => serde_json::from_value(value).map_err(|e| {
            Error::BadRequest(format!("Invalid git repository resource format: {}", e))
        })?,
        None => return Err(Error::NotFound(format!("Resource {} not found", path)).into()),
    };

    let identities: Vec<String> = query
        .git_ssh_identity
        .map(|s| {
            s.split(",")
                .filter_map(|s| {
                    if !s.is_empty() {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or(vec![]);

    let (git_ssh_cmd, filenames) =
        get_git_ssh_cmd(&authed, &user_db, &db, &w_id, identities).await?;

    let commit_hash = get_repo_latest_commit_hash(&git_resource, git_ssh_cmd).await;

    delete_paths(&filenames).await;

    Ok(Json(GitCommitHashResponse { commit_hash: commit_hash? }))
}

async fn write_ssh_file(
    authed: &ApiAuthed,
    user_db: &UserDB,
    db: &DB,
    w_id: &str,
    var_path: &str,
) -> std::result::Result<std::path::PathBuf, (error::Error, std::path::PathBuf)> {
    let id_file_name = format!(".ssh_id_priv_{}", Uuid::new_v4());
    let loc = std::path::Path::new(TMP_DIR)
        .join("ssh_ids")
        .join(id_file_name);

    let userdb_authed = DbWithOptAuthed::from_authed(authed, db.clone(), Some(user_db.clone()));
    let mut content = crate::variables::get_value_internal(&userdb_authed, &w_id, &var_path, false)
        .await
        .map_err(|e| {
            (
                error::Error::NotFound(format!(
                    "Variable {var_path} not found for git ssh identity: {e:#}"
                )),
                loc.clone(),
            )
        })?;
    content.push_str("\n");

    if let Some(p) = &loc.parent() {
        tokio::fs::create_dir_all(p)
            .await
            .map_err(|e| (e.into(), loc.clone()))?;
    }
    tokio::fs::write(&loc, content)
        .await
        .map_err(|e| (e.into(), loc.clone()))?;

    #[cfg(unix)]
    {
        let perm = std::os::unix::fs::PermissionsExt::from_mode(0o600);
        tokio::fs::set_permissions(&loc, perm)
            .await
            .map_err(|e| (e.into(), loc.clone()))?;
    }

    return Ok(loc);
}

async fn delete_paths(paths: &Vec<std::path::PathBuf>) {
    for path in paths {
        let _ = tokio::fs::remove_file(&path).await;
    }
}

async fn get_git_ssh_cmd(
    authed: &ApiAuthed,
    user_db: &UserDB,
    db: &DB,
    w_id: &str,
    git_ssh_identity: Vec<String>,
) -> error::Result<(Option<String>, Vec<std::path::PathBuf>)> {
    if git_ssh_identity.len() > 5 {
        return Err(error::Error::BadRequest(
            "Too many ssh identities, try using at most 1".to_string(),
        ));
    }
    if git_ssh_identity.len() == 0 {
        return Ok((None, vec![]));
    }

    let mut ssh_id_files = vec![];
    let mut file_paths = vec![];
    for var_path in git_ssh_identity.iter() {
        match write_ssh_file(authed, user_db, db, w_id, &var_path).await {
            Ok(loc) => {
                ssh_id_files.push(format!(
                    " -i '{}'",
                    loc.to_string_lossy().replace('\'', r"'\''")
                ));
                file_paths.push(loc);
            }
            Err((e, loc)) => {
                file_paths.push(loc);
                delete_paths(&file_paths).await;
                return Err(e);
            }
        }
    }

    let git_ssh_cmd = format!("ssh -o StrictHostKeyChecking=no{}", ssh_id_files.join(""));
    Ok((Some(git_ssh_cmd), file_paths))
}

async fn get_repo_latest_commit_hash(
    git_resource: &GitRepositoryResource,
    git_ssh_command: Option<String>,
) -> Result<String> {
    // Validate URL and branch to prevent option injection attacks
    validate_git_url(&git_resource.url)?;

    let ref_spec = git_resource
        .branch
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("HEAD");

    // Validate ref_spec if it's not the default HEAD
    if ref_spec != "HEAD" {
        validate_git_ref(ref_spec)?;
    }

    let mut git_cmd = Command::new("git");
    git_cmd.args(["ls-remote", &git_resource.url, ref_spec]);
    if let Some(git_ssh_command) = git_ssh_command {
        git_cmd.env("GIT_SSH_COMMAND", git_ssh_command);
    }
    git_cmd.stderr(Stdio::piped());

    let output = git_cmd
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to execute git command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)
            .unwrap_or_else(|_| "Failed to decode stderr".to_string());
        return Err(Error::BadRequest(format!(
            "Error getting git repo commit hash: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| Error::internal_err(format!("Failed to decode git output: {}", e)))?;

    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        return Err(Error::BadRequest(format!(
            "No commits found for reference '{}' in repository '{}'",
            ref_spec, git_resource.url
        )));
    }

    let commit_hash = lines
        .first()
        .and_then(|line| line.split_whitespace().next())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            Error::BadRequest("Unexpected output format for git ls-remote".to_string())
        })?;

    Ok(commit_hash)
}

#[cfg(all(
    feature = "enterprise",
    any(feature = "nats", feature = "kafka", feature = "sqs_trigger")
))]
pub async fn interpolate(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    s: String,
) -> std::result::Result<String, anyhow::Error> {
    use serde_json::Value;
    use windmill_common::db::DbWithOptAuthed;
    let value = Value::String(s);
    match transform_json_value(
        &DbWithOptAuthed::from_authed(authed, db.clone(), None),
        w_id,
        value,
        &None,
        None,
        0,
    )
    .await?
    {
        Value::String(s) => Ok(s),
        v => Err(anyhow::anyhow!("Expected string, got {:?}", v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use windmill_common::audit::AuditAuthor;
    use windmill_common::db::DbWithOptAuthed;

    fn test_db_with_opt_authed(db: DB) -> DbWithOptAuthed<'static, ApiAuthed> {
        DbWithOptAuthed::DB {
            db,
            audit_author: AuditAuthor {
                username: "test".to_string(),
                email: "test@test.com".to_string(),
                username_override: None,
                token_prefix: None,
            },
        }
    }

    #[tokio::test]
    async fn test_transform_array_over_1000_passthrough() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let dba = test_db_with_opt_authed(pool);

        let arr: Vec<Value> = (0..1001).map(|i| json!(format!("$var:x/{i}"))).collect();
        let input = Value::Array(arr.clone());

        let result = transform_json_value(&dba, "test", input, &None, None, 0)
            .await
            .unwrap();

        assert_eq!(result, Value::Array(arr));
    }

    #[tokio::test]
    async fn test_transform_array_non_matching_strings_passthrough() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let dba = test_db_with_opt_authed(pool);

        let input = json!(["hello", "world", 42, true, null, {"key": "val"}]);

        let result = transform_json_value(&dba, "test", input.clone(), &None, None, 0)
            .await
            .unwrap();

        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_transform_array_resolved_inside_object() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let dba = test_db_with_opt_authed(pool);

        let input = json!({"urls": ["$var:u/test/nonexistent", "plain"]});

        let result = transform_json_value(&dba, "test", input, &None, None, 0).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transform_array_attempts_matching_items() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let dba = test_db_with_opt_authed(pool);

        let input = json!(["$var:u/test/nonexistent", "plain"]);

        let result = transform_json_value(&dba, "test", input, &None, None, 0).await;

        assert!(result.is_err());
    }
}
