use serde::{Deserialize, Serialize};

use crate::error;

pub struct RunnableSettings {
    runnable_id: i64,
    runnable_type: String,
    pub debouncing: DebouncingSettings,
    pub concurrency: ConcurrencySettings,
}

impl RunnableSettings {
    // TODO: Implement caching
    pub async fn fetch() -> error::Result<Self> {
        todo!()
    }
}

// TODO: Add validation logic.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, sqlx::FromRow, sqlx::Decode)]
pub struct DebouncingSettings {
    #[serde(skip_serializing_if = "Option::is_none", alias = "custom_debounce_key")]
    /// debounce key is usually stored in the db
    /// including when:
    ///
    /// 1. User have created custom debounce key from ui or cli
    /// 2. User used default one
    ///
    /// Default: hash(path + step_id + inputs)
    pub debounce_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Debouncing delay will be determined by the first job with the key.
    /// All subsequent jobs with Some will get debounced.
    /// If the job has no delay, it will execute immediately, fully ignoring pending delays.
    pub debounce_delay_s: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_debouncing_time: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_debounces_amount: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// top level arguments to preserve
    /// For every debounce selected arguments will be saved
    /// in the end (when job finally starts) arguments will be appended and passed to runnable
    ///
    /// NOTE: selected args should be the lists.
    pub debounce_args_to_accumulate: Option<Vec<String>>,
}

impl DebouncingSettings {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, sqlx::FromRow, sqlx::Decode)]
pub struct ConcurrencySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_time_window_s: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default)]
pub struct ConcurrencySettingsWithCustom {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_concurrency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_time_window_s: Option<i32>,
}

impl From<ConcurrencySettings> for ConcurrencySettingsWithCustom {
    fn from(
        ConcurrencySettings { concurrency_key, concurrent_limit, concurrency_time_window_s }: ConcurrencySettings,
    ) -> Self {
        ConcurrencySettingsWithCustom {
            custom_concurrency_key: concurrency_key,
            concurrency_time_window_s,
            concurrent_limit,
        }
    }
}

impl From<ConcurrencySettingsWithCustom> for ConcurrencySettings {
    fn from(
        ConcurrencySettingsWithCustom {
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
        }: ConcurrencySettingsWithCustom,
    ) -> Self {
        ConcurrencySettings {
            concurrency_key: custom_concurrency_key,
            concurrency_time_window_s,
            concurrent_limit,
        }
    }
}
