use std::{
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
};

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::{error, make_static};

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

#[derive(sqlx::Type, Debug, Clone, Copy)]
#[sqlx(type_name = "IMPORTER_KIND", rename_all = "lowercase")]
pub enum RunnableKind {
    Script,
    App,
    Flow,
}

pub async fn insert_runnable_settings(
    settings: (ConcurrencySettings, DebouncingSettings),
    runnable_id: i64,
    runnable_kind: RunnableKind,
    db: &Pool<Postgres>,
) -> error::Result<()> {
    let (cs_h, ds_h) = (
        settings.0.get_hash_or_insert(db.clone()).await?,
        settings.1.get_hash_or_insert(db.clone()).await?,
    );

    sqlx::query!(
        "
        INSERT INTO runnable_settings_references
            (runnable_id, runnable_kind, debouncing_settings_hash, concurrency_settings_hash)
        VALUES ($1, $2, $3, $4)
        ",
        runnable_id,
        runnable_kind as RunnableKind,
        ds_h,
        cs_h
    )
    .execute(db)
    .await?;

    Ok(())
}

pub trait RunnableSettings: PartialEq + Default + Hash {
    const NAME: &'static str;

    fn is_default(&self) -> bool {
        self == &Self::default()
    }

    fn insert_into_db(
        &self,
        hash: i64,
        db: Pool<Postgres>,
    ) -> impl Future<Output = error::Result<()>> + Send;

    /// If [[None]] is returned, then there is no settings to find, as they are default and can be easily replicated.
    fn get_hash_or_insert(
        &self,
        db: Pool<Postgres>,
    ) -> impl Future<Output = error::Result<Option<i64>>> {
        // TODO: maybe better not to cache on fs.
        // could cause bugs.
        make_static! {
            static ref RUNNABLE_SETTINGS_EXISTENCE: { i64 => () } in "runnable_settings_existence" <= 1000;
        }

        async move {
            // If it is default/empty, we optimize by telling "there is no hash for this"
            if self.is_default() {
                return Ok(None);
            }

            let hash = {
                // TODO:
                // CRITICAL:
                // what is we add new field. If that new field is None, then will it create another hash?
                // could it mess up linking?
                let mut h = DefaultHasher::new();
                self.hash(&mut h);

                // Since we use single fs cache, we hash the name as well to prevent from collisions.
                Self::NAME.hash(&mut h);
                h.finish() as i64
            };

            // If cache exists locally, then there is no need to insert to db
            // we can just assume db has the values.
            RUNNABLE_SETTINGS_EXISTENCE
                .get_or_insert_async(hash, self.insert_into_db(hash, db))
                .await?;

            Ok(Some(hash))
        }
    }
}

impl RunnableSettings for DebouncingSettings {
    const NAME: &'static str = "debouncing";

    async fn insert_into_db(&self, hash: i64, db: Pool<Postgres>) -> error::Result<()> {
        sqlx::query!(
            "INSERT INTO debouncing_settings (hash, debounce_key, debounce_delay_s, max_total_debouncing_time, max_total_debounces_amount, debounce_args_to_accumulate)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (hash) DO NOTHING",
            hash,
            self.debounce_key,
            self.debounce_delay_s,
            self.max_total_debouncing_time,
            self.max_total_debounces_amount,
            self.debounce_args_to_accumulate.as_deref()
        )
        .execute(&db)
        .await?;
        Ok(())
    }
}

impl RunnableSettings for ConcurrencySettings {
    const NAME: &'static str = "concurrency";
    async fn insert_into_db(&self, hash: i64, db: Pool<Postgres>) -> error::Result<()> {
        sqlx::query!(
            "INSERT INTO concurrency_settings (hash, concurrency_key, concurrent_limit, concurrency_time_window_s)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (hash) DO NOTHING",
            hash,
            self.concurrency_key,
            self.concurrent_limit,
            self.concurrency_time_window_s
        )
        .execute(&db)
        .await?;
        Ok(())
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
