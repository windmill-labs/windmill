use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Copy, Serialize, Default, Hash)]
pub struct RunnableSettings {
    pub debouncing_settings: Option<i64>,
    pub concurrency_settings: Option<i64>,
}

// TODO: Add validation logic.
#[derive(
    Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, sqlx::FromRow, sqlx::Type,
)]
pub struct DebouncingSettings {
    #[serde(skip_serializing_if = "Option::is_none", alias = "custom_debounce_key")]
    pub debounce_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debounce_delay_s: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_debouncing_time: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_debounces_amount: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debounce_args_to_accumulate: Option<Vec<String>>,
}

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, Hash, PartialEq, sqlx::FromRow, sqlx::Decode,
)]
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

impl DebouncingSettings {
    pub fn maybe_fallback(
        self,
        debounce_key: Option<String>,
        debounce_delay_s: Option<i32>,
    ) -> Self {
        Self {
            debounce_key: self.debounce_key.or(debounce_key),
            debounce_delay_s: self.debounce_delay_s.or(debounce_delay_s),
            ..self
        }
    }

    pub fn is_legacy_compatible(&self) -> bool {
        self.max_total_debouncing_time.is_none()
            && self.max_total_debounces_amount.is_none()
            && self.debounce_args_to_accumulate.is_none()
    }
}

impl ConcurrencySettings {
    pub fn maybe_fallback(
        self,
        concurrency_key: Option<String>,
        concurrent_limit: Option<i32>,
        concurrency_time_window_s: Option<i32>,
    ) -> Self {
        Self {
            concurrency_key: self.concurrency_key.or(concurrency_key),
            concurrent_limit: self.concurrent_limit.or(concurrent_limit),
            concurrency_time_window_s: self.concurrency_time_window_s.or(concurrency_time_window_s),
        }
    }
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
