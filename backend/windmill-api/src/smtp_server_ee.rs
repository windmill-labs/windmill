use crate::{auth::AuthCache, db::DB};
use std::{net::SocketAddr, sync::Arc};
use windmill_common::db::UserDB;

pub struct SmtpServer {
    pub auth_cache: Arc<AuthCache>,
    pub db: DB,
    pub user_db: UserDB,
    pub base_internal_url: String,
}

impl SmtpServer {
    pub async fn start_listener_thread(self: Arc<Self>, _addr: SocketAddr) -> anyhow::Result<()> {
        let _ = self.auth_cache;
        let _ = self.db;
        let _ = self.user_db;
        let _ = self.base_internal_url;
        Err(anyhow::anyhow!("Implementation not open source"))
    }
}
