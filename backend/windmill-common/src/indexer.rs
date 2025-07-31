use serde::{Deserialize, Serialize};

use crate::{error, DB};

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
    pub should_clear_job_index: bool,
    pub should_clear_log_index: bool,
}

impl Default for TantivyIndexerSettings {
    fn default() -> Self {
        TantivyIndexerSettings {
            writer_memory_budget: 300_000_000,
            commit_job_max_batch_size: 50_000,
            commit_log_max_batch_size: 10_000,
            refresh_index_period: 300,
            refresh_log_index_period: 300,
            max_indexed_job_log_size: 1_000_000,
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
        should_clear_job_index: config
            .should_clear_job_index
            .unwrap_or(should_clear_job_index),
        should_clear_log_index: config
            .should_clear_log_index
            .unwrap_or(should_clear_log_index),
    })
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

    settings
}
