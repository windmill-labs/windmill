use crate::{db::DB, users::AuthCache, users::UserDB};
use std::{net::SocketAddr, sync::Arc};

pub struct SmtpServer {
    pub auth_cache: Arc<AuthCache>,
    pub db: DB,
    pub user_db: UserDB,
    pub base_internal_url: String,
}

impl SmtpServer {
    pub async fn start_listener_thread(
        self: Arc<Self>,
        _addr: SocketAddr,
    ) -> anyhow::Result<()> {
        crate::smtp_server_ee::SmtpServer {
            auth_cache: self.auth_cache,
            db: self.db,
            user_db: self.user_db,
            base_internal_url: self.base_internal_url,
        }.start_listener_thread(_addr).await
    }
}