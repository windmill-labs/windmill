#[cfg(feature = "private")]
use crate::stats_ee;

use sqlx::Postgres;

use crate::{error::Result, scripts::ScriptLang, DB};

pub async fn get_disable_stats_setting(db: &DB) -> bool {
    #[cfg(feature = "private")]
    {
        return stats_ee::get_disable_stats_setting(db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = db;
        // stats details are closed source
        false
    }
}

pub async fn schedule_stats(db: &DB, http_client: &reqwest::Client) -> () {
    #[cfg(feature = "private")]
    {
        stats_ee::schedule_stats(db, http_client).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, http_client);
        // stats details are closed source
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct JobsUsage { // Remains as is, might be used by OSS or EE logic
    language: Option<ScriptLang>,
    total_duration: i64,
    count: i64,
}

pub enum SendStatsReason { // Remains as is
    Manual,
    Schedule,
    OnStart,
}

pub async fn send_stats(
    http_client: &reqwest::Client,
    db: &DB,
    reason: SendStatsReason,
) -> Result<()> {
    #[cfg(feature = "private")]
    {
        return stats_ee::send_stats(http_client, db, reason).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (http_client, db, reason);
        // stats details are closed source
        Ok(())
    }
}

pub struct ActiveUserUsage { // Remains as is
    pub author_count: Option<i32>,
    pub operator_count: Option<i32>,
}

pub async fn get_user_usage<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
) -> Result<ActiveUserUsage> {
    #[cfg(feature = "private")]
    {
        return stats_ee::get_user_usage(db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = db;
        let usage = ActiveUserUsage { author_count: None, operator_count: None };
        Ok(usage)
    }
}
