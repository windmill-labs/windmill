use anyhow::Context;
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use http::StatusCode;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{value::RawValue, Value};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::prelude::FromRow;
use std::{collections::HashMap, fmt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, JsonResult},
    utils::{not_found_if_none, paginate, report_critical_error, Pagination, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
    INSTANCE_NAME,
};
use windmill_queue::PushArgsOwned;

use crate::{
    db::{ApiAuthed, DB},
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
    users::fetch_api_authed,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_websocket_trigger))
        .route("/list", get(list_websocket_triggers))
        .route("/get/*path", get(get_websocket_trigger))
        .route("/update/*path", post(update_websocket_trigger))
        .route("/delete/*path", delete(delete_websocket_trigger))
        .route("/exists/*path", get(exists_websocket_trigger))
        .route("/setenabled/*path", post(set_enabled))
}

#[derive(Deserialize)]
struct NewWebsocketTrigger {
    path: String,
    url: String,
    script_path: String,
    is_flow: bool,
    enabled: Option<bool>,
    filters: Vec<Box<RawValue>>,
    initial_messages: Vec<Box<RawValue>>,
    url_runnable_args: Box<RawValue>,
}

#[derive(Deserialize)]
struct JsonFilter {
    key: String,
    value: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Filter {
    JsonFilter(JsonFilter),
}

#[derive(Deserialize)]
enum InitialMessage {
    #[serde(rename = "raw_message")]
    RawMessage(String),
    #[serde(rename = "runnable_result")]
    RunnableResult { path: String, args: Box<RawValue>, is_flow: bool },
}

#[derive(FromRow, Serialize, Clone)]
pub struct WebsocketTrigger {
    workspace_id: String,
    path: String,
    url: String,
    script_path: String,
    is_flow: bool,
    edited_by: String,
    email: String,
    edited_at: chrono::DateTime<chrono::Utc>,
    server_id: Option<String>,
    last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    extra_perms: serde_json::Value,
    error: Option<String>,
    enabled: bool,
    filters: Vec<sqlx::types::Json<Box<RawValue>>>,
    initial_messages: Vec<sqlx::types::Json<Box<RawValue>>>,
    url_runnable_args: sqlx::types::Json<Box<RawValue>>,
}

#[derive(Deserialize)]
struct EditWebsocketTrigger {
    path: String,
    url: String,
    script_path: String,
    is_flow: bool,
    filters: Vec<Box<RawValue>>,
    initial_messages: Vec<Box<RawValue>>,
    url_runnable_args: Box<RawValue>,
}

#[derive(Deserialize)]
pub struct ListWebsocketTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

async fn list_websocket_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListWebsocketTriggerQuery>,
) -> error::JsonResult<Vec<WebsocketTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("websocket_trigger")
        .field("*")
        .order_by("edited_at", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();
    if let Some(path) = lst.path {
        sqlb.and_where_eq("script_path", "?".bind(&path));
    }
    if let Some(is_flow) = lst.is_flow {
        sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
    }
    if let Some(path_start) = &lst.path_start {
        sqlb.and_where_like_left("path", path_start);
    }
    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, WebsocketTrigger>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::JsonResult<WebsocketTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as::<_, WebsocketTrigger>(
        r#"SELECT *
          FROM websocket_trigger
          WHERE workspace_id = $1 AND path = $2"#,
    )
    .bind(w_id)
    .bind(path)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

async fn create_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ct): Json<NewWebsocketTrigger>,
) -> error::Result<(StatusCode, String)> {
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Websocket triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }

    let mut tx = user_db.begin(&authed).await?;

    let filters = ct.filters.into_iter().map(sqlx::types::Json).collect_vec();
    let initial_messages = ct
        .initial_messages
        .into_iter()
        .map(sqlx::types::Json)
        .collect_vec();
    sqlx::query_as::<_, WebsocketTrigger>(
      "INSERT INTO websocket_trigger (workspace_id, path, url, script_path, is_flow, enabled, filters, initial_messages, url_runnable_args, edited_by, email, edited_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now()) RETURNING *",
    )
    .bind(&w_id)
    .bind(&ct.path)
    .bind(ct.url)
    .bind(ct.script_path)
    .bind(ct.is_flow)
    .bind(ct.enabled.unwrap_or(true))
    .bind(filters.as_slice())
    .bind(initial_messages.as_slice())
    .bind(sqlx::types::Json(ct.url_runnable_args))
    .bind(&authed.username)
    .bind(&authed.email)
    .fetch_one(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.create",
        ActionKind::Create,
        &w_id,
        Some(ct.path.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("{}", ct.path)))
}

async fn update_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ct): Json<EditWebsocketTrigger>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let filters = ct.filters.into_iter().map(sqlx::types::Json).collect_vec();
    let initial_messages = ct
        .initial_messages
        .into_iter()
        .map(sqlx::types::Json)
        .collect_vec();

    // important to update server_id, last_server_ping and error to NULL to stop current websocket listener
    sqlx::query!(
        "UPDATE websocket_trigger SET url = $1, script_path = $2, path = $3, is_flow = $4, filters = $5, initial_messages = $6, url_runnable_args = $7, edited_by = $8, email = $9, edited_at = now(), server_id = NULL, last_server_ping = NULL, error = NULL
            WHERE workspace_id = $10 AND path = $11",
        ct.url,
        ct.script_path,
        ct.path,
        ct.is_flow,
        filters.as_slice() as &[sqlx::types::Json<Box<RawValue>>],
        initial_messages.as_slice() as &[sqlx::types::Json<Box<RawValue>>],
        sqlx::types::Json(ct.url_runnable_args) as sqlx::types::Json<Box<RawValue>>,
        &authed.username,
        &authed.email,
        w_id,
        path,
    )
    .execute(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.update",
        ActionKind::Create,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(path.to_string())
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}

pub async fn set_enabled(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    // important to set server_id, last_server_ping and error to NULL to stop current websocket listener
    let one_o = sqlx::query_scalar!(
        "UPDATE websocket_trigger SET enabled = $1, email = $2, edited_by = $3, edited_at = now(), server_id = NULL, last_server_ping = NULL, error = NULL
        WHERE path = $4 AND workspace_id = $5 RETURNING 1",
        payload.enabled,
        &authed.email,
        &authed.username,
        path,
        w_id,
    ).fetch_optional(&mut *tx).await?;

    not_found_if_none(one_o.flatten(), "Websocket trigger", path)?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "succesfully updated websocket trigger at path {} to status {}",
        path, payload.enabled
    ))
}

async fn delete_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM websocket_trigger WHERE workspace_id = $1 AND path = $2",
        w_id,
        path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Websocket trigger {path} deleted"))
}

async fn exists_websocket_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM websocket_trigger WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

async fn listen_to_unlistened_websockets(
    db: &DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) -> () {
    match sqlx::query_as::<_, WebsocketTrigger>(
        r#"SELECT *
            FROM websocket_trigger
            WHERE enabled IS TRUE AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')"#
    )
    .fetch_all(db)
    .await
    {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::thread_rng());
            for trigger in triggers {
                maybe_listen_to_websocket(trigger, db.clone(), killpill_rx.resubscribe()).await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching websocket triggers: {:?}", err);
        }
    };
}

pub async fn start_websockets(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) -> () {
    tokio::spawn(async move {
        listen_to_unlistened_websockets(&db, &&killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_websockets(&db, &&killpill_rx).await;
                }
            }
        }
    });
}

async fn maybe_listen_to_websocket(
    ws_trigger: WebsocketTrigger,
    db: DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    match sqlx::query_scalar!(
        "UPDATE websocket_trigger SET server_id = $1, last_server_ping = now() WHERE enabled IS TRUE AND workspace_id = $2 AND path = $3 AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
        *INSTANCE_NAME,
        ws_trigger.workspace_id,
        ws_trigger.path,
    ).fetch_optional(&db).await {
        Ok(has_lock) => {
            if has_lock.flatten().unwrap_or(false) {
                tokio::spawn(listen_to_websocket(ws_trigger, db, killpill_rx));
            } else {
                tracing::info!("Websocket {} already being listened to", ws_trigger.url);
            }
        },
        Err(err) => {
            tracing::error!("Error acquiring lock for websocket {}: {:?}", ws_trigger.path, err);
        }
    };
}

struct SupersetVisitor<'a> {
    key: &'a str,
    value_to_check: &'a Value,
}

impl<'de, 'a> Visitor<'de> for SupersetVisitor<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON object with a specific key at the top level")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            if key == self.key {
                // Deserialize the value for the key and check if it's a superset
                let json_value: Value = map.next_value()?;
                return Ok(is_superset(&json_value, self.value_to_check));
            } else {
                // Skip the value if it's not the one we're interested in
                let _ = map.next_value::<de::IgnoredAny>()?;
            }
        }
        // If the key was not found, return false
        Ok(false)
    }
}

// Function to check if json_value is a superset of value_to_check
fn is_superset(json_value: &Value, value_to_check: &Value) -> bool {
    match (json_value, value_to_check) {
        (Value::Object(json_map), Value::Object(check_map)) => {
            // Check that all keys and values in check_map exist and match in json_map
            check_map.iter().all(|(k, v)| {
                json_map
                    .get(k)
                    .map_or(false, |json_val| is_superset(json_val, v))
            })
        }
        (Value::Array(json_array), Value::Array(check_array)) => {
            // Check that all elements in check_array exist in json_array
            check_array.iter().all(|check_item| {
                json_array
                    .iter()
                    .any(|json_item| is_superset(json_item, check_item))
            })
        }
        _ => json_value == value_to_check,
    }
}

// A function to deserialize and check if the value at the given key is a superset of a passed value
fn is_value_superset<'a, 'de, D>(
    deserializer: D,
    key: &'a str,
    value_to_check: &'a Value,
) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(SupersetVisitor { key, value_to_check })
}

async fn wait_runnable_result(
    path: String,
    is_flow: bool,
    args: &Box<RawValue>,
    ws_trigger: &WebsocketTrigger,
    username_override: String,
    db: &DB,
) -> error::Result<String> {
    let user_db = UserDB::new(db.clone());
    let authed = fetch_api_authed(
        ws_trigger.edited_by.clone(),
        ws_trigger.email.clone(),
        &ws_trigger.workspace_id,
        &db,
        Some(username_override),
    )
    .await?;

    let args = serde_json::from_str::<Option<HashMap<String, Box<RawValue>>>>(args.get())
        .map_err(|e| error::Error::BadRequest(format!("invalid json: {}", e)))?
        .unwrap_or_else(HashMap::new);

    let label_prefix = Some(format!("ws-{}-", ws_trigger.path));
    let (_, job_id) = if is_flow {
        run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            ws_trigger.workspace_id.clone(),
            StripPath(path.clone()),
            RunJobQuery::default(),
            PushArgsOwned { args, extra: None },
            label_prefix,
        )
        .await?
    } else {
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            ws_trigger.workspace_id.clone(),
            StripPath(path.clone()),
            RunJobQuery::default(),
            PushArgsOwned { args, extra: None },
            label_prefix,
        )
        .await?
    };

    let start_time = tokio::time::Instant::now();

    loop {
        if start_time.elapsed() > tokio::time::Duration::from_secs(300) {
            return Err(anyhow::anyhow!(
                "Timed out after 5m waiting for runnable {path} (is_flow: {is_flow}) to complete",
            )
            .into());
        }

        #[derive(sqlx::FromRow)]
        struct RawResult {
            result: Option<sqlx::types::Json<Box<RawValue>>>,
            success: bool,
        }

        let result = sqlx::query_as::<_, RawResult>(
            "SELECT result, success FROM completed_job WHERE id = $1 AND workspace_id = $2",
        )
        .bind(Uuid::parse_str(&job_id).unwrap())
        .bind(&ws_trigger.workspace_id)
        .fetch_optional(db)
        .await;

        match result {
            Ok(Some(r)) => {
                if !r.success {
                    return Err(anyhow::anyhow!(
                        "Runnable {path} (is_flow: {is_flow}) failed: {:?}",
                        r.result
                    )
                    .into());
                } else {
                    return Ok(r.result.map(|r| r.get().to_owned()).unwrap_or_default());
                }
            }
            Ok(None) => {
                // not yet done, wait for 5s and check again
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Error fetching job result for runnable {path} (is_flow: {is_flow}): {err}",
                )
                .into());
            }
        }
    }
}

async fn send_initial_messages(
    ws_trigger: &WebsocketTrigger,
    mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    db: &DB,
) -> error::Result<()> {
    let initial_messages: Vec<InitialMessage> = ws_trigger
        .initial_messages
        .iter()
        .filter_map(|m| serde_json::from_str(m.get()).ok())
        .collect_vec();

    for start_message in initial_messages {
        match start_message {
            InitialMessage::RawMessage(msg) => {
                let msg = if msg.starts_with("\"") && msg.ends_with("\"") {
                    msg[1..msg.len() - 1].to_string()
                } else {
                    msg
                };
                tracing::info!(
                    "Sending raw message initial message to websocket {}: {}",
                    ws_trigger.url,
                    msg
                );
                writer
                    .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                    .await
                    .map_err(to_anyhow)
                    .with_context(|| "failed to send raw message")?;
            }
            InitialMessage::RunnableResult { path, is_flow, args } => {
                tracing::info!(
                    "Running runnable {path} (is_flow: {is_flow}) for initial message to websocket {}",
                    ws_trigger.url,
                );

                let result = wait_runnable_result(
                    path.clone(),
                    is_flow,
                    &args,
                    ws_trigger,
                    "init".to_string(),
                    db,
                )
                .await?;

                tracing::info!(
                    "Sending runnable {path} (is_flow: {is_flow}) result to websocket {}",
                    ws_trigger.url
                );

                let result = if result.starts_with("\"") && result.ends_with("\"") {
                    result[1..result.len() - 1].to_string()
                } else {
                    result
                };

                writer
                    .send(tokio_tungstenite::tungstenite::Message::Text(result))
                    .await
                    .map_err(to_anyhow)
                    .with_context(|| {
                        format!("Failed to send runnable {path} (is_flow: {is_flow}) result")
                    })?;
            }
        }
    }

    Ok(())
}

async fn get_url_from_runnable(
    path: &str,
    is_flow: bool,
    ws_trigger: &WebsocketTrigger,
    db: &DB,
) -> error::Result<String> {
    tracing::info!("Running runnable {path} (is_flow: {is_flow}) to get websocket URL",);

    let result = wait_runnable_result(
        path.to_string(),
        is_flow,
        &ws_trigger.url_runnable_args.0,
        ws_trigger,
        "url".to_string(),
        db,
    )
    .await?;

    if result.starts_with("\"") && result.ends_with("\"") {
        Ok(result[1..result.len() - 1].to_string())
    } else {
        Err(anyhow::anyhow!("Runnable {path} (is_flow: {is_flow}) did not return a string").into())
    }
}

async fn update_ping(db: &DB, ws_trigger: &WebsocketTrigger, error: Option<&str>) -> Option<()> {
    match sqlx::query_scalar!(
        "UPDATE websocket_trigger SET last_server_ping = now(), error = $1 WHERE workspace_id = $2 AND path = $3 AND server_id = $4 AND enabled IS TRUE RETURNING 1",
        error,
        ws_trigger.workspace_id,
        ws_trigger.path,
        *INSTANCE_NAME
    ).fetch_optional(db).await {
        Ok(updated) => {
            if updated.flatten().is_none() {
                tracing::info!("Websocket {} changed, disabled, or deleted, stopping...", ws_trigger.url); 
                return None;
            }
        },
        Err(err) => {
            tracing::warn!("Error updating ping of websocket {}: {:?}", ws_trigger.url, err);
        }
    };

    Some(())
}

async fn loop_ping(db: &DB, ws_trigger: &WebsocketTrigger, error: Option<&str>) -> () {
    loop {
        if let None = update_ping(db, ws_trigger, error).await {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn disable_with_error(db: &DB, ws_trigger: &WebsocketTrigger, error: String) {
    match sqlx::query!(
        "UPDATE websocket_trigger SET enabled = FALSE, error = $1, server_id = NULL, last_server_ping = NULL WHERE workspace_id = $2 AND path = $3",
        error,
        ws_trigger.workspace_id,
        ws_trigger.path,
    )
    .execute(db).await {
        Ok(_) => {
            report_critical_error(format!("Disabling websocket {} because of error: {}", ws_trigger.url, error), db.clone(), Some(&ws_trigger.workspace_id), None).await;
        },
        Err(disable_err) => {
            report_critical_error(
                format!("Could not disable websocket {} with err {}, disabling because of error {}", ws_trigger.path, disable_err, error), 
                db.clone(),
                Some(&ws_trigger.workspace_id),
                None,
            ).await;
        }
    }
}

async fn listen_to_websocket(
    ws_trigger: WebsocketTrigger,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    if let None = update_ping(&db, &ws_trigger, Some("Connecting...")).await {
        return;
    }

    let url = ws_trigger.url.as_str();

    let filters: Vec<Filter> = ws_trigger
        .filters
        .iter()
        .filter_map(|m| serde_json::from_str(m.get()).ok())
        .collect_vec();

    loop {
        let connect_url = if url.starts_with("$") {
            if url.starts_with("$flow:") || url.starts_with("$script:") {
                let path = url.splitn(2, ':').nth(1).unwrap();
                tokio::select! {
                    biased;
                    _ = killpill_rx.recv() => {
                        return;
                    },
                    _ = loop_ping(&db, &ws_trigger, Some(
                        "Waiting on runnable to return websocket URL..."
                    )) => {
                        return;
                    },
                    url_result = get_url_from_runnable(path, url.starts_with("$flow:"), &ws_trigger, &db) => match url_result {
                        Ok(url) => url,
                        Err(err) => {
                            disable_with_error(
                                &db,
                                &ws_trigger,
                                format!(
                                    "Error getting websocket URL from runnable after 5 tries: {:?}",
                                    err
                                ),
                            )
                            .await;
                            return;
                        }
                    },
                }
            } else {
                disable_with_error(
                    &db,
                    &ws_trigger,
                    format!("Invalid websocket runnable path: {}", url),
                )
                .await;
                return;
            }
        } else {
            url.to_string()
        };

        tokio::select! {
            biased;
            _ = killpill_rx.recv() => {
                return;
            },
            _ = loop_ping(&db, &ws_trigger, Some("Connecting...")) => {
                return;
            },
            connection = connect_async(connect_url) => {
                match connection {
                    Ok((ws_stream, _)) => {
                        tracing::info!("Listening to websocket {}", url);
                        if let None = update_ping(&db, &ws_trigger, None).await {
                            return;
                        }
                        let (writer, mut reader) = ws_stream.split();
                        let mut last_ping = tokio::time::Instant::now();

                        tokio::select! {
                            biased;
                            _ = killpill_rx.recv() => {
                                return;
                            }
                            _ = async {
                                if let Err(err) = send_initial_messages(&ws_trigger, writer, &db).await {
                                    disable_with_error(&db, &ws_trigger, format!("Error sending initial messages: {:?}", err)).await;
                                } else {
                                    // if initial messages sent successfully, wait forever
                                    futures::future::pending::<()>().await;
                                }
                            } => {
                                // was disabled => exit
                                return;
                            },
                            _ = async {
                                loop {
                                    tokio::select! {
                                        biased;
                                        msg = reader.next() => {
                                            if let Some(msg) = msg {
                                                if last_ping.elapsed() > tokio::time::Duration::from_secs(5) {
                                                    if let None = update_ping(&db, &ws_trigger, None).await {
                                                        return;
                                                    }
                                                    last_ping = tokio::time::Instant::now();
                                                }
                                                match msg {
                                                    Ok(msg) => {
                                                        match msg {
                                                            tokio_tungstenite::tungstenite::Message::Text(text) => {
                                                                let mut should_handle = true;
                                                                for filter in &filters {
                                                                    match filter {
                                                                        Filter::JsonFilter(JsonFilter { key, value }) => {
                                                                            let mut deserializer = serde_json::Deserializer::from_str(text.as_str());
                                                                            should_handle = match is_value_superset(&mut deserializer, key, &value) {
                                                                                Ok(filter_match) => {
                                                                                    filter_match
                                                                                },
                                                                                Err(err) => {
                                                                                    tracing::warn!("Error deserializing filter for websocket {}: {:?}", url, err);
                                                                                    false
                                                                                }
                                                                            };
                                                                        }
                                                                    }
                                                                    if !should_handle {
                                                                        break;
                                                                    }
                                                                }
                                                                if should_handle {
                                                                    if let Err(err) = run_job(&db, &ws_trigger, text).await {
                                                                        report_critical_error(format!("Failed to trigger job from websocket {}: {:?}", ws_trigger.url, err), db.clone(), Some(&ws_trigger.workspace_id), None).await;
                                                                    };
                                                                }
                                                            },
                                                            _ => {}
                                                        }
                                                    },
                                                    Err(err) => {
                                                        tracing::error!("Error reading from websocket {}: {:?}", url, err);
                                                    }
                                                }
                                            } else {
                                                tracing::error!("Websocket {} closed", url);
                                                if let None =
                                                    update_ping(&db, &ws_trigger, Some("Websocket closed")).await
                                                {
                                                    return;
                                                }
                                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                                break;
                                            }
                                        },
                                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                                            if let None = update_ping(&db, &ws_trigger, None).await {
                                                return;
                                            }
                                            last_ping = tokio::time::Instant::now();
                                        },
                                    }
                                }
                            } => {
                                return;
                            }
                        };
                    }
                    Err(err) => {
                        tracing::error!("Error connecting to websocket {}: {:?}", url, err);
                        if let None =
                            update_ping(&db, &ws_trigger, Some(err.to_string().as_str())).await
                        {
                            return;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn run_job(db: &DB, trigger: &WebsocketTrigger, msg: String) -> anyhow::Result<()> {
    let args = PushArgsOwned {
        args: HashMap::from([("msg".to_string(), to_raw_value(&msg))]),
        extra: Some(HashMap::from([(
            "wm_trigger".to_string(),
            to_raw_value(
                &serde_json::json!({"kind": "websocket", "websocket": { "url": trigger.url }}),
            ),
        )])),
    };
    let label_prefix = Some(format!("ws-{}-", trigger.path));

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some("anonymous".to_string()),
    )
    .await?;

    let user_db = UserDB::new(db.clone());

    let run_query = RunJobQuery::default();

    if trigger.is_flow {
        run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            label_prefix,
        )
        .await?;
    } else {
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            label_prefix,
        )
        .await?;
    }

    Ok(())
}
