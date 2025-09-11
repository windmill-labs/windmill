use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::scripts::ScriptLang;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatsAccumulator {
    pub worker_group: String,
    pub script_lang: Option<ScriptLang>,
    pub workspace_id: String,
    pub job_count: i32,
    pub total_duration_ms: i64,
}

pub type JobStatsMap =
    Arc<RwLock<HashMap<(i64, String, Option<ScriptLang>, String), JobStatsAccumulator>>>;

pub fn get_current_hour() -> i64 {
    let now = Utc::now();
    (now.timestamp() / 3600) * 3600
}

pub async fn accumulate_job_stats(
    stats_map: &JobStatsMap,
    worker_group: &str,
    script_lang: Option<ScriptLang>,
    workspace_id: &str,
    duration_ms: i64,
) {
    let hour = get_current_hour();
    let key = (
        hour,
        worker_group.to_string(),
        script_lang.clone(),
        workspace_id.to_string(),
    );

    let mut stats = stats_map.write().await;
    let entry = stats
        .entry(key.clone())
        .or_insert_with(|| JobStatsAccumulator {
            worker_group: worker_group.to_string(),
            script_lang,
            workspace_id: workspace_id.to_string(),
            job_count: 0,
            total_duration_ms: 0,
        });

    entry.job_count += 1;
    entry.total_duration_ms += duration_ms;
}

pub async fn flush_stats_to_db(
    db: &Pool<Postgres>,
    stats_map: &JobStatsMap,
) -> Result<(), sqlx::Error> {
    let current_stats: Vec<(
        (i64, String, Option<ScriptLang>, String),
        JobStatsAccumulator,
    )> = {
        let mut stats = stats_map.write().await;
        if stats.is_empty() {
            return Ok(());
        }
        stats.drain().collect()
    };

    for ((hour, _worker_group, script_lang, _workspace_id), accumulator) in current_stats {
        let script_lang_str = script_lang.as_ref().map(|l| l.as_str()).unwrap_or("other");

        // Use ON CONFLICT to sum existing values
        sqlx::query(
            r#"
            INSERT INTO worker_group_job_stats 
            (hour, worker_group, script_lang, workspace_id, job_count, total_duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (hour, worker_group, script_lang, workspace_id)
            DO UPDATE SET
                job_count = worker_group_job_stats.job_count + EXCLUDED.job_count,
                total_duration_ms = worker_group_job_stats.total_duration_ms + EXCLUDED.total_duration_ms
            "#,
        )
        .bind(hour)
        .bind(&accumulator.worker_group)
        .bind(script_lang_str)
        .bind(&accumulator.workspace_id)
        .bind(accumulator.job_count)
        .bind(accumulator.total_duration_ms)
        .execute(db)
        .await?;
    }

    Ok(())
}

pub async fn cleanup_old_stats(
    db: &Pool<Postgres>,
    retention_days: i64,
) -> Result<u64, sqlx::Error> {
    let cutoff_timestamp = get_current_hour() - (retention_days * 24 * 3600);

    let result = sqlx::query("DELETE FROM worker_group_job_stats WHERE hour < $1")
        .bind(cutoff_timestamp)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
