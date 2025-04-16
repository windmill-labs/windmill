use futures::future::BoxFuture;
use thiserror::Error;

use crate::{
    error::Error as McpError,
    model::{
        CancelledNotification, CancelledNotificationParam, Extensions, GetExtensions, GetMeta,
        JsonRpcBatchRequestItem, JsonRpcBatchResponseItem, JsonRpcError, JsonRpcMessage,
        JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, Meta, NumberOrString, ProgressToken,
        RequestId, ServerJsonRpcMessage,
    },
    transport::IntoTransport,
};

use axum::http::Extensions as AxumExtensions;

pub trait ProvidesConnectionToken {
    // Returns the token associated with the initial connection, if any.
    fn get_connection_token(&self) -> Arc<String>;
    fn get_extensions(&self) -> &AxumExtensions;
    fn get_workspace_id(&self) -> String;
}

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use server::*;
#[cfg(feature = "tower")]
mod tower;
use tokio_util::sync::CancellationToken;
#[cfg(feature = "tower")]
pub use tower::*;
use tracing::instrument;
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ServiceError {
    #[error("Mcp error: {0}")]
    McpError(McpError),
    #[error("Transport error: {0}")]
    Transport(std::io::Error),
    #[error("Unexpected response type")]
    UnexpectedResponse,
    #[error("task cancelled for reason {}", reason.as_deref().unwrap_or("<unknown>"))]
    Cancelled { reason: Option<String> },
    #[error("request timeout after {}", chrono::Duration::from_std(*timeout).unwrap_or_default())]
    Timeout { timeout: Duration },
}

impl ServiceError {}
trait TransferObject:
    std::fmt::Debug + Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static
{
}

impl<T> TransferObject for T where
    T: std::fmt::Debug
        + serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + 'static
        + Clone
{
}

#[allow(private_bounds, reason = "there's no the third implementation")]
pub trait ServiceRole: std::fmt::Debug + Send + Sync + 'static + Copy + Clone {
    type Req: TransferObject + GetMeta + GetExtensions;
    type Resp: TransferObject;
    type Not: TryInto<CancelledNotification, Error = Self::Not>
        + From<CancelledNotification>
        + TransferObject;
    type PeerReq: TransferObject + GetMeta + GetExtensions;
    type PeerResp: TransferObject;
    type PeerNot: TryInto<CancelledNotification, Error = Self::PeerNot>
        + From<CancelledNotification>
        + TransferObject;
    const IS_CLIENT: bool;
    type Info: TransferObject;
    type PeerInfo: TransferObject;
}

pub type TxJsonRpcMessage<R> =
    JsonRpcMessage<<R as ServiceRole>::Req, <R as ServiceRole>::Resp, <R as ServiceRole>::Not>;
pub type RxJsonRpcMessage<R> = JsonRpcMessage<
    <R as ServiceRole>::PeerReq,
    <R as ServiceRole>::PeerResp,
    <R as ServiceRole>::PeerNot,
>;

pub trait Service<R: ServiceRole>: Send + Sync + 'static {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> impl Future<Output = Result<R::Resp, McpError>> + Send + '_;
    fn handle_notification(
        &self,
        notification: R::PeerNot,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_;
    fn get_peer(&self) -> Option<Peer<R>>;
    fn set_peer(&mut self, peer: Peer<R>);
    fn get_info(&self) -> R::Info;
}

pub trait ServiceExt<R: ServiceRole>: Service<R> + Sized {
    /// Convert this service to a dynamic boxed service
    ///
    /// This could be very helpful when you want to store the services in a collection
    fn into_dyn(self) -> Box<dyn DynService<R>> {
        Box::new(self)
    }
    fn serve<T, E, A>(
        self,
        transport: T,
    ) -> impl Future<Output = Result<RunningService<R, Self>, E>> + Send
    where
        T: IntoTransport<R, E, A> + ProvidesConnectionToken,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized,
    {
        Self::serve_with_ct(self, transport, Default::default())
    }
    fn serve_with_ct<T, E, A>(
        self,
        transport: T,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<RunningService<R, Self>, E>> + Send
    where
        T: IntoTransport<R, E, A> + ProvidesConnectionToken,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized;
}

impl<R: ServiceRole> Service<R> for Box<dyn DynService<R>> {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> impl Future<Output = Result<R::Resp, McpError>> + Send + '_ {
        DynService::handle_request(self.as_ref(), request, context)
    }

    fn handle_notification(
        &self,
        notification: R::PeerNot,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        DynService::handle_notification(self.as_ref(), notification)
    }

    fn get_peer(&self) -> Option<Peer<R>> {
        DynService::get_peer(self.as_ref())
    }

    fn set_peer(&mut self, peer: Peer<R>) {
        DynService::set_peer(self.as_mut(), peer)
    }

    fn get_info(&self) -> R::Info {
        DynService::get_info(self.as_ref())
    }
}

pub trait DynService<R: ServiceRole>: Send + Sync {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> BoxFuture<Result<R::Resp, McpError>>;
    fn handle_notification(&self, notification: R::PeerNot) -> BoxFuture<Result<(), McpError>>;
    fn get_peer(&self) -> Option<Peer<R>>;
    fn set_peer(&mut self, peer: Peer<R>);
    fn get_info(&self) -> R::Info;
}

impl<R: ServiceRole, S: Service<R>> DynService<R> for S {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> BoxFuture<Result<R::Resp, McpError>> {
        Box::pin(self.handle_request(request, context))
    }
    fn handle_notification(&self, notification: R::PeerNot) -> BoxFuture<Result<(), McpError>> {
        Box::pin(self.handle_notification(notification))
    }
    fn get_peer(&self) -> Option<Peer<R>> {
        self.get_peer()
    }
    fn set_peer(&mut self, peer: Peer<R>) {
        self.set_peer(peer)
    }
    fn get_info(&self) -> R::Info {
        self.get_info()
    }
}

use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    sync::{Arc, atomic::AtomicU32},
    time::Duration,
};

use tokio::sync::mpsc;

pub trait RequestIdProvider: Send + Sync + 'static {
    fn next_request_id(&self) -> RequestId;
}

pub trait ProgressTokenProvider: Send + Sync + 'static {
    fn next_progress_token(&self) -> ProgressToken;
}

pub type AtomicU32RequestIdProvider = AtomicU32Provider;
pub type AtomicU32ProgressTokenProvider = AtomicU32Provider;

#[derive(Debug, Default)]
pub struct AtomicU32Provider {
    id: AtomicU32,
}

impl RequestIdProvider for AtomicU32Provider {
    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

impl ProgressTokenProvider for AtomicU32Provider {
    fn next_progress_token(&self) -> ProgressToken {
        ProgressToken(NumberOrString::Number(
            self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ))
    }
}

type Responder<T> = tokio::sync::oneshot::Sender<T>;

/// A handle to a remote request
///
/// You can cancel it by call [`RequestHandle::cancel`] with a reason,
///
/// or wait for response by call [`RequestHandle::await_response`]
#[derive(Debug)]
pub struct RequestHandle<R: ServiceRole> {
    pub rx: tokio::sync::oneshot::Receiver<Result<R::PeerResp, ServiceError>>,
    pub options: PeerRequestOptions,
    pub peer: Peer<R>,
    pub id: RequestId,
    pub progress_token: ProgressToken,
}

impl<R: ServiceRole> RequestHandle<R> {
    pub const REQUEST_TIMEOUT_REASON: &str = "request timeout";
    pub async fn await_response(self) -> Result<R::PeerResp, ServiceError> {
        if let Some(timeout) = self.options.timeout {
            let timeout_result = tokio::time::timeout(timeout, async move {
                self.rx
                    .await
                    .map_err(|_e| ServiceError::Transport(std::io::Error::other("disconnected")))?
            })
            .await;
            match timeout_result {
                Ok(response) => response,
                Err(_) => {
                    let error = Err(ServiceError::Timeout { timeout });
                    // cancel this request
                    let notification = CancelledNotification {
                        params: CancelledNotificationParam {
                            request_id: self.id,
                            reason: Some(Self::REQUEST_TIMEOUT_REASON.to_owned()),
                        },
                        method: crate::model::CancelledNotificationMethod,
                        extensions: Default::default(),
                    };
                    let _ = self.peer.send_notification(notification.into()).await;
                    error
                }
            }
        } else {
            self.rx
                .await
                .map_err(|_e| ServiceError::Transport(std::io::Error::other("disconnected")))?
        }
    }

    /// Cancel this request
    pub async fn cancel(self, reason: Option<String>) -> Result<(), ServiceError> {
        let notification = CancelledNotification {
            params: CancelledNotificationParam { request_id: self.id, reason },
            method: crate::model::CancelledNotificationMethod,
            extensions: Default::default(),
        };
        self.peer.send_notification(notification.into()).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum PeerSinkMessage<R: ServiceRole> {
    Request {
        request: R::Req,
        id: RequestId,
        responder: Responder<Result<R::PeerResp, ServiceError>>,
    },
    Notification {
        notification: R::Not,
        responder: Responder<Result<(), ServiceError>>,
    },
}

/// An interface to fetch the remote client or server
///
/// For general purpose, call [`Peer::send_request`] or [`Peer::send_notification`] to send message to remote peer.
///
/// To create a cancellable request, call [`Peer::send_request_with_option`].
#[derive(Clone)]
pub struct Peer<R: ServiceRole> {
    tx: mpsc::Sender<PeerSinkMessage<R>>,
    request_id_provider: Arc<dyn RequestIdProvider>,
    progress_token_provider: Arc<dyn ProgressTokenProvider>,
    info: Arc<R::PeerInfo>,
}

impl<R: ServiceRole> std::fmt::Debug for Peer<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerSink")
            .field("tx", &self.tx)
            .field("is_client", &R::IS_CLIENT)
            .finish()
    }
}

type ProxyOutbound<R> = mpsc::Receiver<PeerSinkMessage<R>>;

#[derive(Debug, Default)]
pub struct PeerRequestOptions {
    pub timeout: Option<Duration>,
    pub meta: Option<Meta>,
}

impl PeerRequestOptions {
    pub fn no_options() -> Self {
        Self::default()
    }
}

impl<R: ServiceRole> Peer<R> {
    const CLIENT_CHANNEL_BUFFER_SIZE: usize = 1024;
    pub(crate) fn new(
        request_id_provider: Arc<dyn RequestIdProvider>,
        peer_info: R::PeerInfo,
    ) -> (Peer<R>, ProxyOutbound<R>) {
        let (tx, rx) = mpsc::channel(Self::CLIENT_CHANNEL_BUFFER_SIZE);
        (
            Self {
                tx,
                request_id_provider,
                progress_token_provider: Arc::new(AtomicU32ProgressTokenProvider::default()),
                info: peer_info.into(),
            },
            rx,
        )
    }
    pub async fn send_notification(&self, notification: R::Not) -> Result<(), ServiceError> {
        let (responder, receiver) = tokio::sync::oneshot::channel();
        self.tx
            .send(PeerSinkMessage::Notification { notification, responder })
            .await
            .map_err(|_m| {
                ServiceError::Transport(std::io::Error::other("disconnected: receiver dropped"))
            })?;
        receiver.await.map_err(|_e| {
            ServiceError::Transport(std::io::Error::other("disconnected: responder dropped"))
        })?
    }
    pub async fn send_request(&self, request: R::Req) -> Result<R::PeerResp, ServiceError> {
        self.send_request_with_option(request, PeerRequestOptions::no_options())
            .await?
            .await_response()
            .await
    }

    pub async fn send_cancellable_request(
        &self,
        request: R::Req,
        options: PeerRequestOptions,
    ) -> Result<RequestHandle<R>, ServiceError> {
        self.send_request_with_option(request, options).await
    }

    pub async fn send_request_with_option(
        &self,
        mut request: R::Req,
        options: PeerRequestOptions,
    ) -> Result<RequestHandle<R>, ServiceError> {
        let id = self.request_id_provider.next_request_id();
        let progress_token = self.progress_token_provider.next_progress_token();
        request
            .get_meta_mut()
            .set_progress_token(progress_token.clone());
        if let Some(meta) = options.meta.clone() {
            request.get_meta_mut().extend(meta);
        }
        let (responder, receiver) = tokio::sync::oneshot::channel();
        self.tx
            .send(PeerSinkMessage::Request { request, id: id.clone(), responder })
            .await
            .map_err(|_m| ServiceError::Transport(std::io::Error::other("disconnected")))?;
        Ok(RequestHandle { id, rx: receiver, progress_token, options, peer: self.clone() })
    }
    pub fn peer_info(&self) -> &R::PeerInfo {
        &self.info
    }
}

#[derive(Debug)]
pub struct RunningService<R: ServiceRole, S: Service<R>> {
    service: Arc<S>,
    peer: Peer<R>,
    handle: tokio::task::JoinHandle<QuitReason>,
    /// cancellation token
    ct: CancellationToken,
}
impl<R: ServiceRole, S: Service<R>> Deref for RunningService<R, S> {
    type Target = Peer<R>;

    fn deref(&self) -> &Self::Target {
        self.peer()
    }
}

impl<R: ServiceRole, S: Service<R>> RunningService<R, S> {
    #[inline]
    pub fn peer(&self) -> &Peer<R> {
        &self.peer
    }
    #[inline]
    pub fn service(&self) -> &S {
        self.service.as_ref()
    }
    pub async fn waiting(self) -> Result<QuitReason, tokio::task::JoinError> {
        self.handle.await
    }
    pub async fn cancel(self) -> Result<QuitReason, tokio::task::JoinError> {
        self.ct.cancel();
        self.waiting().await
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum QuitReason {
    Cancelled,
    Closed,
}

/// Request execution context
#[derive(Debug, Clone)]
pub struct RequestContext<R: ServiceRole> {
    /// this token will be cancelled when the [`CancelledNotification`] is received.
    pub ct: CancellationToken,
    pub id: RequestId,
    pub meta: Meta,
    pub extensions: Extensions,
    /// An interface to fetch the remote client or server
    pub peer: Peer<R>,
    pub user_token: Arc<String>,
    pub req_extensions: AxumExtensions,
    pub workspace_id: String,
}

/// Use this function to skip initialization process
pub async fn serve_directly<R, S, T, E, A>(
    service: S,
    transport: T,
    peer_info: R::PeerInfo,
) -> Result<RunningService<R, S>, E>
where
    R: ServiceRole,
    S: Service<R>,
    T: IntoTransport<R, E, A> + ProvidesConnectionToken,
    E: std::error::Error + Send + Sync + 'static,
{
    serve_directly_with_ct(service, transport, peer_info, Default::default()).await
}

/// Use this function to skip initialization process
pub async fn serve_directly_with_ct<R, S, T, E, A>(
    service: S,
    transport: T,
    peer_info: R::PeerInfo,
    ct: CancellationToken,
) -> Result<RunningService<R, S>, E>
where
    R: ServiceRole,
    S: Service<R>,
    T: IntoTransport<R, E, A> + ProvidesConnectionToken,
    E: std::error::Error + Send + Sync + 'static,
{
    let (peer, peer_rx) = Peer::new(Arc::new(AtomicU32RequestIdProvider::default()), peer_info);
    let user_token = transport.get_connection_token();
    let req_extensions = transport.get_extensions().clone();
    let workspace_id = transport.get_workspace_id();
    serve_inner(
        service,
        transport,
        peer,
        peer_rx,
        ct,
        user_token,
        req_extensions,
        workspace_id,
    )
    .await
}

#[instrument(skip_all)]
async fn serve_inner<R, S, T, E, A>(
    mut service: S,
    transport: T,
    peer: Peer<R>,
    mut peer_rx: tokio::sync::mpsc::Receiver<PeerSinkMessage<R>>,
    ct: CancellationToken,
    user_token: Arc<String>,
    req_extensions: AxumExtensions,
    workspace_id: String,
) -> Result<RunningService<R, S>, E>
where
    R: ServiceRole,
    S: Service<R>,
    T: IntoTransport<R, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    use futures::{SinkExt, StreamExt};
    const SINK_PROXY_BUFFER_SIZE: usize = 64;
    let (sink_proxy_tx, mut sink_proxy_rx) =
        tokio::sync::mpsc::channel::<TxJsonRpcMessage<R>>(SINK_PROXY_BUFFER_SIZE);
    let peer_info = peer.peer_info();
    if R::IS_CLIENT {
        tracing::info!(?peer_info, "Service initialized as client");
    } else {
        tracing::info!(?peer_info, "Service initialized as server");
    }

    service.set_peer(peer.clone());
    let mut local_responder_pool = HashMap::new();
    let mut local_ct_pool = HashMap::<RequestId, CancellationToken>::new();
    let shared_service = Arc::new(service);
    // for return
    let service = shared_service.clone();

    // let message_sink = tokio::sync::
    // let mut stream = std::pin::pin!(stream);
    let serve_loop_ct = ct.child_token();
    let peer_return: Peer<R> = peer.clone();
    let handle = tokio::spawn(async move {
        let (mut sink, mut stream) = transport.into_transport();
        let mut sink = std::pin::pin!(sink);
        let mut stream = std::pin::pin!(stream);
        let mut batch_messages = VecDeque::<RxJsonRpcMessage<R>>::new();
        #[derive(Debug)]
        enum Event<P, R, T> {
            ProxyMessage(P),
            PeerMessage(R),
            ToSink(T),
        }
        let quit_reason = loop {
            let evt = if let Some(m) = batch_messages.pop_front() {
                Event::PeerMessage(m)
            } else {
                tokio::select! {
                    m = sink_proxy_rx.recv() => {
                        if let Some(m) = m {
                            Event::ToSink(m)
                        } else {
                            continue
                        }
                    }
                    m = stream.next() => {
                        if let Some(m) = m {
                            Event::PeerMessage(m)
                        } else {
                            // input stream closed
                            tracing::info!("input stream terminated");
                            break QuitReason::Closed
                        }
                    }
                    m = peer_rx.recv() => {
                        if let Some(m) = m {
                            Event::ProxyMessage(m)
                        } else {
                            continue
                        }
                    }
                    _ = serve_loop_ct.cancelled() => {
                        tracing::info!("task cancelled");
                        break QuitReason::Cancelled
                    }
                }
            };

            tracing::debug!(?evt, "new event");
            match evt {
                // response and error
                Event::ToSink(m) => {
                    if let Some(id) = match &m {
                        JsonRpcMessage::Response(response) => Some(&response.id),
                        JsonRpcMessage::Error(error) => Some(&error.id),
                        _ => None,
                    } {
                        if let Some(ct) = local_ct_pool.remove(id) {
                            ct.cancel();
                        }
                        let send_result = sink.send(m).await;
                        if let Err(error) = send_result {
                            tracing::error!(%error, "fail to response message");
                        }
                    }
                }
                Event::ProxyMessage(PeerSinkMessage::Request { request, id, responder }) => {
                    local_responder_pool.insert(id.clone(), responder);
                    let send_result = sink
                        .send(JsonRpcMessage::request(request, id.clone()))
                        .await;
                    if let Err(e) = send_result {
                        if let Some(responder) = local_responder_pool.remove(&id) {
                            let _ = responder
                                .send(Err(ServiceError::Transport(std::io::Error::other(e))));
                        }
                    }
                }
                Event::ProxyMessage(PeerSinkMessage::Notification { notification, responder }) => {
                    // catch cancellation notification
                    let mut cancellation_param = None;
                    let notification = match notification.try_into() {
                        Ok::<CancelledNotification, _>(cancelled) => {
                            cancellation_param.replace(cancelled.params.clone());
                            cancelled.into()
                        }
                        Err(notification) => notification,
                    };
                    let send_result = sink.send(JsonRpcMessage::notification(notification)).await;
                    let response = if let Err(e) = send_result {
                        Err(ServiceError::Transport(std::io::Error::other(e)))
                    } else {
                        Ok(())
                    };
                    let _ = responder.send(response);
                    if let Some(param) = cancellation_param {
                        if let Some(responder) = local_responder_pool.remove(&param.request_id) {
                            tracing::info!(id = %param.request_id, reason = param.reason, "cancelled");
                            let _response_result = responder.send(Err(ServiceError::Cancelled {
                                reason: param.reason.clone(),
                            }));
                        }
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Request(JsonRpcRequest {
                    id, request, ..
                })) => {
                    tracing::info!(%id, ?request, "received request");
                    {
                        let service = shared_service.clone();
                        let sink = sink_proxy_tx.clone();
                        let request_ct = serve_loop_ct.child_token();
                        let context_ct = request_ct.child_token();
                        local_ct_pool.insert(id.clone(), request_ct);
                        let context = RequestContext {
                            ct: context_ct,
                            id: id.clone(),
                            peer: peer.clone(),
                            meta: request.get_meta().clone(),
                            extensions: request.extensions().clone(),
                            user_token: user_token.clone(),
                            req_extensions: req_extensions.clone(),
                            workspace_id: workspace_id.clone(),
                        };
                        tokio::spawn(async move {
                            let result = service.handle_request(request, context).await;
                            let response = match result {
                                Ok(result) => {
                                    tracing::info!(%id, ?result, "response message");
                                    JsonRpcMessage::response(result, id)
                                }
                                Err(error) => {
                                    tracing::warn!(%id, ?error, "response error");
                                    JsonRpcMessage::error(error, id)
                                }
                            };
                            let _send_result = sink.send(response).await;
                        });
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Notification(JsonRpcNotification {
                    notification,
                    ..
                })) => {
                    tracing::info!(?notification, "received notification");
                    // catch cancelled notification
                    let notification = match notification.try_into() {
                        Ok::<CancelledNotification, _>(cancelled) => {
                            if let Some(ct) = local_ct_pool.remove(&cancelled.params.request_id) {
                                tracing::info!(id = %cancelled.params.request_id, reason = cancelled.params.reason, "cancelled");
                                ct.cancel();
                            }
                            cancelled.into()
                        }
                        Err(notification) => notification,
                    };
                    {
                        let service = shared_service.clone();
                        tokio::spawn(async move {
                            let result = service.handle_notification(notification).await;
                            if let Err(error) = result {
                                tracing::warn!(%error, "Error sending notification");
                            }
                        });
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Response(JsonRpcResponse {
                    result,
                    id,
                    ..
                })) => {
                    if let Some(responder) = local_responder_pool.remove(&id) {
                        let response_result = responder.send(Ok(result));
                        if let Err(_error) = response_result {
                            tracing::warn!(%id, "Error sending response");
                        }
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Error(JsonRpcError { error, id, .. })) => {
                    if let Some(responder) = local_responder_pool.remove(&id) {
                        let _response_result = responder.send(Err(ServiceError::McpError(error)));
                        if let Err(_error) = _response_result {
                            tracing::warn!(%id, "Error sending response");
                        }
                    }
                }
                Event::PeerMessage(JsonRpcMessage::BatchRequest(batch)) => {
                    batch_messages.extend(
                        batch
                            .into_iter()
                            .map(JsonRpcBatchRequestItem::into_non_batch_message),
                    );
                }
                Event::PeerMessage(JsonRpcMessage::BatchResponse(batch)) => {
                    batch_messages.extend(
                        batch
                            .into_iter()
                            .map(JsonRpcBatchResponseItem::into_non_batch_message),
                    );
                }
            }
        };
        let sink_close_result = sink.close().await;
        if let Err(e) = sink_close_result {
            tracing::error!(%e, "fail to close sink");
        }
        tracing::info!(?quit_reason, "serve finished");
        quit_reason
    });
    Ok(RunningService { service, peer: peer_return, handle, ct })
}
