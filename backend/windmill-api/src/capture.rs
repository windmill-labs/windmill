/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, fmt};

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, head, post},
    Json, Router,
};
use http::HeaderMap;
use hyper::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
};
use windmill_queue::{PushArgs, PushArgsOwned};

use crate::{
    db::{ApiAuthed, DB},
    http_triggers::build_http_trigger_extra,
};

const KEEP_LAST: i64 = 20;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/set_config", post(set_config))
        .route(
            "/ping_config/:trigger_kind/:runnable_kind/*path",
            post(ping_config),
        )
        .route("/get_configs/:runnable_kind/*path", get(get_configs))
        .route("/list/:runnable_kind/*path", get(list_captures))
        .route("/:id", delete(delete_capture))
}

pub fn workspaced_unauthed_service() -> Router {
    Router::new()
        .route(
            "/webhook/:runnable_kind/*path",
            head(|| async {}).post(webhook_payload),
        )
        .route(
            "/http/:runnable_kind/:path/*route_path",
            head(|| async {}).fallback(http_payload),
        )
}

#[derive(sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "TRIGGER_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TriggerKind {
    Webhook,
    Http,
    Websocket,
    Kafka,
    Email,
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TriggerKind::Webhook => "webhook",
            TriggerKind::Http => "http",
            TriggerKind::Websocket => "websocket",
            TriggerKind::Kafka => "kafka",
            TriggerKind::Email => "email",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize)]
struct NewCaptureConfig {
    trigger_kind: TriggerKind,
    path: String,
    is_flow: bool,
    trigger_config: Option<SqlxJson<Box<RawValue>>>,
}

#[derive(Serialize, Deserialize)]
struct CaptureConfig {
    trigger_config: Option<SqlxJson<Box<RawValue>>>,
    trigger_kind: TriggerKind,
    error: Option<String>,
    last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
}

async fn get_configs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
) -> JsonResult<Vec<CaptureConfig>> {
    let mut tx = user_db.begin(&authed).await?;

    let configs = sqlx::query_as!(
        CaptureConfig,
        r#"SELECT trigger_config as "trigger_config: _", trigger_kind as "trigger_kind: _", error, last_server_ping
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3"#,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(configs))
}

async fn set_config(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(nc): Json<NewCaptureConfig>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO capture_config
            (workspace_id, path, is_flow, trigger_kind, trigger_config, owner, email)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (workspace_id, path, is_flow, trigger_kind)
            DO UPDATE SET trigger_config = $5, owner = $6, email = $7, server_id = NULL, last_server_ping = NULL, error = NULL",
        &w_id,
        &nc.path,
        nc.is_flow,
        nc.trigger_kind as TriggerKind,
        nc.trigger_config as Option<SqlxJson<Box<RawValue>>>,
        &authed.username,
        &authed.email,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

async fn ping_config(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, trigger_kind, runnable_kind, path)): Path<(
        String,
        TriggerKind,
        RunnableKind,
        StripPath,
    )>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "UPDATE capture_config SET last_client_ping = now() WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4",
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        trigger_kind as TriggerKind,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Capture {
    id: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    trigger_kind: TriggerKind,
    payload: SqlxJson<Box<serde_json::value::RawValue>>,
    trigger_extra: Option<SqlxJson<Box<serde_json::value::RawValue>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RunnableKind {
    Script,
    Flow,
}

async fn list_captures(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
) -> JsonResult<Vec<Capture>> {
    let mut tx = user_db.begin(&authed).await?;

    let captures = sqlx::query_as!(
        Capture,
        r#"SELECT id, created_at, trigger_kind as "trigger_kind: _", payload as "payload: _", trigger_extra as "trigger_extra: _"
        FROM capture
        WHERE workspace_id = $1
            AND path = $2 AND is_flow = $3
        ORDER BY created_at DESC"#,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(captures))
}

async fn delete_capture(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((_, id)): Path<(String, i64)>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!("DELETE FROM capture WHERE id = $1", id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ActiveCaptureOwner {
    owner: String,
    email: String,
}

pub async fn get_active_capture_owner_and_email(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    kind: &TriggerKind,
) -> Result<(String, String)> {
    let capture_config = sqlx::query_as!(
        ActiveCaptureOwner,
        "SELECT owner, email
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4 AND last_client_ping > NOW() - INTERVAL '10 seconds'",
        &w_id,
        &path,
        is_flow,
        kind as &TriggerKind,
    )
    .fetch_optional(db)
    .await?;

    let capture_config = not_found_if_none(
        capture_config,
        &format!("capture config for {} trigger", kind),
        path,
    )?;

    Ok((capture_config.owner, capture_config.email))
}

async fn get_capture_trigger_config_and_owner<T: DeserializeOwned>(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    kind: &TriggerKind,
) -> Result<(T, String)> {
    #[derive(Deserialize)]
    struct CaptureTriggerConfigAndOwner {
        trigger_config: Option<SqlxJson<Box<RawValue>>>,
        owner: String,
    }

    let capture_config = sqlx::query_as!(
        CaptureTriggerConfigAndOwner,
        r#"SELECT trigger_config as "trigger_config: _", owner
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4 AND last_client_ping > NOW() - INTERVAL '10 seconds'"#,
        &w_id,
        &path,
        is_flow,
        kind as &TriggerKind,
    )
    .fetch_optional(db)
    .await?;

    let capture_config = not_found_if_none(
        capture_config,
        &format!("capture config for {} trigger", kind),
        path,
    )?;

    let trigger_config = not_found_if_none(
        capture_config.trigger_config,
        &format!("capture {} trigger config", kind),
        path,
    )?;

    Ok((
        serde_json::from_str(trigger_config.get()).map_err(|e| {
            Error::InternalErr(format!(
                "error parsing capture config for {} trigger: {}",
                kind, e
            ))
        })?,
        capture_config.owner,
    ))
}

async fn clear_captures_history(db: &DB, w_id: &str) -> Result<()> {
    if *CLOUD_HOSTED {
        /* Retain only KEEP_LAST most recent captures in this workspace. */
        sqlx::query!(
            "DELETE FROM capture
            WHERE workspace_id = $1
                AND created_at <=
                    (
                        SELECT created_at
                            FROM capture
                            WHERE workspace_id = $1
                        ORDER BY created_at DESC
                            OFFSET $2
                            LIMIT 1
                    )",
            &w_id,
            KEEP_LAST,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

pub async fn insert_capture_payload(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    trigger_kind: &TriggerKind,
    payload: PushArgsOwned,
    trigger_extra: Option<Box<RawValue>>,
    owner: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, payload, trigger_extra, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &w_id,
        path,
        is_flow,
        trigger_kind as &TriggerKind,
        SqlxJson(to_raw_value(&PushArgs {
            args: &payload.args,
            extra: payload.extra
        })) as SqlxJson<Box<RawValue>>,
        trigger_extra.map(SqlxJson) as Option<SqlxJson<Box<RawValue>>>,
        owner,
    )
    .execute(db)
    .await?;

    clear_captures_history(db, &w_id).await?;

    Ok(())
}

async fn webhook_payload(
    Extension(db): Extension<DB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
    args: PushArgsOwned,
) -> Result<StatusCode> {
    let (owner, _) = get_active_capture_owner_and_email(
        &db,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        &TriggerKind::Webhook,
    )
    .await?;

    insert_capture_payload(
        &db,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        &TriggerKind::Webhook,
        args,
        Some(to_raw_value(&serde_json::json!({
            "wm_trigger": {
                "kind": "webhook",
            }
        }))),
        &owner,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
struct HttpTriggerConfig {
    route_path: String,
}

async fn http_payload(
    Extension(db): Extension<DB>,
    Path((w_id, kind, path, route_path)): Path<(String, RunnableKind, String, StripPath)>,
    Query(query): Query<HashMap<String, String>>,
    method: http::Method,
    headers: HeaderMap,
    args: PushArgsOwned,
) -> Result<StatusCode> {
    let route_path = route_path.to_path();

    let (http_trigger_config, owner): (HttpTriggerConfig, _) =
        get_capture_trigger_config_and_owner(
            &db,
            &w_id,
            &path,
            matches!(kind, RunnableKind::Flow),
            &TriggerKind::Http,
        )
        .await?;

    let mut router = matchit::Router::new();
    router.insert(&http_trigger_config.route_path, ()).ok();
    let match_ = router.at(route_path).ok();

    let match_ = not_found_if_none(match_, "capture http trigger", &route_path)?;

    let matchit::Match { params, .. } = match_;

    let params: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let extra: HashMap<String, Box<RawValue>> = HashMap::from_iter(vec![(
        "wm_trigger".to_string(),
        build_http_trigger_extra(
            &http_trigger_config.route_path,
            route_path,
            &method,
            &params,
            &query,
            &headers,
        )
        .await,
    )]);

    insert_capture_payload(
        &db,
        &w_id,
        &path,
        matches!(kind, RunnableKind::Flow),
        &TriggerKind::Http,
        args,
        Some(to_raw_value(&extra)),
        &owner,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
