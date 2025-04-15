use crate::{
    error::Error as McpError,
    model::*,
    service::{Peer, RequestContext, RoleClient, Service, ServiceRole},
};

impl<H: ClientHandler> Service<RoleClient> for H {
    async fn handle_request(
        &self,
        request: <RoleClient as ServiceRole>::PeerReq,
        context: RequestContext<RoleClient>,
    ) -> Result<<RoleClient as ServiceRole>::Resp, McpError> {
        match request {
            ServerRequest::PingRequest(_) => self.ping(context).await.map(ClientResult::empty),
            ServerRequest::CreateMessageRequest(request) => self
                .create_message(request.params, context)
                .await
                .map(ClientResult::CreateMessageResult),
            ServerRequest::ListRootsRequest(_) => self
                .list_roots(context)
                .await
                .map(ClientResult::ListRootsResult),
        }
    }

    async fn handle_notification(
        &self,
        notification: <RoleClient as ServiceRole>::PeerNot,
    ) -> Result<(), McpError> {
        match notification {
            ServerNotification::CancelledNotification(notification) => {
                self.on_cancelled(notification.params).await
            }
            ServerNotification::ProgressNotification(notification) => {
                self.on_progress(notification.params).await
            }
            ServerNotification::LoggingMessageNotification(notification) => {
                self.on_logging_message(notification.params).await
            }
            ServerNotification::ResourceUpdatedNotification(notification) => {
                self.on_resource_updated(notification.params).await
            }
            ServerNotification::ResourceListChangedNotification(_notification_no_param) => {
                self.on_resource_list_changed().await
            }
            ServerNotification::ToolListChangedNotification(_notification_no_param) => {
                self.on_tool_list_changed().await
            }
            ServerNotification::PromptListChangedNotification(_notification_no_param) => {
                self.on_prompt_list_changed().await
            }
        };
        Ok(())
    }

    fn get_peer(&self) -> Option<Peer<RoleClient>> {
        self.get_peer()
    }

    fn set_peer(&mut self, peer: Peer<RoleClient>) {
        self.set_peer(peer);
    }

    fn get_info(&self) -> <RoleClient as ServiceRole>::Info {
        self.get_info()
    }
}

#[allow(unused_variables)]
pub trait ClientHandler: Sized + Send + Sync + 'static {
    fn ping(
        &self,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Ok(()))
    }

    fn create_message(
        &self,
        params: CreateMessageRequestParam,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateMessageResult, McpError>> + Send + '_ {
        std::future::ready(Err(
            McpError::method_not_found::<CreateMessageRequestMethod>(),
        ))
    }

    fn list_roots(
        &self,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<ListRootsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListRootsResult::default()))
    }

    fn on_cancelled(
        &self,
        params: CancelledNotificationParam,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_progress(
        &self,
        params: ProgressNotificationParam,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_resource_list_changed(&self) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_tool_list_changed(&self) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_prompt_list_changed(&self) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }

    fn get_peer(&self) -> Option<Peer<RoleClient>>;

    fn set_peer(&mut self, peer: Peer<RoleClient>);

    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

/// Do nothing, just store the peer.
impl ClientHandler for Option<Peer<RoleClient>> {
    fn get_peer(&self) -> Option<Peer<RoleClient>> {
        self.clone()
    }

    fn set_peer(&mut self, peer: Peer<RoleClient>) {
        *self = Some(peer);
    }
}

/// Do nothing, even store the peer.
impl ClientHandler for () {
    fn get_peer(&self) -> Option<Peer<RoleClient>> {
        None
    }

    fn set_peer(&mut self, peer: Peer<RoleClient>) {
        drop(peer);
    }
}

/// Do nothing, even store the peer.
impl ClientHandler for ClientInfo {
    fn get_peer(&self) -> Option<Peer<RoleClient>> {
        None
    }

    fn set_peer(&mut self, peer: Peer<RoleClient>) {
        drop(peer);
    }

    fn get_info(&self) -> ClientInfo {
        self.clone()
    }
}
