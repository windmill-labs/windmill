use crate::{error::Result, scripts::ScriptLang, utils::Mode, DB};

pub async fn get_disable_stats_setting(_db: &DB) -> bool {
    // stats details are closed source

    false
}

pub async fn schedule_stats(
    _instance_name: String,
    _mode: Mode,
    _db: &DB,
    _http_client: &reqwest::Client,
    _is_enterprise: bool,
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
    _mode: &Mode,
    _http_client: &reqwest::Client,
    _db: &DB,
    _is_enterprise: bool,
) -> Result<()> {
    // stats details are closed source
    Ok(())
}
