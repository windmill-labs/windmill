use crate::{db::DB, users::AuthCache};
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
        Err(anyhow::anyhow!("Implementation not open source"))
    }
}
