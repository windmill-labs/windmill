use serde::{Deserialize, Serialize};

use crate::{db::DB, error};

pub const BYTES_PER_MB: u64 = 1_048_576;
pub const BYTES_PER_KB: u64 = 1024;

#[derive(Clone, Debug)]
pub struct TantivyIndexerSettings {
    pub writer_memory_budget: u64,
    pub commit_job_max_batch_size: u64,
    pub commit_log_max_batch_size: u64,
    pub refresh_index_period: u64,
    pub refresh_log_index_period: u64,
    pub max_indexed_job_log_size: usize,
    pub max_index_time_window_secs: i64,
    pub should_clear_job_index: bool,
    pub should_clear_log_index: bool,
}

impl Default for TantivyIndexerSettings {
    fn default() -> Self {
        TantivyIndexerSettings {
            writer_memory_budget: 150_000_000,
            commit_job_max_batch_size: 10_000,
            commit_log_max_batch_size: 5_000,
            refresh_index_period: 300,
            refresh_log_index_period: 300,
            max_indexed_job_log_size: 1_000_000,
            max_index_time_window_secs: 60 * 60 * 24 * 7, // 7 days
            should_clear_job_index: false,
            should_clear_log_index: false,
        }
    }
}
#[derive(Deserialize, Serialize, Default, sqlx::FromRow, Clone)]
pub struct TantivyIndexerSettingsOpt {
    pub writer_memory_budget: Option<u64>,
    pub commit_job_max_batch_size: Option<u64>,
    pub commit_log_max_batch_size: Option<u64>,
    pub refresh_index_period: Option<u64>,
    pub refresh_log_index_period: Option<u64>,
    pub max_indexed_job_log_size: Option<usize>,
    pub max_index_time_window_secs: Option<i64>,
    pub should_clear_job_index: Option<bool>,
    pub should_clear_log_index: Option<bool>,
}

pub async fn load_indexer_config(db: &DB) -> error::Result<TantivyIndexerSettings> {
    let config: TantivyIndexerSettingsOpt =
        sqlx::query_scalar!("SELECT value FROM global_settings WHERE name = 'indexer_settings'",)
            .fetch_optional(db)
            .await?
            .map(|x| serde_json::from_value(x).ok())
            .flatten()
            .unwrap_or_default();

    let TantivyIndexerSettings {
        commit_job_max_batch_size,
        commit_log_max_batch_size,
        refresh_index_period,
        refresh_log_index_period,
        max_indexed_job_log_size,
        max_index_time_window_secs,
        writer_memory_budget,
        should_clear_job_index,
        should_clear_log_index,
    } = get_indexer_rates_from_env();

    Ok(TantivyIndexerSettings {
        writer_memory_budget: config.writer_memory_budget.unwrap_or(writer_memory_budget),
        commit_job_max_batch_size: config
            .commit_job_max_batch_size
            .unwrap_or(commit_job_max_batch_size),
        commit_log_max_batch_size: config
            .commit_log_max_batch_size
            .unwrap_or(commit_log_max_batch_size),
        refresh_index_period: config.refresh_index_period.unwrap_or(refresh_index_period),
        refresh_log_index_period: config
            .refresh_log_index_period
            .unwrap_or(refresh_log_index_period),
        max_indexed_job_log_size: config
            .max_indexed_job_log_size
            .unwrap_or(max_indexed_job_log_size),
        max_index_time_window_secs: config
            .max_index_time_window_secs
            .unwrap_or(max_index_time_window_secs),
        should_clear_job_index: config
            .should_clear_job_index
            .unwrap_or(should_clear_job_index),
        should_clear_log_index: config
            .should_clear_log_index
            .unwrap_or(should_clear_log_index),
    })
}

/// How far back the service log index reaches, in seconds.
///
/// [`crate::service_log_retention_secs`] is the ceiling: past it a line's `log_file` row is
/// deleted and can no longer be indexed. `max_index_time_window_secs` of `0` means "do not
/// shrink below that ceiling", not "unbounded" — both sites that trim and populate the index
/// derive the window here so the two cannot disagree about it.
pub fn service_log_index_window_secs(max_index_time_window_secs: i64) -> i64 {
    let retention = crate::service_log_retention_secs();
    if max_index_time_window_secs > 0 {
        std::cmp::min(max_index_time_window_secs, retention)
    } else {
        retention
    }
}

pub fn get_env_var(env_var: &str) -> Option<u64> {
    match std::env::var(env_var).map(|x| x.parse()) {
        Ok(Ok(i)) => Some(i),
        Err(_) => None,

        Ok(Err(e)) => {
            tracing::error!("Failed to parse env var {}: {}", env_var, e);
            None
        }
    }
}

pub fn get_indexer_rates_from_env() -> TantivyIndexerSettings {
    let mut settings = TantivyIndexerSettings::default();

    if let Some(b) = get_env_var("TANTIVY_INDEX_WRITER_MEMORY_BUDGET__MB") {
        settings.writer_memory_budget = b * BYTES_PER_MB as u64;
    }
    if let Some(b) = get_env_var("TANTIVY_DOC_COMMIT_MAX_BATCH_SIZE") {
        settings.commit_job_max_batch_size = b;
    }
    if let Some(b) = get_env_var("TANTIVY_SERVICE_LOG_COMMIT_MAX_BATCH_SIZE") {
        settings.commit_log_max_batch_size = b;
    }
    if let Some(b) = get_env_var("TANTIVY_REFRESH_INDEX_PERIOD__S") {
        settings.refresh_index_period = b;
    }
    if let Some(b) = get_env_var("TANTIVY_REFRESH_LOG_INDEX_PERIOD__S") {
        settings.refresh_log_index_period = b;
    }
    if let Some(b) = get_env_var("TANTIVY_MAX_INDEXED_JOB_LOG_SIZE__MB") {
        settings.max_indexed_job_log_size = (b * BYTES_PER_MB) as usize;
    }
    if let Some(b) = get_env_var("TANTIVY_MAX_INDEXED_JOB_LOG_SIZE__KB") {
        settings.max_indexed_job_log_size = (b * BYTES_PER_KB) as usize;
    }
    if let Some(b) = get_env_var("TANTIVY_MAX_INDEX_TIME_WINDOW__S") {
        settings.max_index_time_window_secs = b as i64;
    }

    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test rather than several: both halves share the process-wide retention, and the
    // setter half writes it, which parallel tests would race.
    #[test]
    fn retention_rejects_unusable_values_and_the_index_window_clamps_to_it() {
        use crate::{
            service_log_retention_secs, set_service_log_retention_secs,
            DEFAULT_SERVICE_LOG_RETENTION_SECS,
        };

        // See `set_service_log_retention_secs` for why the two unusable directions land apart:
        // too large keeps the intent by capping, non-positive cannot and falls back.
        let rejected: Vec<i64> = [0, -1, i64::MIN]
            .iter()
            .map(|v| {
                set_service_log_retention_secs(*v);
                service_log_retention_secs()
            })
            .collect();
        let capped: Vec<i64> = [i64::MAX, 60 * 60 * 24 * 365 * 101]
            .iter()
            .map(|v| {
                set_service_log_retention_secs(*v);
                service_log_retention_secs()
            })
            .collect();

        set_service_log_retention_secs(60 * 60 * 24 * 3);
        let retention = service_log_retention_secs();
        let windows = [
            // `0` disables the extra shrinking rather than lifting the ceiling — the trap that
            // makes an unset setting look unbounded.
            service_log_index_window_secs(0),
            // Retention is the ceiling: the index cannot reach lines whose `log_file` row is gone.
            service_log_index_window_secs(retention * 2),
            service_log_index_window_secs(60),
        ];
        set_service_log_retention_secs(DEFAULT_SERVICE_LOG_RETENTION_SECS);

        assert_eq!(
            rejected,
            vec![DEFAULT_SERVICE_LOG_RETENTION_SECS; 3],
            "a value that would expire everything must fall back to the default"
        );
        assert_eq!(
            capped,
            vec![60 * 60 * 24 * 365 * 100; 2],
            "an oversized value must cap, not shorten retention to the default"
        );
        assert_eq!(retention, 60 * 60 * 24 * 3);
        assert_eq!(windows, [retention, retention, 60]);
    }
}
