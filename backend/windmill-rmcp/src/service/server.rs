use futures::{SinkExt, StreamExt};
use thiserror::Error;

use super::*;
use crate::model::{
    CancelledNotification, CancelledNotificationParam, ClientInfo, ClientJsonRpcMessage,
    ClientNotification, ClientRequest, ClientResult, CreateMessageRequest,
    CreateMessageRequestParam, CreateMessageResult, ErrorData, ListRootsRequest, ListRootsResult,
    LoggingMessageNotification, LoggingMessageNotificationParam, ProgressNotification,
    ProgressNotificationParam, PromptListChangedNotification, ResourceListChangedNotification,
    ResourceUpdatedNotification, ResourceUpdatedNotificationParam, ServerInfo, ServerNotification,
    ServerRequest, ServerResult, ToolListChangedNotification,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoleServer;

impl ServiceRole for RoleServer {
    type Req = ServerRequest;
    type Resp = ServerResult;
    type Not = ServerNotification;
    type PeerReq = ClientRequest;
    type PeerResp = ClientResult;
    type PeerNot = ClientNotification;
    type Info = ServerInfo;
    type PeerInfo = ClientInfo;
    const IS_CLIENT: bool = false;
}

/// It represents the error that may occur when serving the server.
///
/// if you want to handle the error, you can use `serve_server_with_ct` or `serve_server` with `Result<RunningService<RoleServer, S>, ServerError>`
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("expect initialized request, but received: {0:?}")]
    ExpectedInitRequest(Option<ClientJsonRpcMessage>),

    #[error("expect initialized notification, but received: {0:?}")]
    ExpectedInitNotification(Option<ClientJsonRpcMessage>),

    #[error("connection closed: {0}")]
    ConnectionClosed(String),

    #[error("unexpected initialize result: {0:?}")]
    UnexpectedInitializeResponse(ServerResult),

    #[error("initialize failed: {0}")]
    InitializeFailed(ErrorData),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ClientSink = Peer<RoleServer>;

impl<S: Service<RoleServer>> ServiceExt<RoleServer> for S {
    fn serve_with_ct<T, E, A>(
        self,
        transport: T,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<RunningService<RoleServer, Self>, E>> + Send
    where
        T: IntoTransport<RoleServer, E, A> + ProvidesConnectionToken,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized,
    {
        serve_server_with_ct(self, transport, ct)
    }
}

pub async fn serve_server<S, T, E, A>(
    service: S,
    transport: T,
) -> Result<RunningService<RoleServer, S>, E>
where
    S: Service<RoleServer>,
    T: IntoTransport<RoleServer, E, A> + ProvidesConnectionToken,
    E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
{
    serve_server_with_ct(service, transport, CancellationToken::new()).await
}

/// Helper function to get the next message from the stream
async fn expect_next_message<S>(
    stream: &mut S,
    context: &str,
) -> Result<ClientJsonRpcMessage, ServerError>
where
    S: StreamExt<Item = ClientJsonRpcMessage> + Unpin,
{
    stream
        .next()
        .await
        .ok_or_else(|| ServerError::ConnectionClosed(context.to_string()))
}

/// Helper function to expect a request from the stream
async fn expect_request<S>(
    stream: &mut S,
    context: &str,
) -> Result<(ClientRequest, RequestId), ServerError>
where
    S: StreamExt<Item = ClientJsonRpcMessage> + Unpin,
{
    let msg = expect_next_message(stream, context).await?;
    let msg_clone = msg.clone();
    msg.into_request()
        .ok_or(ServerError::ExpectedInitRequest(Some(msg_clone)))
}

/// Helper function to expect a notification from the stream
async fn expect_notification<S>(
    stream: &mut S,
    context: &str,
) -> Result<ClientNotification, ServerError>
where
    S: StreamExt<Item = ClientJsonRpcMessage> + Unpin,
{
    let msg = expect_next_message(stream, context).await?;
    let msg_clone = msg.clone();
    msg.into_notification()
        .ok_or(ServerError::ExpectedInitNotification(Some(msg_clone)))
}

pub async fn serve_server_with_ct<S, T, E, A>(
    service: S,
    transport: T,
    ct: CancellationToken,
) -> Result<RunningService<RoleServer, S>, E>
where
    S: Service<RoleServer>,
    T: IntoTransport<RoleServer, E, A> + ProvidesConnectionToken,
    E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
{
    let user_token = transport.get_connection_token();
    let req_extensions = transport.get_extensions().clone();
    let (sink, stream) = transport.into_transport();
    let mut sink = Box::pin(sink);
    let mut stream = Box::pin(stream);
    let id_provider = <Arc<AtomicU32RequestIdProvider>>::default();

    // Convert ServerError to std::io::Error, then to E
    let handle_server_error = |e: ServerError| -> E {
        match e {
            ServerError::Io(io_err) => io_err.into(),
            other => std::io::Error::new(std::io::ErrorKind::Other, format!("{}", other)).into(),
        }
    };

    // Get initialize request
    let (request, id) = expect_request(&mut stream, "initialized request")
        .await
        .map_err(handle_server_error)?;

    let ClientRequest::InitializeRequest(peer_info) = &request else {
        return Err(handle_server_error(ServerError::ExpectedInitRequest(Some(
            ClientJsonRpcMessage::request(request, id),
        ))));
    };
    let (peer, peer_rx) = Peer::new(id_provider, peer_info.params.clone());
    let context = RequestContext {
        ct: ct.child_token(),
        id: id.clone(),
        meta: request.get_meta().clone(),
        extensions: request.extensions().clone(),
        peer: peer.clone(),
        user_token: user_token.clone(),
        req_extensions: req_extensions.clone(),
    };
    // Send initialize response
    let init_response = service.handle_request(request.clone(), context).await;
    let mut init_response = match init_response {
        Ok(ServerResult::InitializeResult(init_response)) => init_response,
        Ok(result) => {
            return Err(handle_server_error(
                ServerError::UnexpectedInitializeResponse(result),
            ));
        }
        Err(e) => {
            sink.send(ServerJsonRpcMessage::error(e.clone(), id))
                .await?;
            return Err(handle_server_error(ServerError::InitializeFailed(e)));
        }
    };
    let protocol_version = match peer_info
        .params
        .protocol_version
        .partial_cmp(&init_response.protocol_version)
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported protocol version",
        ))? {
        std::cmp::Ordering::Less => peer_info.params.protocol_version.clone(),
        _ => init_response.protocol_version,
    };
    init_response.protocol_version = protocol_version;
    sink.send(ServerJsonRpcMessage::response(
        ServerResult::InitializeResult(init_response),
        id,
    ))
    .await?;

    // Wait for initialize notification
    let notification = expect_notification(&mut stream, "initialize notification")
        .await
        .map_err(handle_server_error)?;
    let ClientNotification::InitializedNotification(_) = notification else {
        return Err(handle_server_error(ServerError::ExpectedInitNotification(
            Some(ClientJsonRpcMessage::notification(notification)),
        )));
    };
    let _ = service.handle_notification(notification).await;
    // Continue processing service
    serve_inner(
        service,
        (sink, stream),
        peer,
        peer_rx,
        ct,
        user_token,
        req_extensions,
    )
    .await
}

macro_rules! method {
    (peer_req $method:ident $Req:ident() => $Resp: ident ) => {
        pub async fn $method(&self) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ServerRequest::$Req($Req {
                    method: Default::default(),
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ClientResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident) => $Resp: ident ) => {
        pub async fn $method(&self, params: $Param) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ServerRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ClientResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident)) => {
        pub fn $method(
            &self,
            params: $Param,
        ) -> impl Future<Output = Result<(), ServiceError>> + Send + '_ {
            async move {
                let result = self
                    .send_request(ServerRequest::$Req($Req {
                        method: Default::default(),
                        params,
                    }))
                    .await?;
                match result {
                    ClientResult::EmptyResult(_) => Ok(()),
                    _ => Err(ServiceError::UnexpectedResponse),
                }
            }
        }
    };

    (peer_not $method:ident $Not:ident($Param: ident)) => {
        pub async fn $method(&self, params: $Param) -> Result<(), ServiceError> {
            self.send_notification(ServerNotification::$Not($Not {
                method: Default::default(),
                params,
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
    (peer_not $method:ident $Not:ident) => {
        pub async fn $method(&self) -> Result<(), ServiceError> {
            self.send_notification(ServerNotification::$Not($Not {
                method: Default::default(),
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
}

impl Peer<RoleServer> {
    method!(peer_req create_message CreateMessageRequest(CreateMessageRequestParam) => CreateMessageResult);
    method!(peer_req list_roots ListRootsRequest() => ListRootsResult);

    method!(peer_not notify_cancelled CancelledNotification(CancelledNotificationParam));
    method!(peer_not notify_progress ProgressNotification(ProgressNotificationParam));
    method!(peer_not notify_logging_message LoggingMessageNotification(LoggingMessageNotificationParam));
    method!(peer_not notify_resource_updated ResourceUpdatedNotification(ResourceUpdatedNotificationParam));
    method!(peer_not notify_resource_list_changed ResourceListChangedNotification);
    method!(peer_not notify_tool_list_changed ToolListChangedNotification);
    method!(peer_not notify_prompt_list_changed PromptListChangedNotification);
}
