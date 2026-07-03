use sqlx::{Pool, Postgres};

use crate::{
    error,
    runnable_settings::{
        min_version_supports_runnable_settings_v0, private_mod::Q, RunnableSettingsTrait,
    },
    DB,
};

pub use windmill_types::runnable_settings::*;

pub async fn prefetch_cached(
    rs: &RunnableSettings,
    db: &DB,
) -> error::Result<(DebouncingSettings, ConcurrencySettings)> {
    Ok((
        if let Some(hash) = rs.debouncing_settings {
            DebouncingSettings::get(hash, db).await?
        } else {
            Default::default()
        },
        if let Some(hash) = rs.concurrency_settings {
            ConcurrencySettings::get(hash, db).await?
        } else {
            Default::default()
        },
    ))
}

pub async fn prefetch_cached_from_handle(
    hash: Option<i64>,
    db: &DB,
) -> error::Result<(DebouncingSettings, ConcurrencySettings)> {
    let rs = from_handle(hash, db).await?;
    prefetch_cached(&rs, db).await
}

/// Like [`prefetch_cached`], but reuses a held transaction's connection instead
/// of checking out a second one from the pool. Use this on paths that already
/// hold an open `tx` to avoid dual-connection pool contention.
pub async fn prefetch_cached_tx(
    rs: &RunnableSettings,
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> error::Result<(DebouncingSettings, ConcurrencySettings)> {
    Ok((
        if let Some(hash) = rs.debouncing_settings {
            DebouncingSettings::get(hash, &mut **tx).await?
        } else {
            Default::default()
        },
        if let Some(hash) = rs.concurrency_settings {
            ConcurrencySettings::get(hash, &mut **tx).await?
        } else {
            Default::default()
        },
    ))
}

/// Resolve the retry policy (if any) for a job from its `runnable_settings_handle`.
/// Returns `None` when the job carries no retry policy. Read lazily on the
/// failure path only — never on the hot job-pull path.
pub async fn prefetch_retry_from_handle(
    hash: Option<i64>,
    db: &DB,
) -> error::Result<Option<RetrySettings>> {
    let rs = from_handle(hash, db).await?;
    Ok(if let Some(hash) = rs.retry_settings {
        Some(RetrySettings::get(hash, db).await?)
    } else {
        None
    })
}

/// Returns error if provided `hash` has no corresponding entry in db
/// If `hash` is None, returns Default
pub async fn from_handle<'e>(
    hash: Option<i64>,
    db: impl sqlx::PgExecutor<'e>,
) -> error::Result<RunnableSettings> {
    if let Some(hash) = hash {
        super::RUNNABLE_SETTINGS_REFERENCES
            .get_or_insert_async(hash, async {
                sqlx::query_as!(
                    RunnableSettings,
                    r#"SELECT concurrency_settings, debouncing_settings, retry_settings FROM runnable_settings WHERE hash = $1"#,
                    hash
                )
                .fetch_one(db)
                .await
                .map_err(error::Error::from)
            })
            .await
    } else {
        Ok(RunnableSettings::default())
    }
}

pub async fn insert_rs(rs: RunnableSettings, db: &Pool<Postgres>) -> error::Result<Option<i64>> {
    use std::hash::{Hash, Hasher};

    if !min_version_supports_runnable_settings_v0().await
        || (rs.debouncing_settings.is_none()
            && rs.concurrency_settings.is_none()
            && rs.retry_settings.is_none())
    {
        return Ok(None);
    }

    let hash = {
        let mut h = std::hash::DefaultHasher::new();
        rs.hash(&mut h);
        h.finish() as i64
    };

    super::RUNNABLE_SETTINGS_REFERENCES
        .get_or_insert_async(hash, async {
            sqlx::query!(
                "INSERT INTO runnable_settings (hash, debouncing_settings, concurrency_settings, retry_settings)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (hash)
                DO NOTHING",
                hash,
                rs.debouncing_settings,
                rs.concurrency_settings,
                rs.retry_settings
            )
            .execute(db)
            .await?;
            Ok(rs)
        })
        .await?;

    Ok(Some(hash))
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
impl super::private_mod::RunnableSettingsTraitInternal for RetrySettings {
    const SETTINGS_NAME: &str = "retry_settings";
    const INCLUDE_FIELDS: &[&str] = &[
        "constant_attempts",
        "constant_seconds",
        "exponential_attempts",
        "exponential_multiplier",
        "exponential_seconds",
        "exponential_random_factor",
        "retry_if_expr",
    ];
    fn bind_arguments<'a>(&'a self, q: Q<'a>) -> Q<'a> {
        q.bind(&self.constant_attempts)
            .bind(&self.constant_seconds)
            .bind(&self.exponential_attempts)
            .bind(&self.exponential_multiplier)
            .bind(&self.exponential_seconds)
            .bind(&self.exponential_random_factor)
            .bind(&self.retry_if_expr)
    }
}
impl super::RunnableSettingsTrait for RetrySettings {}
