use std::collections::HashMap;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use crate::{
    db::{UserDB, DB},
    jobs::script_path_to_payload,
    users::{require_owner_of_path, Authed, OptAuthed},
    variables::build_crypt,
};
use axum::{
    extract::{Extension, Json, Path, Query},
    routing::{delete, get, post},
    Router,
};
use hyper::StatusCode;
use magic_crypt::MagicCryptTrait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Uuid, FromRow};
use std::str;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    apps::ListAppQuery,
    error::{to_anyhow, Error, JsonResult, Result},
    users::username_to_permissioned_as,
    utils::{
        http_get_from_hub, list_elems_from_hub, not_found_if_none, paginate, Pagination, StripPath,
    },
};
use windmill_queue::{push, JobPayload, RawCode};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_apps))
        .route("/get/p/*path", get(get_app))
        .route("/secret_of/*path", get(get_secret_id))
        .route("/get/v/*id", get(get_app_by_id))
        .route("/exists/*path", get(exists_app))
        .route("/update/*path", post(update_app))
        .route("/delete/*path", delete(delete_app))
        .route("/create", post(create_app))
}

pub fn unauthed_service() -> Router {
    Router::new()
        .route("/execute_component/*path", post(execute_component))
        .route("/public_app/:secret", get(get_public_app_by_secret))
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
    pub edited_at: chrono::DateTime<chrono::Utc>,
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

pub type StaticFields = Map<String, Value>;

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
}

#[derive(Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<serde_json::Value>,
    pub policy: Option<Policy>,
}

async fn list_apps(
    authed: Authed,
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
        .order_desc("favorite.path IS NOT NULL")
        .order_by("app_version.created_at", true)
        .and_where("app.workspace_id = ? OR app.workspace_id = 'starter'".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableApp>(&sql)
        .fetch_all(&mut tx)
        .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_app(
    authed: Authed,
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
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    Ok(Json(app))
}

async fn get_app_by_id(
    authed: Authed,
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
    .fetch_optional(&mut tx)
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
    .fetch_optional(&mut tx)
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

async fn get_secret_id(
    authed: Authed,
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
    .fetch_optional(&mut tx)
    .await?;

    let id = not_found_if_none(id_o, "App", path.to_string())?;

    let mc = build_crypt(&mut tx, &w_id).await?;

    let hx = hex::encode(mc.encrypt_str_to_bytes(id.to_string()));

    tx.commit().await?;

    Ok(hx)
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
            (workspace_id, path, summary, policy, versions)
            VALUES ($1, $2, $3, $4, '{}') RETURNING id",
        w_id,
        app.path,
        app.summary,
        json!(app.policy),
    )
    .fetch_one(&mut tx)
    .await?;

    let v_id = sqlx::query_scalar!(
        "INSERT INTO app_version
            (app_id, value, created_by)
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

    Ok((StatusCode::CREATED, app.path))
}

async fn list_hub_apps(
    Authed { email, .. }: Authed,
    Extension(http_client): Extension<Client>,
) -> JsonResult<serde_json::Value> {
    let flows = list_elems_from_hub(
        http_client,
        "https://hub.windmill.dev/searchUiData?approved=true",
        &email,
    )
    .await?;
    Ok(Json(flows))
}

pub async fn get_hub_app_by_id(
    Authed { email, .. }: Authed,
    Path(id): Path<i32>,
    Extension(http_client): Extension<Client>,
) -> JsonResult<serde_json::Value> {
    let value = http_get_from_hub(
        http_client,
        &format!("https://hub.windmill.dev/apps/{id}/json"),
        &email,
        false,
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
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
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditApp>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();

    let mut tx = user_db.begin(&authed).await?;

    let npath = if ns.policy.is_some() || ns.path.is_some() || ns.summary.is_some() {
        let mut sqlb = SqlBuilder::update_table("app");
        sqlb.and_where_eq("path", "?".bind(&path));
        sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

        if let Some(npath) = &ns.path {
            if npath != path {
                if !authed.is_admin {
                    require_owner_of_path(&w_id, &authed.username, &authed.groups, &path, &db)
                        .await?;
                }
            }
            sqlb.set_str("path", npath);
        }

        if let Some(nsummary) = &ns.summary {
            sqlb.set_str("summary", nsummary);
        }

        if let Some(npolicy) = ns.policy {
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
        let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut tx).await?;
        not_found_if_none(npath_o, "App", path)?
    } else {
        "".to_string()
    };
    if let Some(nvalue) = &ns.value {
        let app_id = sqlx::query_scalar!(
            "SELECT id FROM app WHERE path = $1 AND workspace_id = $2",
            path,
            w_id
        )
        .fetch_one(&mut tx)
        .await?;

        let v_id = sqlx::query_scalar!(
            "INSERT INTO app_version
                (app_id, value, created_by)
                VALUES ($1, $2, $3) RETURNING id",
            app_id,
            nvalue,
            authed.username,
        )
        .fetch_one(&mut tx)
        .await?;

        sqlx::query!(
            "UPDATE app SET versions = array_append(versions, $1) WHERE path = $2 AND workspace_id = $3",
            v_id,
            path,
            w_id
        )
        .execute(&mut tx)
        .await?;
    }
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
    tx.commit().await?;

    Ok(format!("app {} updated (npath: {:?})", path, npath))
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecuteApp {
    pub args: Map<String, serde_json::Value>,
    // - script: script/<path>
    // - flow: flow/<path>
    pub path: Option<String>,
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
    let mut tx = db.begin().await?;

    let policy = if let Some(static_fields) = payload.clone().force_viewer_static_fields {
        let mut hm = HashMap::new();
        if let Some(path) = payload.path.clone() {
            hm.insert(path, static_fields);
        } else {
            hm.insert(
                digest(payload.raw_code.clone().unwrap().content.as_str()),
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
        .fetch_optional(&mut tx)
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

    let (job_payload, args) = match &payload {
        ExecuteApp { args, raw_code: Some(raw_code), path: None, .. } => {
            let content = &raw_code.content;
            let payload = JobPayload::Code(raw_code.clone());
            let path = digest(content);
            let args = build_args(policy, path, args)?;
            (payload, args)
        }
        ExecuteApp { args, raw_code: None, path: Some(path), .. } => {
            let payload = if path.starts_with("script/") {
                script_path_to_payload(path.strip_prefix("script/").unwrap(), &mut tx, &w_id)
                    .await?
            } else if path.starts_with("flow/") {
                JobPayload::Flow(path.strip_prefix("flow/").unwrap().to_string())
            } else {
                return Err(Error::BadRequest(format!(
                    "path must start with script/ or flow/ (got {})",
                    path
                )));
            };
            let args = build_args(policy, path.to_string(), args)?;
            (payload, args)
        }
        _ => unreachable!(),
    };

    let (uuid, tx) = push(
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
        false,
        false,
        None,
        true,
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
    path: String,
    args: &Map<String, Value>,
) -> Result<Map<String, Value>> {
    let static_args = policy
        .triggerables
        .get(&path)
        .map(|x| x.clone())
        .or_else(|| {
            if matches!(policy.execution_mode, ExecutionMode::Viewer) {
                Some(Map::new())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            Error::BadRequest(format!("path {} is not allowed in the app policy", path))
        })?;
    let mut args = args.clone();
    for (k, v) in static_args {
        args.insert(k.to_string(), v.to_owned());
    }
    Ok(args)
}
