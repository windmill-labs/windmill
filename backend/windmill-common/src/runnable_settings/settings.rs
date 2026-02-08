use std::future::Future;

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

/// Returns error if provided `hash` has no corresponding entry in db
/// If `hash` is None, returns Default
pub fn from_handle<'a>(
    hash: Option<i64>,
    db: &'a DB,
) -> impl Future<Output = error::Result<RunnableSettings>> + 'a {
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
            Ok(RunnableSettings::default())
        }
    }
}

pub async fn insert_rs(rs: RunnableSettings, db: &Pool<Postgres>) -> error::Result<Option<i64>> {
    use std::hash::{Hash, Hasher};

    if !min_version_supports_runnable_settings_v0().await
        || (rs.debouncing_settings.is_none() && rs.concurrency_settings.is_none())
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
                "INSERT INTO runnable_settings (hash, debouncing_settings, concurrency_settings)
                VALUES ($1, $2, $3)
                ON CONFLICT (hash)
                DO NOTHING",
                hash,
                rs.debouncing_settings,
                rs.concurrency_settings
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
