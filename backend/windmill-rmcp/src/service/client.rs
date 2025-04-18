use futures::{SinkExt, Stream, StreamExt};
use thiserror::Error;

use super::*;
use crate::model::{
    CallToolRequest, CallToolRequestParam, CallToolResult, CancelledNotification,
    CancelledNotificationParam, ClientInfo, ClientJsonRpcMessage, ClientNotification,
    ClientRequest, ClientResult, CompleteRequest, CompleteRequestParam, CompleteResult,
    GetPromptRequest, GetPromptRequestParam, GetPromptResult, InitializeRequest,
    InitializedNotification, JsonRpcResponse, ListPromptsRequest, ListPromptsResult,
    ListResourceTemplatesRequest, ListResourceTemplatesResult, ListResourcesRequest,
    ListResourcesResult, ListToolsRequest, ListToolsResult, PaginatedRequestParam,
    ProgressNotification, ProgressNotificationParam, ReadResourceRequest, ReadResourceRequestParam,
    ReadResourceResult, RequestId, RootsListChangedNotification, ServerInfo, ServerJsonRpcMessage,
    ServerNotification, ServerRequest, ServerResult, SetLevelRequest, SetLevelRequestParam,
    SubscribeRequest, SubscribeRequestParam, UnsubscribeRequest, UnsubscribeRequestParam,
};

/// It represents the error that may occur when serving the client.
///
/// if you want to handle the error, you can use `serve_client_with_ct` or `serve_client` with `Result<RunningService<RoleClient, S>, ClientError>`
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("expect initialized response, but received: {0:?}")]
    ExpectedInitResponse(Option<ServerJsonRpcMessage>),

    #[error("expect initialized result, but received: {0:?}")]
    ExpectedInitResult(Option<ServerResult>),

    #[error("conflict initialized response id: expected {0}, got {1}")]
    ConflictInitResponseId(RequestId, RequestId),

    #[error("connection closed: {0}")]
    ConnectionClosed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Helper function to get the next message from the stream
async fn expect_next_message<S>(
    stream: &mut S,
    context: &str,
) -> Result<ServerJsonRpcMessage, ClientError>
where
    S: Stream<Item = ServerJsonRpcMessage> + Unpin,
{
    stream
        .next()
        .await
        .ok_or_else(|| ClientError::ConnectionClosed(context.to_string()))
        .map_err(|e| ClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))
}

/// Helper function to expect a response from the stream
async fn expect_response<S>(
    stream: &mut S,
    context: &str,
) -> Result<(ServerResult, RequestId), ClientError>
where
    S: Stream<Item = ServerJsonRpcMessage> + Unpin,
{
    let msg = expect_next_message(stream, context).await?;

    match msg {
        ServerJsonRpcMessage::Response(JsonRpcResponse { id, result, .. }) => Ok((result, id)),
        _ => Err(ClientError::ExpectedInitResponse(Some(msg))),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoleClient;

impl ServiceRole for RoleClient {
    type Req = ClientRequest;
    type Resp = ClientResult;
    type Not = ClientNotification;
    type PeerReq = ServerRequest;
    type PeerResp = ServerResult;
    type PeerNot = ServerNotification;
    type Info = ClientInfo;
    type PeerInfo = ServerInfo;

    const IS_CLIENT: bool = true;
}

pub type ServerSink = Peer<RoleClient>;

impl<S: Service<RoleClient>> ServiceExt<RoleClient> for S {
    fn serve_with_ct<T, E, A>(
        self,
        transport: T,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<RunningService<RoleClient, Self>, E>> + Send
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized,
    {
        serve_client_with_ct(self, transport, ct)
    }
}

pub async fn serve_client<S, T, E, A>(
    service: S,
    transport: T,
) -> Result<RunningService<RoleClient, S>, E>
where
    S: Service<RoleClient>,
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
{
    serve_client_with_ct(service, transport, Default::default()).await
}

pub async fn serve_client_with_ct<S, T, E, A>(
    service: S,
    transport: T,
    ct: CancellationToken,
) -> Result<RunningService<RoleClient, S>, E>
where
    S: Service<RoleClient>,
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
{
    let (sink, stream) = transport.into_transport();
    let mut sink = Box::pin(sink);
    let mut stream = Box::pin(stream);
    let id_provider = <Arc<AtomicU32RequestIdProvider>>::default();

    // Convert ClientError to std::io::Error, then to E
    let handle_client_error = |e: ClientError| -> E {
        match e {
            ClientError::Io(io_err) => io_err.into(),
            other => std::io::Error::new(std::io::ErrorKind::Other, format!("{}", other)).into(),
        }
    };

    // service
    let id = id_provider.next_request_id();
    let init_request = InitializeRequest {
        method: Default::default(),
        params: service.get_info(),
        extensions: Default::default(),
    };
    sink.send(ClientJsonRpcMessage::request(
        ClientRequest::InitializeRequest(init_request),
        id.clone(),
    ))
    .await?;

    let (response, response_id) = expect_response(&mut stream, "initialize response")
        .await
        .map_err(handle_client_error)?;

    if id != response_id {
        return Err(handle_client_error(ClientError::ConflictInitResponseId(
            id,
            response_id,
        )));
    }

    let ServerResult::InitializeResult(initialize_result) = response else {
        return Err(handle_client_error(ClientError::ExpectedInitResult(Some(
            response,
        ))));
    };

    // send notification
    let notification =
        ClientJsonRpcMessage::notification(ClientNotification::InitializedNotification(
            InitializedNotification { method: Default::default(), extensions: Default::default() },
        ));
    sink.send(notification).await?;
    let (peer, peer_rx) = Peer::new(id_provider, initialize_result);
    serve_inner(service, (sink, stream), peer, peer_rx, ct).await
}

macro_rules! method {
    (peer_req $method:ident $Req:ident() => $Resp: ident ) => {
        pub async fn $method(&self) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req { method: Default::default() }))
                .await?;
            match result {
                ServerResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident) => $Resp: ident ) => {
        pub async fn $method(&self, params: $Param) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident)? => $Resp: ident ) => {
        pub async fn $method(&self, params: Option<$Param>) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident)) => {
        pub async fn $method(&self, params: $Param) -> Result<(), ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::EmptyResult(_) => Ok(()),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };

    (peer_not $method:ident $Not:ident($Param: ident)) => {
        pub async fn $method(&self, params: $Param) -> Result<(), ServiceError> {
            self.send_notification(ClientNotification::$Not($Not {
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
            self.send_notification(ClientNotification::$Not($Not {
                method: Default::default(),
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
}

impl Peer<RoleClient> {
    method!(peer_req complete CompleteRequest(CompleteRequestParam) => CompleteResult);
    method!(peer_req set_level SetLevelRequest(SetLevelRequestParam));
    method!(peer_req get_prompt GetPromptRequest(GetPromptRequestParam) => GetPromptResult);
    method!(peer_req list_prompts ListPromptsRequest(PaginatedRequestParam)? => ListPromptsResult);
    method!(peer_req list_resources ListResourcesRequest(PaginatedRequestParam)? => ListResourcesResult);
    method!(peer_req list_resource_templates ListResourceTemplatesRequest(PaginatedRequestParam)? => ListResourceTemplatesResult);
    method!(peer_req read_resource ReadResourceRequest(ReadResourceRequestParam) => ReadResourceResult);
    method!(peer_req subscribe SubscribeRequest(SubscribeRequestParam) );
    method!(peer_req unsubscribe UnsubscribeRequest(UnsubscribeRequestParam));
    method!(peer_req call_tool CallToolRequest(CallToolRequestParam) => CallToolResult);
    method!(peer_req list_tools ListToolsRequest(PaginatedRequestParam)? => ListToolsResult);

    method!(peer_not notify_cancelled CancelledNotification(CancelledNotificationParam));
    method!(peer_not notify_progress ProgressNotification(ProgressNotificationParam));
    method!(peer_not notify_initialized InitializedNotification);
    method!(peer_not notify_roots_list_changed RootsListChangedNotification);
}

impl Peer<RoleClient> {
    /// A wrapper method for [`Peer<RoleClient>::list_tools`].
    ///
    /// This function will call [`Peer<RoleClient>::list_tools`] multiple times until all tools are listed.
    pub async fn list_all_tools(&self) -> Result<Vec<crate::model::Tool>, ServiceError> {
        let mut tools = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_tools(Some(PaginatedRequestParam { cursor }))
                .await?;
            tools.extend(result.tools);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    /// A wrapper method for [`Peer<RoleClient>::list_resources`].
    ///
    /// This function will call [`Peer<RoleClient>::list_resources`] multiple times until all resources are listed.
    pub async fn list_all_prompts(&self) -> Result<Vec<crate::model::Prompt>, ServiceError> {
        let mut prompts = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_prompts(Some(PaginatedRequestParam { cursor }))
                .await?;
            prompts.extend(result.prompts);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(prompts)
    }

    /// A wrapper method for [`Peer<RoleClient>::list_resources`].
    ///
    /// This function will call [`Peer<RoleClient>::list_resources`] multiple times until all resources are listed.
    pub async fn list_all_resources(&self) -> Result<Vec<crate::model::Resource>, ServiceError> {
        let mut resources = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_resources(Some(PaginatedRequestParam { cursor }))
                .await?;
            resources.extend(result.resources);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(resources)
    }

    /// A wrapper method for [`Peer<RoleClient>::list_resource_templates`].
    ///
    /// This function will call [`Peer<RoleClient>::list_resource_templates`] multiple times until all resource templates are listed.
    pub async fn list_all_resource_templates(
        &self,
    ) -> Result<Vec<crate::model::ResourceTemplate>, ServiceError> {
        let mut resource_templates = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_resource_templates(Some(PaginatedRequestParam { cursor }))
                .await?;
            resource_templates.extend(result.resource_templates);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(resource_templates)
    }
}
