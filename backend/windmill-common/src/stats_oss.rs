#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::stats_ee::*;

#[cfg(not(feature = "private"))]
use sqlx::Postgres;

#[cfg(not(feature = "private"))]
use crate::{error::Result, DB};

#[cfg(not(feature = "private"))]
pub async fn get_disable_stats_setting(_db: &DB) -> bool {
    // stats details are closed source

    false
}

#[cfg(not(feature = "private"))]
pub async fn schedule_stats(_db: &DB, _http_client: &reqwest::Client) -> () {
    // stats details are closed source
}

#[cfg(not(feature = "private"))]
pub enum SendStatsReason {
    Manual,
    Schedule,
    OnStart,
}

#[cfg(not(feature = "private"))]
pub async fn send_stats(
    _http_client: &reqwest::Client,
    _db: &DB,
    _reason: SendStatsReason,
) -> Result<()> {
    // stats details are closed source
    Ok(())
}

#[cfg(not(feature = "private"))]
pub struct ActiveUserUsage {
    pub author_count: Option<i32>,
    pub operator_count: Option<i32>,
}

#[cfg(not(feature = "private"))]
pub async fn get_user_usage<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    _db: E,
) -> Result<ActiveUserUsage> {
    let usage = ActiveUserUsage { author_count: None, operator_count: None };
    Ok(usage)
}

#[cfg(not(feature = "private"))]
#[derive(serde::Serialize)]
pub struct Stats {}

#[cfg(not(feature = "private"))]
pub async fn get_stats_payload(_db: &DB, _reason: &SendStatsReason) -> Result<Stats> {
    // stats details are closed source
    Ok(Stats {})
}

#[cfg(not(feature = "private"))]
pub fn encrypt_stats(_stats: &Stats) -> Result<String> {
    // stats details are closed source
    Ok(String::new())
}
