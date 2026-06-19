use serde::{Deserialize, Serialize};

use crate::flows::{ConstantDelay, ExponentialDelay, Retry, RetryIf};

#[derive(Deserialize, Clone, Copy, Serialize, Default, Hash)]
pub struct RunnableSettings {
    pub debouncing_settings: Option<i64>,
    pub concurrency_settings: Option<i64>,
    pub retry_settings: Option<i64>,
}

/// Flattened, dedup-friendly representation of a [`Retry`] policy. Native script
/// retry stores the policy here (via `runnable_settings_handle`) instead of
/// wrapping the script in a one-step flow.
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, Hash, PartialEq, sqlx::FromRow, sqlx::Decode,
)]
pub struct RetrySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant_attempts: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential_attempts: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential_multiplier: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential_random_factor: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_if_expr: Option<String>,
}

impl From<&Retry> for RetrySettings {
    fn from(r: &Retry) -> Self {
        Self {
            constant_attempts: Some(r.constant.attempts as i32),
            constant_seconds: Some(r.constant.seconds as i32),
            exponential_attempts: Some(r.exponential.attempts as i32),
            exponential_multiplier: Some(r.exponential.multiplier as i32),
            exponential_seconds: Some(r.exponential.seconds as i32),
            exponential_random_factor: r.exponential.random_factor.map(|x| x as i32),
            retry_if_expr: r.retry_if.as_ref().map(|x| x.expr.clone()),
        }
    }
}

impl From<RetrySettings> for Retry {
    fn from(s: RetrySettings) -> Self {
        Retry {
            constant: ConstantDelay {
                attempts: s.constant_attempts.unwrap_or(0).max(0) as u32,
                seconds: s.constant_seconds.unwrap_or(0).clamp(0, u16::MAX as i32) as u16,
            },
            exponential: ExponentialDelay {
                attempts: s.exponential_attempts.unwrap_or(0).max(0) as u32,
                // Mirror ExponentialDelay::default().multiplier (1) when absent.
                multiplier: s
                    .exponential_multiplier
                    .unwrap_or(1)
                    .clamp(0, u16::MAX as i32) as u16,
                seconds: s.exponential_seconds.unwrap_or(0).clamp(0, u16::MAX as i32) as u16,
                random_factor: s
                    .exponential_random_factor
                    .map(|x| x.clamp(i8::MIN as i32, i8::MAX as i32) as i8),
            },
            retry_if: s.retry_if_expr.map(|expr| RetryIf { expr }),
        }
    }
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
