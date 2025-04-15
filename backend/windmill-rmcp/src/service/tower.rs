use std::{future::poll_fn, marker::PhantomData};

use tower_service::Service as TowerService;

use crate::service::{Peer, RequestContext, Service, ServiceRole};

pub struct TowerHandler<S, R: ServiceRole> {
    pub service: S,
    pub info: R::Info,
    pub peer: Option<Peer<R>>,
    role: PhantomData<R>,
}

impl<S, R: ServiceRole> TowerHandler<S, R> {
    pub fn new(service: S, info: R::Info) -> Self {
        Self {
            service,
            role: PhantomData,
            info,
            peer: None,
        }
    }
}

impl<S, R: ServiceRole> Service<R> for TowerHandler<S, R>
where
    S: TowerService<R::PeerReq, Response = R::Resp> + Sync + Send + Clone + 'static,
    S::Error: Into<crate::Error>,
    S::Future: Send,
{
    async fn handle_request(
        &self,
        request: R::PeerReq,
        _context: RequestContext<R>,
    ) -> Result<R::Resp, crate::Error> {
        let mut service = self.service.clone();
        poll_fn(|cx| service.poll_ready(cx))
            .await
            .map_err(Into::into)?;
        let resp = service.call(request).await.map_err(Into::into)?;
        Ok(resp)
    }

    fn handle_notification(
        &self,
        _notification: R::PeerNot,
    ) -> impl Future<Output = Result<(), crate::Error>> + Send + '_ {
        std::future::ready(Ok(()))
    }

    fn get_peer(&self) -> Option<Peer<R>> {
        self.peer.clone()
    }

    fn set_peer(&mut self, peer: Peer<R>) {
        self.peer = Some(peer);
    }

    fn get_info(&self) -> R::Info {
        self.info.clone()
    }
}
