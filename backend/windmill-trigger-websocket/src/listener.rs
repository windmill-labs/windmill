use super::{get_url_from_runnable_value, WebsocketConfig, WebsocketTrigger};
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use http::Response;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use tokio::{net::TcpStream, sync::RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use windmill_common::{
    error::{to_anyhow, Error, Result},
    jobs::JobTriggerKind,
    triggers::TriggerMetadata,
    utils::report_critical_error,
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;
use windmill_trigger::filter::{is_value_superset, Filter, JsonFilter};
use windmill_trigger::listener::ListeningTrigger;
use windmill_trigger::trigger_helpers::{
    trigger_runnable, trigger_runnable_and_wait_for_raw_result,
    trigger_runnable_and_wait_for_raw_result_with_error_ctx, TriggerJobArgs,
};
use windmill_trigger::Listener;

async fn send_initial_messages(
    listening_trigger: &ListeningTrigger<WebsocketConfig>,
    writer: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    db: &DB,
) -> Result<()> {
    let initial_messages: Vec<InitialMessage> = listening_trigger
        .trigger_config
        .initial_messages
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|m| serde_json::from_str(m.get()).ok())
        .collect_vec();

    let WebsocketConfig { ref url, .. } = listening_trigger.trigger_config;
    let runnable_kind = if listening_trigger.is_flow {
        "flow"
    } else {
        "script"
    };
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
                    url,
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
                    runnable_kind,
                    path,
                    url,
                );

                let args = raw_value_to_args_hashmap(Some(&args))?;

                if authed_o.is_none() {
                    authed_o = Some(listening_trigger.authed(db, "ws").await?);
                }
                let authed = authed_o.clone().unwrap();

                let result = trigger_runnable_and_wait_for_raw_result_with_error_ctx(
                    db,
                    None,
                    authed.clone(),
                    &listening_trigger.workspace_id,
                    &path,
                    is_flow,
                    PushArgsOwned { args, extra: None },
                    None,
                    None,
                    None,
                    "".to_string(), // doesn't matter as no retry/error handler
                    TriggerMetadata::new(
                        Some(listening_trigger.path.to_owned()),
                        JobTriggerKind::Websocket,
                    ),
                )
                .await
                .map(|r| r.get().to_owned())?;

                tracing::info!(
                    "Sending {} {} result to WebSocket {}",
                    runnable_kind,
                    path,
                    url
                );

                // if the `result` was just a single string, the below removes the surrounding quotes by parsing it as a string.
                // it falls back to the original serialized JSON if it doesn't work.
                let result = serde_json::from_str::<String>(result.as_str()).unwrap_or(result);

                writer
                    .send(tokio_tungstenite::tungstenite::Message::Text(result))
                    .await
                    .map_err(to_anyhow)
                    .with_context(|| format!("Failed to send {} {} result", runnable_kind, path))?;
            }
        }
    }
    Ok(())
}

#[async_trait]
impl Listener for WebsocketTrigger {
    type Consumer = (
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        Response<Option<Vec<u8>>>,
    );
    type Extra = ReturnMessageChannels;
    type ExtraState = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Websocket;
    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        let url = &listening_trigger.trigger_config.url;
        let connect_url: Cow<str> = if url.starts_with("$") {
            if url.starts_with("$flow:") || url.starts_with("$script:") {
                tokio::select! {
                    biased;
                    _ = killpill_rx.recv() => {
                        return Ok(None);
                    },
                    _ = self.loop_ping(&db, listening_trigger, err_message.clone(), Some(
                    "Waiting on runnable to return WebSocket URL...".to_string()
                )) => {
                        return Ok(None);
                    },
                    url_result = {
                        let authed = listening_trigger.authed(db, "ws").await?;
                        let args = listening_trigger.trigger_config.url_runnable_args.as_ref().map(|r| &r.0);
                        let path = url.splitn(2, ':').nth(1).unwrap();
                        get_url_from_runnable_value(path, url.starts_with("$flow:"), db, authed, args, &listening_trigger.workspace_id)
                    } => match url_result {
                        Ok(url) => Cow::Owned(url),
                        Err(err) => {
                            return Err(anyhow::anyhow!("Error getting WebSocket URL from runnable after 5 tries: {:?}", err).into());
                        }
                    },
                }
            } else {
                return Err(anyhow::anyhow!("Invalid WebSocket runnable path: {}", url).into());
            }
        } else {
            Cow::Borrowed(&url)
        };

        let connection = connect_async(&*connect_url)
            .await
            .map(|conn| Some(conn))
            .map_err(|err| to_anyhow(err).into());

        connection
    }
    async fn consume(
        &self,
        db: &DB,
        consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
        _extra_state: Option<&Self::ExtraState>,
    ) {
        let WebsocketConfig { ref url, .. } = listening_trigger.trigger_config;

        tracing::info!("Connected to WebSocket {}", url);

        let (ws_stream, _) = consumer;

        let (mut writer, mut reader) = ws_stream.split();

        // send initial messages
        if listening_trigger.trigger_mode {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                },
                _ = self.loop_ping(db, listening_trigger, err_message.clone(), Some("Sending initial messages...".to_string())) => {
                    return;
                },
                result = send_initial_messages(listening_trigger, &mut writer, &db) => {
                    if let Err(err) = result {
                        self.disable_with_error(&db, listening_trigger, format!("Error sending initial messages: {:?}", err)).await;
                        return
                    } else {
                        tracing::debug!("Initial messages sent successfully to WebSocket {}", url);
                    }
                }
            }
        }

        let (return_message_channels, message_sender_handle) = if listening_trigger.trigger_mode
            && listening_trigger.trigger_config.can_return_message
        {
            let (send_message_tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
            let w_id = listening_trigger.workspace_id.clone();
            let url = url.clone();
            let db = db.clone();
            let handle = tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if let Err(err) = writer
                        .send(tokio_tungstenite::tungstenite::Message::Text(message))
                        .await
                    {
                        report_critical_error(format!("Could not send runnable result to WebSocket {} because of error: {}", url, err), db.clone(), Some(&w_id), None).await;
                    }
                }
            });

            let killpill_rx = killpill_rx.resubscribe();

            let return_message_channels = ReturnMessageChannels { send_message_tx, killpill_rx };

            (Some(return_message_channels), Some(handle))
        } else {
            (None, None)
        };

        tokio::select! {
            biased;
            _ = killpill_rx.recv() => {
            },
            _ = self.loop_ping(db, listening_trigger, err_message.clone(), None) => {
            },
            _ = async {
                    let filters: Vec<Filter> = if listening_trigger.trigger_mode {
                        listening_trigger
                            .trigger_config
                            .filters
                            .iter()
                            .filter_map(|m| serde_json::from_str(m.get()).ok())
                            .collect_vec()
                    } else {
                        vec![]
                    };
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
                                                ("url".to_string(), to_raw_value(&listening_trigger.trigger_config.url)),
                                            ]);
                                            let _ = self.handle_event(db, listening_trigger, text, trigger_info, return_message_channels.clone()).await;
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
                        self.update_ping_and_loop_ping_status(db, listening_trigger, err_message.clone(), Some("WebSocket closed".to_string())).await;
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

    async fn handle_trigger(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        payload: Self::Payload,
        trigger_info: HashMap<String, Box<RawValue>>,
        extra: Option<Self::Extra>,
    ) -> Result<()> {
        let ListeningTrigger {
            path,
            is_flow,
            workspace_id,
            trigger_config,
            script_path,
            error_handling,
            suspended_mode,
            ..
        } = listening_trigger;

        let WebsocketConfig { url, .. } = trigger_config;

        let args = WebsocketTrigger::build_job_args(
            &script_path,
            *is_flow,
            workspace_id,
            db,
            payload,
            trigger_info,
        )
        .await;

        let args = match args {
            Ok(args) => args,
            Err(err) => {
                return Err(err);
            }
        };

        let authed = listening_trigger.authed(db, "ws").await?;

        let (retry, error_handler_path, error_handler_args) = match error_handling.as_ref() {
            Some(error_handling) => (
                error_handling.retry.as_ref(),
                error_handling.error_handler_path.as_deref(),
                error_handling.error_handler_args.as_ref(),
            ),
            None => (None, None, None),
        };
        let trigger = TriggerMetadata::new(Some(path.to_owned()), Self::JOB_TRIGGER_KIND);
        if *suspended_mode || extra.is_none() {
            trigger_runnable(
                db,
                None,
                authed,
                &workspace_id,
                &script_path,
                *is_flow,
                args,
                retry,
                error_handler_path,
                error_handler_args,
                format!("websocket_trigger/{}", listening_trigger.path),
                None,
                *suspended_mode,
                trigger,
            )
            .await?;
        } else if let Some(ReturnMessageChannels { send_message_tx, mut killpill_rx }) = extra {
            let db_ = db.clone();
            let url = url.to_owned();
            let script_path = script_path.to_owned();
            let is_flow = *is_flow;
            let w_id = workspace_id.to_owned();
            let retry = retry.cloned();
            let error_handler_path = error_handler_path.map(|s| s.to_string());
            let error_handler_args = error_handler_args.cloned();
            let trigger_path = path.clone();
            let can_return_error_result = trigger_config.can_return_error_result;
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
                        trigger,
                    ) => {
                        if let Ok((result, success)) = result {
                            if !success && !can_return_error_result {
                                return;
                            }
                            let result = result.get().to_owned();
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
        }

        Ok(())
    }
}

pub struct ReturnMessageChannels {
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

#[derive(Debug, Deserialize)]
enum InitialMessage {
    #[serde(rename = "raw_message")]
    RawMessage(String),
    #[serde(rename = "runnable_result")]
    RunnableResult { path: String, args: Box<RawValue>, is_flow: bool },
}

fn raw_value_to_args_hashmap(
    args: Option<&Box<RawValue>>,
) -> Result<HashMap<String, Box<RawValue>>> {
    let args = if let Some(args) = args {
        serde_json::from_str::<Option<HashMap<String, Box<RawValue>>>>(args.get())
            .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?
            .unwrap_or_else(HashMap::new)
    } else {
        HashMap::new()
    };
    Ok(args)
}
