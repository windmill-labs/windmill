use anyhow::Context;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{value::RawValue, Value};
use sqlx::prelude::FromRow;
use sqlx::types::Json as SqlxJson;
use windmill_queue::PushArgsOwned;
use std::{collections::HashMap, fmt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use windmill_common::{
    error::{self, to_anyhow},
    triggers::TriggerKind,
    utils::report_critical_error,
    worker::to_raw_value,
    INSTANCE_NAME,
};

use crate::{
    capture::{insert_capture_payload, WebsocketTriggerConfig},
    db::{ApiAuthed, DB},
    trigger_helpers::{trigger_runnable, trigger_runnable_and_wait_for_raw_result, TriggerJobArgs},
    users::fetch_api_authed,
};

use std::borrow::Cow;

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
    pub workspace_id: String,
    pub path: String,
    pub url: String,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub extra_perms: serde_json::Value,
    pub error: Option<String>,
    pub enabled: bool,
    pub filters: Vec<SqlxJson<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<SqlxJson<Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_runnable_args: Option<SqlxJson<Box<RawValue>>>,
    pub can_return_message: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<SqlxJson<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SqlxJson<windmill_common::flows::Retry>>,
}

async fn listen_to_unlistened_websockets(
    db: &DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let websocket_triggers = sqlx::query_as::<_, WebsocketTrigger>(
        r#"
    SELECT
        workspace_id,
        path,
        url,
        script_path,
        is_flow,
        edited_by,
        email,
        edited_at,
        server_id,
        last_server_ping,
        extra_perms,
        error,
        enabled,
        filters,
        initial_messages,
        url_runnable_args,
        can_return_message,
        error_handler_path,
        error_handler_args,
        retry
    FROM websocket_trigger
    WHERE
        enabled IS TRUE
        AND (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')
    "#,
    )
    .fetch_all(db)
    .await;

    match websocket_triggers {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::rng());
            for trigger in triggers {
                trigger
                    .maybe_listen_to_websocket(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching WebSocket triggers: {:?}", err);
        }
    };

    match sqlx::query_as!(
        CaptureConfigForWebsocket,
        r#"SELECT path, is_flow, workspace_id, trigger_config as "trigger_config!: _", owner, email FROM capture_config WHERE trigger_kind = 'websocket' AND last_client_ping > NOW() - INTERVAL '10 seconds' AND trigger_config IS NOT NULL AND (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')"#
    )
    .fetch_all(db)
    .await
    {
        Ok(mut captures) => {
            captures.shuffle(&mut rand::rng());
            for capture in captures {
                capture.maybe_listen_to_websocket(db.clone(),  killpill_rx.resubscribe()).await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching capture WebSocket triggers: {:?}", err);
        }
    }
}

pub fn start_websockets(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) -> () {
    tokio::spawn(async move {
        listen_to_unlistened_websockets(&db, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_websockets(&db, &killpill_rx).await;
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

fn raw_value_to_args_hashmap(
    args: Option<&Box<RawValue>>,
) -> error::Result<HashMap<String, Box<RawValue>>> {
    let args = if let Some(args) = args {
        serde_json::from_str::<Option<HashMap<String, Box<RawValue>>>>(args.get())
            .map_err(|e| error::Error::BadRequest(format!("invalid json: {}", e)))?
            .unwrap_or_else(HashMap::new)
    } else {
        HashMap::new()
    };
    Ok(args)
}

async fn loop_ping(db: &DB, ws: &WebsocketEnum, error: Option<&str>) -> () {
    loop {
        if let None = ws.update_ping(db, error).await {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn get_url_from_runnable(
    path: &str,
    is_flow: bool,
    db: &DB,
    authed: ApiAuthed,
    args: Option<&Box<RawValue>>,
    workspace_id: &str,
) -> error::Result<String> {
    tracing::info!(
        "Running {} {} to get WebSocket URL",
        if is_flow { "flow" } else { "script" },
        path
    );

    let args = raw_value_to_args_hashmap(args)?;

    let result = trigger_runnable_and_wait_for_raw_result(
        db,
        None,
        authed,
        workspace_id,
        path,
        is_flow,
        PushArgsOwned { args, extra: None },
        None,
        None,
        None,
        "".to_string(), // doesn't matter as no retry/error handler
    )
    .await?;

    serde_json::from_str::<String>(result.get()).map_err(|_| {
        error::Error::BadConfig(format!(
            "{} {} did not return a string",
            if is_flow { "Flow" } else { "Script" },
            path,
        ))
    })
}

impl WebsocketTrigger {
    async fn maybe_listen_to_websocket(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        match sqlx::query_scalar!(
            "UPDATE websocket_trigger SET server_id = $1, last_server_ping = now(), error = 'Connecting...' WHERE enabled IS TRUE AND workspace_id = $2 AND path = $3 AND (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
        ).fetch_optional(&db).await {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tokio::spawn(listen_to_websocket(WebsocketEnum::Trigger(self), db, killpill_rx));
                } else {
                    tracing::info!("WebSocket {} already being listened to", self.url);
                }
            },
            Err(err) => {
                tracing::error!("Error acquiring lock for WebSocket {}: {:?}", self.path, err);
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
                // allow faster restart of websocket trigger
                sqlx::query!(
                    "UPDATE websocket_trigger SET last_server_ping = NULL WHERE workspace_id = $1 AND path = $2 AND server_id IS NULL",
                    self.workspace_id,
                    self.path,
                ).execute(db).await.ok();
                tracing::info!("WebSocket {} changed, disabled, or deleted, stopping...", self.url); 
                return None;
            }
        },
            Err(err) => {
                tracing::warn!("Error updating ping of WebSocket {}: {:?}", self.url, err);
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
                report_critical_error(format!("Disabling WebSocket {} because of error: {}", self.url, error), db.clone(), Some(&self.workspace_id), None).await;
            },
            Err(disable_err) => {
                report_critical_error(
                    format!("Could not disable WebSocket {} with err {}, disabling because of error {}", self.path, disable_err, error), 
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
    ) -> error::Result<String> {
        get_url_from_runnable(
            &path,
            is_flow,
            db,
            self.fetch_authed(db).await?,
            self.url_runnable_args.as_ref().map(|r| &r.0),
            &self.workspace_id,
        )
        .await
    }

    async fn send_initial_messages(
        &self,
        writer: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        db: &DB,
    ) -> error::Result<()> {
        let initial_messages: Vec<InitialMessage> = self
            .initial_messages
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|m| serde_json::from_str(m.get()).ok())
            .collect_vec();

        let mut authed_o = None;
        for start_message in initial_messages {
            match start_message {
                InitialMessage::RawMessage(msg) => {
                    let msg = if msg.starts_with("\"") && msg.ends_with("\"") {
                        msg[1..msg.len() - 1].to_string()
                    } else {
                        msg
                    };
                    tracing::info!(
                        "Sending raw message initial message to WebSocket {}: {}",
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
                        "Running {} {} for initial message to WebSocket {}",
                        if is_flow { "flow" } else { "script" },
                        path,
                        self.url,
                    );

                    let args = raw_value_to_args_hashmap(Some(&args))?;

                    if authed_o.is_none() {
                        authed_o = Some(self.fetch_authed(db).await?);
                    }
                    let authed = authed_o.clone().unwrap();

                    let result = trigger_runnable_and_wait_for_raw_result(
                        db,
                        None,
                        authed.clone(),
                        &self.workspace_id,
                        &path,
                        is_flow,
                        PushArgsOwned { args, extra: None },
                        None,
                        None,
                        None,
                        "".to_string(), // doesn't matter as no retry/error handler
                    )
                    .await
                    .map(|r| r.get().to_owned())?;

                    tracing::info!(
                        "Sending {} {} result to WebSocket {}",
                        if is_flow { "flow" } else { "script" },
                        path,
                        self.url
                    );

                    // if the `result` was just a single string, the below removes the surrounding quotes by parsing it as a string.
                    // it falls back to the original serialized JSON if it doesn't work.
                    let result = serde_json::from_str::<String>(result.as_str()).unwrap_or(result);

                    writer
                        .send(tokio_tungstenite::tungstenite::Message::Text(result))
                        .await
                        .map_err(to_anyhow)
                        .with_context(|| {
                            format!(
                                "Failed to send {} {} result",
                                if is_flow { "flow" } else { "script" },
                                path
                            )
                        })?;
                }
            }
        }

        Ok(())
    }

    async fn handle(
        &self,
        db: &DB,
        msg: &str,
        trigger_info: HashMap<String, Box<RawValue>>,
        return_message_channels: Option<ReturnMessageChannels>,
    ) -> () {
        if let Err(err) = run_job(db, self, &msg, trigger_info, return_message_channels).await {
            report_critical_error(
                format!(
                    "Failed to trigger job from WebSocket {}: {:?}",
                    self.url, err
                ),
                db.clone(),
                Some(&self.workspace_id),
                None,
            )
            .await;
        };
    }

    async fn fetch_authed(&self, db: &DB) -> error::Result<ApiAuthed> {
        fetch_api_authed(
            self.edited_by.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("ws-{}", self.path)),
        )
        .await
    }
}

impl TriggerJobArgs<&str> for WebsocketTrigger {
    fn v1_payload_fn(payload: &str) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("msg".to_string(), to_raw_value(&payload))])
    }

    fn trigger_kind() -> TriggerKind {
        TriggerKind::Websocket
    }
}

#[derive(Deserialize)]
struct CaptureConfigForWebsocket {
    trigger_config: SqlxJson<WebsocketTriggerConfig>,
    path: String,
    is_flow: bool,
    workspace_id: String,
    owner: String,
    email: String,
}

impl CaptureConfigForWebsocket {
    async fn maybe_listen_to_websocket(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        match sqlx::query_scalar!(
            "UPDATE capture_config SET server_id = $1, last_server_ping = now(), error = 'Connecting...' WHERE last_client_ping > NOW() - INTERVAL '10 seconds' AND workspace_id = $2 AND path = $3 AND is_flow = $4 AND trigger_kind = 'websocket' AND (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
            self.is_flow,
        ).fetch_optional(&db).await {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tokio::spawn(listen_to_websocket(WebsocketEnum::Capture(self), db, killpill_rx));
                } else {
                    tracing::info!("WebSocket {} already being listened to", self.trigger_config.url);
                }
            },
            Err(err) => {
                tracing::error!("Error acquiring lock for capture WebSocket {}: {:?}", self.path, err);
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
                // allow faster restart of websocket capture
                sqlx::query!(
                    "UPDATE capture_config SET last_server_ping = NULL WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = 'websocket' AND server_id IS NULL",
                    self.workspace_id,
                    self.path,
                    self.is_flow,
                ).execute(db).await.ok();
                tracing::info!("WebSocket capture {} changed, disabled, or deleted, stopping...", self.trigger_config.url); 
                return None;
            }
        },
            Err(err) => {
                tracing::warn!("Error updating ping of capture WebSocket {}: {:?}", self.trigger_config.url, err);
            }
        };

        Some(())
    }

    async fn handle(&self, db: &DB, msg: &str, trigger_info: HashMap<String, Box<RawValue>>) -> () {
        let (main_args, preprocessor_args) =
            WebsocketTrigger::build_capture_payloads(&msg, trigger_info);

        if let Err(err) = insert_capture_payload(
            db,
            &self.workspace_id,
            &self.path,
            self.is_flow,
            &TriggerKind::Websocket,
            main_args,
            preprocessor_args,
            &self.owner,
        )
        .await
        {
            tracing::error!("Error inserting capture payload: {:?}", err);
        }
    }

    async fn get_url_from_runnable(
        &self,
        path: &str,
        is_flow: bool,
        db: &DB,
    ) -> error::Result<String> {
        let url_runnable_args = self
            .trigger_config
            .url_runnable_args
            .as_ref()
            .map(to_raw_value);
        get_url_from_runnable(
            &path,
            is_flow,
            db,
            self.fetch_authed(db).await?,
            url_runnable_args.as_ref(),
            &self.workspace_id,
        )
        .await
    }

    async fn fetch_authed(&self, db: &DB) -> error::Result<ApiAuthed> {
        fetch_api_authed(
            self.owner.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("ws-{}", self.get_trigger_path())),
        )
        .await
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        if let Err(err) = sqlx::query!(
            "UPDATE capture_config SET error = $1, server_id = NULL, last_server_ping = NULL WHERE workspace_id = $2 AND path = $3 AND is_flow = $4 AND trigger_kind = 'websocket'",
            error,
            self.workspace_id,
            self.path,
            self.is_flow,
        )
        .execute(db).await {
            tracing::error!("Could not disable WebSocket capture {} ({}) with err {}, disabling because of error {}", self.path, self.workspace_id, err, error);
        }
    }

    fn get_trigger_path(&self) -> String {
        format!(
            "{}-{}",
            if self.is_flow { "flow" } else { "script" },
            self.path
        )
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

    async fn get_url_from_runnable(
        &self,
        path: &str,
        is_flow: bool,
        db: &DB,
    ) -> error::Result<String> {
        match self {
            WebsocketEnum::Trigger(ws) => ws.get_url_from_runnable(path, is_flow, db).await,
            WebsocketEnum::Capture(capture) => {
                capture.get_url_from_runnable(path, is_flow, db).await
            }
        }
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match self {
            WebsocketEnum::Trigger(ws) => ws.disable_with_error(db, error).await,
            WebsocketEnum::Capture(capture) => capture.disable_with_error(db, error).await,
        }
    }
}

struct ReturnMessageChannels {
    send_message_tx: tokio::sync::mpsc::Sender<String>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
}

impl Clone for ReturnMessageChannels {
    fn clone(&self) -> Self {
        Self {
            send_message_tx: self.send_message_tx.clone(),
            killpill_rx: self.killpill_rx.resubscribe(),
        }
    }
}

async fn listen_to_websocket(
    ws: WebsocketEnum,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    let url = match &ws {
        WebsocketEnum::Trigger(ws_trigger) => ws_trigger.url.clone(),
        WebsocketEnum::Capture(capture) => capture.trigger_config.url.clone(),
    };

    let filters: Vec<Filter> = match &ws {
        WebsocketEnum::Trigger(ws_trigger) => ws_trigger
            .filters
            .iter()
            .filter_map(|m| serde_json::from_str(m.get()).ok())
            .collect_vec(),
        WebsocketEnum::Capture(_) => vec![],
    };

    let connect_url: Cow<str> = if url.starts_with("$") {
        if url.starts_with("$flow:") || url.starts_with("$script:") {
            let path = url.splitn(2, ':').nth(1).unwrap();
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                },
                _ = loop_ping(&db, &ws, Some(
                    "Waiting on runnable to return WebSocket URL..."
                )) => {
                    return;
                },

                url_result = ws.get_url_from_runnable(path, url.starts_with("$flow:"), &db) => match url_result {
                    Ok(url) => Cow::Owned(url),
                    Err(err) => {
                        ws.disable_with_error(&db, format!(
                                "Error getting WebSocket URL from runnable after 5 tries: {:?}",
                                err
                            ),
                        )
                        .await;
                        return;
                    }
                },
            }
        } else {
            ws.disable_with_error(&db, format!("Invalid WebSocket runnable path: {}", url))
                .await;
            return;
        }
    } else {
        Cow::Borrowed(&url)
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
                    tracing::info!("Connected to WebSocket {}", url);
                    let (mut writer, mut reader) = ws_stream.split();

                    // send initial messages
                    match &ws {
                        WebsocketEnum::Trigger(ws_trigger) => {
                            tokio::select! {
                                biased;
                                _ = killpill_rx.recv() => {
                                    return;
                                },
                                _ = loop_ping(&db, &ws, Some("Sending initial messages...")) => {
                                    return;
                                },
                                result = ws_trigger.send_initial_messages(&mut writer, &db) => {
                                    if let Err(err) = result {
                                        ws_trigger.disable_with_error(&db, format!("Error sending initial messages: {:?}", err)).await;
                                        return
                                    } else {
                                        tracing::debug!("Initial messages sent successfully to WebSocket {}", url);
                                    }
                                }
                            }
                        },
                        _ => {
                        }
                    }

                    let (return_message_channels, message_sender_handle) = match &ws {
                        WebsocketEnum::Trigger(ws_trigger) if ws_trigger.can_return_message => {
                            let (send_message_tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
                            let w_id = ws_trigger.workspace_id.clone();
                            let url = ws_trigger.url.clone();
                            let db = db.clone();
                            let handle = tokio::spawn(async move {
                                while let Some(message) = rx.recv().await {
                                    if let Err(err) = writer.send(tokio_tungstenite::tungstenite::Message::Text(message)).await {
                                        report_critical_error(format!("Could not send runnable result to WebSocket {} because of error: {}", url, err), db.clone(), Some(&w_id), None).await;
                                    }
                                }
                            });

                            let killpill_rx = killpill_rx.resubscribe();

                            let return_message_channels = ReturnMessageChannels {
                                send_message_tx,
                                killpill_rx
                            };

                            (Some(return_message_channels), Some(handle))
                        },
                        _ => (None, None)
                    };

                    tokio::select! {
                        biased;
                        _ = killpill_rx.recv() => {},
                        _ = loop_ping(&db, &ws, None) => {},
                        _ = async {
                            loop {
                                if let Some(msg) = reader.next().await {
                                    match msg {
                                        Ok(msg) => {
                                            match msg {
                                                tokio_tungstenite::tungstenite::Message::Text(text) => {
                                                    tracing::debug!("Received text message from WebSocket {}: {}", url, text);
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
                                                                        tracing::warn!("Error deserializing filter for WebSocket {}: {:?}", url, err);
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
                                                        let trigger_info = HashMap::from([
                                                            ("url".to_string(), to_raw_value(&url)),
                                                        ]);
                                                        match &ws {
                                                            WebsocketEnum::Trigger(ws_trigger) => {
                                                                ws_trigger.handle(&db, &text, trigger_info, return_message_channels.clone()).await;
                                                            },
                                                            WebsocketEnum::Capture(capture) => {
                                                                capture.handle(&db, &text, trigger_info).await;
                                                            },
                                                        }
                                                    }
                                                },
                                                a @ _ => {
                                                    tracing::debug!("Received non text-message from WebSocket {}: {:?}", url, a);
                                                }
                                            }
                                        },
                                        Err(err) => {
                                            tracing::error!("Error reading from WebSocket {}: {:?}", url, err);
                                        }
                                    }
                                } else {
                                    tracing::error!("WebSocket {} closed", url);
                                    ws.update_ping(&db, Some("WebSocket closed")).await;
                                    break;
                                }
                            }
                        } => {}
                    }
                    // make sure to stop return message handler
                    if let Some(message_sender_handle) = message_sender_handle {
                        message_sender_handle.abort();
                    }
                }
                Err(err) => {
                    tracing::error!("Error connecting to WebSocket {}: {:?}", url, err);
                    ws.update_ping(&db, Some(err.to_string().as_str())).await;
                }
            }
        }
    }
}

async fn run_job(
    db: &DB,
    trigger: &WebsocketTrigger,
    msg: &str,
    trigger_info: HashMap<String, Box<RawValue>>,
    return_message_channels: Option<ReturnMessageChannels>,
) -> anyhow::Result<()> {
    let args = WebsocketTrigger::build_job_args(
        &trigger.script_path,
        trigger.is_flow,
        &trigger.workspace_id,
        db,
        msg,
        trigger_info,
    )
    .await?;

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some(format!("ws-{}", trigger.path)),
    )
    .await?;

    if let Some(ReturnMessageChannels { send_message_tx, mut killpill_rx }) =
        return_message_channels
    {
        let db_ = db.clone();
        let url = trigger.url.clone();
        let script_path = trigger.script_path.clone();
        let is_flow = trigger.is_flow;
        let w_id = trigger.workspace_id.clone();
        let retry = trigger.retry.clone();
        let error_handler_path = trigger.error_handler_path.clone();
        let error_handler_args = trigger.error_handler_args.clone();
        let trigger_path = trigger.path.clone();
        let handle_response_f = async move {
            tokio::select! {
                _ = killpill_rx.recv() => {
                    return;
                },
                result = trigger_runnable_and_wait_for_raw_result(
                    &db_,
                    None,
                    authed,
                    &w_id,
                    &script_path,
                    is_flow,
                    args,
                    retry.as_ref(),
                    error_handler_path.as_deref(),
                    error_handler_args.as_ref(),
                    format!("websocket_trigger/{}", trigger_path),
                ) => {
                    if let Ok(result) = result.map(|r| r.get().to_owned()) {
                        // only send the result if it's not null
                        if result != "null" {
                            tracing::info!("Sending job result to WebSocket {}", url);
                            // if the `result` was just a single string, the below removes the surrounding quotes by parsing it as a string.
                            // it falls back to the original serialized JSON if it doesn't work.
                            let result = serde_json::from_str::<String>(result.as_str()).unwrap_or(result);
                            if let Err(err) = send_message_tx.send(result).await {
                                report_critical_error(format!("Could not send runnable result to WebSocket {} because of error: {}", url, err), db_.clone(), Some(&w_id), None).await;
                            }
                        }
                    }
                }
            };
        };

        tokio::spawn(handle_response_f);
    } else {
        trigger_runnable(
            db,
            None,
            authed,
            &trigger.workspace_id,
            &trigger.script_path,
            trigger.is_flow,
            args,
            trigger.retry.as_ref(),
            trigger.error_handler_path.as_deref(),
            trigger.error_handler_args.as_ref(),
            format!("websocket_trigger/{}", trigger.path),
        )
        .await?;
    }

    Ok(())
}
