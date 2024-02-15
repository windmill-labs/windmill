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
    users::{require_owner_of_path, OptAuthed},
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
use axum::{
    body::StreamBody,
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
    variables::build_crypt,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, PushArgs, PushIsolationLevel, QueueTransaction};

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
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct AppVersion {
    pub id: i64,
    pub app_id: Uuid,
    pub value: serde_json::Value,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct AppWithLastVersion {
    pub id: i64,
    pub path: String,
    pub summary: String,
    pub policy: serde_json::Value,
    pub versions: Vec<i64>,
    pub value: serde_json::Value,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct AppWithLastVersionAndDraft {
    pub id: i64,
    pub path: String,
    pub summary: String,
    pub policy: serde_json::Value,
    pub versions: Vec<i64>,
    pub value: serde_json::Value,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<serde_json::Value>,
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Anonymous,
    Publisher,
    Viewer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    pub on_behalf_of: Option<String>,
    pub on_behalf_of_email: Option<String>,
    //paths:
    // - script/<path>
    // - flow/<path>
    // - rawscript/<sha256>
    pub triggerables: HashMap<String, StaticFields>,
    pub execution_mode: ExecutionMode,
}

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub value: serde_json::Value,
    pub policy: Policy,
    pub draft_only: Option<bool>,
    pub deployment_message: Option<String>,
}

#[derive(Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<serde_json::Value>,
    pub policy: Option<Policy>,
    pub deployment_message: Option<String>,
}

#[derive(Serialize, FromRow)]
pub struct SearchApp {
    path: String,
    value: serde_json::Value,
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

    let rows = sqlx::query_as!(
        SearchApp,
        "SELECT path, app_version.value from app LEFT JOIN app_version ON app_version.id = versions[array_upper(versions, 1)]  WHERE workspace_id = $1 LIMIT $2",
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
) -> JsonResult<AppWithLastVersion> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as!(
        AppWithLastVersion,
        "SELECT app.id, app.path, app.summary, app.versions, app.policy,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by from app, app_version 
        WHERE app.path = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
        path.to_owned(),
        &w_id
    )
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
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as!(
        AppWithLastVersionAndDraft,
        r#"SELECT app.id, app.path, app.summary, app.versions, app.policy,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by,
        app.draft_only, draft.value as "draft?"
        from app
        INNER JOIN app_version ON
        app_version.id = app.versions[array_upper(app.versions, 1)]
        LEFT JOIN draft ON 
        app.path = draft.path AND draft.workspace_id = $2 AND draft.typ = 'app' 
        WHERE app.path = $1 AND app.workspace_id = $2"#,
        path.to_owned(),
        &w_id
    )
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

    let app_o = sqlx::query_as!(
        AppWithLastVersion,
        "SELECT app.id, app.path, app.summary, app.versions, app.policy,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by from app, app_version 
        WHERE app_version.id = $1 AND app.id = app_version.app_id AND app.workspace_id = $2",
        id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", id.to_string())?;
    Ok(Json(app))
}

async fn get_public_app_by_secret(
    Extension(db): Extension<DB>,
    Path((w_id, secret)): Path<(String, String)>,
) -> JsonResult<AppWithLastVersion> {
    let mut tx = db.begin().await?;

    let mc = build_crypt(&mut tx, &w_id).await?;

    let decrypted = mc
        .decrypt_bytes_to_bytes(&(hex::decode(secret)?))
        .map_err(|e| Error::InternalErr(e.to_string()))?;
    let bytes = str::from_utf8(&decrypted).map_err(to_anyhow)?;

    let id: i64 = bytes.parse().map_err(to_anyhow)?;

    let app_o = sqlx::query_as!(
        AppWithLastVersion,
        "SELECT app.id, app.path, app.summary, app.versions, app.policy,
        app.extra_perms, app_version.value, 
        app_version.created_at, app_version.created_by from app, app_version 
        WHERE app.id = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
        id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", id.to_string())?;

    let policy = serde_json::from_value::<Policy>(app.policy.clone()).map_err(to_anyhow)?;

    if !matches!(policy.execution_mode, ExecutionMode::Anonymous) {
        return Err(Error::NotAuthorized(
            "App visibility does not allow public access".to_string(),
        ));
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

    let id = not_found_if_none(id_o, "App", path.to_string())?;

    let mc = build_crypt(&mut tx, &w_id).await?;

    let hx = hex::encode(mc.encrypt_str_to_bytes(id.to_string()));

    tx.commit().await?;

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
        "SELECT EXISTS(SELECT 1 FROM raw_app WHERE path = $1 AND workspace_id = $2)",
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
        &authed.username,
        "apps.create",
        ActionKind::Create,
        &w_id,
        Some(&app.path),
        None,
    )
    .await?;

    let mut args: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(dm) = app.deployment_message {
        args.insert("deployment_message".to_string(), json!(dm));
    }

    let tx = PushIsolationLevel::Transaction(tx);
    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::AppDependencies { path: app.path.clone(), version: v_id },
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
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);

    new_tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: app.path.clone() },
    );

    Ok((StatusCode::CREATED, app.path))
}

async fn list_hub_apps(Extension(db): Extension<DB>) -> impl IntoResponse {
    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        "https://hub.windmill.dev/searchUiData?approved=true",
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

pub async fn get_hub_app_by_id(
    Path(id): Path<i32>,
    Extension(db): Extension<DB>,
) -> JsonResult<serde_json::Value> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("https://hub.windmill.dev/apps/{id}/json"),
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
        &authed.username,
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
            "error deleting deployment metadata for script with path {path} in workspace {w_id}: {e}"
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
                &format!(
                    "'{}'",
                    serde_json::to_string(&json!(npolicy)).map_err(|e| {
                        Error::BadRequest(format!("failed to serialize policy: {}", e))
                    })?
                ),
            );
        }

        sqlb.returning("path");

        let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
        tracing::error!("update_app sql: {}", sql);
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
        &authed.username,
        "apps.update",
        ActionKind::Update,
        &w_id,
        Some(&npath),
        None,
    )
    .await?;

    let tx: PushIsolationLevel<'_, rsmq_async::MultiplexedRsmq> =
        PushIsolationLevel::Transaction(tx);
    let mut args: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(dm) = ns.deployment_message {
        args.insert("deployment_message".to_string(), json!(dm));
    }
    args.insert("parent_path".to_string(), json!(path));

    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::AppDependencies { path: npath.clone(), version: v_id },
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

    let policy = if let Some(static_fields) = payload.clone().force_viewer_static_fields {
        let mut hm = HashMap::new();

        if let Some(path) = payload.path.clone() {
            hm.insert(format!("{}:{path}", payload.component), static_fields);
        } else {
            hm.insert(
                format!(
                    "{}:{}",
                    payload.component,
                    digest(payload.raw_code.clone().unwrap().content.as_str())
                ),
                static_fields,
            );
        }
        Policy {
            execution_mode: ExecutionMode::Viewer,
            triggerables: hm,
            on_behalf_of: None,
            on_behalf_of_email: None,
        }
    } else {
        let policy_o = sqlx::query_scalar!(
            "SELECT policy from app WHERE path = $1 AND workspace_id = $2",
            path,
            &w_id
        )
        .fetch_optional(&db)
        .await?;

        let policy = not_found_if_none(policy_o, "App", path)?;

        serde_json::from_value::<Policy>(policy).map_err(to_anyhow)?
    };

    let (username, permissioned_as, email) = match policy.execution_mode {
        ExecutionMode::Anonymous => {
            let username = opt_authed
                .map(|a| a.username)
                .unwrap_or_else(|| "anonymous".to_string());
            let (permissioned_as, email) = get_on_behalf_of(&policy)?;
            (username, permissioned_as, email)
        }
        ExecutionMode::Publisher => {
            let username = opt_authed.map(|a| a.username).ok_or_else(|| {
                Error::BadRequest("publisher execution mode requires authentication".to_string())
            })?;
            let (permissioned_as, email) = get_on_behalf_of(&policy)?;
            (username, permissioned_as, email)
        }
        ExecutionMode::Viewer => {
            let (username, email) = opt_authed.map(|a| (a.username, a.email)).ok_or_else(|| {
                Error::BadRequest("Required to be authed in viewer mode".to_string())
            })?;
            (
                username.clone(),
                username_to_permissioned_as(&username),
                email,
            )
        }
    };

    let (job_payload, args, tag) = match payload {
        ExecuteApp { args, component, raw_code: Some(raw_code), path: None, .. } => {
            let content = &raw_code.content;
            let payload = JobPayload::Code(raw_code.clone());
            let path = digest(content);
            let args = build_args(policy, &component, path, args)?;
            (payload, args, None)
        }
        ExecuteApp { args, component, raw_code: None, path: Some(path), .. } => {
            let (payload, tag) = get_payload_tag_from_prefixed_path(&path, &db, &w_id).await?;
            let args = build_args(policy, &component, path.to_string(), args)?;
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
        args,
        &username,
        &email,
        permissioned_as,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        tag,
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

fn build_args(
    policy: Policy,
    component: &str,
    path: String,
    args: HashMap<String, Box<RawValue>>,
) -> Result<PushArgs<HashMap<String, Box<RawValue>>>> {
    // disallow var and res access in args coming from the user for security reasons
    for (_, v) in &args {
        let args_str = serde_json::to_string(&v).unwrap_or_else(|_| "".to_string());
        if args_str.contains("$var:") || args_str.contains("$res:") {
            return Err(Error::BadRequest(format!(
            "For security reasons, variable or resource access is not allowed as dynamic argument"
        )));
        }
    }
    let key = format!("{}:{}", component, &path);
    let static_args = policy
        .triggerables
        .get(&key)
        .or_else(|| policy.triggerables.get(&path))
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
    let mut extra = HashMap::new();
    for (k, v) in static_args {
        extra.insert(k.to_string(), v.to_owned());
    }
    Ok(PushArgs { extra, args: sqlx::types::Json(args) })
}
