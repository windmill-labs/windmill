/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use dashmap::DashMap;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::LazyLock;

use windmill_api_auth::{
    build_scope_path_predicate, check_scopes, maybe_refresh_folders, require_owner_of_path,
    require_super_admin_email, ApiAuthed, Tokened,
};
use windmill_common::db::DB;
use windmill_common::per_minute_counter::PerMinuteCounter;
use windmill_common::workspaces::{check_deploy_rules, RuleCheckResult};

use crate::secret_backend_ext::rename_vault_secret;
use crate::var_resource_cache::{auth_identity, cache_resource, get_cached_resource};
use windmill_common::utils::{
    check_proper_path, check_proper_type_name, escape_ilike_pattern, sanitize_db_error,
    BulkDeleteRequest,
};
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
    user_drafts::{
        delete_all_drafts_for_path, delete_own_draft_for_path, fetch_draft_only,
        fetch_draft_only_list_rows, maybe_overlay_draft, UserDraftItemKind, WithDraftOverlay,
        WithDraftQuery,
    },
    utils::{not_found_if_none, paginate, require_admin, Pagination, StripPath},
    variables,
    worker::{CLOUD_HOSTED, WINDMILL_DIR},
    PgDatabase,
};

use async_recursion::async_recursion;
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_resources))
        .route("/list_search", get(list_search_resources))
        .route("/list_names/{type}", get(list_names))
        .route("/get/{*path}", get(get_resource))
        .route("/exists/{*path}", get(exists_resource))
        .route("/get_value/{*path}", get(get_resource_value))
        .route(
            "/get_value_interpolated/{*path}",
            get(get_resource_value_interpolated),
        )
        .route("/update/{*path}", post(update_resource))
        .route("/update_value/{*path}", post(update_resource_value))
        .route(
            "/history/p/{*path}",
            get(get_resource_history).delete(clear_resource_history),
        )
        .route("/history/v/{id}", get(get_resource_version))
        .route("/history/restore/v/{id}", post(restore_resource_version))
        .route("/delete/{*path}", delete(delete_resource))
        .route("/delete_bulk", delete(delete_resources_bulk))
        .route("/create", post(create_resource))
        .route("/git_commit_hash/{*path}", get(get_git_commit_hash))
        .route("/type/list", get(list_resource_types))
        .route("/type/listnames", get(list_resource_types_names))
        .route("/type/get/{name}", get(get_resource_type))
        .route("/type/exists/{name}", get(exists_resource_type))
        .route("/type/update/{name}", post(update_resource_type))
        .route("/type/delete/{name}", delete(delete_resource_type))
        .route(
            "/file_resource_type_to_file_ext_map",
            get(file_resource_ext_to_resource_type),
        )
        .route("/type/create", post(create_resource_type))
}

pub fn public_service() -> Router {
    Router::new().route("/custom_component/{name}", get(custom_component))
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
    /// Doubly optional so an edit can distinguish the two things a plain
    /// `Option` conflates: an absent field leaves the extension alone, while an
    /// explicit `null` clears it. A hub pull relies on both — a type that stops
    /// being a file type has to stop being one locally too.
    #[serde(default, deserialize_with = "windmill_common::more_serde::double_option")]
    pub format_extension: Option<Option<String>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Labels inherited from the parent folder, computed at read time.
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited_labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_specific: Option<bool>,
    /// `Some(true)` only on synthesized draft-only rows; `None` on deployed rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub draft_only: Option<bool>,
    /// True when the authed user has a per-user draft at this path (drives the
    /// `*` suffix on the resources page).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub is_draft: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateResource {
    pub path: String,
    pub value: Option<Box<RawValue>>,
    pub description: Option<String>,
    pub resource_type: String,
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub ws_specific: Option<bool>,
}
#[derive(Deserialize)]
struct EditResource {
    path: Option<String>,
    description: Option<String>,
    value: Option<Box<RawValue>>,
    resource_type: Option<String>,
    labels: Option<Vec<String>>,
    ws_specific: Option<bool>,
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
    pub broad_filter: Option<String>,
    pub label: Option<String>,
    /// When true, append per-user draft-only rows; picker callers leave it off
    /// to stay deployed-only. See list synthesis in scripts.rs.
    pub include_draft_only: Option<bool>,
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
    let allowed = build_scope_path_predicate(&authed, "resources", "read");
    let rows = sqlx::query!(
        "SELECT value->>'name' as name, path from resource WHERE resource_type = $1 AND workspace_id = $2",
        rt,
        &w_id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter_map(|x| x.name.map(|name| NamePath { name, path: x.path }))
    .filter(|np| allowed(&np.path))
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize, FromRow)]
pub struct SearchResource {
    path: String,
    /// Pretty-printed JSON, capped at `SEARCH_RESOURCE_VALUE_MAX_CHARS`.
    value: String,
    truncated: bool,
}

/// This route hands the browser every readable resource's value at once and the client keeps
/// them all in memory, so without a cap a workspace of large JSON resources sends tens of MB
/// and freezes the tab. Content search only fuzzy-matches and previews a few lines of each.
/// The value is spelled out in `listSearchResource`'s openapi.yaml description; change both.
const SEARCH_RESOURCE_VALUE_MAX_CHARS: i32 = 4000;

async fn list_search_resources(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<SearchResource>> {
    let mut tx = user_db.begin(&authed).await?;
    let n = 1000;

    let allowed = build_scope_path_predicate(&authed, "resources", "read");
    let rows = sqlx::query_as!(
        SearchResource,
        // `OFFSET 0` fences the subquery so the planner cannot pull it up: without it
        // jsonb_pretty is inlined into both the left() and the length(), serializing
        // every value twice.
        r#"SELECT resource.path,
                  COALESCE(left(pretty.value, $3), '') as "value!",
                  COALESCE(length(pretty.value) > $3, false) as "truncated!"
           FROM resource, LATERAL (SELECT jsonb_pretty(resource.value) as value OFFSET 0) pretty
           WHERE workspace_id = $1 LIMIT $2"#,
        &w_id,
        n,
        SEARCH_RESOURCE_VALUE_MAX_CHARS
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter(|r| allowed(&r.path))
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_resources(
    authed: ApiAuthed,
    Query(lq): Query<ListResourceQuery>,
    Query(pagination): Query<Pagination>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
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
            "resource.labels",
            "folder_labels(resource.workspace_id, resource.path) as inherited_labels",
            "ws_specific.path IS NOT NULL as ws_specific",
        ])
        // Scalar EXISTS flags the authed user's per-user draft without fanning rows out.
        .field(
            &"EXISTS(SELECT 1 FROM draft WHERE draft.workspace_id = resource.workspace_id \
              AND draft.path = resource.path AND draft.typ = 'resource' \
              AND draft.email = ?) as is_draft"
                .bind(&authed.email),
        )
        .left()
        .join("variable")
        .on("variable.path = resource.path AND variable.workspace_id = resource.workspace_id")
        .left()
        .join("ws_specific")
        .on("ws_specific.path = resource.path AND ws_specific.workspace_id = resource.workspace_id AND ws_specific.item_kind = 'resource'")
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
        let pat = format!("%{}%", escape_ilike_pattern(description));
        sqlb.and_where("resource.description ILIKE ?".bind(&pat));
    }

    if let Some(value) = &lq.value {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(value) {
            sqlb.and_where("resource.value @> ?".bind(&v.to_string()));
        } else {
            sqlb.and_where("FALSE");
        }
    }

    if let Some(broad_filter) = &lq.broad_filter {
        let pat = format!("%{}%", escape_ilike_pattern(broad_filter));
        sqlb.and_where(
            "(resource.path ILIKE ? OR resource.description ILIKE ? OR resource_type ILIKE ? OR resource.value::text ILIKE ?)"
                .bind(&pat).bind(&pat).bind(&pat).bind(&pat)
        );
    }

    if let Some(label) = &lq.label {
        for l in label.split(',') {
            sqlb.and_where(
                "(resource.labels @> ARRAY[?] OR folder_labels(resource.workspace_id, resource.path) @> ARRAY[?])"
                    .bind(&l.trim())
                    .bind(&l.trim()),
            );
        }
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let allowed = build_scope_path_predicate(&authed, "resources", "read");
    let mut rows = sqlx::query_as::<_, ListableResource>(&sql)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .filter(|r| allowed(&r.path))
        .collect::<Vec<_>>();

    tx.commit().await?;

    // Append the authed user's draft-only resources; see scripts.rs.
    // `resource_type` / `resource_type_exclude` are deliberately NOT in the bail-out
    // list (the resources page always passes `resource_type_exclude`); they're applied
    // per-row below against the draft JSON's `resource_type` instead.
    if lq.include_draft_only.unwrap_or(false)
        && !authed.is_operator
        && offset == 0
        && lq.path_start.is_none()
        && lq.path.is_none()
        && lq.description.is_none()
        && lq.value.is_none()
        && lq.broad_filter.is_none()
        && lq.label.is_none()
    {
        let rt_filter: Option<Vec<&str>> = lq
            .resource_type
            .as_deref()
            .map(|s| s.split(',').map(str::trim).collect());
        let rt_exclude: Option<Vec<&str>> = lq
            .resource_type_exclude
            .as_deref()
            .map(|s| s.split(',').map(str::trim).collect());
        let draft_only_rows =
            fetch_draft_only_list_rows(&db, &w_id, &authed.email, UserDraftItemKind::Resource)
                .await?;

        for row in draft_only_rows {
            let v: serde_json::Value =
                serde_json::from_str(row.value.0.get()).unwrap_or(serde_json::Value::Null);
            // ResourceEditor's `ResourceState`: { path, description, args, labels?, wsSpecific, resource_type? }
            let path = v
                .get("path")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            if path.is_empty() || !allowed(&path) {
                continue;
            }
            let description = v
                .get("description")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            let value = v.get("args").cloned();
            let resource_type = v
                .get("resource_type")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            // Mirror the deployed query's resource_type narrowing for the
            // synthesized rows (see the gate comment above).
            if let Some(ref rts) = rt_filter {
                if !rts.contains(&resource_type.as_str()) {
                    continue;
                }
            }
            if let Some(ref excl) = rt_exclude {
                if excl.contains(&resource_type.as_str()) {
                    continue;
                }
            }
            let labels = v.get("labels").and_then(|x| {
                x.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
            });
            let ws_specific = v.get("wsSpecific").and_then(|x| x.as_bool());

            rows.push(ListableResource {
                workspace_id: w_id.clone(),
                path,
                value,
                description,
                resource_type,
                extra_perms: serde_json::Value::Object(serde_json::Map::new()),
                created_by: None,
                edited_at: Some(row.created_at),
                is_linked: None,
                is_refreshed: None,
                is_oauth: None,
                is_expired: None,
                refresh_error: None,
                account: None,
                labels,
                // No deployed row to inherit folder labels from.
                inherited_labels: None,
                ws_specific,
                draft_only: Some(true),
                // Synthesized rows are the authed user's draft.
                is_draft: Some(true),
            });
        }
    }

    Ok(Json(rows))
}

async fn get_resource(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(q): Query<WithDraftQuery>,
) -> JsonResult<WithDraftOverlay> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let resource_o = sqlx::query_as!(
        ListableResource,
        // `null::bool` columns align with the struct fields; deployed rows are never draft-only.
        "SELECT resource.workspace_id, resource.path, resource.value, resource.description,
        resource.resource_type, resource.extra_perms, resource.created_by, resource.edited_at,
        resource.labels,
        folder_labels(resource.workspace_id, resource.path) as \"inherited_labels?\",
        (now() > account.expires_at) as is_expired, account.refresh_token != '' as is_refreshed,
        account.refresh_error,
        variable.path IS NOT NULL as is_linked,
        variable.is_oauth as \"is_oauth?\",
        variable.account,
        ws_specific.path IS NOT NULL as ws_specific,
        null::bool as draft_only,
        null::bool as is_draft
        FROM resource
        LEFT JOIN variable ON variable.path = resource.path AND variable.workspace_id = $2
        LEFT JOIN account ON variable.account = account.id AND account.workspace_id = $2
        LEFT JOIN ws_specific ON ws_specific.path = resource.path AND ws_specific.workspace_id = $2 AND ws_specific.item_kind = 'resource'
        WHERE resource.path = $1 AND resource.workspace_id = $2",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if resource_o.is_none() && q.get_draft {
        // No deployed row + `get_draft`: synthesize the response from the draft
        // alone (`no_deployed = true`); see scripts.rs.
        if let Some(overlay) =
            fetch_draft_only(&db, &w_id, &authed.email, UserDraftItemKind::Resource, path).await?
        {
            return Ok(Json(overlay));
        }
    }
    if resource_o.is_none() {
        explain_resource_perm_error(&path, &w_id, &db, &authed).await?;
    }
    let resource = not_found_if_none(resource_o, "Resource", path)?;
    let overlay = maybe_overlay_draft(
        &db,
        &w_id,
        &authed.email,
        UserDraftItemKind::Resource,
        path,
        q.get_draft,
        resource,
    )
    .await?;
    Ok(Json(overlay))
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
        // A job's WM_TOKEN must never reach this superadmin-only path even if it
        // runs on behalf of a superadmin (GHSA-hfh4-cx4h-3fcr). Read the job
        // provenance from the *authenticated* identity, never the caller-supplied
        // `job_id` param (which comes from an untrusted query string).
        if db_with_opt_authed.authed().and_then(|a| a.job_id).is_some() {
            return Err(Error::NotAuthorized(
                "CUSTOM_INSTANCE_DB cannot be resolved from a job token ($WM_TOKEN)".to_string(),
            ));
        }
        require_super_admin_email(db_with_opt_authed.db(), &db_with_opt_authed.email()).await?;
        let mut pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
        pg_creds.dbname = dbname.to_string();
        let pg_creds = serde_json::to_value(&pg_creds)
            .map_err(|e| Error::internal_err(format!("Error serializing pg creds: {}", e)))?;
        return Ok(Some(pg_creds));
    }

    // Scope the cache to the caller's full authorization identity (not just email): the
    // cached value is already decrypted/interpolated under this caller's RLS context, so it
    // must never be served to a context that resolves to different permissions. Only
    // job-independent values are ever stored (see the write below), so a hit is always safe
    // to return regardless of the current `job_id`.
    let cache_identity = allow_cache.then(|| match db_with_opt_authed.authed() {
        Some(authed) => auth_identity(authed),
        None => format!("\0system:{}", db_with_opt_authed.email()),
    });

    if let Some(identity) = cache_identity.as_deref() {
        if let Some(cached_value) = get_cached_resource(&workspace, &path, identity) {
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
        // Track whether interpolation pulled in a `$WM_*` contextual variable. If it did, the
        // result is job-dependent (and may embed `$WM_TOKEN`) and must not be cached; if not,
        // it's job-independent and safe to cache and to serve to any job context.
        let used_job_context = std::sync::atomic::AtomicBool::new(false);
        let r = transform_json_value_tracked(
            &db_with_opt_authed,
            workspace,
            value,
            &job_id,
            token_for_context,
            0,
            &used_job_context,
        )
        .await?;
        if let Some(identity) = cache_identity.as_deref() {
            if !used_job_context.load(std::sync::atomic::Ordering::Relaxed) {
                cache_resource(&workspace, &path, identity, r.clone());
            }
        }
        Ok(Some(r))
    } else {
        Ok(None)
    }
}

// Maximum recursion depth for variable/resource interpolation. Each nested
// object/array level and each `$res:`/`$var:` indirection consumes one unit of
// depth. This bounds runtime cost and, crucially, prevents a stack overflow
// from mutually-recursive `$res:` references (e.g. resource A -> `$res:B` and
// resource B -> `$res:A`), which any workspace member with resource write
// access could otherwise use to crash the API process.
pub const MAX_RESOURCE_INTERPOLATION_DEPTH: u8 = 50;

pub async fn transform_json_value(
    db_with_opt_authed: &DbWithOptAuthed<'_, ApiAuthed>,
    workspace: &str,
    v: Value,
    job_id: &Option<Uuid>,
    token: Option<&str>,
    depth: u8,
) -> Result<Value> {
    // Discard the job-context flag; callers that need it use `transform_json_value_tracked`.
    let used_job_context = std::sync::atomic::AtomicBool::new(false);
    transform_json_value_tracked(
        db_with_opt_authed,
        workspace,
        v,
        job_id,
        token,
        depth,
        &used_job_context,
    )
    .await
}

/// Like [`transform_json_value`], but records into `used_job_context` whether the value
/// contains a `$WM_*` contextual variable (resolved from `job_id`/`token`). A value that did
/// not is job-independent and safe to cache; one that did must not be cached or shared across
/// jobs.
#[async_recursion]
pub async fn transform_json_value_tracked(
    db_with_opt_authed: &DbWithOptAuthed<'_, ApiAuthed>,
    workspace: &str,
    v: Value,
    job_id: &Option<Uuid>,
    token: Option<&str>,
    depth: u8,
    used_job_context: &std::sync::atomic::AtomicBool,
) -> Result<Value> {
    if depth >= MAX_RESOURCE_INTERPOLATION_DEPTH {
        return Err(Error::internal_err(format!(
            "Maximum resource/variable interpolation depth ({MAX_RESOURCE_INTERPOLATION_DEPTH}) exceeded; this usually indicates a circular `$res:` or `$var:` reference"
        )));
    }
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();

            let v =
                crate::variables::get_value_internal(&db_with_opt_authed, workspace, path, false)
                    .await?;
            Ok(Value::String(v))
        }
        Value::String(y) if y.starts_with("$jsonvar:") => {
            let path = y.strip_prefix("$jsonvar:").unwrap();

            let v =
                crate::variables::get_value_internal(&db_with_opt_authed, workspace, path, false)
                    .await?;
            serde_json::from_str::<Value>(&v).map_err(|e| {
                Error::internal_err(format!("Failed to parse $jsonvar value as JSON: {e}"))
            })
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
                transform_json_value_tracked(
                    db_with_opt_authed,
                    workspace,
                    v,
                    job_id,
                    token,
                    depth + 1,
                    used_job_context,
                )
                .await
            } else {
                Ok(Value::Null)
            }
        }
        // `$WM_*` is the reserved contextual-variable namespace (`$WM_TOKEN`, `$WM_JOB_ID`,
        // ...); its resolved value depends on the job, so a value containing one is
        // job-dependent and must never be cached — including on a no-job read, where the
        // placeholder is left unresolved (caching it would then serve a stale placeholder to a
        // later job read). Any other `$...` string (custom workspace envs, `$5.00`, `$HOME`, jq
        // paths) is NOT interpolated here — it resolves to itself regardless of context and so
        // stays cacheable (handled by the catch-all below). Note: custom workspace envs are
        // intentionally not resolved inside resource values (they remain available to scripts).
        Value::String(y) if y.starts_with("$WM_") => {
            used_job_context.store(true, std::sync::atomic::Ordering::Relaxed);
            let Some(job_id) = *job_id else {
                // No job context to resolve against; leave the placeholder unchanged.
                return Ok(Value::String(y));
            };
            let mut tx = db_with_opt_authed.begin().await?;
            let job = sqlx::query!(
                "SELECT
                    v2_job.permissioned_as_email,
                    v2_job.created_by,
                    v2_job.parent_job,
                    v2_job.permissioned_as,
                    v2_job.runnable_path,
                    CASE WHEN v2_job.trigger_kind = 'schedule'::job_trigger_kind THEN v2_job.trigger END AS schedule_path,
                    CASE WHEN v2_job.trigger_kind = 'ci_test'::job_trigger_kind THEN v2_job.trigger END AS tested_runnable,
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
                job.tested_runnable.clone(),
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
                arr[i] = transform_json_value_tracked(
                    db_with_opt_authed,
                    workspace,
                    val,
                    job_id,
                    token,
                    depth + 1,
                    used_job_context,
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
                let v = transform_json_value_tracked(
                    db_with_opt_authed,
                    workspace,
                    b,
                    job_id,
                    token,
                    depth + 1,
                    used_job_context,
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

/// Resource types the platform writes on its own behalf rather than users editing them:
/// `state` backs `setState` and `cache` backs cached job results, so both are rewritten on
/// every job run. Workspace export skips them for the same reason the `record_resource_version`
/// trigger does; that trigger repeats this list in SQL, so the two have to move together.
pub const INTERNAL_RESOURCE_TYPES: [&str; 2] = ["state", "cache"];

/// Versions retained per resource path. The monitor sweep enforces it eventually; the history
/// endpoint caps its own read at the same number so a burst of writes between two sweeps cannot
/// make the drawer fetch an unbounded list.
pub const MAX_RESOURCE_VERSIONS: i64 = 100;

/// Above any plausible hand-editing cadence, and the rate at which the retained history stops
/// being useful: at this many writes a minute, MAX_RESOURCE_VERSIONS covers only five minutes.
/// Counting is per process, so N servers raise the effective threshold to N times this; erring
/// low is the safe direction for something that only ever logs.
const RESOURCE_WRITE_ADVISORY_PER_MIN: u32 = 20;

/// Writes seen per (workspace, path) per minute. Purely advisory, and deliberately so: nothing
/// is throttled, the count is per process and resets on restart, so it undercounts across
/// servers. That is affordable for a log line and is what keeps this off the write path proper.
static RESOURCE_WRITE_RATES: LazyLock<PerMinuteCounter<(String, String)>> =
    LazyLock::new(PerMinuteCounter::new);

/// Notice a caller rewriting one resource in a loop and point them at a store meant for it.
/// Counts writes rather than versions: an unchanged value records nothing, but it still costs a
/// row rewrite, an audit entry and a webhook, and is still a caller who wants a different store.
fn note_resource_write(w_id: &str, path: &str, resource_type: &str) {
    // `state` and `cache` are rewritten once per job by design. Counting them would fire the
    // advisory hardest on exactly the traffic it is not about, which is how a warning becomes
    // noise people filter out.
    if INTERNAL_RESOURCE_TYPES.contains(&resource_type) {
        return;
    }
    let writes = RESOURCE_WRITE_RATES.increment((w_id.to_string(), path.to_string()));
    // Once per minute per path: `==` rather than `>=` so a sustained loop logs at the crossing
    // and then stays quiet until the bucket rolls over.
    if writes == RESOURCE_WRITE_ADVISORY_PER_MIN {
        tracing::warn!(
            workspace_id = %w_id,
            path = %path,
            "resource written {} times in a minute; each write rewrites the whole value, and a \
             changed one also records a version. High-frequency writes belong in a state resource, \
             object storage, or a datatable.",
            RESOURCE_WRITE_ADVISORY_PER_MIN
        );
    }
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
    check_proper_path(&resource.path)?;
    check_proper_type_name(&resource.resource_type)?;
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
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
                    "You have reached the maximum number of resources (10000) on cloud. Check your usage in Workspace Settings > General > Cloud Quotas. Contact support@windmill.dev to increase the limit"
                        .to_string(),
                ));
        }
    }
    let authed = maybe_refresh_folders(&resource.path, &w_id, authed, &db).await;

    authorize_azure_devops_reference(
        &authed,
        &db,
        &user_db,
        &w_id,
        resource
            .value
            .as_deref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v.get()).ok())
            .as_ref(),
    )
    .await?;

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
    if update_if_exists {
        sqlx::query!(
            "INSERT INTO resource
                (workspace_id, path, value, description, resource_type, created_by, edited_at, labels)
                VALUES ($1, $2, $3, $4, $5, $6, now(), $7) ON CONFLICT (workspace_id, path)
                DO UPDATE SET value = EXCLUDED.value, description = EXCLUDED.description, resource_type = EXCLUDED.resource_type, edited_at = now(), labels = EXCLUDED.labels",
            w_id,
            resource.path,
            raw_json as sqlx::types::Json<&RawValue>,
            resource.description,
            resource.resource_type,
            authed.username,
            resource.labels.as_deref() as Option<&[String]>
        )
        .execute(&mut *tx)
        .await
        .map_err(sanitize_db_error)?;
    } else {
        // Create-only (the default): DO NOTHING + a row-count guard, so a path that appears between
        // check_path_conflict above and this insert is rejected rather than overwritten. A plain
        // DO UPDATE here would clobber a concurrently-created resource, breaking create-only callers
        // (e.g. Compare & Deploy "Create in <other>").
        let inserted = sqlx::query!(
            "INSERT INTO resource
                (workspace_id, path, value, description, resource_type, created_by, edited_at, labels)
                VALUES ($1, $2, $3, $4, $5, $6, now(), $7) ON CONFLICT (workspace_id, path) DO NOTHING",
            w_id,
            resource.path,
            raw_json as sqlx::types::Json<&RawValue>,
            resource.description,
            resource.resource_type,
            authed.username,
            resource.labels.as_deref() as Option<&[String]>
        )
        .execute(&mut *tx)
        .await
        .map_err(sanitize_db_error)?;
        if inserted.rows_affected() == 0 {
            return Err(Error::BadRequest(format!(
                "Resource {} already exists",
                resource.path
            )));
        }
    }

    // Mirror update_resource: Some(true) inserts, Some(false) clears (only
    // meaningful on the upsert path, since a pure create has no existing row),
    // None leaves the existing flag alone.
    match resource.ws_specific {
        Some(true) => {
            sqlx::query!(
                "INSERT INTO ws_specific (workspace_id, item_kind, path) VALUES ($1, 'resource', $2) ON CONFLICT DO NOTHING",
                w_id,
                resource.path,
            )
            .execute(&mut *tx)
            .await?;

            mark_linked_variables_ws_specific(&mut tx, &authed, &w_id, &resource.path).await?;
        }
        Some(false) if update_if_exists => {
            sqlx::query!(
                "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2",
                w_id,
                resource.path,
            )
            .execute(&mut *tx)
            .await?;
        }
        _ => {}
    }

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
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
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

    // Capture resource data for trashbin before deleting
    let trash_resource: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM resource t WHERE path = $1 AND workspace_id = $2",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    // Fetch the resource value before deleting, so we can find linked $var: references
    let resource_value: Option<Option<serde_json::Value>> =
        sqlx::query_scalar("SELECT value FROM resource WHERE path = $1 AND workspace_id = $2")
            .bind(path)
            .bind(&w_id)
            .fetch_optional(&mut *tx)
            .await?;

    // Collect all $var: paths referenced in the resource value
    let mut linked_var_paths: Vec<String> = Vec::new();
    if let Some(Some(ref value)) = resource_value {
        collect_var_refs(value, &mut linked_var_paths);
    }

    // A scoped token must not delete linked variables it lacks variables:write for.
    check_linked_var_delete_scopes(&authed, &linked_var_paths)?;

    // Capture linked variables for trashbin before deleting them
    let trash_linked_vars: Vec<serde_json::Value> = if linked_var_paths.is_empty() {
        Vec::new()
    } else {
        let placeholders: Vec<String> = linked_var_paths
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 2))
            .collect();
        let query = format!(
            "SELECT to_jsonb(t) FROM variable t WHERE workspace_id = $1 AND path IN ({})",
            placeholders.join(", ")
        );
        let mut q = sqlx::query_scalar::<_, serde_json::Value>(&query).bind(&w_id);
        for var_path in &linked_var_paths {
            q = q.bind(var_path);
        }
        q.fetch_all(&mut *tx).await?
    };

    sqlx::query!(
        "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2",
        w_id,
        path
    )
    .execute(&mut *tx)
    .await?;

    let deleted_path = sqlx::query_scalar!(
        "DELETE FROM resource WHERE path = $1 AND workspace_id = $2 RETURNING path",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    not_found_if_none(deleted_path, "Resource", &path)?;

    // Delete linked variables that are actually referenced in the resource value
    let deleted_linked_variables: Vec<String> = if linked_var_paths.is_empty() {
        Vec::new()
    } else {
        // Clean up any ws_specific rows for these variables first
        // (mark_linked_variables_ws_specific may have auto-inserted them) so
        // they don't survive the variable deletion as orphans — a variable
        // later recreated at the same path would otherwise inherit the stale
        // ws_specific flag.
        sqlx::query!(
            "DELETE FROM ws_specific
             WHERE workspace_id = $1 AND item_kind = 'variable' AND path = ANY($2)",
            w_id,
            &linked_var_paths
        )
        .execute(&mut *tx)
        .await?;

        let placeholders: Vec<String> = linked_var_paths
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 2))
            .collect();
        let query = format!(
            "DELETE FROM variable WHERE workspace_id = $1 AND path IN ({}) RETURNING path",
            placeholders.join(", ")
        );
        let mut q = sqlx::query_scalar::<_, String>(&query).bind(&w_id);
        for var_path in &linked_var_paths {
            q = q.bind(var_path);
        }
        q.fetch_all(&mut *tx).await?
    };

    if let Some(res_data) = trash_resource {
        let mut trash_data = serde_json::json!({"row": res_data});
        if !trash_linked_vars.is_empty() {
            trash_data["linked_variables"] = serde_json::Value::Array(trash_linked_vars);
        }
        windmill_common::trashbin::move_to_trash(
            &mut *tx,
            &w_id,
            "resource",
            path,
            trash_data,
            &authed.username,
        )
        .await?;
    }

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

    // Resource gone for everyone: wipe ALL users' drafts at this path (and any linked
    // variables cascaded into) so teammates' drafts don't orphan. Idempotent on no-draft.
    delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Resource, path).await?;
    for var_path in &deleted_linked_variables {
        delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Variable, var_path).await?;
    }

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
        WebhookMessage::DeleteResource { workspace: w_id.clone(), path: path.to_owned() },
    );

    for var_path in &deleted_linked_variables {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            DeployedObject::Variable {
                path: var_path.clone(),
                parent_path: Some(var_path.clone()),
            },
            Some(format!(
                "Variable '{}' deleted (linked resource deleted)",
                var_path
            )),
            true,
            None,
        )
        .await?;

        webhook.send_message(
            w_id.clone(),
            WebhookMessage::DeleteVariable { workspace: w_id.clone(), path: var_path.clone() },
        );
    }

    Ok(format!("resource {} deleted", path))
}

/// Recursively collect all `$var:path` references from a JSON value.
fn collect_var_refs(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(var_path) = s.strip_prefix("$var:") {
                out.push(var_path.to_string());
            }
        }
        serde_json::Value::Object(m) => {
            for v in m.values() {
                collect_var_refs(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_var_refs(v, out);
            }
        }
        _ => {}
    }
}

/// Deleting a resource cascades into the `$var:` variables its value references. A
/// scoped token must not use that cascade to delete variables it could not delete
/// directly via `delete_variable` (which gates on `variables:write:<path>`), so require
/// `variables:write` for EVERY linked variable and fail the whole delete otherwise.
///
/// No co-located-path exemption: a resource and a variable may share a path, and a
/// resource-write token can create a resource over an existing standalone variable and
/// self-reference it, so "same path as the deleted resource" is attacker-forgeable and
/// cannot stand in for variable scope. No-op for unscoped tokens (`check_scopes` passes),
/// so a full token's cascade cleanup is unchanged.
fn check_linked_var_delete_scopes(authed: &ApiAuthed, linked_var_paths: &[String]) -> Result<()> {
    for var_path in linked_var_paths {
        check_scopes(authed, || format!("variables:write:{}", var_path))?;
    }
    Ok(())
}

/// Marks every variable referenced by the resource at `resource_path` as workspace-specific.
///
/// AUTH CONTRACT: this mutates `ws_specific` and does NOT check authorization itself. The caller
/// MUST verify that `authed` has write access to the resource at `resource_path` in `w_id` (e.g. via
/// `require_owner_of_path`) before calling it.
pub async fn mark_linked_variables_ws_specific(
    tx: &mut Transaction<'_, Postgres>,
    authed: &ApiAuthed,
    w_id: &str,
    resource_path: &str,
) -> Result<()> {
    let resource_value: Option<Option<serde_json::Value>> =
        sqlx::query_scalar("SELECT value FROM resource WHERE path = $1 AND workspace_id = $2")
            .bind(resource_path)
            .bind(w_id)
            .fetch_optional(&mut **tx)
            .await?;

    let mut linked_var_paths: Vec<String> = Vec::new();
    if let Some(Some(ref value)) = resource_value {
        collect_var_refs(value, &mut linked_var_paths);
    }

    linked_var_paths.sort();
    linked_var_paths.dedup();

    if linked_var_paths.is_empty() {
        return Ok(());
    }

    // RETURNING gives us only the rows actually inserted (RFC: ON CONFLICT
    // DO NOTHING + RETURNING returns the affected rows, i.e. the new ones).
    let newly_marked: Vec<String> = sqlx::query_scalar(
        "INSERT INTO ws_specific (workspace_id, item_kind, path)
         SELECT workspace_id, 'variable', path
         FROM variable
         WHERE workspace_id = $1 AND path = ANY($2::text[])
         ON CONFLICT DO NOTHING
         RETURNING path",
    )
    .bind(w_id)
    .bind(&linked_var_paths)
    .fetch_all(&mut **tx)
    .await?;

    // Audit each variable that was actually flipped to ws_specific so the
    // change is traceable to the resource save that caused it.
    for var_path in &newly_marked {
        let mut params = HashMap::new();
        params.insert("via_resource", resource_path);
        audit_log(
            &mut **tx,
            authed,
            "variables.set_ws_specific",
            ActionKind::Update,
            w_id,
            Some(var_path),
            Some(params),
        )
        .await?;
    }

    Ok(())
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

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
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

    // Capture resources for trashbin per path before bulk delete, and
    // collect $var: references so we can cascade-delete the linked variables
    // (matching single-resource delete semantics).
    let mut linked_var_paths: Vec<String> = Vec::new();
    for path in &request.paths {
        let trash_resource: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT to_jsonb(t) FROM resource t WHERE path = $1 AND workspace_id = $2",
        )
        .bind(path)
        .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(res_data) = trash_resource {
            // Per-resource linked vars so each resource's trash entry carries
            // exactly the variables that vanished with it (matching the
            // single-delete shape: trash_data["linked_variables"]).
            let mut this_linked: Vec<String> = Vec::new();
            if let Some(value) = res_data.get("value") {
                collect_var_refs(value, &mut this_linked);
            }
            this_linked.sort();
            this_linked.dedup();

            let trash_linked_vars: Vec<serde_json::Value> = if this_linked.is_empty() {
                Vec::new()
            } else {
                let placeholders: Vec<String> = this_linked
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", i + 2))
                    .collect();
                let query = format!(
                    "SELECT to_jsonb(t) FROM variable t WHERE workspace_id = $1 AND path IN ({})",
                    placeholders.join(", ")
                );
                let mut q = sqlx::query_scalar::<_, serde_json::Value>(&query).bind(&w_id);
                for var_path in &this_linked {
                    q = q.bind(var_path);
                }
                q.fetch_all(&mut *tx).await?
            };

            let mut trash_data = serde_json::json!({"row": res_data});
            if !trash_linked_vars.is_empty() {
                trash_data["linked_variables"] = serde_json::Value::Array(trash_linked_vars);
            }
            windmill_common::trashbin::move_to_trash(
                &mut *tx,
                &w_id,
                "resource",
                path,
                trash_data,
                &authed.username,
            )
            .await?;

            linked_var_paths.extend(this_linked);
        }
    }
    linked_var_paths.sort();
    linked_var_paths.dedup();

    // A scoped token must not delete linked variables it lacks variables:write for.
    check_linked_var_delete_scopes(&authed, &linked_var_paths)?;

    sqlx::query!(
        "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = ANY($2)",
        w_id,
        &request.paths
    )
    .execute(&mut *tx)
    .await?;

    let deleted_paths = sqlx::query_scalar!(
        "DELETE FROM resource WHERE path = ANY($1) AND workspace_id = $2 RETURNING path",
        &request.paths,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    // Cascade-clean linked variables: delete any ws_specific 'variable' rows
    // (typically auto-inserted by mark_linked_variables_ws_specific when the
    // resource was ws_specific) BEFORE deleting the variable rows themselves
    // — otherwise those ws_specific rows survive as orphans and a later
    // variable created at the same path would inherit a stale flag.
    if !linked_var_paths.is_empty() {
        sqlx::query!(
            "DELETE FROM ws_specific
             WHERE workspace_id = $1 AND item_kind = 'variable' AND path = ANY($2)",
            w_id,
            &linked_var_paths
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM variable WHERE workspace_id = $1 AND path = ANY($2)",
            w_id,
            &linked_var_paths
        )
        .execute(&mut *tx)
        .await?;
    }

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

    // Wipe ALL users' drafts at these paths (and linked variables); see delete_resource.
    for path in &deleted_paths {
        delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Resource, path).await?;
    }
    for var_path in &linked_var_paths {
        delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Variable, var_path).await?;
    }

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
    // A rename moves the resource (and its linked variable) to ns.path, so the
    // destination must also be within the token's write scope, not just the
    // source path.
    if let Some(npath) = ns.path.as_deref() {
        check_scopes(&authed, || format!("resources:write:{}", npath))?;
        check_proper_path(npath)?;
    }
    if let Some(nrt) = ns.resource_type.as_deref() {
        check_proper_type_name(nrt)?;
    }
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
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
    if let Some(nvalue) = &ns.value {
        sqlb.set_str("value", nvalue.to_string());
    }
    if let Some(nrt) = &ns.resource_type {
        sqlb.set_str("resource_type", nrt);
    }
    if let Some(ndesc) = ns.description {
        sqlb.set_str("description", ndesc);
    }
    sqlb.set_str("edited_at", "now()");

    sqlb.returning("path");
    let authed = maybe_refresh_folders(path, &w_id, authed, &db).await;

    authorize_azure_devops_reference(
        &authed,
        &db,
        &user_db,
        &w_id,
        ns.value
            .as_deref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v.get()).ok())
            .as_ref(),
    )
    .await?;

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

            // Update ci_test_reference when a tested resource is renamed
            sqlx::query!(
                "UPDATE ci_test_reference SET tested_item_path = $1 WHERE tested_item_path = $2 AND workspace_id = $3 AND tested_item_kind = 'resource'",
                npath,
                path,
                w_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "UPDATE ws_specific SET path = $1 WHERE workspace_id = $2 AND item_kind = 'resource' AND path = $3",
                npath,
                w_id,
                path
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "UPDATE ws_specific SET path = $1 WHERE workspace_id = $2 AND item_kind = 'variable' AND path = $3",
                npath,
                w_id,
                path
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let npath_o: Option<String> = sqlx::query_scalar(&sql)
        .fetch_optional(&mut *tx)
        .await
        .map_err(sanitize_db_error)?;

    let npath = not_found_if_none(npath_o, "Resource", path)?;

    if let Some(nlabels) = &ns.labels {
        sqlx::query!(
            "UPDATE resource SET labels = $1 WHERE path = $2 AND workspace_id = $3",
            nlabels as &[String],
            &npath,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(ws_specific) = ns.ws_specific {
        if ws_specific {
            sqlx::query!(
                "INSERT INTO ws_specific (workspace_id, item_kind, path) VALUES ($1, 'resource', $2) ON CONFLICT DO NOTHING",
                w_id,
                &npath,
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2",
                w_id,
                &npath,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // Only re-mark linked variables when something that could change them
    // actually changed: a new value (different $var: refs) or ws_specific
    // freshly enabled. Skipping when neither changed avoids re-running an
    // INSERT (and audit logs) on every save.
    let needs_remark = ns.value.is_some() || ns.ws_specific == Some(true);
    if needs_remark {
        let effective_ws_specific = if let Some(ws_specific) = ns.ws_specific {
            ws_specific
        } else {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2)",
            )
            .bind(&w_id)
            .bind(&npath)
            .fetch_one(&mut *tx)
            .await?
        };

        if effective_ws_specific {
            mark_linked_variables_ws_specific(&mut tx, &authed, &w_id, &npath).await?;
        }
    }

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

    // On rename the draft at the OLD path orphans (no SQL FK); clear the deployer's
    // own (+ legacy NULL) there, teammates keep theirs (StaleDraftModal). The linked
    // variable renames alongside the resource, so its old-path draft orphans too.
    if let Some(old_path) = old_path_if_renamed {
        delete_own_draft_for_path(
            &db,
            &w_id,
            UserDraftItemKind::Resource,
            old_path,
            &authed.email,
        )
        .await?;
        delete_own_draft_for_path(
            &db,
            &w_id,
            UserDraftItemKind::Variable,
            old_path,
            &authed.email,
        )
        .await?;
    }

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
            workspace: w_id.clone(),
            old_path: path.to_owned(),
            new_path: npath.clone(),
        },
    );

    // Trigger CI tests for items that reference this resource
    {
        let db2 = db.clone();
        let npath2 = npath.clone();
        let email2 = authed.email.clone();
        let username2 = authed.username.clone();
        tokio::spawn(async move {
            if let Err(e) = windmill_dep_map::ci_tests::trigger_ci_tests_for_item(
                &db2, &w_id, &npath2, "resource", &email2, &username2,
            )
            .await
            {
                tracing::error!(%e, "error triggering CI tests after resource update");
            }
        });
    }

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
    set_resource_value(
        &authed,
        &db,
        &user_db,
        &webhook,
        &w_id,
        path,
        nv.value,
        "resources.update",
    )
    .await?;
    Ok(format!("value of resource {} updated", path))
}

/// Write a resource's value and run everything that has to follow it: a version row, the audit
/// entry, deployment metadata, the webhook, and dependent CI tests. Shared with version restore
/// so a restored value is indistinguishable downstream from any other edit.
async fn set_resource_value(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    webhook: &WebhookShared,
    w_id: &str,
    path: &str,
    value: Option<serde_json::Value>,
    audit_action: &str,
) -> Result<()> {
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        w_id,
        AuditAuthorable::username(authed),
        &authed.groups,
        authed.is_admin,
        db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    authorize_azure_devops_reference(authed, db, user_db, w_id, value.as_ref()).await?;

    let mut tx = user_db.clone().begin(authed).await?;

    // `RETURNING resource_type` rather than a second lookup: the advisory below has to know the
    // type to leave `state` and `cache` alone, and this statement already runs.
    let updated = sqlx::query_scalar!(
        "UPDATE resource SET value = $1, edited_at = now() WHERE path = $2 AND workspace_id = $3
         RETURNING resource_type",
        value,
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(resource_type) = updated else {
        return Err(Error::NotFound(format!("Resource {} not found", path)));
    };
    audit_log(
        &mut *tx,
        authed,
        audit_action,
        ActionKind::Update,
        w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    note_resource_write(w_id, path, &resource_type);

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        db,
        w_id,
        DeployedObject::Resource { path: path.to_string(), parent_path: Some(path.to_string()) },
        None,
        true,
        None,
    )
    .await?;

    webhook.send_message(
        w_id.to_string(),
        WebhookMessage::UpdateResource {
            workspace: w_id.to_string(),
            old_path: path.to_owned(),
            new_path: path.to_owned(),
        },
    );

    // Trigger CI tests for items that reference this resource
    {
        let db2 = db.clone();
        let w_id2 = w_id.to_string();
        let path2 = path.to_string();
        let email2 = authed.email.clone();
        let username2 = authed.username.clone();
        tokio::spawn(async move {
            if let Err(e) = windmill_dep_map::ci_tests::trigger_ci_tests_for_item(
                &db2, &w_id2, &path2, "resource", &email2, &username2,
            )
            .await
            {
                tracing::error!(%e, "error triggering CI tests after resource value update");
            }
        });
    }

    Ok(())
}

#[derive(Serialize)]
struct ResourceVersion {
    /// Addresses a version; `version` is the per-resource number it is presented by.
    id: i64,
    version: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    created_by: Option<String>,
}

#[derive(Serialize)]
struct ResourceVersionWithValue {
    id: i64,
    version: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    created_by: Option<String>,
    value: Option<serde_json::Value>,
    /// `$var:`/`$res:` paths in this version that no longer resolve. Restoring is still allowed —
    /// the reference may be about to be recreated, and refusing would make a version unrestorable
    /// for a reason outside the resource — but the caller should say so before confirming.
    missing_references: Vec<String>,
}

/// Deliberately does not carry the resource's live value. Every write mints a version, so the
/// newest one already holds it, and `resource_version` rows are immutable — reading the live
/// value from `resource` instead would mean comparing a mutable row against this list, which at
/// READ COMMITTED can disagree with it. The drawer reads only versions, and by id, so there is
/// nothing for a concurrent write to make incoherent.
#[derive(Serialize)]
pub struct ResourceHistory {
    versions: Vec<ResourceVersion>,
    /// Whether this resource's history can ever fill, so the drawer can say an empty one is
    /// permanent rather than promising versions from the next edit. Decided here rather than by
    /// shipping the type out, so INTERNAL_RESOURCE_TYPES is not restated in TypeScript too.
    /// Compared against nothing, so unlike the value it carries no coherence risk.
    versioned: bool,
}

async fn get_resource_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<ResourceHistory> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let versions = sqlx::query_as!(
        ResourceVersion,
        "SELECT id, version, created_at, created_by FROM resource_version
         WHERE workspace_id = $1 AND path = $2 ORDER BY id DESC LIMIT $3",
        w_id,
        path,
        MAX_RESOURCE_VERSIONS
    )
    .fetch_all(&mut *tx)
    .await?;
    let resource_type = sqlx::query_scalar!(
        "SELECT resource_type FROM resource WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(ResourceHistory {
        versions,
        versioned: resource_type
            .map(|t| !INTERNAL_RESOURCE_TYPES.contains(&t.as_str()))
            .unwrap_or(true),
    }))
}

/// Collect the references in `value` that no longer point at anything. Covers the three
/// path-addressed forms — `$var:` and `$jsonvar:` resolve against `variable`, `$res:` against
/// `resource`. `$encrypted:` carries its payload inline so there is nothing to look up, and
/// neither resolution nor this check follows references transitively.
///
/// Runs on the caller's RLS-scoped transaction, so a referenced item the caller cannot read is
/// reported as missing. That errs towards warning rather than staying silent, and the reference
/// is unusable to them either way.
async fn missing_references(
    tx: &mut sqlx::PgConnection,
    w_id: &str,
    value: Option<&serde_json::Value>,
) -> Result<Vec<String>> {
    const VAR_PREFIXES: [&str; 2] = ["$var:", "$jsonvar:"];
    const RES_PREFIX: &str = "$res:";

    let mut refs: Vec<String> = vec![];
    fn collect(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            // Stated once: adding a form to the consts above must not need a second edit here,
            // or the new form would be collected but never checked (or the reverse).
            serde_json::Value::String(s)
                if VAR_PREFIXES.iter().any(|p| s.starts_with(p)) || s.starts_with(RES_PREFIX) =>
            {
                out.push(s.clone())
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| collect(x, out)),
            _ => {}
        }
    }
    if let Some(v) = value {
        collect(v, &mut refs);
    }
    refs.sort();
    refs.dedup();

    let mut var_paths: Vec<String> = vec![];
    let mut res_paths: Vec<String> = vec![];
    for r in &refs {
        match VAR_PREFIXES.iter().find_map(|p| r.strip_prefix(p)) {
            Some(p) => var_paths.push(p.to_string()),
            None => {
                if let Some(p) = r.strip_prefix(RES_PREFIX) {
                    res_paths.push(p.to_string())
                }
            }
        }
    }

    // Sets, not Vecs: a value carries as many references as its author put in it, and a linear
    // scan per reference would make the cost of one version fetch quadratic in caller-controlled
    // input.
    let existing_vars: std::collections::HashSet<String> = sqlx::query_scalar!(
        "SELECT path FROM variable WHERE workspace_id = $1 AND path = ANY($2)",
        w_id,
        &var_paths[..]
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();
    let existing_res: std::collections::HashSet<String> = sqlx::query_scalar!(
        "SELECT path FROM resource WHERE workspace_id = $1 AND path = ANY($2)",
        w_id,
        &res_paths[..]
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    Ok(refs
        .into_iter()
        .filter(
            |r| match VAR_PREFIXES.iter().find_map(|p| r.strip_prefix(p)) {
                Some(p) => !existing_vars.contains(p),
                None => match r.strip_prefix(RES_PREFIX) {
                    Some(p) => !existing_res.contains(p),
                    None => false,
                },
            },
        )
        .collect())
}

async fn get_resource_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> JsonResult<ResourceVersionWithValue> {
    let mut tx = user_db.begin(&authed).await?;

    let row = sqlx::query!(
        "SELECT id, version, path, created_at, created_by, value FROM resource_version
         WHERE workspace_id = $1 AND id = $2",
        w_id,
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let row = not_found_if_none(row, "ResourceVersion", id.to_string())?;
    check_scopes(&authed, || format!("resources:read:{}", row.path))?;

    let missing = missing_references(&mut tx, &w_id, row.value.as_ref()).await?;
    tx.commit().await?;

    Ok(Json(ResourceVersionWithValue {
        id: row.id,
        version: row.version,
        created_at: row.created_at,
        created_by: row.created_by,
        value: row.value,
        missing_references: missing,
    }))
}

/// Drop every version of a resource except the one matching its current value.
///
/// The remediation for a credential that was stored inline in a resource value (which `wmill`
/// pushes and `setResource` writes can do, unlike the UI form) and has since been rotated:
/// without this, rotating leaves the old secret readable in the history by anyone who can read
/// the resource. All-but-current rather than per-version because such a secret sits in every
/// version from its introduction to the rotation, so deleting them one at a time would silently
/// leave copies behind.
async fn clear_resource_history(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("resources:write:{}", path))?;
    require_owner_of_path(&authed, path)?;

    // Confirm the caller can see the resource under their own RLS before deleting anything with
    // the unrestricted pool — resource_version grants users SELECT only, so the delete cannot run
    // through user_db.
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource WHERE workspace_id = $1 AND path = $2)",
        w_id,
        path
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if !exists.unwrap_or(false) {
        return Err(Error::NotFound(format!("Resource {} not found", path)));
    }

    let mut tx = db.begin().await?;
    let deleted = sqlx::query!(
        "DELETE FROM resource_version rv
         WHERE rv.workspace_id = $1 AND rv.path = $2
           AND rv.id != (
               SELECT max(id) FROM resource_version l
               WHERE l.workspace_id = rv.workspace_id AND l.path = rv.path
           )",
        w_id,
        path
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "resources.clear_history",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!(
        "cleared {} past versions of resource {}",
        deleted.rows_affected(),
        path
    ))
}

async fn restore_resource_version(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, id)): Path<(String, i64)>,
) -> Result<String> {
    let mut tx = user_db.clone().begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT path, value, version FROM resource_version WHERE workspace_id = $1 AND id = $2",
        w_id,
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let row = not_found_if_none(row, "ResourceVersion", id.to_string())?;
    tx.commit().await?;

    check_scopes(&authed, || format!("resources:write:{}", row.path))?;

    // Writes the old value forward as a new version rather than rewinding, so the history stays
    // append-only, the restore is itself attributable, and the restore can be undone in turn.
    set_resource_value(
        &authed,
        &db,
        &user_db,
        &webhook,
        &w_id,
        &row.path,
        row.value,
        "resources.restore_version",
    )
    .await?;

    Ok(format!(
        "resource {} restored to version {}",
        row.path, row.version
    ))
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
        "SELECT workspace_id, name, schema, description, created_by, edited_at, format_extension, is_fileset from resource_type WHERE (workspace_id = $1 OR workspace_id = 'admins') ORDER \
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
        "SELECT workspace_id, name, schema, description, created_by, edited_at, format_extension, is_fileset from resource_type WHERE name = $1 AND (workspace_id = $2 OR workspace_id = 'admins')",
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
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    check_proper_type_name(&resource_type.name)?;

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
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
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
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
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
    if let Some(format_extension) = ns.format_extension.clone() {
        match format_extension {
            Some(ext) => sqlb.set_str("format_extension", ext),
            None => sqlb.set("format_extension", "NULL"),
        };
    }
    sqlb.set_str("edited_at", "now()");
    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;

    // Creation refuses the pair outright, so an edit must too — otherwise the same
    // impossible type (a set of files that is also one file) is reachable by setting
    // either half on an existing row. Whichever half the request omits is read from
    // the row being edited, inside this transaction and with the row locked: read
    // outside it, two concurrent edits each supplying one half would both pass.
    let current = sqlx::query!(
        "SELECT is_fileset, format_extension FROM resource_type
         WHERE name = $1 AND workspace_id = $2 FOR UPDATE",
        &name,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let effective_is_fileset = ns
        .is_fileset
        .unwrap_or_else(|| current.as_ref().map(|c| c.is_fileset).unwrap_or(false));
    let effective_format_extension = match &ns.format_extension {
        Some(value) => value.clone(),
        None => current.and_then(|c| c.format_extension),
    };
    if effective_is_fileset && effective_format_extension.is_some() {
        return Err(Error::BadRequest(
            "A fileset resource type cannot have a format_extension".to_string(),
        ));
    }

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
    feature = "amqp_trigger",
    all(
        feature = "enterprise",
        any(
            feature = "sqs_trigger",
            feature = "gcp_trigger",
            feature = "azure_trigger",
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

/// Checks whether an IP address belongs to a private, loopback, link-local, or
/// otherwise reserved range that should not be reachable from git operations.
fn is_private_or_reserved_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                // RFC 1122 "this network": the whole /8, not just the
                // unspecified address `is_unspecified()` matches — stacks that
                // map 0.x.y.z onto the local host make `0.0.0.1` a bypass.
                || v4.octets()[0] == 0 // 0.0.0.0/8
                || v4.is_broadcast()
                // 100.64.0.0/10 (Carrier-grade NAT / CGNAT)
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                // fc00::/7 (unique local address) — std has no stable is_unique_local()
                || (seg[0] & 0xfe00) == 0xfc00
                // fe80::/10 (link-local) — std has no stable is_unicast_link_local()
                || (seg[0] & 0xffc0) == 0xfe80
                // IPv4-mapped IPv6 (::ffff:x.x.x.x) — check the inner v4
                || v6.to_ipv4_mapped().map_or(false, |v4| {
                    is_private_or_reserved_ip(&IpAddr::V4(v4))
                })
        }
    }
}

/// Extracts the hostname from a git URL.
///
/// Handles standard URLs (`https://host/path`, `ssh://user@host/path`) and
/// SCP-style (`user@host:path`).
fn extract_host_from_git_url(url: &str) -> Option<String> {
    if let Some(after_scheme) = url.split("://").nth(1) {
        // The authority ends at the first '/', '?', or '#'; the credentials '@'
        // must be searched only within it, else a '@' in the path/query/fragment
        // mis-scopes the host (SSRF bypass, GHSA-p5cj-8cfh-mjv6).
        let authority_end = after_scheme
            .find(|c| c == '/' || c == '?' || c == '#')
            .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..authority_end];
        let host_part = match authority.rfind('@') {
            Some(pos) => &authority[pos + 1..],
            None => authority,
        };
        // Handle IPv6 in brackets: [::1]
        if host_part.starts_with('[') {
            let end = host_part.find(']')?;
            let host = &host_part[1..end];
            return if host.is_empty() {
                None
            } else {
                Some(host.to_lowercase())
            };
        }
        let host = host_part.rsplit_once(':').map_or(host_part, |(h, _)| h);
        if host.is_empty() {
            return None;
        }
        return Some(host.to_lowercase());
    }

    // SCP-style: user@host:path. The host is bounded by the first ':' (which
    // begins the path); credentials are taken from the last '@' within that
    // authority, mirroring the scheme path so a planted '@' cannot mis-scope it.
    if url.contains('@') {
        let authority = url.split(':').next().unwrap_or(url);
        let host = match authority.rfind('@') {
            Some(pos) => &authority[pos + 1..],
            None => authority,
        };
        if host.is_empty() {
            return None;
        }
        return Some(host.to_lowercase());
    }

    None
}

/// Strip the userinfo from a git URL. These probes run against URLs that embed a
/// credential (a `$var:` token, or one minted from an `AZURE_DEVOPS_TOKEN(...)`
/// placeholder), and their errors are persisted as the repository's sync status and
/// rendered in the UI. git's own redaction cannot be relied on — it drops the userinfo
/// from `unable to access '<url>'` but echoes it in `could not read Password for
/// '<url>'` — so anything that formats a probe URL has to strip it here.
fn redact_git_url_credentials(url: &str) -> String {
    match git_url_userinfo_range(url) {
        Some(r) => format!("{}***{}", &url[..r.start], &url[r.end..]),
        None => url.to_string(),
    }
}

/// Byte range of a git URL's userinfo (the credentials before the authority's '@'),
/// for both `scheme://user[:pass]@host/path` and SCP-style `user@host:path`.
///
/// The authority ends at the first '/', '?' or '#' and the credentials are the *last*
/// '@' within it, so an '@' planted in the path cannot mis-scope the split
/// (GHSA-p5cj-8cfh-mjv6).
fn git_url_userinfo_range(url: &str) -> Option<std::ops::Range<usize>> {
    let (authority_start, authority) = match url.find("://") {
        Some(scheme_sep) => {
            let start = scheme_sep + 3;
            let after = &url[start..];
            let end = after
                .find(|c| c == '/' || c == '?' || c == '#')
                .unwrap_or(after.len());
            (start, &after[..end])
        }
        // SCP-style `[user@]host:path` has no scheme, and its authority is bounded by
        // the first ':' — never by the last '@', which an '@' in the path would move
        // (the same mis-scoping `extract_host_from_git_url` guards against). scp syntax
        // has no password field, so bounding this way cannot cut a credential in half.
        None if url.contains('@') => (0, url.split(':').next().unwrap_or(url)),
        None => return None,
    };
    let at = authority.rfind('@')?;
    (at > 0).then(|| authority_start..authority_start + at)
}

fn git_url_userinfo(url: &str) -> Option<&str> {
    git_url_userinfo_range(url).map(|r| &url[r])
}

/// Validates a git URL to prevent option injection, SSRF, and local file read.
async fn validate_git_url(url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() {
        return Err(Error::BadRequest("Git URL cannot be empty".to_string()));
    }
    if url.starts_with('-') {
        return Err(Error::BadRequest(
            "Git URL cannot start with '-' (potential option injection)".to_string(),
        ));
    }
    if url.contains('\0') || url.contains('\n') || url.contains('\r') {
        return Err(Error::BadRequest(
            "Git URL contains invalid characters".to_string(),
        ));
    }
    // Reject query/fragment components. git remote URLs never need them, and
    // allowing them lets the URL's true authority (what git actually dials)
    // diverge from the host we validate, e.g.
    // `http://127.0.0.1/repo.git#@github.com/...` (SSRF, GHSA-p5cj-8cfh-mjv6).
    if url.contains('?') || url.contains('#') {
        return Err(Error::BadRequest(
            "Git URL cannot contain '?' or '#' characters".to_string(),
        ));
    }
    // Every probe URL is validated, so this catches a caller that reached git without
    // expanding the placeholder — which git would otherwise report as an unresolvable
    // host, the placeholder's own '/' having truncated the authority.
    if url.contains(AZURE_DEVOPS_TOKEN_PLACEHOLDER) {
        return Err(Error::BadRequest(
            "Git URL still contains an unexpanded AZURE_DEVOPS_TOKEN(...) placeholder".to_string(),
        ));
    }

    let lower = url.to_lowercase();

    // Allowlist of URL formats — blocks file://, ftp://, local paths, etc.
    let has_valid_scheme = lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("git://")
        || lower.starts_with("ssh://");

    // SCP-style: user@host:path (no scheme, has @ before :)
    let is_scp_style = !url.contains("://") && url.contains('@') && url.contains(':');

    if !has_valid_scheme && !is_scp_style {
        return Err(Error::BadRequest(
            "Git URL must use https://, http://, git://, ssh://, or user@host:path format"
                .to_string(),
        ));
    }

    let host = extract_host_from_git_url(url)
        .ok_or_else(|| Error::BadRequest("Could not parse hostname from git URL".to_string()))?;

    // CI/dev escape hatch: integration tests run their git remote (a Gitea
    // container) on localhost, which the network-target checks below reject.
    // Scheme and option-injection validation above still applies.
    if std::env::var("ALLOW_LOCAL_GIT_REMOTES").is_ok_and(|v| v == "true" || v == "1") {
        return Ok(());
    }

    if host == "localhost" || host.ends_with(".local") || host == "[::1]" {
        return Err(Error::BadRequest(
            "Git URLs targeting localhost or local network are not allowed".to_string(),
        ));
    }

    // Check literal IP addresses
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_or_reserved_ip(&ip) {
            return Err(Error::BadRequest(
                "Git URLs targeting private or reserved IP addresses are not allowed".to_string(),
            ));
        }
    } else {
        // Hostname — resolve via DNS and reject if any address is private. Fail
        // closed: a resolution error (or an empty answer) must not skip the
        // private-IP check, else an unresolvable-at-check-time host slips
        // straight through to git.
        let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host(format!("{}:443", host))
            .await
            .map_err(|e| {
                Error::BadRequest(format!("Failed to resolve git URL host '{}': {}", host, e))
            })?
            .collect();
        if addrs.is_empty() {
            return Err(Error::BadRequest(format!(
                "Git URL host '{}' did not resolve to any address",
                host
            )));
        }
        for addr in addrs {
            if is_private_or_reserved_ip(&addr.ip()) {
                return Err(Error::BadRequest(
                    "Git URL hostname resolves to a private or reserved IP address".to_string(),
                ));
            }
        }
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

    let Some(git_repo_resource_value) = git_repo_resource_value else {
        return Err(Error::NotFound(format!("Resource {} not found", path)).into());
    };

    // App-backed repos store a tokenless URL, so the `ls-remote` below can't
    // authenticate. Reuse the poller's REST head lookup, which mints an
    // installation token server-side rather than embedding one in a URL here.
    // It returns `None` for a repo that isn't app-backed, which is exactly the
    // ls-remote case below.
    //
    // Admin-only for the same reason as the archive route: the repository is
    // named by the resource's own `url`, which anyone with write on its path
    // controls, so reading it would let a caller aim the installation
    // credential at any repository it can reach.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    if git_repo_resource_value
        .get("is_github_app")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        require_admin(authed.is_admin, &authed.username)?;
        if let Some((_, commit_hash)) =
            windmill_common::git_sync_ee::get_app_repo_head_for_autopull(&db, &w_id, path).await?
        {
            return Ok(Json(GitCommitHashResponse { commit_hash }));
        }
    }

    let mut git_resource: GitRepositoryResource = serde_json::from_value(git_repo_resource_value)
        .map_err(|e| {
        Error::BadRequest(format!("Invalid git repository resource format: {}", e))
    })?;
    git_resource.url =
        resolve_azure_devops_url(&db_with_opt_authed, &w_id, &git_resource.url, false).await?;

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
    let loc = std::path::Path::new(&*WINDMILL_DIR)
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

/// Run a git remote probe with a hard per-command deadline. The auto-pull poller
/// walks every repository sequentially in one monitor pass, so a single
/// unresponsive remote must not stall the whole pass (or leave a hung child
/// process behind — `kill_on_drop` reaps it when the timeout fires).
const GIT_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// `git` command for a remote probe, with HTTP redirects disabled. `validate_git_url`
/// only vets the host in the URL; git's default (`http.followRedirects=initial`)
/// would let a validated public remote 302 the probe onto a private or link-local
/// address that no check ever sees. Build every probe through this.
fn git_probe_command() -> Command {
    let mut git_cmd = Command::new("git");
    git_cmd.args(["-c", "http.followRedirects=false"]);
    git_cmd
}

/// Whether git gave up on a redirect it was told not to follow. libcurl reports it
/// as a plain status, so the code is all there is to key on.
fn is_refused_redirect(stderr: &str) -> bool {
    stderr.contains("returned error: 30")
}

/// Decode a failed probe's stderr, naming the remedy when the remote redirected
/// somewhere the `.git` retry could not reach (an `http://` URL upgraded to https,
/// say) — `git_probe_command` refuses redirects, so nothing else explains the status.
///
/// `probe_url` is the URL the probe ran against, and its userinfo is scrubbed from the
/// output: git strips credentials from some messages but not all — a token in the
/// username position comes back verbatim in `could not read Password for
/// 'https://<token>@host'` — and these strings are persisted as a repository's sync
/// status and rendered in the UI.
fn git_probe_stderr(stderr: Vec<u8>, probe_url: &str) -> String {
    let stderr =
        String::from_utf8(stderr).unwrap_or_else(|_| "Failed to decode stderr".to_string());
    // Scrub the `<userinfo>@` form git prints, not the bare userinfo: a one-character
    // username would otherwise be replaced everywhere it happens to occur.
    let stderr = match git_url_userinfo(probe_url) {
        Some(userinfo) => stderr.replace(&format!("{userinfo}@"), "***@"),
        None => stderr,
    };
    if is_refused_redirect(&stderr) {
        format!(
            "{} (the remote redirects, and redirects are not followed; set the repository URL to the address it redirects to)",
            stderr.trim_end()
        )
    } else {
        stderr
    }
}

/// The `.git` form of an http(s) repository URL, when that is a different URL.
/// Hosts that serve the bare path as a redirect (gitlab.com answers
/// `/group/project` with a 301 to `/group/project.git`) are otherwise unreachable
/// now that probes refuse redirects.
fn dot_git_url(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/');
    let lower = url.to_lowercase();
    let after_scheme = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))?;
    // Only a non-empty path may be extended. With nothing after the authority,
    // appending lands inside it instead — `https://host` becomes the unvalidated
    // host `https://host.git`, `https://host:8443` a bogus port — which is the hop
    // onto an unchecked address that refusing redirects exists to prevent.
    let path = after_scheme.split_once('/')?.1;
    if path.is_empty() || path.ends_with(".git") {
        return None;
    }
    Some(format!("{}.git", url))
}

/// Run a remote probe, retrying against [`dot_git_url`] if the remote answered the
/// URL as given with a redirect. Extending the path keeps the retry on the host
/// `validate_git_url` already cleared, which is exactly what following the redirect
/// would not guarantee. `build` must produce the probe for the URL it is handed.
///
/// A retry that also fails reports the *original* failure, so the caller's message
/// describes the URL the user configured.
async fn run_git_probe_for_url<F>(url: &str, what: &str, build: F) -> Result<std::process::Output>
where
    F: Fn(&str) -> Command,
{
    let output = run_git_probe(build(url), what).await?;
    if output.status.success() || !is_refused_redirect(&String::from_utf8_lossy(&output.stderr)) {
        return Ok(output);
    }
    let Some(retry_url) = dot_git_url(url) else {
        return Ok(output);
    };
    let retried = run_git_probe(build(&retry_url), what).await?;
    Ok(if retried.status.success() {
        retried
    } else {
        output
    })
}

async fn run_git_probe(mut git_cmd: Command, what: &str) -> Result<std::process::Output> {
    git_cmd.kill_on_drop(true);
    match tokio::time::timeout(GIT_PROBE_TIMEOUT, git_cmd.output()).await {
        Ok(output) => {
            output.map_err(|e| Error::internal_err(format!("Failed to execute git command: {}", e)))
        }
        Err(_) => Err(Error::internal_err(format!(
            "git {} timed out after {}s",
            what,
            GIT_PROBE_TIMEOUT.as_secs()
        ))),
    }
}

/// Git-sync repository URLs may carry `AZURE_DEVOPS_TOKEN(<path/to/azure/resource>)`
/// where a credential belongs: an Azure DevOps access token minted at use time from
/// that `azure` resource's client credentials. The hub sync scripts expand it in
/// TypeScript before running git; the probes below shell out to git from the backend,
/// so they must expand it too or git is handed the literal placeholder (whose '/'
/// truncates the authority, and curl rejects the resulting hostname).
const AZURE_DEVOPS_TOKEN_PLACEHOLDER: &str = "AZURE_DEVOPS_TOKEN(";

/// Azure DevOps resource id the token is minted for, and the endpoint that mints it —
/// both identical to the hub sync scripts', so a repository that authenticates for a
/// sync job authenticates for these probes too.
const AZURE_DEVOPS_RESOURCE_ID: &str = "499b84ac-1321-427f-aa17-267ca6975798/.default";
const AZURE_LOGIN_HOST: &str = "https://login.microsoftonline.com";

/// Minted tokens, keyed by a digest of the credentials they came from — never by
/// resource path, so a cache hit cannot hand a token to a caller who was not able to
/// read the resource itself. Auto-pull probes every repository on an interval; without
/// this, every tick would mint a fresh token.
static AZURE_DEVOPS_TOKEN_CACHE: LazyLock<DashMap<String, (String, i64)>> =
    LazyLock::new(DashMap::new);

/// Shaved off a token's advertised lifetime so one is never handed out as it expires.
const AZURE_TOKEN_EXPIRY_MARGIN_S: i64 = 60;

/// Lifetime assumed when the token response omits `expires_in`.
const AZURE_TOKEN_FALLBACK_LIFETIME_S: i64 = 300;

/// Whether the span `start..end` of `url` is the userinfo of an https authority.
/// The placeholder contains '/', which truncates the authority for any left-to-right
/// parse, so terminators falling inside the span are skipped rather than honored.
///
/// https only: over plaintext an on-path attacker answers the probe's first request
/// with a Basic challenge, and git retries carrying the minted token.
fn span_is_https_userinfo(url: &str, start: usize, end: usize) -> bool {
    let Some(scheme_sep) = url.find("://") else {
        return false;
    };
    if !url[..scheme_sep].eq_ignore_ascii_case("https") {
        return false;
    }
    let body_start = scheme_sep + 3;
    if start < body_start {
        return false;
    }
    let authority_end = url[body_start..]
        .char_indices()
        .map(|(i, c)| (body_start + i, c))
        .find(|&(i, c)| (i < start || i >= end) && (c == '/' || c == '?' || c == '#'))
        .map_or(url.len(), |(i, _)| i);
    // Taking the authority's *last* '@' is what makes one inside the span harmless:
    // such a match sits before `end` and fails the comparison.
    url[body_start..authority_end]
        .rfind('@')
        .is_some_and(|rel| end <= body_start + rel)
}

/// Locate the placeholder in a git URL, returning `(whole placeholder, resource path)`.
fn parse_azure_devops_placeholder(url: &str) -> Result<Option<(&str, &str)>> {
    let Some(start) = url.find(AZURE_DEVOPS_TOKEN_PLACEHOLDER) else {
        return Ok(None);
    };
    let after = &url[start + AZURE_DEVOPS_TOKEN_PLACEHOLDER.len()..];
    // Greedy to the last ')', matching the hub scripts' `AZURE_DEVOPS_TOKEN\((.+)\)`.
    let end = after.rfind(')').ok_or_else(|| {
        Error::BadRequest(
            "Git repository URL has an unterminated AZURE_DEVOPS_TOKEN(...) placeholder"
                .to_string(),
        )
    })?;
    let end = start + AZURE_DEVOPS_TOKEN_PLACEHOLDER.len() + end + 1;
    // Anywhere but the userinfo, the minted token would be spliced into a part of the
    // URL that git echoes verbatim in its failure messages (which are persisted as the
    // repository's sync status) and that credential redaction does not cover.
    if !span_is_https_userinfo(url, start, end) {
        return Err(Error::BadRequest(
            "The AZURE_DEVOPS_TOKEN(...) placeholder must be the credentials of an https git URL, i.e. directly before the '@'".to_string(),
        ));
    }
    Ok(Some((
        &url[start..end],
        &after[..end - start - AZURE_DEVOPS_TOKEN_PLACEHOLDER.len() - 1],
    )))
}

/// Hosts an Azure DevOps token may be sent to. The minted token is an AAD token for
/// the Azure DevOps resource id, so Microsoft is the only party it is meaningful to.
fn is_azure_devops_host(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    host == "dev.azure.com"
        || host.ends_with(".dev.azure.com")
        || host == "visualstudio.com"
        || host.ends_with(".visualstudio.com")
}

/// Expand an `AZURE_DEVOPS_TOKEN(...)` placeholder in a git URL, or return the URL
/// unchanged when it has none. The referenced resource is read through `dba`, so an
/// authed caller only reaches credentials they can already read.
async fn resolve_azure_devops_url(
    dba: &DbWithOptAuthed<'_, ApiAuthed>,
    w_id: &str,
    url: &str,
    allow_cache: bool,
) -> Result<String> {
    // Trim first: the http(s) gates the callers apply trim too, so a stored URL with
    // leading whitespace must not reach the scheme check here as a non-http one.
    let url = url.trim();
    let Some((placeholder, resource_path)) = parse_azure_devops_placeholder(url)? else {
        return Ok(url.to_string());
    };

    // Vet the destination before minting: a URL the host checks would reject must not
    // cost a live credential (nor cache one), and whoever can edit the URL would
    // otherwise drive a token mint per poll tick.
    let probe_url = url.replace(placeholder, "windmill");
    validate_git_url(&probe_url).await?;

    // The background poller reads the referenced resource under the system identity,
    // which bypasses RLS. Confining the destination is what keeps that from becoming an
    // exfiltration primitive: whoever can write this URL picks both the resource path
    // and the host, so an unconfined splice would hand a credential they cannot read to
    // a host they choose. Unlike `$var:`, which substitutes a whole value and so cannot
    // place a secret inside a caller-chosen URL, this placeholder is a substring.
    let host = extract_host_from_git_url(&probe_url)
        .ok_or_else(|| Error::BadRequest("Could not parse hostname from git URL".to_string()))?;
    if !is_azure_devops_host(&host) {
        return Err(Error::BadRequest(format!(
            "An AZURE_DEVOPS_TOKEN(...) placeholder is only allowed on an Azure DevOps URL (dev.azure.com or visualstudio.com), not '{host}'"
        )));
    }

    let value =
        get_resource_value_interpolated_internal(dba, w_id, resource_path, None, None, allow_cache)
            .await
            .map_err(|e| {
                Error::BadRequest(format!(
                    "Azure resource '{resource_path}' referenced by the git repository URL could not be read: {e}"
                ))
            })?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Azure resource '{resource_path}' referenced by the git repository URL was not found"
                ))
            })?;

    let field = |name: &str| -> Result<String> {
        value
            .get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                Error::BadRequest(format!(
                    "Azure resource '{resource_path}' referenced by the git repository URL has no '{name}'"
                ))
            })
    };
    let token = mint_azure_devops_token(
        &field("azureTenantId")?,
        &field("azureClientId")?,
        &field("azureClientSecret")?,
        allow_cache,
    )
    .await?;

    Ok(url.replace(placeholder, &token))
}

/// Gate writing an `AZURE_DEVOPS_TOKEN(...)` reference into a resource value.
///
/// The background probes mint from the named `azure` resource under the system identity,
/// which bypasses RLS, and no principal exists at that point to authorize against — so
/// authorization cannot be enforced where the credential is used, only where the
/// reference is introduced. A read check alone would not survive that gap: the reference
/// names a resource whose own value stays mutable, and repointing it at `$res:`/`$var:`
/// the writer cannot read would be a later write this never sees.
///
/// Hence workspace admin, who can already read every resource in the workspace: the
/// escalation a mutable reference would otherwise buy is one the configurer already has.
/// The read check stays as a typo guard, so a reference to a nonexistent resource fails
/// at configuration time rather than as a puzzling sync error later.
///
/// Only the `url` field is inspected, and with the same parser the probes use, so the
/// path checked here is exactly the path they will resolve.
pub async fn authorize_azure_devops_reference(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    value: Option<&serde_json::Value>,
) -> Result<()> {
    let Some(url) = value.and_then(|v| v.get("url")).and_then(|u| u.as_str()) else {
        return Ok(());
    };
    let Some((_, resource_path)) = parse_azure_devops_placeholder(url.trim())? else {
        return Ok(());
    };

    if !authed.is_admin {
        return Err(Error::PermissionDenied(format!(
            "Only a workspace admin can point a git repository URL at AZURE_DEVOPS_TOKEN({resource_path}): background sync mints that credential under an identity that bypasses resource permissions"
        )));
    }

    let dba = DbWithOptAuthed::from_authed(authed, db.clone(), Some(user_db.clone()));
    let readable =
        get_resource_value_interpolated_internal(&dba, w_id, resource_path, None, None, false)
            .await
            .unwrap_or(None);
    if readable.is_none() {
        return Err(Error::PermissionDenied(format!(
            "Cannot reference AZURE_DEVOPS_TOKEN({resource_path}) in a git repository URL: no such resource"
        )));
    }
    Ok(())
}

/// `allow_cache` carries the caller's freshness requirement through to the token, not
/// just to the resource read: an on-demand check must not succeed on a token minted
/// before the Azure app's permissions were last changed.
async fn mint_azure_devops_token(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    allow_cache: bool,
) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    for part in [tenant_id, client_id, client_secret] {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    let cache_key = hex::encode(hasher.finalize());
    let now = chrono::Utc::now().timestamp();
    // Entries are only ever replaced by a later mint for the same credentials, so a
    // rotated secret's entry would otherwise sit here for the process's lifetime.
    AZURE_DEVOPS_TOKEN_CACHE.retain(|_, (_, expires_at)| *expires_at > now);
    if allow_cache {
        let cached = AZURE_DEVOPS_TOKEN_CACHE
            .get(&cache_key)
            .map(|e| e.value().0.clone());
        if let Some(token) = cached {
            return Ok(token);
        }
    }

    let response = windmill_common::utils::HTTP_CLIENT
        .post(format!("{AZURE_LOGIN_HOST}/{tenant_id}/oauth2/token"))
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
            ("resource", AZURE_DEVOPS_RESOURCE_ID),
        ])
        .send()
        .await
        .map_err(|e| {
            Error::BadRequest(format!("Failed to request an Azure DevOps token: {e:#}"))
        })?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::BadRequest(format!(
            "Azure DevOps token request failed ({status}): {}",
            windmill_common::utils::truncate_with_ellipsis(&body, 500)
        )));
    }

    #[derive(Deserialize)]
    struct AzureTokenResponse {
        access_token: String,
        expires_in: Option<Value>,
    }
    let parsed: AzureTokenResponse = serde_json::from_str(&body)
        .map_err(|e| Error::BadRequest(format!("Unexpected Azure DevOps token response: {e}")))?;

    // The v1 token endpoint returns `expires_in` as a string, the v2 one as a number.
    let lifetime = parsed
        .expires_in
        .as_ref()
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .unwrap_or(AZURE_TOKEN_FALLBACK_LIFETIME_S);
    AZURE_DEVOPS_TOKEN_CACHE.insert(
        cache_key,
        (
            parsed.access_token.clone(),
            now + (lifetime - AZURE_TOKEN_EXPIRY_MARGIN_S).max(0),
        ),
    );

    Ok(parsed.access_token)
}

/// System identity used by background git-sync polling. SECURITY: bypasses resource
/// RLS — see [`resolve_git_repository_resource`] for the caller obligations.
fn git_sync_system_dba(db: &DB) -> DbWithOptAuthed<'static, ApiAuthed> {
    DbWithOptAuthed::DB {
        db: db.clone(),
        audit_author: windmill_common::audit::AuditAuthor {
            username: "git_sync_auto_pull".to_string(),
            email: windmill_common::users::SUPERADMIN_SYNC_EMAIL.to_string(),
            username_override: None,
            token_prefix: None,
        },
    }
}

async fn get_repo_latest_commit_hash(
    git_resource: &GitRepositoryResource,
    git_ssh_command: Option<String>,
) -> Result<String> {
    // Validate URL and branch to prevent option injection and SSRF attacks
    validate_git_url(&git_resource.url).await?;

    let ref_spec = git_resource
        .branch
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("HEAD");

    // Validate ref_spec if it's not the default HEAD
    if ref_spec != "HEAD" {
        validate_git_ref(ref_spec)?;
    }

    let output = run_git_probe_for_url(&git_resource.url, "ls-remote", |url| {
        let mut git_cmd = git_probe_command();
        git_cmd.args(["ls-remote", url, ref_spec]);
        if let Some(git_ssh_command) = git_ssh_command.as_deref() {
            git_cmd.env("GIT_SSH_COMMAND", git_ssh_command);
        }
        git_cmd.stderr(Stdio::piped());
        git_cmd
    })
    .await?;

    if !output.status.success() {
        let stderr = git_probe_stderr(output.stderr, &git_resource.url);
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
            ref_spec,
            redact_git_url_credentials(&git_resource.url)
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

/// Load a git-sync repository resource's value with `$var:`/`$res:` references
/// resolved. Shared by the auto-pull poller (`get_git_repo_head_for_autopull`)
/// and deploy-mode detection so the interpolation lives in exactly one place.
///
/// SECURITY: reads under the system identity (`SUPERADMIN_SYNC_EMAIL`), so it
/// **bypasses resource RLS** and returns fully-interpolated JSON that **may
/// contain credentials** — an embedded `$var:` token in the URL, or the `azure`
/// resource named by an `AZURE_DEVOPS_TOKEN(...)` placeholder. Both name a path
/// chosen by whoever can write the repository URL, not by the reader. Callers must
/// have already authorized access to `w_id`, must use it only for git-sync
/// `git_repository` resources, and must **not** return the resolved value to a
/// client — derive and return only non-sensitive facts. Pass `allow_cache=true`
/// for the poller (avoids re-decrypting/re-auditing a `$var:` secret every tick);
/// pass `false` for on-demand reads that must reflect the current resource.
pub async fn resolve_git_repository_resource(
    db: &DB,
    w_id: &str,
    git_repo_resource_path: &str,
    allow_cache: bool,
) -> Result<Option<serde_json::Value>> {
    let resource_path = git_repo_resource_path
        .strip_prefix("$res:")
        .unwrap_or(git_repo_resource_path);

    get_resource_value_interpolated_internal(
        &git_sync_system_dba(db),
        w_id,
        resource_path,
        None,
        None,
        allow_cache,
    )
    .await
}

/// Resolve a workspace git-sync repository and return its current head commit
/// `(ref_spec, sha)` for the tracked branch, for background auto-pull polling.
/// Returns `Ok(None)` for repos that cannot be polled in-process (GitHub-App
/// repos, which sync via webhooks instead).
pub async fn get_git_repo_head_for_autopull(
    db: &DB,
    w_id: &str,
    git_repo_resource_path: &str,
) -> Result<Option<(String, String)>> {
    let value = resolve_git_repository_resource(db, w_id, git_repo_resource_path, true)
        .await?
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "Git repository resource '{}' not found",
                git_repo_resource_path
                    .strip_prefix("$res:")
                    .unwrap_or(git_repo_resource_path)
            ))
        })?;

    if value
        .get("is_github_app")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(None);
    }

    let mut git_resource: GitRepositoryResource = serde_json::from_value(value)
        .map_err(|e| Error::BadRequest(format!("Invalid git repository resource: {}", e)))?;

    // The SSH identity is supplied per-call in the authed commit-hash path; the
    // background poller has none, so an SSH remote can't authenticate here. Fail
    // with an actionable message instead of a confusing ls-remote auth error —
    // these repos should use an HTTPS token URL or the GitHub App for auto-pull.
    if !git_resource.url.trim_start().starts_with("http://")
        && !git_resource.url.trim_start().starts_with("https://")
    {
        return Err(Error::BadRequest(
            "Automatic pull can't authenticate an SSH git remote in the background. Use an HTTPS URL with an embedded token, or connect the repository through the GitHub App.".to_string(),
        ));
    }
    git_resource.url =
        resolve_azure_devops_url(&git_sync_system_dba(db), w_id, &git_resource.url, true).await?;

    if let Some(branch) = git_resource.branch.as_deref().filter(|s| !s.is_empty()) {
        let branch = branch.to_string();
        let sha = get_repo_latest_commit_hash(&git_resource, None).await?;
        return Ok(Some((branch, sha)));
    }

    // No explicit branch: resolve the remote's default-branch NAME along with
    // its head in one call. Fork sync needs the concrete name to scope
    // `wm-fork/<branch>/*`, so a bare "HEAD" ref would silently disable it.
    validate_git_url(&git_resource.url).await?;
    let output = run_git_probe_for_url(&git_resource.url, "ls-remote --symref HEAD", |url| {
        let mut git_cmd = git_probe_command();
        git_cmd.args(["ls-remote", "--symref", url, "HEAD"]);
        git_cmd.stderr(Stdio::piped());
        git_cmd
    })
    .await?;
    if !output.status.success() {
        let stderr = git_probe_stderr(output.stderr, &git_resource.url);
        return Err(Error::BadRequest(format!(
            "Error resolving git repo HEAD: {}",
            stderr
        )));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| Error::internal_err(format!("Failed to decode git output: {}", e)))?;
    let (branch, sha) = parse_ls_remote_symref_head(&stdout);
    let sha = sha.ok_or_else(|| {
        Error::BadRequest(format!(
            "No HEAD found in repository '{}'",
            redact_git_url_credentials(&git_resource.url)
        ))
    })?;
    Ok(Some((branch.unwrap_or_else(|| "HEAD".to_string()), sha)))
}

/// Parse `git ls-remote --symref <url> HEAD` output: the `ref:` line names the
/// default branch, the plain line carries its head sha.
fn parse_ls_remote_symref_head(stdout: &str) -> (Option<String>, Option<String>) {
    let mut branch = None;
    let mut sha = None;
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("ref:"), Some(target)) => {
                if let Some(name) = target.strip_prefix("refs/heads/") {
                    branch = Some(name.to_string());
                }
            }
            (Some(hash), Some("HEAD")) => {
                sha = Some(hash.to_string());
            }
            _ => {}
        }
    }
    (branch, sha)
}

/// List the head sha of every `wm-fork/<base_branch>/*` branch — plus any
/// `extra_refs` (dev workspaces' environment-label branches, e.g. `dev`,
/// `staging`) — of a workspace git-sync repository in one `git ls-remote` call,
/// for parent-managed fork sync polling. Same auth model and app-repo exclusion
/// as [`get_git_repo_head_for_autopull`]: returns `Ok(None)` for
/// GitHub-App-backed repos (polled over the API instead) and errors on SSH
/// remotes.
pub async fn get_git_repo_fork_heads_for_autopull(
    db: &DB,
    w_id: &str,
    git_repo_resource_path: &str,
    base_branch: &str,
    extra_refs: &[String],
) -> Result<Option<Vec<(String, String)>>> {
    let resource_path = git_repo_resource_path
        .strip_prefix("$res:")
        .unwrap_or(git_repo_resource_path);

    let dba = git_sync_system_dba(db);
    let value =
        get_resource_value_interpolated_internal(&dba, w_id, resource_path, None, None, true)
            .await?
            .ok_or_else(|| {
                Error::BadRequest(format!(
                    "Git repository resource '{}' not found",
                    resource_path
                ))
            })?;

    if value
        .get("is_github_app")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(None);
    }

    let mut git_resource: GitRepositoryResource = serde_json::from_value(value)
        .map_err(|e| Error::BadRequest(format!("Invalid git repository resource: {}", e)))?;
    if !git_resource.url.trim_start().starts_with("http://")
        && !git_resource.url.trim_start().starts_with("https://")
    {
        return Err(Error::BadRequest(
            "Automatic pull can't authenticate an SSH git remote in the background. Use an HTTPS URL with an embedded token, or connect the repository through the GitHub App.".to_string(),
        ));
    }
    git_resource.url = resolve_azure_devops_url(&dba, w_id, &git_resource.url, true).await?;
    validate_git_url(&git_resource.url).await?;
    validate_git_ref(base_branch)?;

    for r in extra_refs {
        validate_git_ref(r)?;
    }

    let output = run_git_probe_for_url(&git_resource.url, "ls-remote (fork branches)", |url| {
        let mut git_cmd = git_probe_command();
        git_cmd.args([
            "ls-remote",
            url,
            &format!("refs/heads/wm-fork/{}/*", base_branch),
        ]);
        for r in extra_refs {
            git_cmd.arg(format!("refs/heads/{}", r));
        }
        git_cmd.stderr(Stdio::piped());
        git_cmd
    })
    .await?;
    if !output.status.success() {
        let stderr = git_probe_stderr(output.stderr, &git_resource.url);
        return Err(Error::BadRequest(format!(
            "Error listing fork branches: {}",
            stderr
        )));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| Error::internal_err(format!("Failed to decode git output: {}", e)))?;
    let heads = stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let sha = parts.next()?;
            let branch = parts.next()?.strip_prefix("refs/heads/")?;
            Some((branch.to_string(), sha.to_string()))
        })
        .collect();
    Ok(Some(heads))
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
    use std::sync::{Arc, Mutex};
    use windmill_common::audit::AuditAuthor;
    use windmill_common::db::DbWithOptAuthed;

    #[test]
    fn parse_symref_head_resolves_default_branch() {
        let out = "ref: refs/heads/main\tHEAD\n7ddb8cec9a0000000000000000000000000000aa\tHEAD\n";
        assert_eq!(
            parse_ls_remote_symref_head(out),
            (
                Some("main".to_string()),
                Some("7ddb8cec9a0000000000000000000000000000aa".to_string())
            )
        );
        // Detached/unknown symref: sha still parses, branch stays None.
        let out2 = "1234567890000000000000000000000000000000\tHEAD\n";
        assert_eq!(
            parse_ls_remote_symref_head(out2),
            (
                None,
                Some("1234567890000000000000000000000000000000".to_string())
            )
        );
        assert_eq!(parse_ls_remote_symref_head(""), (None, None));
    }

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

    // Regression test for WIN-1957: deeply nested structures must be bounded so
    // that interpolation cannot recurse without limit (which would otherwise
    // overflow the stack).
    #[tokio::test]
    async fn test_transform_json_value_bounds_recursion_depth() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let dba = test_db_with_opt_authed(pool);

        // Build an object nested deeper than the allowed interpolation depth.
        // No `$res:`/`$var:` leaves are involved, so this exercises the depth
        // guard purely on structural recursion (no DB lookups required).
        let mut input = Value::String("plain".to_string());
        for _ in 0..(MAX_RESOURCE_INTERPOLATION_DEPTH as usize + 5) {
            let mut m = serde_json::Map::new();
            m.insert("a".to_string(), input);
            input = Value::Object(m);
        }

        let result = transform_json_value(&dba, "test", input, &None, None, 0).await;

        let err = result.expect_err("deeply nested value should be rejected");
        assert!(
            err.to_string().contains("interpolation depth"),
            "unexpected error: {err}"
        );
    }

    // Regression test for WIN-1957: two resources whose values reference each
    // other via `$res:` must NOT recurse forever (stack overflow / process
    // crash). With the depth guard the resolution terminates with an error.
    //
    // This test needs the real `workspace`/`resource` schema, so it uses
    // `#[sqlx::test]` which provisions a migrated ephemeral database per test
    // (the bare `DATABASE_URL` database in CI has no migrations applied, which
    // previously made the workspace INSERT panic with `relation "workspace"
    // does not exist` — WIN-1958).
    #[sqlx::test(migrations = "../migrations")]
    async fn test_transform_json_value_mutual_resource_recursion_terminates(pool: DB) {
        let w_id = format!("dostest{}", Uuid::new_v4().simple());

        sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test@windmill.dev')")
            .bind(&w_id)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO resource (workspace_id, path, value, resource_type) VALUES \
             ($1, 'f/test/dos_a', $2, 'object'), \
             ($1, 'f/test/dos_b', $3, 'object')",
        )
        .bind(&w_id)
        .bind(json!("$res:f/test/dos_b"))
        .bind(json!("$res:f/test/dos_a"))
        .execute(&pool)
        .await
        .unwrap();

        let dba = test_db_with_opt_authed(pool.clone());
        let result = transform_json_value(
            &dba,
            &w_id,
            Value::String("$res:f/test/dos_a".to_string()),
            &None,
            None,
            0,
        )
        .await;

        // The ephemeral test database is dropped automatically, so no manual
        // row cleanup is required.
        let err = result.expect_err("mutually recursive resources should error, not crash");
        assert!(
            err.to_string().contains("interpolation depth"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_extract_host_from_git_url() {
        // Standard HTTPS
        assert_eq!(
            extract_host_from_git_url("https://github.com/user/repo.git"),
            Some("github.com".to_string())
        );
        // HTTPS with port
        assert_eq!(
            extract_host_from_git_url("https://git.example.com:8443/repo.git"),
            Some("git.example.com".to_string())
        );
        // SSH with scheme
        assert_eq!(
            extract_host_from_git_url("ssh://git@github.com/user/repo.git"),
            Some("github.com".to_string())
        );
        // SCP-style
        assert_eq!(
            extract_host_from_git_url("git@github.com:user/repo.git"),
            Some("github.com".to_string())
        );
        // Git protocol
        assert_eq!(
            extract_host_from_git_url("git://example.com/repo.git"),
            Some("example.com".to_string())
        );
        // IPv6 in brackets
        assert_eq!(
            extract_host_from_git_url("http://[::1]:8080/repo.git"),
            Some("::1".to_string())
        );
        // Fragment/query must not leak into the authority (GHSA-p5cj-8cfh-mjv6):
        // the host is the real authority, not the '@' planted in the fragment/query.
        assert_eq!(
            extract_host_from_git_url(
                "http://127.0.0.1:40173/repo.git#@github.com/windmill-labs/windmill.git"
            ),
            Some("127.0.0.1".to_string())
        );
        assert_eq!(
            extract_host_from_git_url(
                "http://127.0.0.1:40173/repo.git?@github.com/windmill-labs/windmill.git"
            ),
            Some("127.0.0.1".to_string())
        );
        // Path-less authority terminated by the fragment (exercises the '#' branch
        // of the authority boundary directly).
        assert_eq!(
            extract_host_from_git_url("http://127.0.0.1#@github.com"),
            Some("127.0.0.1".to_string())
        );
        // SCP-style with a planted extra '@' must resolve to the real host, not the
        // credential segment.
        assert_eq!(
            extract_host_from_git_url("a@b@127.0.0.1:user/repo.git"),
            Some("127.0.0.1".to_string())
        );
        // No host extractable
        assert_eq!(extract_host_from_git_url("/local/path"), None);
        assert_eq!(
            extract_host_from_git_url("file:///etc/passwd"),
            Some("".to_string()).filter(|s| !s.is_empty())
        );
    }

    #[test]
    fn test_is_private_or_reserved_ip() {
        use std::net::IpAddr;
        // Loopback
        assert!(is_private_or_reserved_ip(
            &"127.0.0.1".parse::<IpAddr>().unwrap()
        ));
        assert!(is_private_or_reserved_ip(
            &"127.0.0.2".parse::<IpAddr>().unwrap()
        ));
        // Private ranges
        assert!(is_private_or_reserved_ip(
            &"10.0.0.1".parse::<IpAddr>().unwrap()
        ));
        assert!(is_private_or_reserved_ip(
            &"172.16.0.1".parse::<IpAddr>().unwrap()
        ));
        assert!(is_private_or_reserved_ip(
            &"192.168.1.1".parse::<IpAddr>().unwrap()
        ));
        // Link-local / cloud metadata
        assert!(is_private_or_reserved_ip(
            &"169.254.169.254".parse::<IpAddr>().unwrap()
        ));
        // CGNAT
        assert!(is_private_or_reserved_ip(
            &"100.64.0.1".parse::<IpAddr>().unwrap()
        ));
        // "This network" 0.0.0.0/8, not just the unspecified address
        assert!(is_private_or_reserved_ip(
            &"0.0.0.0".parse::<IpAddr>().unwrap()
        ));
        assert!(is_private_or_reserved_ip(
            &"0.0.0.1".parse::<IpAddr>().unwrap()
        ));
        // IPv6 loopback
        assert!(is_private_or_reserved_ip(&"::1".parse::<IpAddr>().unwrap()));
        // IPv6 unique local address (fc00::/7)
        assert!(is_private_or_reserved_ip(
            &"fd00::1".parse::<IpAddr>().unwrap()
        ));
        assert!(is_private_or_reserved_ip(
            &"fc00::1".parse::<IpAddr>().unwrap()
        ));
        // IPv6 link-local (fe80::/10)
        assert!(is_private_or_reserved_ip(
            &"fe80::1".parse::<IpAddr>().unwrap()
        ));
        // IPv4-mapped IPv6
        assert!(is_private_or_reserved_ip(
            &"::ffff:127.0.0.1".parse::<IpAddr>().unwrap()
        ));
        // Public IPs should pass
        assert!(!is_private_or_reserved_ip(
            &"8.8.8.8".parse::<IpAddr>().unwrap()
        ));
        assert!(!is_private_or_reserved_ip(
            &"140.82.121.4".parse::<IpAddr>().unwrap()
        ));
        // Public IPv6 should pass
        assert!(!is_private_or_reserved_ip(
            &"2606:2800:220:1:248:1893:25c8:1946"
                .parse::<IpAddr>()
                .unwrap()
        ));
    }

    #[tokio::test]
    async fn test_validate_git_url_blocks_file_scheme() {
        let result = validate_git_url("file:///etc/passwd").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("https://"));
    }

    #[tokio::test]
    async fn test_validate_git_url_blocks_private_ips() {
        assert!(validate_git_url("http://127.0.0.1/repo.git").await.is_err());
        assert!(validate_git_url("http://169.254.169.254/latest/meta-data/")
            .await
            .is_err());
        assert!(validate_git_url("http://10.0.0.1/repo.git").await.is_err());
        assert!(validate_git_url("http://172.16.0.1/repo.git")
            .await
            .is_err());
        assert!(validate_git_url("http://192.168.1.1/repo.git")
            .await
            .is_err());
        assert!(validate_git_url("git://0.0.0.0/repo.git").await.is_err());
        // IPv6 loopback, unique-local, and link-local literals
        assert!(validate_git_url("git://[::1]/repo.git").await.is_err());
        assert!(validate_git_url("git://[fd00::1]/repo.git").await.is_err());
        assert!(validate_git_url("git://[fe80::1]/repo.git").await.is_err());
    }

    #[tokio::test]
    async fn test_validate_git_url_blocks_localhost() {
        assert!(validate_git_url("http://localhost/repo.git").await.is_err());
        assert!(validate_git_url("http://myhost.local/repo.git")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_git_url_blocks_local_paths() {
        assert!(validate_git_url("/etc/passwd").await.is_err());
        assert!(validate_git_url("../relative/path").await.is_err());
        assert!(validate_git_url("./local/repo").await.is_err());
    }

    /// Minimal loopback HTTP server: replies to every request with `response` and
    /// records the request lines it saw. Returns its port and that log.
    async fn spawn_http_stub(response: String) -> (u16, Arc<Mutex<Vec<String>>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_srv = requests.clone();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let line = String::from_utf8_lossy(&buf[..n])
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                requests_srv.lock().unwrap().push(line);
                let _ = sock.write_all(response.as_bytes()).await;
            }
        });
        (port, requests)
    }

    /// A probe against `path` on a stub that redirects everything to `target_port`.
    async fn probe_redirecting_stub(
        path: &str,
        target_port: u16,
    ) -> (std::process::Output, Arc<Mutex<Vec<String>>>) {
        let (redirector_port, redirector_requests) = spawn_http_stub(format!(
            "HTTP/1.1 301 Moved Permanently\r\nLocation: http://127.0.0.1:{}/moved/info/refs?service=git-upload-pack\r\nContent-Length: 0\r\n\r\n",
            target_port
        ))
        .await;
        let url = format!("http://127.0.0.1:{}{}", redirector_port, path);
        let output = run_git_probe_for_url(&url, "ls-remote (test)", |url| {
            let mut git_cmd = git_probe_command();
            git_cmd.args(["ls-remote", url, "HEAD"]);
            git_cmd.env("GIT_TERMINAL_PROMPT", "0");
            // curl routes even loopback through an ambient `http_proxy`, which
            // would leave the stub unreached and the assertions vacuously true.
            git_cmd.env("no_proxy", "*").env("NO_PROXY", "*");
            git_cmd.stderr(Stdio::piped());
            git_cmd
        })
        .await
        .unwrap();
        (output, redirector_requests)
    }

    #[tokio::test]
    async fn test_git_probe_refuses_redirects() {
        // `validate_git_url` only vets the URL's host, so a probe that follows a
        // 301 dials an address nothing validated. Redirect one probe and require
        // git to stop at the 301 without ever reaching the target.
        let (target_port, target_requests) = spawn_http_stub(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n".to_string(),
        )
        .await;
        let (output, _) = probe_redirecting_stub("/repo.git", target_port).await;

        assert!(!output.status.success());
        assert!(
            target_requests.lock().unwrap().is_empty(),
            "git followed the redirect to the unvalidated target"
        );
        let stderr = git_probe_stderr(output.stderr, "");
        assert!(
            stderr.contains("301") && stderr.contains("redirects are not followed"),
            "the failure should name the refused redirect and its remedy, got: {stderr}"
        );
    }

    #[tokio::test]
    async fn test_git_probe_retries_dot_git_on_redirect() {
        // gitlab.com answers `/group/project` with a redirect to the `.git` form,
        // which refusing redirects would otherwise make unreachable. The retry must
        // extend the path only — never leave the host `validate_git_url` cleared.
        let (target_port, target_requests) = spawn_http_stub(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n".to_string(),
        )
        .await;
        let (_, redirector_requests) = probe_redirecting_stub("/group/project", target_port).await;

        let requests = redirector_requests.lock().unwrap().clone();
        assert!(
            requests
                .iter()
                .any(|r| r.contains("/group/project.git/info/refs")),
            "the redirect should have been retried against the .git path, saw: {requests:?}"
        );
        assert!(
            target_requests.lock().unwrap().is_empty(),
            "the retry left the validated host"
        );
    }

    #[test]
    fn test_dot_git_url_only_extends_the_path() {
        assert_eq!(
            dot_git_url("https://gitlab.com/group/project").as_deref(),
            Some("https://gitlab.com/group/project.git")
        );
        assert_eq!(
            dot_git_url("https://gitlab.com/group/project/").as_deref(),
            Some("https://gitlab.com/group/project.git")
        );
        assert_eq!(
            dot_git_url("https://user:tok@gitlab.com:8443/group/project").as_deref(),
            Some("https://user:tok@gitlab.com:8443/group/project.git")
        );
        // A pathless URL has only its authority to extend, so `example.com` would
        // become the unvalidated host `example.com.git`. Never retry those.
        assert_eq!(dot_git_url("https://example.com"), None);
        assert_eq!(dot_git_url("https://example.com/"), None);
        assert_eq!(dot_git_url("http://example.com:8080/"), None);
        // Nothing to retry: already the `.git` form, or no HTTP redirect surface.
        assert_eq!(dot_git_url("https://gitlab.com/group/project.git"), None);
        assert_eq!(dot_git_url("git@github.com:user/repo"), None);
        assert_eq!(dot_git_url("ssh://git@github.com/user/repo"), None);
    }

    #[tokio::test]
    async fn test_validate_git_url_fails_closed_on_unresolvable_host() {
        // `.invalid` never resolves (RFC 6761). The private-IP check is only
        // meaningful if a failed lookup rejects instead of falling through.
        let result = validate_git_url("https://this-host-does-not-exist.invalid/repo.git").await;
        assert!(
            result.is_err(),
            "an unresolvable host was allowed — does this resolver synthesize records for NXDOMAIN?"
        );
        assert!(result.unwrap_err().to_string().contains("resolve"));
    }

    #[tokio::test]
    async fn test_validate_git_url_allows_valid_urls() {
        // Needs DNS: validation fails closed on a host it cannot resolve.
        assert!(validate_git_url("https://github.com/user/repo.git")
            .await
            .is_ok());
        assert!(validate_git_url("git@github.com:user/repo.git")
            .await
            .is_ok());
        assert!(validate_git_url("ssh://git@github.com/user/repo.git")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_validate_git_url_blocks_option_injection() {
        assert!(validate_git_url("-evil").await.is_err());
        assert!(validate_git_url("--upload-pack=evil").await.is_err());
    }

    #[test]
    fn test_parse_azure_devops_placeholder() {
        // The resource path holds '/', so the placeholder must be cut at its own
        // closing ')' rather than at the first path separator.
        let url = "https://AZURE_DEVOPS_TOKEN(f/azure/devops)@dev.azure.com/org/proj/_git/repo";
        assert_eq!(
            parse_azure_devops_placeholder(url).unwrap(),
            Some(("AZURE_DEVOPS_TOKEN(f/azure/devops)", "f/azure/devops"))
        );
        assert_eq!(
            parse_azure_devops_placeholder("https://token@github.com/user/repo.git").unwrap(),
            None
        );
        assert!(parse_azure_devops_placeholder(
            "https://AZURE_DEVOPS_TOKEN(f/azure@dev.azure.com/o"
        )
        .is_err());
        // Outside the userinfo the minted token would land in a URL component that git
        // echoes back in its errors and redaction does not cover.
        assert!(parse_azure_devops_placeholder(
            "https://dev.azure.com/org/AZURE_DEVOPS_TOKEN(f/azure)/repo"
        )
        .is_err());
        assert!(parse_azure_devops_placeholder(
            "ssh://AZURE_DEVOPS_TOKEN(f/azure)@dev.azure.com/o"
        )
        .is_err());
        // Plaintext would let an on-path Basic challenge harvest the minted token.
        assert!(parse_azure_devops_placeholder(
            "http://AZURE_DEVOPS_TOKEN(f/azure)@dev.azure.com/o"
        )
        .is_err());
        // A `user:token` userinfo is still the credentials position.
        assert_eq!(
            parse_azure_devops_placeholder("https://u:AZURE_DEVOPS_TOKEN(f/azure)@dev.azure.com/o")
                .unwrap(),
            Some(("AZURE_DEVOPS_TOKEN(f/azure)", "f/azure"))
        );
    }

    #[test]
    fn test_is_azure_devops_host() {
        assert!(is_azure_devops_host("dev.azure.com"));
        assert!(is_azure_devops_host("vssps.dev.azure.com"));
        assert!(is_azure_devops_host("myorg.visualstudio.com"));
        // The whole point: a token must never be splice-able onto a chosen host.
        assert!(!is_azure_devops_host("attacker.example"));
        assert!(!is_azure_devops_host("dev.azure.com.attacker.example"));
        assert!(!is_azure_devops_host("notvisualstudio.com"));
        assert!(!is_azure_devops_host("github.com"));
    }

    #[test]
    fn test_redact_git_url_credentials() {
        assert_eq!(
            redact_git_url_credentials("https://tok@dev.azure.com/o/p"),
            "https://***@dev.azure.com/o/p"
        );
        assert_eq!(
            redact_git_url_credentials("https://user:tok@github.com/u/r.git"),
            "https://***@github.com/u/r.git"
        );
        // SCP-style `[user@]host:path` carries its credential in the user position.
        assert_eq!(
            redact_git_url_credentials("tok@github.com:u/r.git"),
            "***@github.com:u/r.git"
        );
        // A '@' in the path must not be mistaken for the credentials separator.
        assert_eq!(
            redact_git_url_credentials("https://github.com/u/r@v1.git"),
            "https://github.com/u/r@v1.git"
        );
        // A userinfo that also occurs in the scheme must not be redacted there.
        assert_eq!(
            redact_git_url_credentials("https://s@https.com/r"),
            "https://***@https.com/r"
        );
    }

    #[test]
    fn test_git_probe_stderr_scrubs_the_probe_url_credentials() {
        // git echoes a username-position token verbatim in this message, and the result
        // is persisted as the repository's sync status.
        let stderr = b"fatal: could not read Password for 'https://SECRET@dev.azure.com'".to_vec();
        let out = git_probe_stderr(stderr, "https://SECRET@dev.azure.com/o/p");
        assert!(!out.contains("SECRET"), "token survived redaction: {out}");
    }

    #[tokio::test]
    async fn test_validate_git_url_blocks_fragment_query_ssrf() {
        // GHSA-p5cj-8cfh-mjv6: a loopback authority must stay blocked, and the
        // fragment/query `@public-host` bypasses of #8600 must be rejected so the
        // host git dials can never diverge from the validated host.
        assert!(validate_git_url("http://127.0.0.1:40173/repo.git")
            .await
            .is_err());
        assert!(validate_git_url(
            "http://127.0.0.1:40173/repo.git#@github.com/windmill-labs/windmill.git"
        )
        .await
        .is_err());
        assert!(validate_git_url(
            "http://127.0.0.1:40173/repo.git?@github.com/windmill-labs/windmill.git"
        )
        .await
        .is_err());
        // A legitimate public repo URL still validates.
        assert!(
            validate_git_url("https://github.com/windmill-labs/windmill.git")
                .await
                .is_ok()
        );
    }
}
