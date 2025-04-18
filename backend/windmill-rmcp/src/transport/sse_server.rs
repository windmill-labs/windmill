use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, Query, Request, State},
    http::{Extensions, StatusCode},
    response::{
        Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use futures::{Sink, SinkExt, Stream};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::{CancellationToken, PollSender};
use tracing::Instrument;

use crate::{
    RoleServer, Service,
    model::ClientJsonRpcMessage,
    service::{RxJsonRpcMessage, TxJsonRpcMessage},
};
type SessionId = Arc<str>;
type TxStore =
    Arc<tokio::sync::RwLock<HashMap<SessionId, tokio::sync::mpsc::Sender<ClientJsonRpcMessage>>>>;
pub type TransportReceiver = ReceiverStream<RxJsonRpcMessage<RoleServer>>;

const DEFAULT_AUTO_PING_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct App {
    txs: TxStore,
    transport_tx: tokio::sync::mpsc::UnboundedSender<SseServerTransport>,
    post_path: Arc<str>,
    full_message_path: Arc<str>,
    sse_ping_interval: Duration,
    ct: CancellationToken,
}

impl App {
    pub fn new(
        post_path: String,
        full_message_path: String,
        sse_ping_interval: Duration,
        ct: CancellationToken,
    ) -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<SseServerTransport>,
    ) {
        let (transport_tx, transport_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                txs: Default::default(),
                transport_tx,
                post_path: post_path.into(),
                full_message_path: full_message_path.into(),
                sse_ping_interval,
                ct,
            },
            transport_rx,
        )
    }
}

fn session_id() -> SessionId {
    let id = format!("{:016x}", rand::random::<u128>());
    Arc::from(id)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEventQuery {
    pub session_id: String,
}

#[axum::debug_handler]
async fn post_event_handler(
    State(app): State<App>,
    Query(PostEventQuery { session_id }): Query<PostEventQuery>,
    Json(message): Json<ClientJsonRpcMessage>,
) -> Result<StatusCode, StatusCode> {
    tracing::debug!(session_id, ?message, "new client message");
    let tx = {
        let rg = app.txs.read().await;
        rg.get(session_id.as_str())
            .ok_or(StatusCode::NOT_FOUND)?
            .clone()
    };
    if tx.send(message).await.is_err() {
        tracing::error!("send message error");
        return Err(StatusCode::GONE);
    }
    Ok(StatusCode::ACCEPTED)
}

#[axum::debug_handler]
async fn sse_handler(
    State(app): State<App>,
    Path(path_params): Path<HashMap<String, String>>,
    request: Request,
) -> Result<Sse<impl Stream<Item = Result<Event, io::Error>>>, Response<String>> {
    let session = session_id();
    tracing::info!(%session, "sse connection");
    use tokio_stream::{StreamExt, wrappers::ReceiverStream};
    use tokio_util::sync::PollSender;
    let (from_client_tx, from_client_rx) = tokio::sync::mpsc::channel(64);
    let (to_client_tx, to_client_rx) = tokio::sync::mpsc::channel(64);
    app.txs
        .write()
        .await
        .insert(session.clone(), from_client_tx);
    let session = session.clone();
    let stream = ReceiverStream::new(from_client_rx);
    let sink = PollSender::new(to_client_tx);
    // Construct the full nested path
    let workspace_id = match path_params.get("workspace_id") {
        // Use the name from your route definition
        Some(id) => id.clone(), // Clone it for use in the formatted string
        None => String::from("test"),
    };
    let transport = SseServerTransport {
        stream,
        sink,
        session_id: session.clone(),
        tx_store: app.txs.clone(),
        req_extensions: request.extensions().clone(),
        workspace_id: workspace_id.clone(),
    };
    let transport_send_result = app.transport_tx.send(transport);
    if transport_send_result.is_err() {
        tracing::warn!("send transport out error");
        let mut response =
            Response::new("fail to send out transport, it seems server is closed".to_string());
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        return Err(response);
    }
    let ping_interval = app.sse_ping_interval;
    let post_path = app.post_path.clone();
    let full_endpoint_path = app
        .full_message_path
        .replace(":workspace_id", &workspace_id);
    let server_ct = app.ct.clone();

    // Clone variables needed for the cleanup task *before* they are moved by async_stream
    let session_for_cleanup = session.clone();
    let server_ct_for_cleanup = server_ct.clone();
    let tx_store_for_cleanup = app.txs.clone();

    let mut message_stream = ReceiverStream::new(to_client_rx);
    let client_stream = async_stream::stream! {
        yield Ok(Event::default()
                .event("endpoint")
                .data(format!("{full_endpoint_path}{post_path}?sessionId={session}")));

        loop {
            tokio::select! {
                biased;
                _ = server_ct.cancelled() => {
                    tracing::info!(%session, "SSE connection cancelled via token.");
                    break;
                }
                maybe_message = message_stream.next() => {
                    match maybe_message {
                        Some(message) => {
                            match serde_json::to_string(&message) {
                                Ok(bytes) => yield Ok(Event::default().event("message").data(bytes)),
                                Err(e) => {
                                    tracing::error!(%session, "Failed to serialize message: {}", e);
                                    yield Err(io::Error::new(io::ErrorKind::InvalidData, e));
                                    break;
                                }
                            }
                        }
                        None => {
                            tracing::info!(%session, "Message channel closed, ending SSE stream.");
                            break;
                        }
                    }
                }
            }
        }
        tracing::debug!(%session, "SSE client_stream finished.");
    };

    // Clean up the tx entry when the SSE connection handler finishes (either normally or cancelled)
    tokio::spawn(async move {
        server_ct_for_cleanup.cancelled().await;
        tracing::debug!(session=%session_for_cleanup, "Removing session from tx store due to cancellation or handler exit.");
        tx_store_for_cleanup
            .write()
            .await
            .remove(&session_for_cleanup);
    });

    Ok(Sse::new(client_stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

pub struct SseServerTransport {
    stream: ReceiverStream<RxJsonRpcMessage<RoleServer>>,
    sink: PollSender<TxJsonRpcMessage<RoleServer>>,
    session_id: SessionId,
    tx_store: TxStore,
    req_extensions: Extensions,
    workspace_id: String,
}

impl crate::service::ProvidesAxiumExtensions for SseServerTransport {
    fn get_extensions(&self) -> &Extensions {
        &self.req_extensions
    }

    fn get_workspace_id(&self) -> String {
        self.workspace_id.clone()
    }
}

impl Sink<TxJsonRpcMessage<RoleServer>> for SseServerTransport {
    type Error = io::Error;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_ready_unpin(cx)
            .map_err(std::io::Error::other)
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> Result<(), Self::Error> {
        self.sink
            .start_send_unpin(item)
            .map_err(std::io::Error::other)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_flush_unpin(cx)
            .map_err(std::io::Error::other)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let inner_close_result = self
            .sink
            .poll_close_unpin(cx)
            .map_err(std::io::Error::other);
        if inner_close_result.is_ready() {
            let session_id = self.session_id.clone();
            let tx_store = self.tx_store.clone();
            tokio::spawn(async move {
                tx_store.write().await.remove(&session_id);
            });
        }
        inner_close_result
    }
}

impl Stream for SseServerTransport {
    type Item = RxJsonRpcMessage<RoleServer>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use futures::StreamExt;
        self.stream.poll_next_unpin(cx)
    }
}

#[derive(Debug, Clone)]
pub struct SseServerConfig {
    pub bind: SocketAddr,
    pub sse_path: String,
    pub post_path: String,
    pub ct: CancellationToken,
    pub sse_keep_alive: Option<Duration>,
    pub full_message_path: String,
}

#[derive(Debug)]
pub struct SseServer {
    transport_rx: tokio::sync::mpsc::UnboundedReceiver<SseServerTransport>,
    pub config: SseServerConfig,
}

impl SseServer {
    pub async fn serve(bind: SocketAddr) -> io::Result<Self> {
        Self::serve_with_config(SseServerConfig {
            bind,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: CancellationToken::new(),
            sse_keep_alive: None,
            full_message_path: "/message".to_string(),
        })
        .await
    }
    pub async fn serve_with_config(config: SseServerConfig) -> io::Result<Self> {
        let (sse_server, service) = Self::new(config);
        let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
        let ct = sse_server.config.ct.child_token();
        let server = axum::serve(listener, service).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("sse server cancelled");
        });
        tokio::spawn(
            async move {
                if let Err(e) = server.await {
                    tracing::error!(error = %e, "sse server shutdown with error");
                }
            }
            .instrument(tracing::info_span!("sse-server", bind_address = %sse_server.config.bind)),
        );
        Ok(sse_server)
    }

    /// Warning: This function creates a new SseServer instance with the provided configuration.
    /// `App.post_path` may be incorrect if using `Router` as an embedded router.
    pub fn new(config: SseServerConfig) -> (SseServer, Router) {
        let (app, transport_rx) = App::new(
            config.post_path.clone(),
            config.full_message_path.clone(),
            config.sse_keep_alive.unwrap_or(DEFAULT_AUTO_PING_INTERVAL),
            config.ct.clone(),
        );
        let router = Router::new()
            .route(&config.sse_path, get(sse_handler))
            .route(&config.post_path, post(post_event_handler))
            .with_state(app);

        let server = SseServer { transport_rx, config };

        (server, router)
    }

    pub fn with_service<S, F>(mut self, service_provider: F) -> CancellationToken
    where
        S: Service<RoleServer>,
        F: Fn() -> S + Send + 'static,
    {
        use crate::service::ServiceExt;
        let ct = self.config.ct.clone();
        tokio::spawn(async move {
            while let Some(transport) = self.next_transport().await {
                let service = service_provider();
                let ct = self.config.ct.child_token();
                tokio::spawn(async move {
                    let server = service.serve_with_ct(transport, ct).await?;
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                });
            }
        });
        ct
    }

    pub fn cancel(&self) {
        self.config.ct.cancel();
    }

    pub async fn next_transport(&mut self) -> Option<SseServerTransport> {
        self.transport_rx.recv().await
    }
}

impl Stream for SseServer {
    type Item = SseServerTransport;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.transport_rx.poll_recv(cx)
    }
}
