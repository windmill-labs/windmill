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
use sqlx::types::Json as SqlxJson;
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
    capture::{insert_capture_payload, TriggerKind, WebsocketTriggerConfig},
    db::{ApiAuthed, DB},
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
    users::fetch_api_authed,
};

use std::borrow::Cow;

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
pub struct JsonFilter {
    key: String,
    value: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Filter {
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
    filters: Vec<SqlxJson<Box<RawValue>>>,
    initial_messages: Vec<SqlxJson<Box<RawValue>>>,
    url_runnable_args: SqlxJson<Box<RawValue>>,
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

    let filters = ct.filters.into_iter().map(SqlxJson).collect_vec();
    let initial_messages = ct.initial_messages.into_iter().map(SqlxJson).collect_vec();
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
    .bind(SqlxJson(ct.url_runnable_args))
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

    let filters = ct.filters.into_iter().map(SqlxJson).collect_vec();
    let initial_messages = ct.initial_messages.into_iter().map(SqlxJson).collect_vec();

    // important to update server_id, last_server_ping and error to NULL to stop current websocket listener
    sqlx::query!(
        "UPDATE websocket_trigger SET url = $1, script_path = $2, path = $3, is_flow = $4, filters = $5, initial_messages = $6, url_runnable_args = $7, edited_by = $8, email = $9, edited_at = now(), server_id = NULL, last_server_ping = NULL, error = NULL
            WHERE workspace_id = $10 AND path = $11",
        ct.url,
        ct.script_path,
        ct.path,
        ct.is_flow,
        filters.as_slice() as &[SqlxJson<Box<RawValue>>],
        initial_messages.as_slice() as &[SqlxJson<Box<RawValue>>],
        SqlxJson(ct.url_runnable_args) as SqlxJson<Box<RawValue>>,
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
    rsmq: &Option<rsmq_async::MultiplexedRsmq>,
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
                trigger.maybe_listen_to_websocket(db.clone(), rsmq.clone(), killpill_rx.resubscribe()).await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching websocket triggers: {:?}", err);
        }
    };

    match sqlx::query_as!(
        CaptureConfigForWebsocket,
        r#"SELECT path, is_flow, workspace_id, trigger_config as "trigger_config!: _", owner FROM capture_config WHERE trigger_kind = 'websocket' AND last_client_ping > NOW() - INTERVAL '10 seconds' AND trigger_config IS NOT NULL AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')"#
    )
    .fetch_all(db)
    .await
    {
        Ok(mut captures) => {
            captures.shuffle(&mut rand::thread_rng());
            for capture in captures {
                capture.maybe_listen_to_websocket(db.clone(), rsmq.clone(), killpill_rx.resubscribe()).await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching capture websocket triggers: {:?}", err);
        }
    }
}

pub async fn start_websockets(
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    tokio::spawn(async move {
        listen_to_unlistened_websockets(&db, &rsmq, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_websockets(&db, &rsmq, &killpill_rx).await;
                }
            }
        }
    });
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
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
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
            rsmq.clone(),
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
            rsmq.clone(),
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
            result: Option<SqlxJson<Box<RawValue>>>,
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

async fn loop_ping(db: &DB, ws: &WebsocketEnum, error: Option<&str>) -> () {
    loop {
        if let None = ws.update_ping(db, error).await {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

impl WebsocketTrigger {
    async fn maybe_listen_to_websocket(
        self,
        db: DB,
        rsmq: Option<rsmq_async::MultiplexedRsmq>,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        match sqlx::query_scalar!(
            "UPDATE websocket_trigger SET server_id = $1, last_server_ping = now() WHERE enabled IS TRUE AND workspace_id = $2 AND path = $3 AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
        ).fetch_optional(&db).await {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tokio::spawn(listen_to_websocket(WebsocketEnum::Trigger(self), db, rsmq, killpill_rx));
                } else {
                    tracing::info!("Websocket {} already being listened to", self.url);
                }
            },
            Err(err) => {
                tracing::error!("Error acquiring lock for websocket {}: {:?}", self.path, err);
            }
        };
    }

    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match sqlx::query_scalar!(
        "UPDATE websocket_trigger SET last_server_ping = now(), error = $1 WHERE workspace_id = $2 AND path = $3 AND server_id = $4 AND enabled IS TRUE RETURNING 1",
        error,
        self.workspace_id,
        self.path,
        *INSTANCE_NAME
    ).fetch_optional(db).await {
        Ok(updated) => {
            if updated.flatten().is_none() {
                tracing::info!("Websocket {} changed, disabled, or deleted, stopping...", self.url); 
                return None;
            }
        },
            Err(err) => {
                tracing::warn!("Error updating ping of websocket {}: {:?}", self.url, err);
            }
        };

        Some(())
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match sqlx::query!(
            "UPDATE websocket_trigger SET enabled = FALSE, error = $1, server_id = NULL, last_server_ping = NULL WHERE workspace_id = $2 AND path = $3",
            error,
            self.workspace_id,
            self.path,
        )
        .execute(db).await {
            Ok(_) => {
                report_critical_error(format!("Disabling websocket {} because of error: {}", self.url, error), db.clone(), Some(&self.workspace_id), None).await;
            },
            Err(disable_err) => {
                report_critical_error(
                    format!("Could not disable websocket {} with err {}, disabling because of error {}", self.path, disable_err, error), 
                    db.clone(),
                    Some(&self.workspace_id),
                    None,
                ).await;
            }
        }
    }

    async fn get_url_from_runnable(
        &self,
        path: &str,
        is_flow: bool,
        db: &DB,
        rsmq: Option<rsmq_async::MultiplexedRsmq>,
    ) -> error::Result<String> {
        tracing::info!("Running runnable {path} (is_flow: {is_flow}) to get websocket URL",);

        let result = wait_runnable_result(
            path.to_string(),
            is_flow,
            &self.url_runnable_args.0,
            self,
            "url".to_string(),
            db,
            rsmq,
        )
        .await?;

        if result.starts_with("\"") && result.ends_with("\"") {
            Ok(result[1..result.len() - 1].to_string())
        } else {
            Err(
                anyhow::anyhow!("Runnable {path} (is_flow: {is_flow}) did not return a string")
                    .into(),
            )
        }
    }

    async fn send_initial_messages(
        &self,
        mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        db: &DB,
        rsmq: Option<rsmq_async::MultiplexedRsmq>,
    ) -> error::Result<()> {
        let initial_messages: Vec<InitialMessage> = self
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
                        self.url,
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
                        self.url,
                    );

                    let result = wait_runnable_result(
                        path.clone(),
                        is_flow,
                        &args,
                        self,
                        "init".to_string(),
                        db,
                        rsmq.clone(),
                    )
                    .await?;

                    tracing::info!(
                        "Sending runnable {path} (is_flow: {is_flow}) result to websocket {}",
                        self.url
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

    async fn handle(
        &self,
        db: &DB,
        rsmq: Option<rsmq_async::MultiplexedRsmq>,
        args: PushArgsOwned,
    ) -> () {
        if let Err(err) = run_job(db, rsmq, self, args).await {
            report_critical_error(
                format!(
                    "Failed to trigger job from websocket {}: {:?}",
                    self.url, err
                ),
                db.clone(),
                Some(&self.workspace_id),
                None,
            )
            .await;
        };
    }
}

#[derive(Deserialize)]
struct CaptureConfigForWebsocket {
    trigger_config: SqlxJson<WebsocketTriggerConfig>,
    path: String,
    is_flow: bool,
    workspace_id: String,
    owner: String,
}

impl CaptureConfigForWebsocket {
    async fn maybe_listen_to_websocket(
        self,
        db: DB,
        rsmq: Option<rsmq_async::MultiplexedRsmq>,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        match sqlx::query_scalar!(
            "UPDATE capture_config SET server_id = $1, last_server_ping = now() WHERE last_client_ping > NOW() - INTERVAL '10 seconds' AND workspace_id = $2 AND path = $3 AND is_flow = $4 AND trigger_kind = 'websocket' AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
            self.is_flow,
        ).fetch_optional(&db).await {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tokio::spawn(listen_to_websocket(WebsocketEnum::Capture(self), db, rsmq, killpill_rx));
                } else {
                    tracing::info!("Websocket {} already being listened to", self.trigger_config.url);
                }
            },
            Err(err) => {
                tracing::error!("Error acquiring lock for capture websocket {}: {:?}", self.path, err);
            }
        };
    }

    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match sqlx::query_scalar!(
        "UPDATE capture_config SET last_server_ping = now(), error = $1 WHERE workspace_id = $2 AND path = $3 AND is_flow = $4 AND trigger_kind = 'websocket' AND server_id = $5 AND last_client_ping > NOW() - INTERVAL '10 seconds' RETURNING 1",
        error,
        self.workspace_id,
        self.path,
        self.is_flow,
        *INSTANCE_NAME
    ).fetch_optional(db).await {
        Ok(updated) => {
            if updated.flatten().is_none() {
                tracing::info!("Websocket capture {} changed, disabled, or deleted, stopping...", self.trigger_config.url); 
                return None;
            }
        },
            Err(err) => {
                tracing::warn!("Error updating ping of capture websocket {}: {:?}", self.trigger_config.url, err);
            }
        };

        Some(())
    }

    async fn handle(&self, db: &DB, args: PushArgsOwned) -> () {
        if let Err(err) = insert_capture_payload(
            db,
            &self.workspace_id,
            &self.path,
            self.is_flow,
            &TriggerKind::Websocket,
            PushArgsOwned { args: args.args, extra: None },
            args.extra.as_ref().map(to_raw_value),
            &self.owner,
        )
        .await
        {
            tracing::error!("Error inserting capture payload: {:?}", err);
        }
    }
}

enum WebsocketEnum {
    Trigger(WebsocketTrigger),
    Capture(CaptureConfigForWebsocket),
}

impl WebsocketEnum {
    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match self {
            WebsocketEnum::Trigger(ws) => ws.update_ping(db, error).await,
            WebsocketEnum::Capture(capture) => capture.update_ping(db, error).await,
        }
    }
}

async fn listen_to_websocket(
    ws: WebsocketEnum,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    if let None = ws.update_ping(&db, Some("Connecting")).await {
        return;
    }

    let url = match &ws {
        WebsocketEnum::Trigger(ws_trigger) => &ws_trigger.url,
        WebsocketEnum::Capture(capture) => &capture.trigger_config.url,
    };

    let filters: Vec<Filter> = match &ws {
        WebsocketEnum::Trigger(ws_trigger) => ws_trigger
            .filters
            .iter()
            .filter_map(|m| serde_json::from_str(m.get()).ok())
            .collect_vec(),
        WebsocketEnum::Capture(_) => vec![],
    };

    loop {
        let connect_url: Cow<str> = match &ws {
            WebsocketEnum::Trigger(ws_trigger) => {
                if url.starts_with("$") {
                    if url.starts_with("$flow:") || url.starts_with("$script:") {
                        let path = url.splitn(2, ':').nth(1).unwrap();
                        tokio::select! {
                            biased;
                            _ = killpill_rx.recv() => {
                                return;
                            },
                            _ = loop_ping(&db, &ws, Some(
                                "Waiting on runnable to return websocket URL..."
                            )) => {
                                return;
                            },
                            url_result = ws_trigger.get_url_from_runnable(path, url.starts_with("$flow:"), &db, rsmq.clone()) => match url_result {
                                Ok(url) => Cow::Owned(url),
                                Err(err) => {
                                    ws_trigger.disable_with_error(&db, format!(
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
                        ws_trigger
                            .disable_with_error(
                                &db,
                                format!("Invalid websocket runnable path: {}", url),
                            )
                            .await;
                        return;
                    }
                } else {
                    Cow::Borrowed(url)
                }
            }
            WebsocketEnum::Capture(capture) => Cow::Borrowed(&capture.trigger_config.url),
        };

        tokio::select! {
            biased;
            _ = killpill_rx.recv() => {
                return;
            },
            _ = loop_ping(&db, &ws, Some("Connecting...")) => {
                return;
            },
            connection = connect_async(connect_url.as_ref()) => {
                match connection {
                    Ok((ws_stream, _)) => {
                        tracing::info!("Listening to websocket {}", url);
                        if let None = ws.update_ping(&db, None).await {
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
                                match &ws {
                                    WebsocketEnum::Trigger(ws_trigger) => {
                                        if let Err(err) = ws_trigger.send_initial_messages(writer, &db, rsmq.clone()).await {
                                            ws_trigger.disable_with_error(&db, format!("Error sending initial messages: {:?}", err)).await;
                                        } else {
                                            // if initial messages sent successfully, wait forever
                                            futures::future::pending::<()>().await;
                                        }
                                    },
                                    WebsocketEnum::Capture(_) => {
                                        futures::future::pending::<()>().await;
                                    }
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
                                                    if let None = ws.update_ping(&db, None).await {
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

                                                                    let args = HashMap::from([("msg".to_string(), to_raw_value(&text))]);
                                                                    let extra = Some(HashMap::from([(
                                                                        "wm_trigger".to_string(),
                                                                        to_raw_value(&serde_json::json!({"kind": "websocket", "websocket": { "url": url }})),
                                                                    )]));

                                                                    let args = PushArgsOwned { args, extra };
                                                                    match &ws {
                                                                        WebsocketEnum::Trigger(ws_trigger) => {
                                                                            ws_trigger.handle(&db, rsmq.clone(), args).await;
                                                                        },
                                                                        WebsocketEnum::Capture(capture) => {
                                                                            capture.handle(&db, args).await;
                                                                        }
                                                                    }
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
                                                if let None = ws.update_ping(&db, Some("Websocket closed")).await {
                                                    return;
                                                }
                                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                                break;
                                            }
                                        },
                                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                                            if let None = ws.update_ping(&db, None).await {
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
                        if let None = ws.update_ping(&db, Some(err.to_string().as_str())).await {
                            return;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn run_job(
    db: &DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    trigger: &WebsocketTrigger,
    args: PushArgsOwned,
) -> anyhow::Result<()> {
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
            rsmq,
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
            rsmq,
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
