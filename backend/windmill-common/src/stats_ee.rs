use sqlx::Postgres;

use crate::{error::Result, scripts::ScriptLang, DB};

pub async fn get_disable_stats_setting(_db: &DB) -> bool {
    // stats details are closed source

    false
}

pub async fn schedule_stats(
    _instance_name: String,
    _db: &DB,
    _http_client: &reqwest::Client,
) -> () {
    // stats details are closed source
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct JobsUsage {
    language: Option<ScriptLang>,
    total_duration: i64,
    count: i64,
}

pub async fn send_stats(
    _instance_name: &String,
    _http_client: &reqwest::Client,
    _db: &DB,
    _is_enterprise: bool,
) -> Result<()> {
    // stats details are closed source
    Ok(())
}

pub struct ActiveUserUsage {
    pub author_count: Option<i32>,
    pub operator_count: Option<i32>,
}

pub async fn get_user_usage<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    _db: E,
) -> Result<ActiveUserUsage> {
    let usage = ActiveUserUsage { author_count: None, operator_count: None };
    Ok(usage)
}
