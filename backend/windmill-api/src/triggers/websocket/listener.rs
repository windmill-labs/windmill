use std::{borrow::Cow, collections::HashMap, sync::Arc};

use super::WebsocketTrigger;
use crate::triggers::{
    listener::ListeningTrigger, trigger_helpers::trigger_runnable_and_wait_for_raw_result,
    websocket::WebsocketConfig, Listener,
};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use http::Response;
use serde_json::value::RawValue;
use tokio::{net::TcpStream, sync::RwLock};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use windmill_common::{
    error::{to_anyhow, Error, Result},
    jobs::JobTriggerKind,
    utils::report_critical_error,
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;

impl ListeningTrigger<WebsocketConfig> {
    async fn get_url_from_runnable(&self, db: &DB) -> Result<String> {
        let runnable_kind = if self.is_flow { "Flow" } else { "Script" };
        tracing::info!(
            "Running {} {} to get WebSocket URL",
            runnable_kind.to_lowercase(),
            self.path
        );

        let authed = self.authed(db, "ws").await?;

        let args = raw_value_to_args_hashmap(
            self.trigger_config.url_runnable_args.as_ref().map(|r| &r.0),
        )?;

        let result = trigger_runnable_and_wait_for_raw_result(
            db,
            None,
            authed,
            &self.workspace_id,
            &self.path,
            self.is_flow,
            PushArgsOwned { args, extra: None },
            None,
            None,
            None,
            "".to_string(), // doesn't matter as no retry/error handler
        )
        .await?;

        serde_json::from_str::<String>(result.get()).map_err(|_| {
            Error::BadConfig(format!(
                "{} {} did not return a string",
                runnable_kind, self.path,
            ))
        })
    }
}

#[async_trait]
impl Listener for WebsocketTrigger {
    type Consumer = (
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        Response<Option<Vec<u8>>>,
    );
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
                let path = url.splitn(2, ':').nth(1).unwrap();
                tokio::select! {
                    biased;
                    _ = killpill_rx.recv() => {
                        return Ok(None);
                    },
                    _ = self.loop_ping(&db, listening_trigger, err_message.clone()) => {
                        return Ok(None);
                    },

                    url_result = listening_trigger.get_url_from_runnable(&db) => match url_result {
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

        let connection = connect_async(connect_url.as_ref())
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
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) {
        tracing::info!(
            "Connected to WebSocket {}",
            &listening_trigger.trigger_config.url
        );

        let (ws_stream, _) = consumer;

        let (mut writer, mut reader) = ws_stream.split();

        let WebsocketConfig { ref url, .. } = listening_trigger.trigger_config;

        // send initial messages
        if !listening_trigger.capture_mode {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                },
                _ = self.loop_ping(&db, listening_trigger, Some("Sending initial messages...")) => {
                    return;
                },
                result = ws_trigger.send_initial_messages(&mut writer, &db) => {
                    if let Err(err) = result {
                        self.disable_with_error(&db, listening_trigger, format!("Error sending initial messages: {:?}", err)).await;
                        return
                    } else {
                        tracing::debug!("Initial messages sent successfully to WebSocket {}", url);
                    }
                }
            }
        }

        let (return_message_channels, message_sender_handle) = if !listening_trigger.capture_mode {
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
                        report_critical_error(format!("Could not send runnable result to WebSocket {} because of error: {}", url, err), db, Some(&w_id), None).await;
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
                                                ("url".to_string(), to_raw_value(&listening_trigger.trigger_config.url)),
                                            ]);
                                            self.handle_event(db, listening_trigger, text, trigger_info).await;

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
}

struct ReturnMessageChannels {
    send_message_tx: tokio::sync::mpsc::Sender<String>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
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
