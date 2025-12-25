use std::{
    future::Future,
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::{
    error,
    runnable_settings::{
        min_version_supports_runnable_settings_v0, private_mod::Q, RunnableSettingsTrait,
    },
    DB,
};

#[derive(Deserialize, Clone, Copy, Serialize, Default, Hash)]
pub struct RunnableSettings {
    pub debouncing_settings: Option<i64>,
    pub concurrency_settings: Option<i64>,
}

impl RunnableSettings {
    pub async fn prefetch_cached<'a>(
        &self,
        db: &DB,
    ) -> error::Result<(DebouncingSettings, ConcurrencySettings)> {
        Ok((
            if let Some(hash) = self.debouncing_settings {
                DebouncingSettings::get(hash, db).await?
            } else {
                Default::default()
            },
            if let Some(hash) = self.concurrency_settings {
                ConcurrencySettings::get(hash, db).await?
            } else {
                Default::default()
            },
        ))
    }

    pub async fn prefetch_cached_from_handle<'a>(
        hash: Option<i64>,
        db: &'a DB,
    ) -> error::Result<(DebouncingSettings, ConcurrencySettings)> {
        Self::from_runnable_settings_handle(hash, db)
            .await?
            .prefetch_cached(db)
            .await
    }
    /// Returns error if provided `hash` has no corresponding entry in db
    /// If `hash` is None, returnes Default
    pub fn from_runnable_settings_handle<'a>(
        hash: Option<i64>,
        db: &'a DB,
    ) -> impl Future<Output = error::Result<Self>> + 'a {
        async move {
            if let Some(hash) = hash {
                super::RUNNABLE_SETTINGS_REFERENCES
                .get_or_insert_async(hash, async {
                    sqlx::query_as!(
                        RunnableSettings,
                        r#"SELECT concurrency_settings, debouncing_settings FROM runnable_settings WHERE hash = $1"#,
                        hash
                    )
                    .fetch_one(db)
                    .await
                    .map_err(error::Error::from)
                })
                .await
            } else {
                Ok(Self::default())
            }
        }
    }

    pub async fn insert_cached(self, db: &Pool<Postgres>) -> error::Result<Option<i64>> {
        if !min_version_supports_runnable_settings_v0().await
            || (self.debouncing_settings.is_none() && self.concurrency_settings.is_none())
        {
            return Ok(None);
        }

        let hash = {
            let mut h = std::hash::DefaultHasher::new();
            self.hash(&mut h);
            h.finish() as i64
        };

        super::RUNNABLE_SETTINGS_REFERENCES
            .get_or_insert_async(hash, async {
                sqlx::query!(
                    "INSERT INTO runnable_settings (hash, debouncing_settings, concurrency_settings)
                VALUES ($1, $2, $3)
                ON CONFLICT (hash)
                DO NOTHING",
                    hash,
                    self.debouncing_settings,
                    self.concurrency_settings
                )
                .execute(db)
                .await?;
                // .map_err(error::Error::from)
                Ok(self)
            })
            .await?;

        Ok(Some(hash))
    }
}

// TODO: Add validation logic.
#[derive(
    Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, sqlx::FromRow, sqlx::Type,
)]
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

impl super::private_mod::RunnableSettingsTraitInternal for DebouncingSettings {
    const SETTINGS_NAME: &str = "debouncing_settings";
    const INCLUDE_FIELDS: &[&str] = &[
        "debounce_key",
        "debounce_delay_s",
        "max_total_debouncing_time",
        "max_total_debounces_amount",
        "debounce_args_to_accumulate",
    ];
    fn bind_arguments<'a>(&'a self, q: Q<'a>) -> Q<'a> {
        q.bind(&self.debounce_key)
            .bind(&self.debounce_delay_s)
            .bind(&self.max_total_debouncing_time)
            .bind(&self.max_total_debounces_amount)
            .bind(&self.debounce_args_to_accumulate)
    }
}

impl super::RunnableSettingsTrait for DebouncingSettings {}
impl super::private_mod::RunnableSettingsTraitInternal for ConcurrencySettings {
    const SETTINGS_NAME: &str = "concurrency_settings";
    const INCLUDE_FIELDS: &[&str] = &[
        "concurrency_key",
        "concurrent_limit",
        "concurrency_time_window_s",
    ];
    fn bind_arguments<'a>(&'a self, q: Q<'a>) -> Q<'a> {
        q.bind(&self.concurrency_key)
            .bind(&self.concurrent_limit)
            .bind(&self.concurrency_time_window_s)
    }
}
impl super::RunnableSettingsTrait for ConcurrencySettings {}

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
