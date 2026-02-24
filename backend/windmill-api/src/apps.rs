use std::{collections::HashMap, sync::Arc};

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use crate::{
    auth::OptTokened,
    db::{ApiAuthed, DB},
    jobs::RunJobQuery,
    users::{require_owner_of_path, OptAuthed},
    utils::{check_scopes, WithStarredInfoQuery},
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
#[cfg(feature = "parquet")]
use crate::{
    job_helpers_oss::{
        download_s3_file_internal, get_random_file_name, get_s3_resource,
        get_workspace_s3_resource_and_check_paths, upload_file_from_req, DownloadFileQuery,
    },
    users::fetch_api_authed_from_permissioned_as,
};
use axum::response::Response;
use axum::{
    body::Body,
    extract::{Extension, Json, Multipart, Path, Query},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use futures::future::{FutureExt, TryFutureExt};
use hyper::StatusCode;
#[cfg(feature = "parquet")]
use itertools::Itertools;
use lazy_static::lazy_static;
use magic_crypt::MagicCryptTrait;
#[cfg(feature = "parquet")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use sha2::{Digest, Sha256};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Uuid, FromRow};
use std::str;
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::{
    apps::{AppScriptId, ListAppQuery, APP_WORKSPACED_ROUTE},
    auth::TOKEN_PREFIX_LEN,
    cache::{self, future::FutureCachedExt},
    db::{DbWithOptAuthed, UserDB},
    error::{to_anyhow, Error, JsonResult, Result},
    jobs::{get_payload_tag_from_prefixed_path, JobPayload, RawCode},
    users::username_to_permissioned_as,
    utils::{
        http_get_from_hub, not_found_if_none, paginate, query_elems_from_hub, require_admin,
        Pagination, RunnableKind, StripPath,
    },
    variables::{build_crypt, build_crypt_with_key_suffix, encrypt},
    worker::{to_raw_value, CLOUD_HOSTED},
    workspaces::{check_user_against_rule, ProtectionRuleKind, RuleCheckResult},
    HUB_BASE_URL,
};
#[cfg(feature = "parquet")]
use windmill_object_store::object_store_reexports::{Attribute, Attributes};
use windmill_store::resources::get_resource_value_interpolated_internal;

use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, PushArgs, PushArgsOwned, PushIsolationLevel};

#[cfg(feature = "parquet")]
use hmac::Mac;
#[cfg(feature = "parquet")]
use windmill_common::{jwt, oauth2::HmacSha256, variables::get_workspace_key};
#[cfg(feature = "parquet")]
use windmill_types::s3::{S3Object, S3Permission};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_apps))
        .route("/list_search", get(list_search_apps))
        .route("/get/p/*path", get(get_app))
        .route("/get/lite/*path", get(get_app_lite))
        .route("/get/draft/*path", get(get_app_w_draft))
        .route("/secret_of/*path", get(get_secret_id))
        .route(
            "/secret_of_latest_version/*path",
            get(get_latest_version_secret_id),
        )
        .route("/get/v/*id", get(get_app_by_id))
        .route("/get_data/v/*id", get(get_raw_app_data))
        .route("/exists/*path", get(exists_app))
        .route("/update/*path", post(update_app))
        .route("/update_raw/*path", post(update_app_raw))
        .route("/delete/*path", delete(delete_app))
        .route("/create", post(create_app))
        .route("/create_raw", post(create_app_raw))
        .route("/history/p/*path", get(get_app_history))
        .route("/get_latest_version/*path", get(get_latest_version))
        .route("/history_update/a/:id/v/:version", post(update_app_history))
        .route(
            "/list_paths_from_workspace_runnable/:runnable_kind/*path",
            get(list_paths_from_workspace_runnable),
        )
        .route("/custom_path_exists/*custom_path", get(custom_path_exists))
        .route("/sign_s3_objects", post(sign_s3_objects))
}

pub fn unauthed_service() -> Router {
    Router::new()
        .route("/execute_component/*path", post(execute_component))
        .route("/upload_s3_file/*path", post(upload_s3_file_from_app))
        .route("/delete_s3_file", delete(delete_s3_file_from_app))
        .route("/download_s3_file/*path", get(download_s3_file_from_app))
        .route("/public_app/:secret", get(get_public_app_by_secret))
        .route("/public_resource/*path", get(get_public_resource))
        .route("/get_data/v/*id", get(get_raw_app_data))
}
pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_apps))
        .route("/hub/get/:id", get(get_hub_app_by_id))
        .route("/hub/get_raw/:id", get(get_hub_raw_app_by_id))
}

#[derive(FromRow, Deserialize, Serialize)]
pub struct ListableApp {
    pub id: i64,
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub version: i64,
    pub extra_perms: serde_json::Value,
    pub execution_mode: String,
    pub starred: bool,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
    pub has_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub raw_app: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

// #[derive(FromRow, Serialize, Deserialize)]
// pub struct AppVersion {
//     pub id: i64,
//     pub app_id: Uuid,
//     pub value: sqlx::types::Json<Box<RawValue>>,
//     pub created_by: String,
//     pub created_at: chrono::DateTime<chrono::Utc>,
// }

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AppWithLastVersion {
    pub id: i64,
    pub path: String,
    pub summary: String,
    pub policy: sqlx::types::Json<Box<RawValue>>,
    pub versions: Vec<i64>,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_path: Option<String>,
    pub raw_app: bool,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_secret: Option<String>,
}

#[derive(Serialize, FromRow)]
pub struct AppWithLastVersionAndStarred {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub app: AppWithLastVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct AppWithLastVersionAndDraft {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub app: AppWithLastVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<sqlx::types::Json<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
}

#[derive(Serialize)]
pub struct AppHistory {
    pub app_id: i64,
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
}

#[derive(Deserialize)]
pub struct AppHistoryUpdate {
    pub deployment_msg: Option<String>,
}

pub type StaticFields = HashMap<String, Box<RawValue>>;
pub type OneOfFields = HashMap<String, Vec<Box<RawValue>>>;
pub type AllowUserResources = Vec<String>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Anonymous,
    Publisher,
    Viewer,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PolicyTriggerableInputs {
    static_inputs: StaticFields,
    one_of_inputs: OneOfFields,
    #[serde(default)]
    allow_user_resources: AllowUserResources,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct S3Input {
    allowed_resources: Vec<String>,
    allow_user_resources: bool,
    allow_workspace_resource: bool,
    file_key_regex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct S3Key {
    s3_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Policy {
    pub on_behalf_of: Option<String>,
    pub on_behalf_of_email: Option<String>,
    //paths:
    // - script/<path>
    // - flow/<path>
    // - rawscript/<sha256>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggerables: Option<HashMap<String, StaticFields>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggerables_v2: Option<HashMap<String, PolicyTriggerableInputs>>,
    pub execution_mode: ExecutionMode,
    pub s3_inputs: Option<Vec<S3Input>>,
    pub allowed_s3_keys: Option<Vec<S3Key>>,
}

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub policy: Policy,
    pub draft_only: Option<bool>,
    pub deployment_message: Option<String>,
    pub custom_path: Option<String>,
    pub preserve_on_behalf_of: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<sqlx::types::Json<Box<RawValue>>>,
    pub policy: Option<Policy>,
    pub deployment_message: Option<String>,
    pub custom_path: Option<String>,
    pub preserve_on_behalf_of: Option<bool>,
}

#[derive(Serialize, FromRow)]
pub struct SearchApp {
    path: String,
    value: sqlx::types::Json<Box<RawValue>>,
}
async fn list_search_apps(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<SearchApp>> {
    #[cfg(feature = "enterprise")]
    let n = 1000;

    #[cfg(not(feature = "enterprise"))]
    let n = 3;
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_as::<_, SearchApp>(
        "SELECT path, app_version.value from app LEFT JOIN app_version ON app_version.id = versions[array_upper(versions, 1)]  WHERE workspace_id = $1 LIMIT $2",
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

async fn list_apps(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListAppQuery>,
) -> JsonResult<Vec<ListableApp>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("app")
        .fields(&[
            "app.id",
            "app.workspace_id",
            "app.path",
            "app.summary",
            "app.versions[array_upper(app.versions, 1)] as version",
            "app.policy->>'execution_mode' as execution_mode",
            "app_version.created_at as edited_at",
            "app.extra_perms",
            "favorite.path IS NOT NULL as starred",
            "draft.path IS NOT NULL as has_draft",
            "draft_only",
            "app_version.raw_app",
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'app' AND favorite.workspace_id = app.workspace_id AND favorite.path = app.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .left()
        .join("app_version")
        .on(
            "app_version.id = versions[array_upper(versions, 1)]"
        )
        .left()
        .join("draft")
        .on(
            "draft.path = app.path AND draft.workspace_id = app.workspace_id AND draft.typ = 'app'"
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("app_version.created_at", true)
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

    if !lq.include_draft_only.unwrap_or(false) || authed.is_operator {
        sqlb.and_where("app.draft_only IS NOT TRUE");
    }

    if lq.with_deployment_msg.unwrap_or(false) {
        sqlb.join("deployment_metadata dm")
            .left()
            .on("dm.app_version = app.versions[array_upper(app.versions, 1)]")
            .fields(&["dm.deployment_msg"]);
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableApp>(&sql)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_raw_app_data(
    Path((w_id, secret_with_ext)): Path<(String, String)>,
    Extension(db): Extension<DB>,
) -> Result<Response> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let object_store = windmill_object_store::get_object_store().await;

    // tracing::info!("secret_with_ext: {}", secret_with_ext);
    let mut splitted = secret_with_ext.split('.');
    let secret_id = splitted.next().unwrap_or("");

    if secret_id.is_empty() {
        return Err(Error::BadRequest("Invalid secret".to_string()));
    }

    let id = get_id_from_secret(
        &db,
        &w_id,
        secret_id.to_string(),
        Some(BUNDLE_SECRET_PREFIX),
    )
    .await?;

    let file_type = splitted.next().unwrap_or("");
    let file_type = if file_type == "css" {
        "css"
    } else if file_type == "js" {
        "js"
    } else {
        return Err(Error::BadRequest(
            "Invalid file type, only .css and .js are supported".to_string(),
        ));
    };
    // tracing::info!("file_type: {}", file_type);

    #[allow(unused_assignments)]
    let mut body: Option<Body> = None;
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = object_store {
        let path = format!("/app_bundles/{}/{}.{}", w_id, id, file_type);
        let stream = os
            .get(&windmill_object_store::object_store_reexports::Path::from(
                path,
            ))
            .await
            .map_err(windmill_object_store::object_store_error_to_error)?
            .bytes()
            .await
            .map_err(windmill_object_store::object_store_error_to_error)?;
        tracing::info!("stream: {}", stream.len());
        body = Some(Body::from(stream));
    }

    if body.is_none() {
        let get_raw_app_file = sqlx::query_scalar!(
            "SELECT data FROM app_bundles WHERE app_version_id = $1 AND file_type = $2 AND w_id = $3",
            id,
            file_type,
            &w_id,
        )
        .fetch_optional(&db)
        .await?;
        if let Some(file) = get_raw_app_file {
            body = Some(Body::from(file));
        }
    }

    if let Some(body) = body {
        // let stream = tokio_util::io::ReaderStream::new(file);
        let res = Response::builder().header(
            http::header::CONTENT_TYPE,
            if file_type == "css" {
                "text/css"
            } else {
                "text/javascript"
            },
        );
        Ok(res.body(body).unwrap())
    } else {
        return Err(Error::NotFound("File not found".to_string()));
    }
}

// async fn get_app_version(
//     authed: ApiAuthed,
//     Extension(user_db): Extension<UserDB>,
//     Path((w_id, path)): Path<(String, StripPath)>,
// ) -> JsonResult<i64> {
//     let path = path.to_path();
//     let mut tx = user_db.begin(&authed).await?;

//     let version_o = sqlx::query_scalar!(
//         "SELECT app.versions[array_upper(app.versions, 1)] as version FROM app
//             WHERE app.path = $1 AND app.workspace_id = $2",
//         path,
//         &w_id,
//     )
//     .fetch_optional(&mut *tx)
//     .await?
//     .flatten();
//     tx.commit().await?;

//     let version = not_found_if_none(version_o, "App", path)?;
//     Ok(Json(version))
// }

async fn get_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<WithStarredInfoQuery>,
) -> JsonResult<AppWithLastVersionAndStarred> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let app_o = if query.with_starred_info.unwrap_or(false) {
        sqlx::query_as::<_, AppWithLastVersionAndStarred>(
            "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
            app.extra_perms, app_version.value, 
            app_version.created_at, app_version.created_by, favorite.path IS NOT NULL as starred, app_version.raw_app
            FROM app
            JOIN app_version
            ON app_version.id = app.versions[array_upper(app.versions, 1)]
            LEFT JOIN favorite
            ON favorite.favorite_kind = 'app' 
                AND favorite.workspace_id = app.workspace_id 
                AND favorite.path = app.path 
                AND favorite.usr = $3
            WHERE app.path = $1 AND app.workspace_id = $2",
        )
        .bind(path.to_owned())
        .bind(&w_id)
        .bind(&authed.username)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_as::<_, AppWithLastVersionAndStarred>(
            "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
            app.extra_perms, app_version.value, 
            app_version.created_at, app_version.created_by, NULL as starred, app_version.raw_app
            FROM app, app_version
            WHERE app.path = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
        )
        .bind(path.to_owned())
        .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?
    };
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    Ok(Json(app))
}

async fn get_app_lite(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AppWithLastVersion> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
        app.extra_perms, coalesce(app_version_lite.value::json, app_version.value) as value, 
        app_version.created_at, app_version.created_by, NULL as starred, app_version.raw_app
        FROM app, app_version
        LEFT JOIN app_version_lite ON app_version_lite.id = app_version.id
        WHERE app.path = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
    )
    .bind(path.to_owned())
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    Ok(Json(app))
}

async fn get_app_w_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AppWithLastVersionAndDraft> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersionAndDraft>(
        r#"
        SELECT 
            app.id, 
            app.path, 
            app.summary, 
            app.versions, 
            app.policy, 
            app.custom_path,
            app.extra_perms, 
            app_version.value,
            app_version.created_at, 
            app_version.created_by,
            app.draft_only,
            draft.value AS "draft",
            app_version.raw_app
        FROM app
        INNER JOIN app_version 
            ON app_version.id = app.versions[array_upper(app.versions, 1)]
        LEFT JOIN draft 
            ON app.path = draft.path 
        AND draft.workspace_id = $2 
        AND draft.typ = 'app'
        WHERE app.path = $1 
        AND app.workspace_id = $2
    "#,
    )
    .bind(path.to_owned())
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    Ok(Json(app))
}

async fn get_app_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<AppHistory>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", &path))?;
    let mut tx = user_db.begin(&authed).await?;
    let query_result = sqlx::query!(
        "SELECT a.id as app_id, av.id as version_id, dm.deployment_msg as deployment_msg
        FROM app a LEFT JOIN app_version av ON a.id = av.app_id LEFT JOIN deployment_metadata dm ON av.id = dm.app_version
        WHERE a.workspace_id = $1 AND a.path = $2
        ORDER BY created_at DESC",
        w_id,
        path,
    ).fetch_all(&mut *tx).await?;
    tx.commit().await?;

    let result: Vec<AppHistory> = query_result
        .into_iter()
        .map(|row| AppHistory {
            app_id: row.app_id,
            version: row.version_id,
            deployment_msg: row.deployment_msg,
        })
        .collect();
    return Ok(Json(result));
}

async fn get_latest_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<AppHistory>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT a.id as app_id, av.id as version_id, dm.deployment_msg as deployment_msg
        FROM app a LEFT JOIN app_version av ON a.id = av.app_id LEFT JOIN deployment_metadata dm ON av.id = dm.app_version
        WHERE a.workspace_id = $1 AND a.path = $2
        ORDER BY created_at DESC",
        w_id,
        path,
    ).fetch_optional(&mut *tx).await?;
    tx.commit().await?;

    if let Some(row) = row {
        let result = AppHistory {
            app_id: row.app_id,
            version: row.version_id,
            deployment_msg: row.deployment_msg,
        };

        return Ok(Json(Some(result)));
    } else {
        return Ok(Json(None));
    }
}

async fn update_app_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, app_id, app_version)): Path<(String, i64, i64)>,
    Json(app_history_update): Json<AppHistoryUpdate>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    let app_path = sqlx::query_scalar!("SELECT path FROM app WHERE id = $1", app_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(app_path) = app_path else {
        tx.commit().await?;
        return Err(Error::NotFound(
            format!("App with ID {app_id} not found").to_string(),
        ));
    };

    check_scopes(&authed, || format!("apps:write:{}", &app_path))?;

    sqlx::query!(
        "INSERT INTO deployment_metadata (workspace_id, path, app_version, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path, app_version) WHERE app_version IS NOT NULL DO UPDATE SET deployment_msg = EXCLUDED.deployment_msg",
        w_id,
        app_path,
        app_version,
        app_history_update.deployment_msg,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(());
}

async fn custom_path_exists(
    Extension(db): Extension<DB>,
    Path((w_id, custom_path)): Path<(String, String)>,
) -> JsonResult<bool> {
    let as_workspaced_route = *APP_WORKSPACED_ROUTE.read().await;

    let exists =
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM app WHERE custom_path = $1 AND ($2::TEXT IS NULL OR workspace_id = $2))",
            custom_path,
            if *CLOUD_HOSTED || as_workspaced_route { Some(&w_id) } else { None }
        )
        .fetch_one(&db)
        .await?.unwrap_or(false);
    Ok(Json(exists))
}

async fn get_app_by_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> JsonResult<AppWithLastVersion> {
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by, app_version.raw_app
        FROM app, app_version 
        WHERE app_version.id = $1 AND app.id = app_version.app_id AND app.workspace_id = $2",
    )
    .bind(&id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", id.to_string())?;

    check_scopes(&authed, || format!("apps:read:{}", &app.path))?;

    Ok(Json(app))
}

async fn get_public_app_by_secret(
    OptAuthed(opt_authed): OptAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, secret)): Path<(String, String)>,
) -> JsonResult<AppWithLastVersion> {
    let id = get_id_from_secret(&db, &w_id, secret, None).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
        null as extra_perms, coalesce(app_version_lite.value::json, app_version.value::json) as value,
        app_version.created_at, app_version.created_by, app_version.raw_app
        FROM app, app_version
        LEFT JOIN app_version_lite ON app_version_lite.id = app_version.id
        WHERE app.id = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]")
        .bind(&id)
        .bind(&w_id)
    .fetch_optional(&db)
    .await?;

    let mut app = not_found_if_none(app_o, "App", id.to_string())?;

    let policy = serde_json::from_str::<Policy>(app.policy.0.get()).map_err(to_anyhow)?;

    if !matches!(policy.execution_mode, ExecutionMode::Anonymous) {
        if opt_authed.is_none() {
            return Err(Error::NotAuthorized(
                "App visibility does not allow public access and you are not logged in".to_string(),
            ));
        } else {
            let authed = opt_authed.unwrap();
            let mut tx = user_db.begin(&authed).await?;
            let is_visible = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM app WHERE id = $1 AND workspace_id = $2)",
                id,
                &w_id
            )
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            if !is_visible.unwrap_or(false) {
                return Err(Error::NotAuthorized(
                    "App visibility does not allow public access and you are logged in but you have no read-access to that app".to_string(),
                ));
            }
        }
    }

    // Compute bundle_secret for raw apps
    if app.raw_app {
        app.bundle_secret = Some(compute_bundle_secret(&db, &w_id, &app.versions).await?);
    }

    Ok(Json(app))
}

async fn get_id_from_secret(
    db: &DB,
    w_id: &str,
    secret: String,
    prefix: Option<&str>,
) -> Result<i64> {
    let mc = build_crypt(db, w_id).await?;
    let decrypted = mc
        .decrypt_bytes_to_bytes(&(hex::decode(secret)?))
        .map_err(|e| Error::internal_err(e.to_string()))?;
    let mut bytes = str::from_utf8(&decrypted).map_err(to_anyhow)?;
    if let Some(prefix) = prefix {
        if !bytes.starts_with(prefix) {
            return Err(Error::BadRequest("Invalid secret".to_string()));
        }
        bytes = bytes.strip_prefix(prefix).unwrap_or("");
    }
    let id: i64 = bytes.parse().map_err(to_anyhow)?;
    Ok(id)
}

async fn get_public_resource(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();

    let res = if path.starts_with("f/app_themes/") {
        sqlx::query_scalar!(
            "SELECT value from resource WHERE path = $1 AND workspace_id = $2",
            path.to_owned(),
            &w_id
        )
        .fetch_optional(&db)
        .await?
    } else {
        sqlx::query_scalar!(
            "SELECT value from resource WHERE path = $1 AND workspace_id = $2 AND resource_type = 'json_schema'",
            path.to_owned(),
            &w_id
        )
        .fetch_optional(&db)
        .await?
    };

    Ok(Json(res.flatten()))
}

async fn get_secret_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let id_o = sqlx::query_scalar!(
        "SELECT app.id FROM app
        WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let id = not_found_if_none(id_o, "App", path.to_string())?;

    let mc = build_crypt(&db, &w_id).await?;

    let hx = hex::encode(mc.encrypt_str_to_bytes(id.to_string()));

    Ok(hx)
}

const BUNDLE_SECRET_PREFIX: &str = "bundle_";

pub async fn compute_bundle_secret(db: &DB, w_id: &str, versions: &[i64]) -> Result<String> {
    let version_id = versions
        .last()
        .ok_or_else(|| Error::internal_err("App has no versions".to_string()))?;
    let mc = build_crypt(db, w_id).await?;
    let hx =
        hex::encode(mc.encrypt_str_to_bytes(format!("{}{}", BUNDLE_SECRET_PREFIX, version_id)));
    Ok(hx)
}

async fn get_latest_version_secret_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let id_o = sqlx::query_scalar!(
        "SELECT app.versions[array_upper(app.versions, 1)] FROM app
        WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    tx.commit().await?;

    let id = not_found_if_none(id_o, "App", path.to_string())?;

    let mc = build_crypt(&db, &w_id).await?;

    let hx = hex::encode(mc.encrypt_str_to_bytes(format!("{}{}", BUNDLE_SECRET_PREFIX, id)));

    Ok(hx)
}

async fn store_raw_app_file<'a>(
    w_id: &str,
    id: &i64,
    file_type: &str,
    data: bytes::Bytes,
    tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
) -> Result<()> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    {
        let object_store = windmill_object_store::get_object_store().await;

        let path: String = format!("/app_bundles/{}/{}.{}", w_id, id, file_type);

        if let Some(os) = object_store {
            if let Err(e) = os
                .put(
                    &windmill_object_store::object_store_reexports::Path::from(path.clone()),
                    data.into(),
                )
                .await
            {
                tracing::error!("Failed to put snapshot to s3 at {path}: {:?}", e);
                return Err(windmill_common::error::Error::ExecutionErr(format!(
                    "Failed to put {path} to s3"
                )));
            }
            tracing::info!("Successfully put snapshot to s3 at {path}");
            return Ok(());
        }
    }

    sqlx::query!(
        "INSERT INTO app_bundles (app_version_id, w_id, file_type, data) VALUES ($1, $2, $3, $4)",
        id,
        w_id,
        file_type,
        data.to_vec()
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
macro_rules! process_app_multipart {
    ($authed:expr, $user_db:expr, $db:expr, $w_id:expr, $path:expr, $multipart:expr, $internal_fn:expr) => {
        async {
            let mut saved_app = None;
            let mut uploaded_js = false;

            let mut multipart = $multipart;
            while let Some(field) = multipart.next_field().await.unwrap() {
                let name = field.name().unwrap().to_string();
                let data = field.bytes().await.unwrap();
                if name == "app" {
                    let app = serde_json::from_slice(&data).map_err(to_anyhow)?;
                    let (ntx, npath, nid) = $internal_fn(
                        $authed.clone(),
                        $db.clone(),
                        $user_db.clone(),
                        $w_id,
                        $path,
                        true,
                        app,
                    )
                    .await?;
                    saved_app = Some((npath, nid, ntx));
                } else if name == "js" {
                    if let Some((_npath, id, tx)) = saved_app.as_mut() {
                        store_raw_app_file($w_id, &id, "js", data, tx).await?;
                        uploaded_js = true;
                    } else {
                        return Err(Error::BadRequest(
                            "App payload need to be created first".to_string(),
                        ));
                    }
                } else if name == "css" {
                    if let Some((_npath, id, tx)) = saved_app.as_mut() {
                        store_raw_app_file($w_id, &id, "css", data, tx).await?;
                    } else {
                        return Err(Error::BadRequest(
                            "App payload need to be created first".to_string(),
                        ));
                    }
                } else {
                    return Err(Error::BadRequest(format!("Unsupported field: {}", name)));
                }
            }
            if !uploaded_js {
                return Err(Error::BadRequest("js or css file not uploaded".to_string()));
            }
            if let Some((npath, id, tx)) = saved_app {
                tx.commit().await?;
                Ok((npath, id))
            } else {
                Err(Error::BadRequest("App not created".to_string()))
            }
        }
    };
}

async fn create_app_raw<'a>(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    multipart: Multipart,
) -> Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create apps for security reasons".to_string(),
        ));
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

    let (path, _id) = process_app_multipart!(
        authed,
        user_db,
        db,
        &w_id,
        "",
        multipart,
        |authed, db, user_db, w_id, _path, raw_app, app| create_app_internal(
            authed, db, user_db, w_id, raw_app, app
        )
    )
    .await?;

    check_scopes(&authed, || format!("apps:write:{}", path))?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: path.clone() },
    );
    Ok((StatusCode::CREATED, path))
}

async fn list_paths_from_workspace_runnable(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;
    let runnables = sqlx::query_scalar!(
        r#"SELECT a.path
            FROM workspace_runnable_dependencies wru 
            JOIN app a
                ON wru.app_path = a.path AND wru.workspace_id = a.workspace_id
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

async fn create_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(app): Json<CreateApp>,
) -> Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create apps for security reasons".to_string(),
        ));
    }
    let path = app.path.clone();
    check_scopes(&authed, || format!("apps:write:{}", &path))?;

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

    let (new_tx, _path, _id) = create_app_internal(authed, db, user_db, &w_id, false, app).await?;

    new_tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: path.clone() },
    );

    Ok((StatusCode::CREATED, path))
}

async fn create_app_internal<'a>(
    authed: ApiAuthed,
    db: sqlx::Pool<sqlx::Postgres>,
    user_db: UserDB,
    w_id: &String,
    raw_app: bool,
    mut app: CreateApp,
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, String, i64)> {
    if *CLOUD_HOSTED {
        let nb_apps =
            sqlx::query_scalar!("SELECT COUNT(*) FROM app WHERE workspace_id = $1", &w_id)
                .fetch_one(&db)
                .await?;
        if nb_apps.unwrap_or(0) >= 1000 {
            return Err(Error::BadRequest(
                    "You have reached the maximum number of apps (1000) on cloud. Contact support@windmill.dev to increase the limit"
                        .to_string(),
                ));
        }
        if app.summary.len() > 300 {
            return Err(Error::BadRequest(
                "Summary must be less than 300 characters on cloud".to_string(),
            ));
        }
    }
    let mut tx = user_db.clone().begin(&authed).await?;
    let should_preserve = app.preserve_on_behalf_of.unwrap_or(false)
        && windmill_common::can_preserve_on_behalf_of(&authed)
        && app.policy.on_behalf_of.is_some();

    if !should_preserve {
        app.policy.on_behalf_of = Some(username_to_permissioned_as(&authed.username));
        app.policy.on_behalf_of_email = Some(authed.email.clone());
    }
    let path = app.path.clone();
    if &app.path == "" {
        return Err(Error::BadRequest("App path cannot be empty".to_string()));
    }
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        &app.path,
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
    if let Some(custom_path) = &app.custom_path {
        require_admin(authed.is_admin, &authed.username)?;
        let as_workspaced_route = *APP_WORKSPACED_ROUTE.read().await;

        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM app WHERE custom_path = $1 AND ($2::TEXT IS NULL OR workspace_id = $2))",
            custom_path,
            if *CLOUD_HOSTED || as_workspaced_route { Some(w_id) } else { None }
        )
        .fetch_one(&mut *tx)
        .await?.unwrap_or(false);

        if exists {
            return Err(Error::BadRequest(format!(
                "App with custom path {} already exists",
                custom_path
            )));
        }
    }
    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'app'",
        &app.path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;
    let id = sqlx::query_scalar!(
        "INSERT INTO app
            (workspace_id, path, summary, policy, versions, draft_only, custom_path)
            VALUES ($1, $2, $3, $4, '{}', $5, $6) RETURNING id",
        w_id,
        app.path,
        app.summary,
        json!(app.policy),
        app.draft_only,
        app.custom_path
            .as_ref()
            .map(|s| if s.is_empty() { None } else { Some(s) })
            .flatten()
    )
    .fetch_one(&mut *tx)
    .await?;
    let v_id = sqlx::query_scalar!(
        "INSERT INTO app_version
            (app_id, value, created_by, raw_app)
            VALUES ($1, $2::text::json, $3, $4) RETURNING id",
        id,
        //to preserve key orders
        serde_json::to_string(&app.value).unwrap(),
        authed.username,
        raw_app
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE id = $2",
        v_id,
        id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "apps.create",
        ActionKind::Create,
        w_id,
        Some(&app.path),
        None,
    )
    .await?;
    if should_preserve {
        if let Some(ref obo_email) = app.policy.on_behalf_of_email {
            if obo_email != &authed.email {
                audit_log(
                    &mut *tx,
                    &authed,
                    "apps.on_behalf_of",
                    ActionKind::Create,
                    w_id,
                    Some(&app.path),
                    Some([("on_behalf_of", obo_email.as_str()), ("action", "create")].into()),
                )
                .await?;
            }
        }
    }
    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = &app.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }
    let tx = PushIsolationLevel::Transaction(tx);
    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        w_id,
        JobPayload::AppDependencies {
            path: app.path.clone(),
            version: v_id,
            debouncing_settings: Default::default(),
        },
        PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        None,
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
        false,
        None,
        None,
        None,
    )
    .await?;
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);

    Ok((new_tx, path, v_id))
}

async fn list_hub_apps(Extension(db): Extension<DB>) -> impl IntoResponse {
    let (status_code, headers, body) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/searchUiData?approved=true", *HUB_BASE_URL.read().await),
        None,
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, body))
}

pub async fn get_hub_app_by_id(
    Path(id): Path<i32>,
    Extension(db): Extension<DB>,
) -> JsonResult<Box<serde_json::value::RawValue>> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("{}/apps/{}/json", *HUB_BASE_URL.read().await, id),
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

pub async fn get_hub_raw_app_by_id(
    Path(id): Path<i32>,
    Extension(db): Extension<DB>,
) -> JsonResult<Box<serde_json::value::RawValue>> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("{}/raw_apps/{}/json", *HUB_BASE_URL.read().await, id),
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

async fn delete_app(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;

    if path == "g/all/setup_app" && w_id == "admins" {
        return Err(Error::BadRequest(
            "Cannot delete the global setup app".to_string(),
        ));
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

    // Check if it's a raw app before deletion
    let is_raw_app = sqlx::query_scalar!(
        "SELECT app_version.raw_app FROM app
         JOIN app_version ON app_version.id = app.versions[array_upper(app.versions, 1)]
         WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(false);

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'app'",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM app WHERE path = $1 AND workspace_id = $2",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "apps.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    let deployed_object = if is_raw_app {
        DeployedObject::RawApp {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        }
    } else {
        DeployedObject::App {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        }
    };

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        deployed_object,
        Some(format!("App '{}' deleted", path)),
        true,
        None,
    )
    .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE path = $1 AND workspace_id = $2 AND app_version IS NOT NULL",
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
        w_id.clone().clone(),
        WebhookMessage::DeleteApp { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("app {} deleted", path))
}

async fn update_app(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditApp>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot update apps for security reasons".to_string(),
        ));
    }
    // create_app_internal(authed, user_db, db, &w_id, &mut app).await?;
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;

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

    let opath = path.to_string();
    let (new_tx, npath, _v_id) =
        update_app_internal(authed, db, user_db, &w_id, path, false, ns).await?;
    new_tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateApp {
            workspace: w_id.clone(),
            old_path: opath.clone(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("app {} updated (npath: {:?})", opath, npath))
}

async fn update_app_raw<'a>(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    multipart: Multipart,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot update apps for security reasons".to_string(),
        ));
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

    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;
    let opath = path.to_string();
    let (npath, _id) = process_app_multipart!(
        authed,
        user_db,
        db,
        &w_id,
        path,
        multipart,
        update_app_internal
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateApp {
            workspace: w_id.clone(),
            old_path: opath.to_owned(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("app {} updated (npath: {:?})", opath, npath))
}
// async fn create_app_internal<'a>(
//     authed: ApiAuthed,
//     db: sqlx::Pool<sqlx::Postgres>,
//     user_db: UserDB,
//     w_id: &String,
//     app: &mut CreateApp,
// )

async fn update_app_internal<'a>(
    authed: ApiAuthed,
    db: sqlx::Pool<sqlx::Postgres>,
    user_db: UserDB,
    w_id: &str,
    path: &str,
    raw_app: bool,
    ns: EditApp,
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, String, i64)> {
    use sql_builder::prelude::*;
    let mut tx = user_db.clone().begin(&authed).await?;

    let mut preserved_on_behalf_of: Option<String> = None;
    let npath = if ns.policy.is_some()
        || ns.path.is_some()
        || ns.summary.is_some()
        || ns.custom_path.is_some()
    {
        let mut sqlb = SqlBuilder::update_table("app");
        sqlb.and_where_eq("path", "?".bind(&path));
        sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

        sqlb.set("draft_only", "NULL");
        if let Some(npath) = &ns.path {
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

        if let Some(nsummary) = &ns.summary {
            sqlb.set_str("summary", nsummary);
        }

        if let Some(ncustom_path) = &ns.custom_path {
            require_admin(authed.is_admin, &authed.username)?;
            let as_workspaced_route = *APP_WORKSPACED_ROUTE.read().await;

            if ncustom_path.is_empty() {
                sqlb.set("custom_path", "NULL");
            } else {
                let exists = sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM app WHERE custom_path = $1 AND ($2::TEXT IS NULL OR workspace_id = $2) AND NOT (path = $3 AND workspace_id = $4))",
                    ncustom_path,
                    if *CLOUD_HOSTED || as_workspaced_route { Some(w_id) } else { None },
                    path,
                    w_id
                )
                .fetch_one(&mut *tx)
                .await?.unwrap_or(false);

                if exists {
                    return Err(Error::BadRequest(format!(
                        "App with custom path {} already exists",
                        ncustom_path
                    )));
                }
                sqlb.set_str("custom_path", ncustom_path);
            }
        }

        if let Some(mut npolicy) = ns.policy {
            let should_preserve = ns.preserve_on_behalf_of.unwrap_or(false)
                && windmill_common::can_preserve_on_behalf_of(&authed)
                && npolicy.on_behalf_of.is_some();

            if should_preserve {
                if let Some(ref obo_email) = npolicy.on_behalf_of_email {
                    if obo_email != &authed.email {
                        preserved_on_behalf_of = Some(obo_email.clone());
                    }
                }
            } else {
                npolicy.on_behalf_of = Some(username_to_permissioned_as(&authed.username));
                npolicy.on_behalf_of_email = Some(authed.email.clone());
            }
            sqlb.set(
                "policy",
                quote(serde_json::to_string(&json!(npolicy)).map_err(|e| {
                    Error::BadRequest(format!("failed to serialize policy: {}", e))
                })?),
            );
        }

        sqlb.returning("path");

        let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
        let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut *tx).await?;
        not_found_if_none(npath_o, "App", path)?
    } else {
        path.to_owned()
    };
    let v_id = if let Some(nvalue) = &ns.value {
        let app_id = sqlx::query_scalar!(
            "SELECT id FROM app WHERE path = $1 AND workspace_id = $2",
            npath,
            w_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let v_id = sqlx::query_scalar!(
            "INSERT INTO app_version
                (app_id, value, created_by, raw_app)
                VALUES ($1, $2::text::json, $3, $4) RETURNING id",
            app_id,
            //to preserve key orders
            serde_json::to_string(&nvalue).unwrap(),
            authed.username,
            raw_app
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE path = $2 AND workspace_id = $3",
            v_id,
            npath,
            w_id
        )
        .execute(&mut *tx)
        .await?;
        v_id
    } else {
        let v_id = sqlx::query_scalar!(
            "SELECT  app.versions[array_upper(app.versions, 1)] FROM app WHERE path = $1 AND workspace_id = $2",
            npath,
            w_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if let Some(v_id) = v_id {
            v_id
        } else {
            return Err(Error::BadRequest(format!(
                "App with path {} not found",
                npath
            )));
        }
    };
    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'app'",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "apps.update",
        ActionKind::Update,
        w_id,
        Some(&npath),
        None,
    )
    .await?;
    if let Some(on_behalf_of) = preserved_on_behalf_of {
        audit_log(
            &mut *tx,
            &authed,
            "apps.on_behalf_of",
            ActionKind::Update,
            w_id,
            Some(&npath),
            Some(
                [
                    ("on_behalf_of", on_behalf_of.as_str()),
                    ("action", "update"),
                ]
                .into(),
            ),
        )
        .await?;
    }
    let tx = PushIsolationLevel::Transaction(tx);
    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = ns.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }
    args.insert("parent_path".to_string(), to_raw_value(&path));
    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        w_id,
        JobPayload::AppDependencies {
            path: npath.clone(),
            version: v_id,
            debouncing_settings: Default::default(),
        },
        PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        None,
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
        false,
        None,
        None,
        None,
    )
    .await?;
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);
    Ok((new_tx, npath, v_id))
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecuteApp {
    /// The app version to execute. Fallback to `path` if not provided.
    pub version: Option<i64>,
    /// The app script id (from the `app_script` table) to execute.
    pub id: Option<i64>,
    pub args: HashMap<String, Box<RawValue>>,
    // - script: script/<path>
    // - flow: flow/<path>
    pub path: Option<String>,
    pub component: String,
    pub raw_code: Option<RawCode>,
    // if set, the app is executed as viewer with the given static fields
    pub force_viewer_static_fields: Option<StaticFields>,
    pub force_viewer_one_of_fields: Option<OneOfFields>,
    pub force_viewer_allow_user_resources: Option<AllowUserResources>,
    /// Runnable query parameters (e.g., memory_id for chat-enabled flows)
    pub run_query_params: Option<RunJobQuery>,
}

fn digest(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code);
    let result = hasher.finalize();
    format!("rawscript/{:x}", result)
}

async fn get_on_behalf_details_from_policy_and_authed(
    policy: &Policy,
    opt_authed: &Option<ApiAuthed>,
) -> Result<(String, String, String)> {
    let (username, permissioned_as, email) = match policy.execution_mode {
        ExecutionMode::Anonymous => {
            let username = opt_authed
                .as_ref()
                .map(|a| a.username.clone())
                .unwrap_or_else(|| "anonymous".to_string());
            let (permissioned_as, email) = get_on_behalf_of(&policy)?;
            (username, permissioned_as, email)
        }
        ExecutionMode::Publisher => {
            let username = opt_authed
                .as_ref()
                .map(|a| a.username.clone())
                .ok_or_else(|| {
                    Error::BadRequest(
                        "publisher execution mode requires authentication".to_string(),
                    )
                })?;
            let (permissioned_as, email) = get_on_behalf_of(&policy)?;
            (username, permissioned_as, email)
        }
        ExecutionMode::Viewer => {
            let (username, email) = opt_authed
                .as_ref()
                .map(|a| (a.username.clone(), a.email.clone()))
                .ok_or_else(|| {
                    Error::BadRequest("Required to be authed in viewer mode".to_string())
                })?;
            (
                username.clone(),
                username_to_permissioned_as(&username),
                email,
            )
        }
    };

    Ok((username, permissioned_as, email))
}

/// Convert the triggerables from the old format to the new format.
fn empty_triggerables(mut policy: Policy) -> Policy {
    use std::mem::take;
    if let Some(triggerables) = take(&mut policy.triggerables) {
        let mut triggerables_v2 = take(&mut policy.triggerables_v2).unwrap_or_default();
        for (k, static_inputs) in triggerables.into_iter() {
            triggerables_v2.insert(
                k,
                PolicyTriggerableInputs { static_inputs, ..Default::default() },
            );
        }
        policy.triggerables_v2 = Some(triggerables_v2);
    }
    policy
}

async fn execute_component(
    OptAuthed(opt_authed): OptAuthed,
    tokened: OptTokened,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<ExecuteApp>,
) -> Result<String> {
    match (payload.path.is_some(), payload.raw_code.is_some()) {
        (false, false) => {
            return Err(Error::BadRequest(
                "path or raw_code is required".to_string(),
            ))
        }
        (true, true) => {
            return Err(Error::BadRequest(
                "path and raw_code cannot be set at the same time".to_string(),
            ))
        }
        _ => {}
    };

    let path = path.to_path();
    let (arc_policy, policy): (Arc<Policy>, Policy);
    let policy_triggerables_default = Default::default();

    // Two cases here:
    // 1. The component is executed from the editor (i.e. in "preview" mode), then:
    //    - The policy is set to default (in `Viewer` execution mode).
    //    - The policy triggerables are built by the frontend and retrieved from the request
    //      payload.
    //    - In case of inline script, the `RawCode` from the request is pushed as is to the
    //      job queue.
    // 2. Otherwise (i.e. "run" mode):
    //    - The policy and triggerables are fetched from the database.
    //    - In case of inline script, if an entry exists in the `app_script` table, push
    //      an `AppScript` job payload, as in (.1) otherwise.
    let (policy, policy_triggerables) = match payload {
        // 1. "preview" mode.
        ExecuteApp {
            force_viewer_static_fields: Some(static_inputs),
            force_viewer_one_of_fields,
            force_viewer_allow_user_resources,
            ..
        } => (
            &Policy { execution_mode: ExecutionMode::Viewer, ..Default::default() },
            &PolicyTriggerableInputs {
                static_inputs,
                one_of_inputs: force_viewer_one_of_fields.unwrap_or_default(),
                allow_user_resources: force_viewer_allow_user_resources.unwrap_or_default(),
            },
        ),
        // 2. "run" mode.
        _ => {
            // Policy is fetched from the database on app `path` and `workspace_id`.
            let policy_fut = sqlx::query_scalar!(
                "SELECT policy as \"policy: sqlx::types::Json<Box<RawValue>>\"
                FROM app WHERE app.path = $1 AND app.workspace_id = $2 LIMIT 1",
                path,
                &w_id,
            )
            .fetch_optional(&db)
            .map_err(Into::<Error>::into)
            .map(|policy_o| Result::Ok(not_found_if_none(policy_o?, "App", path)?))
            .map(|policy| Result::Ok(serde_json::from_str(policy?.get())?))
            .map_ok(empty_triggerables);

            // 1. The app `version` is provided: cache the fetched policy.
            // 2. Otherwise, always fetch the policy from the database.
            let policy = if let Some(id) = payload.version {
                let cache = cache::anon!({ u64 => Arc<Policy> } in "policy" <= 1000);
                arc_policy = policy_fut.map_ok(Arc::new).cached(cache, id as u64).await?;
                &*arc_policy
            } else {
                policy = policy_fut.await?;
                &policy
            };

            // Compute the path for the triggerables map:
            // - flow: `flow/<payload.path>`
            // - script: `script/<payload.path>`
            // - inline script: `rawscript/<sha256(raw_code.content)>`
            let path = match &payload {
                // flow or script: just use the `payload.path`.
                ExecuteApp { path: Some(path), .. } => path,
                // inline script: without entry in the `app_script` table.
                ExecuteApp { raw_code: Some(raw_code), id: None, .. } => &digest(&raw_code.content),
                // inline script: with an entry in the `app_script` table.
                ExecuteApp { raw_code: Some(_), id: Some(id), .. } => {
                    let cache = cache::anon!({ u64 => Arc<String> } in "appscriptpath" <= 10000);
                    // `id` is unique, cache the result.
                    &*sqlx::query_scalar!(
                        "SELECT format('rawscript/%s', code_sha256) as \"path!: String\"
                        FROM app_script WHERE id = $1 LIMIT 1",
                        id
                    )
                    .fetch_one(&db)
                    .map_err(Into::<Error>::into)
                    .map_ok(Arc::new)
                    .cached(cache, *id as u64)
                    .await?
                }
                _ => unreachable!(),
            };

            // Retrieve the triggerables from the policy on `path` or `<component>:<path>`.
            let triggerables_v2 = policy
                .triggerables_v2
                .as_ref()
                .ok_or_else(|| Error::BadRequest(format!("Policy is missing triggerables")))?;
            let policy_triggerables = triggerables_v2
                .get(path) // start with `path` in case we can avoid the next` format!`.
                .or_else(|| triggerables_v2.get(&format!("{}:{}", payload.component, &path)))
                .or(match policy.execution_mode {
                    ExecutionMode::Viewer => Some(&policy_triggerables_default),
                    _ => None,
                })
                .ok_or_else(|| Error::BadRequest(format!("Path {path} forbidden by policy")))?;

            (policy, policy_triggerables)
        }
    };

    // Check rate limit for anonymous (public) executions
    if matches!(policy.execution_mode, ExecutionMode::Anonymous) && opt_authed.is_none() {
        if let Some(limit) = crate::workspaces::get_public_app_rate_limit(&db, &w_id).await? {
            if limit > 0 {
                crate::public_app_rate_limit::check_and_increment(&w_id, limit)?;
            }
        }
    }

    // Execution is publisher and an user is authenticated: check if the user is authorized to
    // execute the app.
    if let (ExecutionMode::Publisher, Some(authed)) = (policy.execution_mode, opt_authed.as_ref()) {
        lazy_static! {
            /// Cache for the permit to execute an app component.
            static ref PERMIT_CACHE: cache::Cache<[u8; 32], bool> = cache::Cache::new(1000);
        }

        // Avoid allocation for the permit key using a sha256 hash of:
        // - the user email,
        // - the application path,
        // - the workspace id.
        let permit_key: [u8; 32] = [authed.email.as_bytes(), path.as_bytes(), &w_id.as_bytes()]
            .iter()
            .fold(Sha256::new(), |hasher, bytes| hasher.chain_update(bytes))
            .finalize()
            .into();
        let permit_fut = PERMIT_CACHE.get_or_insert_async(&permit_key, async {
            let mut tx = user_db.clone().begin(authed).await?;
            // Permissions are checked by the database; just fetch a row from app using `user_db`:
            let row = sqlx::query_scalar!(
                "SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2 LIMIT 1",
                path,
                &w_id,
            )
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;
            Result::Ok(row.is_some_and(|x| x.is_some()))
        });

        if !permit_fut.await? {
            return Err(Error::NotAuthorized(format!(
                "Missing read permissions on the `{}` app to execute `{}` runnable",
                path, payload.component
            )));
        }
    }

    let (username, permissioned_as, email) =
        get_on_behalf_details_from_policy_and_authed(&policy, &opt_authed).await?;

    let (args, job_id) = build_args(
        policy,
        policy_triggerables,
        payload.args,
        opt_authed.as_ref(),
        &user_db,
        &db,
        &w_id,
    )
    .await?;

    let is_flow = payload
        .path
        .as_ref()
        .map(|p| p.starts_with("flow/"))
        .unwrap_or(false);

    let (job_payload, tag, on_behalf_of) = match (payload.path, payload.raw_code, payload.id) {
        // flow or script:
        (Some(path), None, None) => get_payload_tag_from_prefixed_path(&path, &db, &w_id).await?,
        // inline script: in "preview" mode or without entry in the `app_script` table.
        (None, Some(raw_code), None) => (JobPayload::Code(raw_code), None, None),
        // inline script: in "run" mode and with an entry in the `app_script` table.
        (None, Some(RawCode { language, path, cache_ttl, .. }), Some(id)) => (
            JobPayload::AppScript { id: AppScriptId(id), cache_ttl, language, path },
            None,
            None,
        ),
        _ => unreachable!(),
    };
    let tx = PushIsolationLevel::IsolatedRoot(db.clone());

    let (email, permissioned_as) = if let Some(on_behalf_of) = on_behalf_of.as_ref() {
        (
            on_behalf_of.email.as_str(),
            on_behalf_of.permissioned_as.clone(),
        )
    } else {
        (email.as_str(), permissioned_as)
    };

    let end_user_email = opt_authed.as_ref().map(|a| a.email.clone());

    let (uuid, mut tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        &username,
        email,
        permissioned_as,
        opt_authed
            .and_then(|a| a.token_prefix)
            .or_else(|| tokened.token.map(|t| t[0..TOKEN_PREFIX_LEN].to_string()))
            .as_deref(),
        None,
        None,
        None,
        None,
        None,
        job_id,
        false,
        false,
        None,
        true,
        tag,
        None,
        None,
        None,
        None,
        false,
        end_user_email,
        None,
        None,
    )
    .await?;

    // Apply runnable query parameters if provided
    if let Some(ref run_query) = payload.run_query_params {
        if is_flow {
            crate::jobs::process_flow_run_query_params(&mut tx, uuid, run_query).await?;
        }
    }

    tx.commit().await?;

    Ok(uuid.to_string())
}

#[cfg(not(feature = "parquet"))]
async fn upload_s3_file_from_app() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(not(feature = "parquet"))]
async fn delete_s3_file_from_app() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(feature = "parquet")]
#[derive(Debug, Deserialize, Clone)]
struct UploadFileToS3Query {
    file_key: Option<String>,
    file_extension: Option<String>,
    s3_resource_path: Option<String>,
    content_type: Option<String>,
    content_disposition: Option<String>,
    force_viewer_file_key_regex: Option<String>,
    force_viewer_allow_user_resources: Option<bool>,
    force_viewer_allow_workspace_resource: Option<bool>,
    force_viewer_allowed_resources: Option<String>,
}

#[cfg(feature = "parquet")]
#[derive(Serialize, Deserialize)]
struct S3DeleteTokenClaims {
    file_key: String,
    on_behalf_of_email: String,
    permissioned_as: String,
    username: String,
    s3_resource_path: Option<String>,
    workspace: String,
    pub exp: usize,
}

#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct S3TokenRequestBody {
    s3_objects: Vec<S3Object>,
}
#[cfg(feature = "parquet")]
async fn sign_s3_objects(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<S3TokenRequestBody>,
) -> Result<Json<Vec<S3Object>>> {
    let workspace_key = get_workspace_key(&w_id, &db).await?;

    let futures = body.s3_objects.into_iter().map(|s3_object| async {
        let exp = (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp();
        let mut message = format!("file_key={}&exp={}", s3_object.s3.clone(), exp);
        if let Some(ref storage) = s3_object.storage {
            message = format!("{}&storage={}", message, storage);
        }

        let mut max = HmacSha256::new_from_slice(workspace_key.as_bytes())
            .map_err(|err| Error::internal_err(format!("Failed to create hmac: {}", err)))?;
        max.update(message.as_bytes());
        let result = max.finalize();
        let signature = hex::encode(result.into_bytes());

        let presigned = format!("exp={}&sig={}", exp, signature);

        Ok::<_, Error>(S3Object { presigned: Some(presigned), ..s3_object })
    });

    let signed_s3_objects = futures::future::try_join_all(futures).await?;

    Ok(Json(signed_s3_objects))
}

#[cfg(not(feature = "parquet"))]
async fn sign_s3_objects() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(feature = "parquet")]
#[derive(Serialize)]
struct AppUploadFileResponse {
    file_key: String,
    delete_token: String,
}

#[cfg(feature = "parquet")]
async fn upload_s3_file_from_app(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<UploadFileToS3Query>,
    request: axum::extract::Request,
) -> JsonResult<AppUploadFileResponse> {
    let policy = if let Some(file_key_regex) = query.force_viewer_file_key_regex {
        Some(Policy {
            execution_mode: ExecutionMode::Viewer,
            triggerables: None,
            triggerables_v2: None,
            on_behalf_of: None,
            on_behalf_of_email: None,
            s3_inputs: Some(vec![S3Input {
                file_key_regex: file_key_regex,
                allow_user_resources: query.force_viewer_allow_user_resources.unwrap_or(false),
                allow_workspace_resource: query
                    .force_viewer_allow_workspace_resource
                    .unwrap_or(false),
                allowed_resources: query
                    .force_viewer_allowed_resources
                    .map(|s| s.split(',').map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
            }]),
            allowed_s3_keys: None,
        })
    } else {
        let policy_o = sqlx::query_scalar!(
            "SELECT policy from app WHERE path = $1 AND workspace_id = $2",
            &path.0,
            &w_id
        )
        .fetch_optional(&db)
        .await?;

        policy_o
            .map(|p| serde_json::from_value::<Policy>(p).map_err(to_anyhow))
            .transpose()?
    };

    let user_db = UserDB::new(db.clone());

    let (s3_resource_opt, file_key, on_behalf_of_email, permissioned_as, username) = if policy
        .as_ref()
        .is_some_and(|p| p.s3_inputs.is_some())
    {
        let policy = policy.unwrap();
        let s3_inputs = policy.s3_inputs.as_ref().unwrap();

        let (username, permissioned_as, email) =
            get_on_behalf_details_from_policy_and_authed(&policy, &opt_authed).await?;

        let on_behalf_authed = fetch_api_authed_from_permissioned_as(
            permissioned_as.clone(),
            email.clone(),
            &w_id,
            &db,
            Some(username.clone()),
        )
        .await?;

        if let Some(file_key) = query.file_key {
            // file key is provided => requires workspace, user or list policy and must match the regex
            let matching_s3_inputs = if let Some(ref s3_resource_path) = query.s3_resource_path {
                s3_inputs
                    .iter()
                    .filter(|s3_input| {
                        s3_input.allowed_resources.contains(s3_resource_path)
                            || s3_input.allow_user_resources
                    })
                    .sorted_by_key(|i| i.allow_user_resources) // consider user resources last
                    .collect::<Vec<_>>()
            } else {
                s3_inputs
                    .iter()
                    .filter(|s3_input| s3_input.allow_workspace_resource)
                    .collect::<Vec<_>>()
            };

            let matched_input = matching_s3_inputs.iter().find(|s3_input| {
                match Regex::new(&s3_input.file_key_regex) {
                    Ok(re) => re.is_match(&file_key),
                    Err(e) => {
                        tracing::error!("Error compiling regex: {}", e);
                        false
                    }
                }
            });

            if let Some(matched_input) = matched_input {
                if let Some(ref s3_resource_path) = query.s3_resource_path {
                    if matched_input.allow_user_resources {
                        if let Some(authed) = opt_authed {
                            let db_with_opt_authed = DbWithOptAuthed::from_authed(
                                &authed,
                                db.clone(),
                                Some(user_db.clone()),
                            );
                            (
                                Some(
                                    get_s3_resource(
                                        &db_with_opt_authed,
                                        &w_id,
                                        s3_resource_path,
                                        None,
                                        None,
                                    )
                                    .await?,
                                ),
                                file_key,
                                email,
                                permissioned_as,
                                username,
                            )
                        } else {
                            return Err(Error::BadRequest(
                                "User resources are not allowed without being logged in"
                                    .to_string(),
                            ));
                        }
                    } else {
                        let db_with_opt_authed = DbWithOptAuthed::from_authed(
                            &on_behalf_authed,
                            db.clone(),
                            Some(user_db.clone()),
                        );
                        (
                            Some(
                                get_s3_resource(
                                    &db_with_opt_authed,
                                    &w_id,
                                    s3_resource_path,
                                    None,
                                    None,
                                )
                                .await?,
                            ),
                            file_key,
                            email,
                            permissioned_as,
                            username,
                        )
                    }
                } else {
                    let db_with_opt_authed = DbWithOptAuthed::from_authed(
                        &on_behalf_authed,
                        db.clone(),
                        Some(user_db.clone()),
                    );
                    let (_, s3_resource_opt) = get_workspace_s3_resource_and_check_paths(
                        &db_with_opt_authed,
                        Some(&on_behalf_authed),
                        &w_id,
                        None,
                        &[(&file_key, S3Permission::WRITE)],
                        None,
                    )
                    .await?;
                    (s3_resource_opt, file_key, email, permissioned_as, username)
                }
            } else {
                return Err(Error::BadRequest(
                    "No matching s3 resource found for the given file key".to_string(),
                ));
            }
        } else {
            // no file key => requires unnamed upload policy => allow workspace resource and file_key_regex is empty
            let has_unnamed_policy = s3_inputs.iter().any(|s3_input| {
                s3_input.allow_workspace_resource && s3_input.file_key_regex.is_empty()
            });

            if !has_unnamed_policy {
                return Err(Error::BadRequest(
                    "no policy found for unnamed s3 file upload".to_string(),
                ));
            }

            // for now, we place all files into `windmill_uploads` folder with a random name
            // TODO: make the folder configurable via the workspace settings
            let file_key = get_random_file_name(query.file_extension);
            let db_with_opt_authed =
                DbWithOptAuthed::from_authed(&on_behalf_authed, db.clone(), None);
            let (_, s3_resource_opt) = get_workspace_s3_resource_and_check_paths(
                &db_with_opt_authed,
                Some(&on_behalf_authed),
                &w_id,
                None,
                &[(&file_key, S3Permission::WRITE)],
                None,
            )
            .await?;

            (s3_resource_opt, file_key, email, permissioned_as, username)
        }
    } else {
        // backward compatibility (no policy)
        // if no policy but logged in, use the user's auth to get the s3 resource
        if let Some(authed) = opt_authed {
            let file_key = query
                .file_key
                .unwrap_or_else(|| get_random_file_name(query.file_extension));

            let (on_behalf_of_email, permissioned_as, username) = (
                authed.email.clone(),
                username_to_permissioned_as(&authed.username),
                authed.display_username().to_string(),
            );

            if let Some(ref s3_resource_path) = query.s3_resource_path {
                let db_with_opt_authed =
                    DbWithOptAuthed::from_authed(&authed, db.clone(), Some(user_db.clone()));
                (
                    Some(
                        get_s3_resource(&db_with_opt_authed, &w_id, s3_resource_path, None, None)
                            .await?,
                    ),
                    file_key,
                    on_behalf_of_email,
                    permissioned_as,
                    username,
                )
            } else {
                let db_with_opt_authed = DbWithOptAuthed::from_authed(&authed, db.clone(), None);
                let (_, s3_resource) = get_workspace_s3_resource_and_check_paths(
                    &db_with_opt_authed,
                    Some(&authed),
                    &w_id,
                    None,
                    &[(&file_key, S3Permission::WRITE)],
                    None,
                )
                .await?;

                (
                    s3_resource,
                    file_key,
                    on_behalf_of_email,
                    permissioned_as,
                    username,
                )
            }
        } else {
            return Err(Error::BadRequest("Missing s3 policy".to_string()));
        }
    };

    let s3_resource = s3_resource_opt.ok_or(Error::internal_err(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = windmill_object_store::build_object_store_client(&s3_resource).await?;

    let options = Attributes::from_iter(vec![
        (
            Attribute::ContentType,
            query.content_type.unwrap_or_else(|| {
                mime_guess::from_path(&file_key)
                    .first_or_octet_stream()
                    .to_string()
            }),
        ),
        (
            Attribute::ContentDisposition,
            query.content_disposition.unwrap_or("inline".to_string()),
        ),
    ])
    .into();

    let _put_result = upload_file_from_req(s3_client, &file_key, request, options).await?;

    let delete_token = jwt::encode_with_internal_secret(S3DeleteTokenClaims {
        file_key: file_key.clone(),
        on_behalf_of_email,
        permissioned_as,
        username,
        s3_resource_path: query.s3_resource_path,
        workspace: w_id.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp() as usize,
    })
    .await?;

    return Ok(Json(AppUploadFileResponse { file_key, delete_token }));
}

#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct DeleteS3FileQuery {
    delete_token: String,
}

#[cfg(feature = "parquet")]
async fn delete_s3_file_from_app(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<DeleteS3FileQuery>,
) -> Result<()> {
    let S3DeleteTokenClaims {
        file_key,
        on_behalf_of_email,
        permissioned_as,
        username,
        s3_resource_path,
        workspace,
        ..
    } = jwt::decode_with_internal_secret::<S3DeleteTokenClaims>(&query.delete_token).await?;

    let path = windmill_object_store::object_store_reexports::Path::parse(file_key.as_str())
        .map_err(|e| Error::internal_err(format!("Error parsing file key: {}", e)))?;

    if workspace != w_id {
        return Err(Error::BadRequest("Invalid workspace".to_string()));
    }

    let on_behalf_authed = fetch_api_authed_from_permissioned_as(
        permissioned_as,
        on_behalf_of_email,
        &w_id,
        &db,
        Some(username),
    )
    .await?;

    let s3_resource = if let Some(s3_resource_path) = s3_resource_path {
        let db_with_opt_authed =
            DbWithOptAuthed::from_authed(&on_behalf_authed, db.clone(), Some(user_db.clone()));
        get_s3_resource(
            &db_with_opt_authed,
            &w_id,
            s3_resource_path.as_str(),
            None,
            None,
        )
        .await?
    } else {
        let db_with_opt_authed = DbWithOptAuthed::from_authed(&on_behalf_authed, db.clone(), None);
        let (_, s3_resource) = get_workspace_s3_resource_and_check_paths(
            &db_with_opt_authed,
            Some(&on_behalf_authed),
            &w_id,
            None,
            &[(&path.to_string(), S3Permission::DELETE)],
            None,
        )
        .await?;

        s3_resource.ok_or(Error::internal_err(
            "No files storage resource defined at the workspace level".to_string(),
        ))?
    };

    let s3_client = windmill_object_store::build_object_store_client(&s3_resource).await?;

    s3_client.delete(&path).await.map_err(|err| {
        tracing::error!("Error deleting file: {:?}", err);
        Error::internal_err(format!("Error deleting file: {}", err.to_string()))
    })?;

    Ok(())
}

#[cfg(not(feature = "parquet"))]
async fn download_s3_file_from_app() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(feature = "parquet")]
async fn get_on_behalf_authed_from_app(
    db: &DB,
    path: &str,
    w_id: &str,
    opt_authed: &Option<ApiAuthed>,
    force_allowed_s3_keys: Option<Vec<S3Key>>,
) -> Result<(ApiAuthed, Policy)> {
    let policy = if let Some(force_allowed_s3_keys) = force_allowed_s3_keys {
        Policy {
            execution_mode: ExecutionMode::Viewer,
            triggerables: None,
            triggerables_v2: None,
            on_behalf_of: None,
            on_behalf_of_email: None,
            s3_inputs: None,
            allowed_s3_keys: Some(force_allowed_s3_keys),
        }
    } else {
        // TODO: improve db query to not return uneeded fields
        let policy_o = sqlx::query_scalar!(
            "SELECT policy from app WHERE path = $1 AND workspace_id = $2",
            path,
            w_id
        )
        .fetch_optional(db)
        .await?;

        policy_o
            .map(|p| serde_json::from_value::<Policy>(p).map_err(to_anyhow))
            .transpose()?
            .unwrap_or_else(|| Policy {
                execution_mode: ExecutionMode::Viewer,
                triggerables: None,
                triggerables_v2: None,
                on_behalf_of: None,
                on_behalf_of_email: None,
                s3_inputs: None,
                allowed_s3_keys: None,
            })
    };

    let (username, permissioned_as, email) =
        get_on_behalf_details_from_policy_and_authed(&policy, &opt_authed).await?;

    let on_behalf_authed =
        fetch_api_authed_from_permissioned_as(permissioned_as, email, &w_id, &db, Some(username))
            .await?;

    Ok((on_behalf_authed, policy))
}

#[cfg(feature = "parquet")]
async fn check_if_allowed_to_access_s3_file_from_app(
    db: &DB,
    opt_authed: &Option<ApiAuthed>,
    file_query: &AppS3FileQuery,
    w_id: &str,
    path: &str,
    policy: &Policy,
) -> Result<()> {
    // if anonymous, check that the file was the result of an app script ran by an anonymous user in the last 3 hours
    // otherwise, if logged in, allow any file (TODO: change that when we implement better s3 policy)

    if file_query.sig.is_some() {
        #[cfg(feature = "private")]
        {
            crate::s3_proxy_ee::validate_s3_signature(
                &file_query.s3,
                &file_query.sig,
                &file_query.exp,
                &file_query.storage,
                w_id,
                &db,
            )
            .await?;
            Ok(())
        }
        #[cfg(not(feature = "private"))]
        return Err(Error::InternalErr(
            "Internal error: signature validation is not supported in open source mode".to_string(),
        ));
    } else if opt_authed.is_some() {
        Ok(())
    } else {
        let allowed = policy
            .allowed_s3_keys
            .as_ref()
            .unwrap()
            .iter()
            .any(|key| key.s3_path == file_query.s3 && key.storage == file_query.storage)
            || {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS (
                SELECT 1 FROM v2_job_completed c JOIN v2_job j USING (id)
                WHERE j.workspace_id = $2
                    AND (j.kind = 'appscript' OR j.kind = 'preview')
                    AND j.created_by = 'anonymous'
                    AND c.started_at > now() - interval '3 hours'
                    AND j.runnable_path LIKE $3 || '/%'
                    AND c.result @> ('{"s3":"' || $1 ||  '"}')::jsonb
            )"#,
                    file_query.s3,
                    w_id,
                    path,
                )
                .fetch_one(db)
                .await?
                .unwrap_or(false)
            };

        if !allowed {
            Err(Error::BadRequest("File restricted".to_string()))
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "parquet")]
#[derive(Deserialize, Debug)]
struct AppS3FileQuery {
    s3: String,
    storage: Option<String>,
    sig: Option<String>,
    #[cfg(feature = "private")]
    exp: Option<String>,
}

#[cfg(feature = "parquet")]
#[derive(Deserialize, Debug)]
struct AppS3FileQueryWithForceViewerAllowedS3Keys {
    #[serde(flatten)]
    pub file_query: AppS3FileQuery,
    pub force_viewer_allowed_s3_keys: Option<String>,
}

#[cfg(feature = "parquet")]
async fn download_s3_file_from_app(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<AppS3FileQueryWithForceViewerAllowedS3Keys>,
) -> Result<Response> {
    use crate::db::OptJobAuthed;

    let path = path.to_path();

    let force_viewer_allowed_s3_keys = if let Some(force_viewer_allowed_s3_keys) =
        query.force_viewer_allowed_s3_keys.clone()
    {
        Some(serde_json::from_str::<Vec<S3Key>>(&force_viewer_allowed_s3_keys).unwrap_or_default())
    } else {
        None
    };

    let (on_behalf_authed, policy) =
        get_on_behalf_authed_from_app(&db, &path, &w_id, &opt_authed, force_viewer_allowed_s3_keys)
            .await?;

    check_if_allowed_to_access_s3_file_from_app(
        &db,
        &opt_authed,
        &query.file_query,
        &w_id,
        &path,
        &policy,
    )
    .await?;

    download_s3_file_internal(
        OptJobAuthed { authed: on_behalf_authed, job_id: None },
        &db,
        None,
        &w_id,
        DownloadFileQuery {
            file_key: query.file_query.s3,
            s3_resource_path: None,
            storage: query.file_query.storage,
        },
    )
    .await
}

fn get_on_behalf_of(policy: &Policy) -> Result<(String, String)> {
    let permissioned_as = policy
        .on_behalf_of
        .as_ref()
        .ok_or_else(|| {
            Error::BadRequest(
                "on_behalf_of is missing in the app policy and is required for anonymous execution"
                    .to_string(),
            )
        })?
        .to_string();
    let email = policy
        .on_behalf_of_email
        .as_ref()
        .ok_or_else(|| {
            Error::BadRequest(
                "on_behalf_of_email is missing in the app policy and is required for anonymous execution"
                    .to_string(),
            )
        })?
        .to_string();
    Ok((permissioned_as, email))
}

pub async fn require_is_writer(authed: &ApiAuthed, path: &str, w_id: &str, db: DB) -> Result<()> {
    return crate::users::require_is_writer(
        authed,
        path,
        w_id,
        db,
        "SELECT extra_perms FROM app WHERE path = $1 AND workspace_id = $2",
        "app",
    )
    .await;
}

async fn exists_app(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        path.to_path(),
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn build_args(
    policy: &Policy,
    PolicyTriggerableInputs {
        static_inputs,
        one_of_inputs,
        allow_user_resources,
    }: &PolicyTriggerableInputs,
    mut args: HashMap<String, Box<RawValue>>,
    authed: Option<&ApiAuthed>,
    user_db: &UserDB,
    db: &DB,
    w_id: &str,
) -> Result<(PushArgsOwned, Option<Uuid>)> {
    let mut job_id: Option<Uuid> = None;
    let mut safe_args = HashMap::<String, Box<RawValue>>::new();

    // tracing::error!("{:?}", allow_user_resources);
    for k in allow_user_resources.iter() {
        if let Some(arg_val) = args.get(k) {
            let key = serde_json::from_str::<String>(arg_val.get()).ok();
            if let Some(path) =
                key.and_then(|x| x.clone().strip_prefix("$res:").map(|x| x.to_string()))
            {
                if let Some(authed) = authed {
                    let db_with_opt_authed =
                        DbWithOptAuthed::from_authed(authed, db.clone(), Some(user_db.clone()));
                    let res = get_resource_value_interpolated_internal(
                        &db_with_opt_authed,
                        w_id,
                        &path,
                        None,
                        None,
                        false,
                    )
                    .await?;
                    if res.is_none() {
                        return Err(Error::BadRequest(format!(
                            "Resource {} not found or not allowed for viewer",
                            path
                        )));
                    }
                    let job_id = if let Some(job_id) = job_id {
                        job_id
                    } else {
                        job_id = Some(ulid::Ulid::new().into());
                        job_id.unwrap()
                    };
                    let mc = build_crypt_with_key_suffix(&db, &w_id, &job_id.to_string()).await?;
                    let encrypted = encrypt(&mc, to_raw_value(&res.unwrap()).get());
                    safe_args.insert(
                        k.to_string(),
                        to_raw_value(&format!("$encrypted:{encrypted}")),
                    );
                } else {
                    return Err(Error::BadRequest(
                        "User resources are not allowed without being logged in".to_string(),
                    ));
                }
            }
        }
    }

    for (k, v) in one_of_inputs {
        if safe_args.contains_key(k) {
            continue;
        }
        if let Some(arg_val) = args.get(k) {
            let arg_str = arg_val.get();

            let options_str_vec = v.iter().map(|x| x.get()).collect::<Vec<&str>>();
            if options_str_vec.contains(&arg_str) {
                safe_args.insert(k.to_string(), arg_val.clone());
                args.remove(k);
                continue;
            }

            // check if multiselect
            if let Ok(args_str_vec) = serde_json::from_str::<Vec<Box<RawValue>>>(arg_val.get()) {
                if args_str_vec
                    .iter()
                    .all(|x| options_str_vec.contains(&x.get()))
                {
                    safe_args.insert(k.to_string(), arg_val.clone());
                    args.remove(k);
                    continue;
                }
            }

            return Err(Error::BadRequest(format!(
                "argument {} with value {} must be one of [{}]",
                k,
                arg_str,
                options_str_vec.join(",")
            )));
        }
    }

    for (k, v) in args {
        if safe_args.contains_key(&k) {
            continue;
        }

        let arg_str = v.get();

        if arg_str.starts_with("\"$ctx:") {
            let prop = arg_str.trim_start_matches("\"$ctx:").trim_end_matches("\"");
            let value = match prop {
                "username" => authed.as_ref().map(|a| {
                    serde_json::to_value(a.username_override.as_ref().unwrap_or(&a.username))
                }),
                "email" => authed.as_ref().map(|a| serde_json::to_value(&a.email)),
                "workspace" => Some(serde_json::to_value(&w_id)),
                "groups" => authed.as_ref().map(|a| serde_json::to_value(&a.groups)),
                "author" => Some(serde_json::to_value(&policy.on_behalf_of_email)),
                _ => {
                    return Err(Error::BadRequest(format!(
                        "context variable {} not allowed",
                        prop
                    )))
                }
            };
            safe_args.insert(
                k.to_string(),
                to_raw_value(&value.unwrap_or(Ok(serde_json::Value::Null)).map_err(|e| {
                    Error::internal_err(format!(
                        "failed to serialize ctx variable for {}: {}",
                        k, e
                    ))
                })?),
            );
        } else if !arg_str.contains("\"$var:") && !arg_str.contains("\"$res:") {
            safe_args.insert(k.to_string(), v);
        } else {
            safe_args.insert(
                k.to_string(),
                RawValue::from_string(
                    arg_str
                        .replace(
                            "$var:",
                            "The following variable has been omitted for security reasons: ",
                        )
                        .replace(
                            "$res:",
                            "The following resource has been omitted for security reasons, to allow it, toggle: 'Allow resources from users' on that field input: ",
                        ),
                )
                .map_err(|e| {
                    Error::internal_err(format!(
                        "failed to remove sensitive variable(s)/resource(s) with error: {}",
                        e
                    ))
                })?,
            );
        }
    }
    let mut extra = HashMap::new();
    for (k, v) in static_inputs {
        extra.insert(k.to_string(), v.to_owned());
    }
    Ok((
        PushArgsOwned { extra: Some(extra), args: safe_args },
        job_id,
    ))
}
