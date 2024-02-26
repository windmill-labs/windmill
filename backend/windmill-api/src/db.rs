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

use sqlx::{migrate::Migrate, pool::PoolConnection, Pool, Postgres};
use windmill_common::{
    db::{Authable, Authed},
    error::Error,
};

pub type DB = Pool<Postgres>;

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
            tracing::info!("Acquiring global PG lock  for migration purposes (if there are migrations to apply, will apply migrations or wait for the first acquirer of the lock to apply them)");
            let r = self.inner.lock().await;
            tracing::info!("Acquired global PG lock for migration purposes");
            r
        }.boxed()
    }

    fn unlock(
        &mut self,
    ) -> futures::prelude::future::BoxFuture<'_, Result<(), sqlx::migrate::MigrateError>> {
        async {
            tracing::info!("Releasing PG lock");
            let r = self.inner.unlock().await;
            tracing::info!("Released PG lock");
            r
        }.boxed()
        
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
        }.boxed()
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
