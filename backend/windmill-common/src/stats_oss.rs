use crate::DB;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ActiveUserUsage {
    pub author_count: Option<i32>,
    pub operator_count: Option<i32>,
}

#[derive(Clone)]
pub enum SendStatsReason {
    Manual,
    Schedule,
    OnStart,
}

pub async fn get_disable_stats_setting(_db: &DB) -> bool {
    crate::stats_ee::get_disable_stats_setting(_db).await
}

pub async fn schedule_stats(_db: &DB, _http_client: &reqwest::Client) -> () {
    crate::stats_ee::schedule_stats(_db, _http_client).await
}

pub async fn send_stats(
    _db: &DB,
    _http_client: &reqwest::Client,
    _reason: SendStatsReason,
    _basic_auth: Option<String>,
) -> anyhow::Result<()> {
    crate::stats_ee::send_stats(_db, _http_client, _reason, _basic_auth).await
}

pub async fn get_user_usage(
    _db: &DB,
    _workspace_id: Option<&str>,
    _days: Option<i32>,
) -> anyhow::Result<ActiveUserUsage> {
    crate::stats_ee::get_user_usage(_db, _workspace_id, _days).await
}