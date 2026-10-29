use super::{
    check_websocket_url_for_ssrf, get_url_from_runnable_value, proxy::connect_async_with_proxy,
    ssrf_error_to_error, WebsocketConfig, WebsocketTrigger,
};
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use http::Response;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, RwLock},
    time::Instant,
};
use tokio_tungstenite::{tungstenite, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use windmill_common::ssrf::SsrfValidationError;
use windmill_common::{
    error::{to_anyhow, Error, Result},
    jobs::JobTriggerKind,
    triggers::TriggerMetadata,
    utils::report_critical_error,
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;
use windmill_trigger::filter::CompiledFilters;
use windmill_trigger::listener::{reconnect_delay, update_rw_lock, ListeningTrigger};
use windmill_trigger::trigger_helpers::{
    trigger_runnable, trigger_runnable_and_wait_for_raw_result,
    trigger_runnable_and_wait_for_raw_result_with_error_ctx, TriggerJobArgs,
};
use windmill_trigger::Listener;

/// Unit of the connect backoff (see [`reconnect_delay`]). Tests shrink it to avoid
/// waiting out real seconds; nothing else writes it.
#[doc(hidden)]
pub static CONNECT_RETRY_BASE_DELAY_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1000);

/// How often an open connection is pinged. A connection that delivers no frame at all
/// (message, ping or pong) for [`MISSED_PINGS_BEFORE_RECONNECT`] intervals is presumed
/// dead and reopened: a peer that vanished without a close (NAT timeout, dropped route)
/// is otherwise never noticed, since the read just never returns. Tests shrink it;
/// nothing else writes it.
#[doc(hidden)]
pub static PING_INTERVAL_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(30_000);
const MISSED_PINGS_BEFORE_RECONNECT: u32 = 4;

/// How long a stopping listener waits to hand the server its Close frame.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

struct ConnectFailure {
    error: String,
    /// Whether the same attempt can succeed later. Only transient failures are
    /// retried: anything else disables the trigger, so it must be a failure no retry
    /// can fix (a rejected URL, a missing runnable, a 4xx upgrade response).
    transient: bool,
}

/// Network-level IO errors and HTTP 5xx/408/425/429 handshake responses are
/// typically transient (edge proxies like Cloudflare return 502/520 sporadically,
/// and a gateway under load answers 408), while other errors (bad URL, protocol
/// mismatch, other 4xx) point at configuration and would fail identically on every
/// attempt.
fn is_transient_connect_error(err: &tungstenite::Error) -> bool {
    match err {
        tungstenite::Error::Io(_) => true,
        tungstenite::Error::Http(resp) => {
            resp.status().is_server_error() || matches!(resp.status().as_u16(), 408 | 425 | 429)
        }
        _ => false,
    }
}

/// A URL runnable that ran and failed, or whose result could not be awaited (the
/// result wait times out, the DB blips), may succeed on the next run: worker group
/// saturated, upstream API down. Anything else (missing runnable, no permission,
/// non-string result, workers too old) fails the same way every time. An allowlist, so
/// that a new error kind disables rather than re-running a job every few minutes forever.
fn is_transient_runnable_error(err: &Error) -> bool {
    matches!(
        err,
        Error::InternalErr(_)
            | Error::InternalErrLoc { .. }
            | Error::InternalErrAt(..)
            | Error::Anyhow { .. }
            | Error::SqlErr { .. }
            | Error::ConnectingToDatabase(_)
            | Error::BadGateway(_)
    )
}

async fn connect_once(
    db: &DB,
    listening_trigger: &ListeningTrigger<WebsocketConfig>,
    err_message: Arc<RwLock<Option<String>>>,
) -> std::result::Result<<WebsocketTrigger as Listener>::Consumer, ConnectFailure> {
    let url = &listening_trigger.trigger_config.url;
    let connect_url: Cow<str> = if url.starts_with("$") {
        let Some((kind, path)) = url
            .split_once(':')
            .filter(|(kind, _)| *kind == "$flow" || *kind == "$script")
        else {
            return Err(ConnectFailure {
                error: format!("Invalid WebSocket runnable path: {}", url),
                transient: false,
            });
        };
        update_rw_lock(
            err_message,
            Some("Waiting on runnable to return WebSocket URL...".to_string()),
        )
        .await;
        let args = listening_trigger
            .trigger_config
            .url_runnable_args
            .as_ref()
            .map(|r| &r.0);
        let url = async {
            let authed = listening_trigger.authed(db, "ws").await?;
            get_url_from_runnable_value(
                path,
                kind == "$flow",
                db,
                authed,
                args,
                &listening_trigger.workspace_id,
            )
            .await
        }
        .await
        .map_err(|err| ConnectFailure {
            transient: is_transient_runnable_error(&err),
            error: format!("Error getting WebSocket URL from runnable: {}", err),
        })?;
        Cow::Owned(url)
    } else {
        Cow::Borrowed(url)
    };

    let validated = check_websocket_url_for_ssrf(&connect_url)
        .await
        .map_err(|err| ConnectFailure {
            transient: matches!(
                err,
                SsrfValidationError::ResolutionFailed { .. } | SsrfValidationError::NoAddresses(_)
            ),
            error: ssrf_error_to_error(err).to_string(),
        })?;

    connect_async_with_proxy(&*connect_url, validated.pinned_addrs())
        .await
        .map_err(|err| ConnectFailure {
            transient: is_transient_connect_error(&err),
            error: format!("Error connecting to WebSocket: {}", err),
        })
}

/// Owns the write half of a connection. Ends when `close_rx` resolves, which its
/// sender's drop does: whether the consumer returns or is cancelled (killpill, lost
/// lease), the server then gets a Close frame instead of a vanished peer it may keep
/// counting against a per-client connection limit until its own timeout.
async fn write_messages(
    mut writer: WsWriter,
    mut rx: mpsc::Receiver<Message>,
    close_rx: oneshot::Receiver<()>,
    url: String,
    db: DB,
    w_id: String,
) {
    let forward = async {
        while let Some(message) = rx.recv().await {
            let is_ping = matches!(message, Message::Ping(_));
            if let Err(err) = writer.send(message).await {
                if is_ping {
                    tracing::warn!("Could not ping WebSocket {}: {}", url, err);
                } else {
                    report_critical_error(
                        format!(
                            "Could not send runnable result to WebSocket {} because of error: {}",
                            url, err
                        ),
                        db.clone(),
                        Some(&w_id),
                        None,
                    )
                    .await;
                }
            }
        }
    };
    tokio::select! {
        _ = close_rx => {},
        _ = forward => {},
    }
    let close = Message::Close(Some(tungstenite::protocol::CloseFrame {
        code: tungstenite::protocol::frame::coding::CloseCode::Normal,
        reason: "".into(),
    }));
    match tokio::time::timeout(CLOSE_TIMEOUT, writer.send(close)).await {
        Ok(Ok(())) => tracing::debug!("Closed WebSocket {}", url),
        Ok(Err(err)) => tracing::debug!("Error closing WebSocket {}: {}", url, err),
        Err(_) => tracing::debug!("Timed out closing WebSocket {}", url),
    }
}

async fn send_initial_messages(
    listening_trigger: &ListeningTrigger<WebsocketConfig>,
    writer: &mut WsWriter,
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
    const RECONNECT_ON_CONSUMER_EXIT: bool = true;
    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        // The caller runs `loop_ping` alongside, so the lease and the status line stay
        // alive across the sleeps, and cancels this future on killpill or a lost lease.
        let mut attempt = 0;
        loop {
            attempt += 1;
            match connect_once(db, listening_trigger, err_message.clone()).await {
                Ok(conn) => return Ok(Some(conn)),
                // Only retry in trigger mode, where a failure disables the trigger until
                // a human re-enables it: capture mode is an interactive test where
                // instant feedback beats resilience (an `Io` error can also be a
                // permanent misconfiguration, e.g. a typo'd host).
                Err(ConnectFailure { error, transient: true })
                    if listening_trigger.trigger_mode =>
                {
                    let delay = reconnect_delay(
                        Duration::from_millis(
                            CONNECT_RETRY_BASE_DELAY_MS.load(std::sync::atomic::Ordering::Relaxed),
                        ),
                        attempt,
                    );
                    let next_attempt_at =
                        chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
                    tracing::warn!(
                        "Transient error connecting to WebSocket for trigger {} (attempt {}), retrying in {:?}: {}",
                        listening_trigger.path,
                        attempt,
                        delay,
                        error
                    );
                    update_rw_lock(
                        err_message.clone(),
                        Some(format!(
                            "Connection attempt {} failed, retrying in {:?} (at {}): {}",
                            attempt,
                            delay,
                            next_attempt_at.format("%Y-%m-%d %H:%M:%S UTC"),
                            error
                        )),
                    )
                    .await;
                    tokio::time::sleep(delay).await;
                }
                Err(ConnectFailure { error, .. }) => {
                    let error = if attempt > 1 {
                        format!("{} (connection attempt {})", error, attempt)
                    } else {
                        error
                    };
                    return Err(Error::Generic(
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        error,
                    ));
                }
            }
        }
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

        let (send_message_tx, rx) = mpsc::channel::<Message>(100);
        let (_close_tx, close_rx) = oneshot::channel::<()>();
        tokio::spawn(write_messages(
            writer,
            rx,
            close_rx,
            url.clone(),
            db.clone(),
            listening_trigger.workspace_id.clone(),
        ));

        let return_message_channels = if listening_trigger.trigger_mode
            && listening_trigger.trigger_config.can_return_message
        {
            Some(ReturnMessageChannels {
                send_message_tx: send_message_tx.clone(),
                killpill_rx: killpill_rx.resubscribe(),
            })
        } else {
            None
        };

        // Shared heartbeat state: tracks the latest value of the configured state_field
        let heartbeat_state: Arc<RwLock<Option<serde_json::Value>>> = Arc::new(RwLock::new(None));

        tokio::select! {
            biased;
            _ = killpill_rx.recv() => {
            },
            _ = self.loop_ping(db, listening_trigger, err_message.clone(), None) => {
            },
            // Heartbeat timer
            _ = async {
                match listening_trigger.trigger_config.heartbeat {
                    Some(ref hb) if listening_trigger.trigger_mode => {
                        let interval_duration = std::time::Duration::from_secs(hb.interval_secs);
                        let mut interval = tokio::time::interval(interval_duration);
                        // Skip the first immediate tick
                        interval.tick().await;

                        loop {
                            interval.tick().await;

                            let msg = if hb.state_field.is_some() {
                                let state_val = heartbeat_state.read().await;
                                match &*state_val {
                                    Some(val) => hb.message.replace("{{state}}", &val.to_string()),
                                    None => hb.message.replace("{{state}}", "null"),
                                }
                            } else {
                                hb.message.clone()
                            };

                            tracing::debug!("Sending heartbeat to WebSocket {}: {}", url, msg);

                            if let Err(err) = send_message_tx.send(Message::Text(msg)).await {
                                tracing::error!("Failed to send heartbeat to WebSocket {}: {}", url, err);
                                break;
                            }
                        }
                    }
                    _ => futures::future::pending::<()>().await,
                }
            } => {},
            // Message reader
            _ = async {
                let filters = if listening_trigger.trigger_mode {
                    CompiledFilters::parse(
                        listening_trigger.trigger_config.filters.iter().map(|m| m.get()),
                        listening_trigger.trigger_config.filter_logic == "or",
                        &listening_trigger.path,
                    )
                } else {
                    CompiledFilters::default()
                };
                let ping_every = Duration::from_millis(PING_INTERVAL_MS.load(std::sync::atomic::Ordering::Relaxed));
                let liveness_timeout = ping_every * MISSED_PINGS_BEFORE_RECONNECT;
                let mut ping_interval = tokio::time::interval_at(Instant::now() + ping_every, ping_every);
                // Capture is an interactive test against whatever endpoint the user is
                // trying out: keep it to plain reads.
                let liveness = listening_trigger.trigger_mode;
                ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                let mut last_received = Instant::now();
                let mut close_frame = None;
                let status = loop {
                    tokio::select! {
                        // Drain frames that already arrived before judging liveness.
                        biased;
                        msg = reader.next() => {
                            let Some(msg) = msg else {
                                tracing::error!("WebSocket {} closed", url);
                                break match close_frame.take() {
                                    Some(tungstenite::protocol::CloseFrame { code, reason }) => {
                                        format!("WebSocket closed by server ({}: {})", code, reason)
                                    }
                                    None => "WebSocket closed".to_string(),
                                };
                            };
                            last_received = Instant::now();
                            match msg {
                                Ok(Message::Text(text)) => {
                                    tracing::debug!("Received text message from WebSocket {}: {}", url, text);

                                    // Extract heartbeat state from every incoming message
                                    if let Some(ref hb) = listening_trigger.trigger_config.heartbeat {
                                        if let Some(ref field) = hb.state_field {
                                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                                if let Some(val) = parsed.get(field) {
                                                    *heartbeat_state.write().await = Some(val.clone());
                                                }
                                            }
                                        }
                                    }

                                    if filters.matches(&text) {
                                        let trigger_info = HashMap::from([
                                            ("url".to_string(), to_raw_value(&listening_trigger.trigger_config.url)),
                                        ]);
                                        let _ = self.handle_event(db, listening_trigger, text, trigger_info, return_message_channels.clone()).await;
                                    }
                                },
                                Ok(Message::Close(frame)) => {
                                    tracing::info!("Received close frame from WebSocket {}: {:?}", url, frame);
                                    close_frame = frame;
                                },
                                Ok(msg) => {
                                    tracing::debug!("Received non text-message from WebSocket {}: {:?}", url, msg);
                                },
                                // tungstenite does not recover a connection after a read
                                // error: reconnect rather than read from it again.
                                Err(err) => {
                                    tracing::error!("Error reading from WebSocket {}: {:?}", url, err);
                                    break format!("Error reading from WebSocket: {}", err);
                                }
                            }
                        },
                        _ = ping_interval.tick(), if liveness => {
                            if last_received.elapsed() >= liveness_timeout {
                                tracing::error!("No frame received from WebSocket {} in {:?}, reconnecting", url, liveness_timeout);
                                break format!("No message or pong received from WebSocket in {:?}", liveness_timeout);
                            }
                            // Never wait on the queue: behind a peer that stopped reading it
                            // can stay full, and blocking here would stall this liveness check.
                            if let Err(mpsc::error::TrySendError::Closed(_)) = send_message_tx.try_send(Message::Ping(Vec::new())) {
                                break "WebSocket writer stopped".to_string();
                            }
                        },
                    }
                };
                self.update_ping_and_loop_ping_status(db, listening_trigger, err_message.clone(), Some(status)).await;
            } => {}
        }
    }

    async fn handle_trigger(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        payload: Self::Payload,
        mut trigger_info: HashMap<String, Box<RawValue>>,
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

        trigger_info.insert("trigger_path".to_string(), to_raw_value(path));
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
        let can_return_message = trigger_config.can_return_message;
        if *suspended_mode || extra.is_none() || !can_return_message {
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
                                if let Err(err) = send_message_tx.send(Message::Text(result)).await {
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
    send_message_tx: mpsc::Sender<Message>,
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
pub(crate) enum InitialMessage {
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
