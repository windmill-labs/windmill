#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::smtp_server_ee::*;

#[cfg(not(feature = "private"))]
use crate::{auth::AuthCache, db::DB};
#[cfg(not(feature = "private"))]
use std::{net::SocketAddr, sync::Arc};
#[cfg(not(feature = "private"))]
use windmill_common::db::UserDB;

#[cfg(not(feature = "private"))]
pub struct SmtpServer {
    pub auth_cache: Arc<AuthCache>,
    pub db: DB,
    pub user_db: UserDB,
    pub base_internal_url: String,
}

#[cfg(not(feature = "private"))]
impl SmtpServer {
    pub async fn start_listener_thread(self: Arc<Self>, _addr: SocketAddr) -> anyhow::Result<()> {
        let _ = self.auth_cache;
        let _ = self.db;
        let _ = self.user_db;
        let _ = self.base_internal_url;
        Err(anyhow::anyhow!("Implementation not open source"))
    }
}
