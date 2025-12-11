mod settings;
use itertools::Itertools;
pub use settings::*;
use std::{
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
};

use serde::{de::DeserializeOwned, Serialize};
use sqlx::{postgres::PgRow, FromRow, Pool, Postgres};

use crate::{error, make_static};

lazy_static::lazy_static! {
    static ref WMDEBUG_FORCE_RUNNABLE_SETTINGS_V0: bool = std::env::var("WMDEBUG_FORCE_RUNNABLE_SETTINGS_V0").is_ok();
    pub static ref MIN_VERSION_RUNNABLE_SETTINGS_V0: semver::Version = semver::Version::new(1, 592, 0);
}

pub async fn min_version_supports_runnable_settings_v0() -> bool {
    // Check if workers support workspace dependencies feature
    if !*WMDEBUG_FORCE_RUNNABLE_SETTINGS_V0
        && !*crate::worker::MIN_VERSION_SUPPORTS_RUNNABLE_SETTINGS_V0
            .read()
            .await
    {
        tracing::debug!(
            "Internal: min version does not support runnable settings v0, falling back to old system",
        );

        false
    } else {
        true
    }
}

make_static! {
    static ref RUNNABLE_INDIVIDUAL_SETTINGS: { i64 => serde_json::Value } in "runnable_individual_settings" <= 1000;
    static ref RUNNABLE_SETTINGS_REFERENCES: { i64 => RunnableSettings } in "runnable_settings_references" <= 1000;
}

pub trait RunnableSettingsTrait:
    PartialEq
    + Default
    + Hash
    + for<'r> FromRow<'r, PgRow>
    + Serialize
    + DeserializeOwned
    + private_mod::RunnableSettingsTraitInternal
    + Send
    + Unpin
{
    fn is_default(&self) -> bool {
        self == &Self::default()
    }

    /// get [[Self]] from cache or fetch from db
    /// if not found, returns Error
    fn get<'a>(
        hash: i64,
        db: &'a Pool<Postgres>,
    ) -> impl Future<Output = Result<Self, error::Error>>
    where
        Self: 'a,
    {
        async move {
            let v = RUNNABLE_INDIVIDUAL_SETTINGS
                .get_or_insert_async(hash, async {
                    let r = sqlx::query_as::<Postgres, Self>(&format!(
                        "SELECT {} FROM {} WHERE hash = $1",
                        Self::INCLUDE_FIELDS.iter().join(","),
                        Self::SETTINGS_NAME,
                    ))
                    .bind(hash)
                    .fetch_one(db)
                    .await?;

                    serde_json::to_value(&r).map_err(error::Error::from)
                })
                .await?;
            // TODO: Less parsing
            serde_json::from_value(v).map_err(error::Error::from)
        }
    }

    /// set [[Self]] in db
    /// returns optional `hash` that `MUST` be
    fn set<'a>(&self, db: &'a Pool<Postgres>) -> impl Future<Output = error::Result<Option<i64>>> {
        async move {
            // If it is default/empty, we optimize by telling "there is no hash for this"
            if self.is_default() {
                return Ok(None);
            }

            let hash = {
                let mut h = DefaultHasher::new();
                self.hash(&mut h);

                // Since we use single fs cache, we hash the name as well to prevent from collisions.
                Self::SETTINGS_NAME.hash(&mut h);
                h.finish() as i64
            };

            // If already exists in cache, then it is already inserted, we will just ignore the result.
            RUNNABLE_INDIVIDUAL_SETTINGS
                .get_or_insert_async(hash, async move {
                    // If it is not in cache
                    // this means we don't know if it is in db
                    // in that case, we INSERT to be sure
                    let sql = dbg!(format!(
                        "INSERT INTO {} (hash, {})
             VALUES ($1, {})
             ON CONFLICT (hash) DO NOTHING",
                        Self::SETTINGS_NAME,
                        Self::INCLUDE_FIELDS.iter().join(","),
                        (1..=Self::INCLUDE_FIELDS.len())
                            .into_iter()
                            .map(|i| format!("${}", i + 1))
                            .join(",")
                    ));

                    self.bind_arguments(sqlx::query(&sql).bind(hash) /* First bind hash */) /* Then bind user settings */
                        .execute(db)
                        .await?;

                    // Cache existing value for this worker
                    serde_json::to_value(&self).map_err(error::Error::from)
                })
                .await?;

            Ok(Some(hash))
        }
    }
}

mod private_mod {
    use sqlx::{query::Query, Database, Postgres};

    pub type Q<'a> = Query<'a, Postgres, <Postgres as Database>::Arguments<'a>>;

    pub trait RunnableSettingsTraitInternal {
        const SETTINGS_NAME: &'static str;
        const INCLUDE_FIELDS: &'static [&'static str];

        fn bind_arguments<'a>(&'a self, q: Q<'a>) -> Q<'a>;
    }
}
