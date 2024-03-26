/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use futures::FutureExt;
#[cfg(feature = "enterprise")]
use sqlx::Executor;

use sqlx::{
    migrate::{Migrate, MigrateError},
    pool::PoolConnection,
    PgConnection, Pool, Postgres,
};
use windmill_common::{
    db::{Authable, Authed},
    error::Error,
};

pub type DB = Pool<Postgres>;

async fn current_database(conn: &mut PgConnection) -> Result<String, MigrateError> {
    // language=SQL
    Ok(sqlx::query_scalar("SELECT current_database()")
        .fetch_one(conn)
        .await?)
}

// inspired from rails: https://github.com/rails/rails/blob/6e49cc77ab3d16c06e12f93158eaf3e507d4120e/activerecord/lib/active_record/migration.rb#L1308
fn generate_lock_id(database_name: &str) -> i64 {
    const CRC_IEEE: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
    // 0x3d32ad9e chosen by fair dice roll
    0x3d32ad9e * (CRC_IEEE.checksum(database_name.as_bytes()) as i64)
}

struct CustomMigrator {
    inner: PoolConnection<Postgres>,
}
impl Migrate for CustomMigrator {
    fn ensure_migrations_table(
        &mut self,
    ) -> futures::prelude::future::BoxFuture<'_, Result<(), sqlx::migrate::MigrateError>> {
        self.inner.ensure_migrations_table()
    }

    fn dirty_version(
        &mut self,
    ) -> futures::prelude::future::BoxFuture<'_, Result<Option<i64>, sqlx::migrate::MigrateError>>
    {
        self.inner.dirty_version()
    }

    fn list_applied_migrations(
        &mut self,
    ) -> futures::prelude::future::BoxFuture<
        '_,
        Result<Vec<sqlx::migrate::AppliedMigration>, sqlx::migrate::MigrateError>,
    > {
        self.inner.list_applied_migrations()
    }

    fn lock(
        &mut self,
    ) -> futures::prelude::future::BoxFuture<'_, Result<(), sqlx::migrate::MigrateError>> {
        async {
            if std::env::var("SKIP_PG_LOCK").is_ok() {
                tracing::info!("Skipping PG lock acquisition");
                return Ok(());
            }

            let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *self.inner)
                .await?;
            tracing::info!("Acquiring global PG lock for potential migration with pid: {pid:?}");
            let database_name = current_database(&mut *self.inner).await?;
            let lock_id = generate_lock_id(&database_name);

            let mut r = false;

            while !r {
                r = sqlx::query_scalar!("SELECT pg_try_advisory_lock($1)", lock_id)
                    .fetch_one(&mut *self.inner)
                    .await
                    .map_err(|e| {
                        tracing::error!("Error acquiring lock: {e}");
                        sqlx::migrate::MigrateError::Execute(e)
                    })?
                    .unwrap_or(false);
                if !r {
                    tracing::info!("PG lock already acquired by another server or worker, retrying in 5s. (look for the advisory lock in pg_lock with granted = true)");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
            tracing::info!("Acquired global PG lock");

            return Ok(());
        }
        .boxed()
    }

    fn unlock(
        &mut self,
    ) -> futures::prelude::future::BoxFuture<'_, Result<(), sqlx::migrate::MigrateError>> {
        async {
            if std::env::var("SKIP_PG_UNLOCK").is_ok() {
                tracing::info!("Skipping PG lock release");
                return Ok(());
            }
            tracing::info!("Releasing PG lock");
            let database_name = current_database(&mut *self.inner).await?;
            let lock_id = generate_lock_id(&database_name);
            let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(lock_id)
                .execute(&mut *self.inner)
                .await?;

            tracing::info!("Released PG lock");
            Ok(())
        }
        .boxed()
    }

    fn apply<'e: 'm, 'm>(
        &'e mut self,
        migration: &'m sqlx::migrate::Migration,
    ) -> futures::prelude::future::BoxFuture<
        'm,
        Result<std::time::Duration, sqlx::migrate::MigrateError>,
    > {
        async {
            tracing::info!(
                "Started applying migration {}: {}",
                migration.version,
                migration.description
            );
            let r = self.inner.apply(migration).await;
            tracing::info!("Finished applying migration {}", migration.version);
            r
        }
        .boxed()
    }

    fn revert<'e: 'm, 'm>(
        &'e mut self,
        migration: &'m sqlx::migrate::Migration,
    ) -> futures::prelude::future::BoxFuture<
        'm,
        Result<std::time::Duration, sqlx::migrate::MigrateError>,
    > {
        self.inner.revert(migration)
    }
}

pub async fn migrate(db: &DB) -> Result<(), Error> {
    let migrator = db.acquire().await?;
    let mut custom_migrator = CustomMigrator { inner: migrator };
    match sqlx::migrate!("../migrations")
        .run_direct(&mut custom_migrator)
        .await
    {
        Ok(_) => Ok(()),
        Err(sqlx::migrate::MigrateError::VersionMissing(e)) => {
            tracing::error!("Database had been applied more migrations than this container. 
            This usually mean than another container on a more recent version migrated the database and this one is on an earlier version.
            Please update the container to latest. Not critical, but may cause issues if migration introduced a breaking change. Version missing: {e}");
            Ok(())
        }
        Err(err) => Err(err),
    }?;

    #[cfg(feature = "enterprise")]
    if let Err(e) = windmill_migrations(&mut custom_migrator, db).await {
        tracing::error!("Could not apply windmill custom migrations: {e}")
    }

    Ok(())
}

#[cfg(feature = "enterprise")]
async fn windmill_migrations(migrator: &mut CustomMigrator, db: &DB) -> Result<(), Error> {
    if std::env::var("MIGRATION_NO_BYPASSRLS").is_ok() {
        #[cfg(feature = "enterprise")]
        {
            migrator.lock().await?;
            let has_done_migration = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = 'bypassrls_1-2')",
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);

            if !has_done_migration {
                let query = include_str!("../../custom_migrations/bypassrls_1.sql");
                tracing::info!("Applying bypassrls_1.sql");
                let mut tx: sqlx::Transaction<'_, Postgres> = db.begin().await?;
                tx.execute(query).await?;
                tracing::info!("Applied bypassrls_1.sql");
                sqlx::query!("INSERT INTO windmill_migrations (name) VALUES ('bypassrls_1-2')")
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
            }
            migrator.unlock().await?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ApiAuthed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    // (folder name, can write, is owner)
    pub folders: Vec<(String, bool, bool)>,
    pub scopes: Option<Vec<String>>,
    pub username_override: Option<String>,
}

impl From<ApiAuthed> for Authed {
    fn from(value: ApiAuthed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
        }
    }
}

impl ApiAuthed {
    pub fn display_username(&self) -> &str {
        self.username_override.as_ref().unwrap_or(&self.username)
    }
}

impl Authable for ApiAuthed {
    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[std::string::String]> {
        self.scopes.as_ref().map(|x| x.as_slice())
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }
}
