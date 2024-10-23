use std::collections::HashMap;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use crate::{
    db::{ApiAuthed, DB},
    resources::get_resource_value_interpolated_internal,
    users::{require_owner_of_path, OptAuthed},
    utils::WithStarredInfoQuery,
    variables::encrypt,
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
use axum::{
    extract::{Extension, Json, Path, Query},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use hyper::StatusCode;
use magic_crypt::MagicCryptTrait;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use sha2::{Digest, Sha256};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Uuid, FromRow};
use std::str;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    apps::ListAppQuery,
    db::UserDB,
    error::{to_anyhow, Error, JsonResult, Result},
    jobs::{get_payload_tag_from_prefixed_path, JobPayload, RawCode},
    users::username_to_permissioned_as,
    utils::{
        http_get_from_hub, not_found_if_none, paginate, query_elems_from_hub, Pagination, StripPath,
    },
    variables::{build_crypt, build_crypt_with_key_suffix},
    worker::to_raw_value,
    HUB_BASE_URL,
};

use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, PushArgs, PushArgsOwned, PushIsolationLevel, QueueTransaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_apps))
        .route("/list_search", get(list_search_apps))
        .route("/get/p/*path", get(get_app))
        .route("/get/draft/*path", get(get_app_w_draft))
        .route("/secret_of/*path", get(get_secret_id))
        .route("/get/v/*id", get(get_app_by_id))
        .route("/exists/*path", get(exists_app))
        .route("/update/*path", post(update_app))
        .route("/delete/*path", delete(delete_app))
        .route("/create", post(create_app))
        .route("/history/p/*path", get(get_app_history))
        .route("/get_latest_version/*path", get(get_latest_version))
        .route("/history_update/a/:id/v/:version", post(update_app_history))
}

pub fn unauthed_service() -> Router {
    Router::new()
        .route("/execute_component/*path", post(execute_component))
        .route("/public_app/:secret", get(get_public_app_by_secret))
        .route("/public_resource/*path", get(get_public_resource))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_apps))
        .route("/hub/get/:id", get(get_hub_app_by_id))
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
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct AppVersion {
    pub id: i64,
    pub app_id: Uuid,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, FromRow)]
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
    pub id: i64,
    pub path: String,
    pub summary: String,
    pub policy: sqlx::types::Json<Box<RawValue>>,
    pub versions: Vec<i64>,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: serde_json::Value,
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Anonymous,
    Publisher,
    Viewer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PolicyTriggerableInputs {
    static_inputs: StaticFields,
    one_of_inputs: OneOfFields,
    #[serde(default)]
    allow_user_resources: AllowUserResources,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
}

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub policy: Policy,
    pub draft_only: Option<bool>,
    pub deployment_message: Option<String>,
}

#[derive(Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<sqlx::types::Json<Box<RawValue>>>,
    pub policy: Option<Policy>,
    pub deployment_message: Option<String>,
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
            "draft_only"
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

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableApp>(&sql)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<WithStarredInfoQuery>,
) -> JsonResult<AppWithLastVersionAndStarred> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let app_o = if query.with_starred_info.unwrap_or(false) {
        sqlx::query_as::<_, AppWithLastVersionAndStarred>(
            "SELECT app.id, app.path, app.summary, app.versions, app.policy,
            app.extra_perms, app_version.value, 
            app_version.created_at, app_version.created_by, favorite.path IS NOT NULL as starred
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
            "SELECT app.id, app.path, app.summary, app.versions, app.policy,
            app.extra_perms, app_version.value, 
            app_version.created_at, app_version.created_by, NULL as starred
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

async fn get_app_w_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AppWithLastVersionAndDraft> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersionAndDraft>(
        r#"SELECT app.id, app.path, app.summary, app.versions, app.policy,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by,
        app.draft_only, draft.value as "draft"
        from app
        INNER JOIN app_version ON
        app_version.id = app.versions[array_upper(app.versions, 1)]
        LEFT JOIN draft ON 
        app.path = draft.path AND draft.workspace_id = $2 AND draft.typ = 'app' 
        WHERE app.path = $1 AND app.workspace_id = $2"#,
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
    let mut tx = user_db.begin(&authed).await?;
    let query_result = sqlx::query!(
        "SELECT a.id as app_id, av.id as version_id, dm.deployment_msg as deployment_msg
        FROM app a LEFT JOIN app_version av ON a.id = av.app_id LEFT JOIN deployment_metadata dm ON av.id = dm.app_version
        WHERE a.workspace_id = $1 AND a.path = $2
        ORDER BY created_at DESC",
        w_id,
        path.to_path(),
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

    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT a.id as app_id, av.id as version_id, dm.deployment_msg as deployment_msg
        FROM app a LEFT JOIN app_version av ON a.id = av.app_id LEFT JOIN deployment_metadata dm ON av.id = dm.app_version
        WHERE a.workspace_id = $1 AND a.path = $2
        ORDER BY created_at DESC",
        w_id,
        path.to_path(),
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

    if app_path.is_none() {
        tx.commit().await?;
        return Err(Error::NotFound(
            format!("App with ID {app_id} not found").to_string(),
        ));
    }

    sqlx::query!(
        "INSERT INTO deployment_metadata (workspace_id, path, app_version, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path, app_version) WHERE app_version IS NOT NULL DO UPDATE SET deployment_msg = $4",
        w_id,
        app_path.unwrap(),
        app_version,
        app_history_update.deployment_msg,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(());
}

async fn get_app_by_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> JsonResult<AppWithLastVersion> {
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by from app, app_version 
        WHERE app_version.id = $1 AND app.id = app_version.app_id AND app.workspace_id = $2",
    )
    .bind(&id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", id.to_string())?;
    Ok(Json(app))
}

async fn get_public_app_by_secret(
    OptAuthed(opt_authed): OptAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, secret)): Path<(String, String)>,
) -> JsonResult<AppWithLastVersion> {
    let mc = build_crypt(&db, &w_id).await?;

    let decrypted = mc
        .decrypt_bytes_to_bytes(&(hex::decode(secret)?))
        .map_err(|e| Error::InternalErr(e.to_string()))?;
    let bytes = str::from_utf8(&decrypted).map_err(to_anyhow)?;

    let id: i64 = bytes.parse().map_err(to_anyhow)?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy,
        null as extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by from app, app_version 
        WHERE app.id = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]")
        .bind(&id)
        .bind(&w_id)
    .fetch_optional(&db)
    .await?;

    let app = not_found_if_none(app_o, "App", id.to_string())?;

    let policy = serde_json::from_str::<Policy>(app.policy.0.get()).map_err(to_anyhow)?;

    if matches!(policy.execution_mode, ExecutionMode::Anonymous) {
        return Ok(Json(app));
    }

    if opt_authed.is_none() {
        {
            return Err(Error::NotAuthorized(
                "App visibility does not allow public access and you are not logged in".to_string(),
            ));
        }
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

    Ok(Json(app))
}

async fn get_public_resource(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();
    if !path.starts_with("f/app_themes/") {
        return Err(Error::BadRequest(
            "Only app themes are public resources".to_string(),
        ));
    }
    let res = sqlx::query_scalar!(
        "SELECT value from resource WHERE path = $1 AND workspace_id = $2",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    Ok(Json(res))
}

async fn get_secret_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
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

async fn create_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut app): Json<CreateApp>,
) -> Result<(StatusCode, String)> {
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();

    app.policy.on_behalf_of = Some(username_to_permissioned_as(&authed.username));
    app.policy.on_behalf_of_email = Some(authed.email.clone());

    if &app.path == "" {
        return Err(Error::BadRequest("App path cannot be empty".to_string()));
    }

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        &app.path,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "App with path {} already exists",
            &app.path
        )));
    }

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'app'",
        &app.path,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    let id = sqlx::query_scalar!(
        "INSERT INTO app
            (workspace_id, path, summary, policy, versions, draft_only)
            VALUES ($1, $2, $3, $4, '{}', $5) RETURNING id",
        w_id,
        app.path,
        app.summary,
        json!(app.policy),
        app.draft_only,
    )
    .fetch_one(&mut tx)
    .await?;

    let v_id = sqlx::query_scalar!(
        "INSERT INTO app_version
            (app_id, value, created_by)
            VALUES ($1, $2::text::json, $3) RETURNING id",
        id,
        //to preserve key orders
        serde_json::to_string(&app.value).unwrap(),
        authed.username,
    )
    .fetch_one(&mut tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE id = $2",
        v_id,
        id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed,
        "apps.create",
        ActionKind::Create,
        &w_id,
        Some(&app.path),
        None,
    )
    .await?;

    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = app.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }

    let tx = PushIsolationLevel::Transaction(tx);
    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::AppDependencies { path: app.path.clone(), version: v_id },
        PushArgs { args: &args, extra: None },
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
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);

    new_tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: app.path.clone() },
    );

    Ok((StatusCode::CREATED, app.path))
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

async fn delete_app(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();

    if path == "g/all/setup_app" && w_id == "admins" {
        return Err(Error::BadRequest(
            "Cannot delete the global setup app".to_string(),
        ));
    }

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

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::App {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        },
        Some(format!("App '{}' deleted", path)),
        rsmq,
        true,
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
        Error::InternalErr(format!(
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
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditApp>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();

    let npath = if ns.policy.is_some() || ns.path.is_some() || ns.summary.is_some() {
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
                .fetch_one(&mut tx)
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

        if let Some(mut npolicy) = ns.policy {
            npolicy.on_behalf_of = Some(username_to_permissioned_as(&authed.username));
            npolicy.on_behalf_of_email = Some(authed.email.clone());
            sqlb.set(
                "policy",
                quote(serde_json::to_string(&json!(npolicy)).map_err(|e| {
                    Error::BadRequest(format!("failed to serialize policy: {}", e))
                })?),
            );
        }

        sqlb.returning("path");

        let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
        let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut tx).await?;
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
        .fetch_one(&mut tx)
        .await?;

        let v_id = sqlx::query_scalar!(
            "INSERT INTO app_version
                (app_id, value, created_by)
                VALUES ($1, $2::text::json, $3) RETURNING id",
            app_id,
            //to preserve key orders
            serde_json::to_string(&nvalue).unwrap(),
            authed.username,
        )
        .fetch_one(&mut tx)
        .await?;

        sqlx::query!(
            "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE path = $2 AND workspace_id = $3",
            v_id,
            npath,
            w_id
        )
        .execute(&mut tx)
        .await?;
        v_id
    } else {
        let v_id = sqlx::query_scalar!(
            "SELECT  app.versions[array_upper(app.versions, 1)] FROM app WHERE path = $1 AND workspace_id = $2",
            npath,
            w_id
        )
        .fetch_one(&mut tx)
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
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed,
        "apps.update",
        ActionKind::Update,
        &w_id,
        Some(&npath),
        None,
    )
    .await?;

    let tx: PushIsolationLevel<'_, rsmq_async::MultiplexedRsmq> =
        PushIsolationLevel::Transaction(tx);
    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = ns.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }
    args.insert("parent_path".to_string(), to_raw_value(&path));

    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::AppDependencies { path: npath.clone(), version: v_id },
        PushArgs { args: &args, extra: None },
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
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);
    new_tx.commit().await?;

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

#[derive(Debug, Deserialize, Clone)]
pub struct ExecuteApp {
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
}

fn digest(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code);
    let result = hasher.finalize();
    format!("rawscript/{:x}", result)
}

async fn execute_component(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
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

    let policy = match payload.clone() {
        ExecuteApp {
            force_viewer_static_fields: Some(static_fields),
            force_viewer_one_of_fields: Some(one_of_fields),
            force_viewer_allow_user_resources: Some(allow_user_resources),
            ..
        } => {
            let mut hm = HashMap::new();

            if let Some(path) = payload.path.clone() {
                hm.insert(
                    format!("{}:{path}", payload.component),
                    PolicyTriggerableInputs {
                        static_inputs: static_fields,
                        one_of_inputs: one_of_fields,
                        allow_user_resources,
                    },
                );
            } else {
                hm.insert(
                    format!(
                        "{}:{}",
                        payload.component,
                        digest(payload.raw_code.clone().unwrap().content.as_str())
                    ),
                    PolicyTriggerableInputs {
                        static_inputs: static_fields,
                        one_of_inputs: one_of_fields,
                        allow_user_resources,
                    },
                );
            }
            Policy {
                execution_mode: ExecutionMode::Viewer,
                triggerables: None,
                triggerables_v2: Some(hm),
                on_behalf_of: None,
                on_behalf_of_email: None,
            }
        }
        _ => {
            let policy_o = sqlx::query_scalar!(
                "SELECT policy from app WHERE path = $1 AND workspace_id = $2",
                path,
                &w_id
            )
            .fetch_optional(&db)
            .await?;

            let policy = not_found_if_none(policy_o, "App", path)?;

            serde_json::from_value::<Policy>(policy).map_err(to_anyhow)?
        }
    };

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

    let (job_payload, (args, job_id), tag) = match payload {
        ExecuteApp { args, component, raw_code: Some(raw_code), path: None, .. } => {
            let content = &raw_code.content;
            let payload = JobPayload::Code(raw_code.clone());
            let path = digest(content);
            let args = build_args(
                policy,
                &component,
                path,
                args,
                opt_authed.as_ref(),
                &user_db,
                &db,
                &w_id,
            )
            .await?;
            (payload, args, None)
        }
        ExecuteApp { args, component, raw_code: None, path: Some(path), .. } => {
            let (payload, tag) = get_payload_tag_from_prefixed_path(&path, &db, &w_id).await?;
            let args = build_args(
                policy,
                &component,
                path.to_string(),
                args,
                opt_authed.as_ref(),
                &user_db,
                &db,
                &w_id,
            )
            .await?;
            (payload, args, tag)
        }
        _ => unreachable!(),
    };
    let tx = windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        &username,
        &email,
        permissioned_as,
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
    )
    .await?;
    tx.commit().await?;

    Ok(uuid.to_string())
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
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn build_args(
    policy: Policy,
    component: &str,
    path: String,
    mut args: HashMap<String, Box<RawValue>>,
    authed: Option<&ApiAuthed>,
    user_db: &UserDB,
    db: &DB,
    w_id: &str,
) -> Result<(PushArgsOwned, Option<Uuid>)> {
    let mut job_id: Option<Uuid> = None;
    let key = format!("{}:{}", component, &path);
    let (static_inputs, one_of_inputs, allow_user_resources) = match policy {
        Policy { triggerables_v2: Some(t), .. } => {
            let PolicyTriggerableInputs { static_inputs, one_of_inputs, allow_user_resources } = t
                .get(&key)
                .or_else(|| t.get(&path))
                .map(|x| x.clone())
                .or_else(|| {
                    if matches!(policy.execution_mode, ExecutionMode::Viewer) {
                        Some(PolicyTriggerableInputs {
                            static_inputs: HashMap::new(),
                            one_of_inputs: HashMap::new(),
                            allow_user_resources: Vec::new(),
                        })
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    Error::BadRequest(format!("path {} is not allowed in the app policy", path))
                })?;

            (static_inputs, one_of_inputs, allow_user_resources)
        }
        Policy { triggerables: Some(t), .. } => {
            let static_inputs = t
                .get(&key)
                .or_else(|| t.get(&path))
                .map(|x| x.clone())
                .or_else(|| {
                    if matches!(policy.execution_mode, ExecutionMode::Viewer) {
                        Some(HashMap::new())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    Error::BadRequest(format!("path {} is not allowed in the app policy", path))
                })?;

            (static_inputs, HashMap::new(), Vec::new())
        }
        _ => Err(Error::BadRequest(format!(
            "Policy is missing triggerables for {}",
            key
        )))?,
    };

    let mut safe_args = HashMap::<String, Box<RawValue>>::new();

    // tracing::error!("{:?}", allow_user_resources);
    for k in allow_user_resources.iter() {
        if let Some(arg_val) = args.get(k) {
            let key = serde_json::from_str::<String>(arg_val.get()).ok();
            if let Some(path) =
                key.and_then(|x| x.clone().strip_prefix("$res:").map(|x| x.to_string()))
            {
                if let Some(authed) = authed {
                    let res = get_resource_value_interpolated_internal(
                        authed,
                        Some(user_db.clone()),
                        db,
                        w_id,
                        &path,
                        None,
                        "",
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
        if safe_args.contains_key(&k) {
            continue;
        }
        if let Some(arg_val) = args.get(&k) {
            let arg_str = arg_val.get();

            let options_str_vec = v.iter().map(|x| x.get()).collect::<Vec<&str>>();
            if options_str_vec.contains(&arg_str) {
                safe_args.insert(k.to_string(), arg_val.clone());
                args.remove(&k);
                continue;
            }

            // check if multiselect
            if let Ok(args_str_vec) = serde_json::from_str::<Vec<Box<RawValue>>>(arg_val.get()) {
                if args_str_vec
                    .iter()
                    .all(|x| options_str_vec.contains(&x.get()))
                {
                    safe_args.insert(k.to_string(), arg_val.clone());
                    args.remove(&k);
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
                    Error::InternalErr(format!("failed to serialize ctx variable for {}: {}", k, e))
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
                    Error::InternalErr(format!(
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
