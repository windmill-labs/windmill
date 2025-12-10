/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use futures::FutureExt;
use sqlx::{
    migrate::{Migrate, MigrateError},
    pool::PoolConnection,
    Executor, PgConnection, Postgres,
};

use tokio::task::JoinHandle;
use windmill_audit::audit_oss::AuditAuthorable;
pub use windmill_common::db::DB;
use windmill_common::{
    db::{Authable, Authed, AuthedRef},
    error::Error,
    utils::generate_lock_id,
};

async fn current_database(conn: &mut PgConnection) -> Result<String, MigrateError> {
    // language=SQL
    Ok(sqlx::query_scalar("SELECT current_database()")
        .fetch_one(conn)
        .await?)
}

lazy_static::lazy_static! {
    pub static ref OVERRIDDEN_MIGRATIONS: std::collections::HashMap<i64, String> = vec![(20221207103910, include_str!(
                        "../../custom_migrations/create_workspace_without_md5.sql"
                    ).to_string()),
                    (20240216100535, include_str!(
                        "../../migrations/20240216100535_improve_policies.up.sql"
                    ).replace("public.", "")),
                    (20240403083110, include_str!(
                        "../../migrations/20240403083110_remove_team_id_constraint.up.sql"
                    ).replace("public.", "")),
                    (20240613150524, include_str!(
                        "../../migrations/20240613150524_add_job_perms.up.sql"
                    ).replace("public.", "")),
                    (20250102145420, include_str!(
                        "../../migrations/20250102145420_more_captures.up.sql"
                    ).replace("public.", "")),
                    (20250429211554, include_str!(
                        "../../migrations/20250429211554_create_indices_on_queue.up.sql"
                    ).replace("public.", "")),
                     (20241006144414, include_str!(
                        "../../custom_migrations/grant_all_current_schema.sql"
                    ).to_string()),
                    (20221105003256, "DELETE FROM workspace_invite WHERE workspace_id = 'demo' AND email = 'ruben@windmill.dev';".to_string()),
                    (20221123151919, "".to_string()),
                    (20251105100125, include_str!(
                        "../../migrations/20251105100125_legacy_sql_result_flag.up.sql"
                    ).replace("✅", "")),
                    ].into_iter().collect();
}

pub struct CustomMigrator {
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
                r = match tokio::time::timeout(std::time::Duration::from_secs(5), sqlx::query_scalar!("SELECT pg_try_advisory_lock($1)", lock_id)  
                    .fetch_one(&mut *self.inner))
                    .await
                    {
                        Ok(Ok(r)) => r.unwrap_or(false),
                        Ok(Err(e)) => {
                            tracing::error!("Error acquiring lock: {e:#}");
                            return Err(sqlx::migrate::MigrateError::Execute(e));
                        }
                        Err(e) => {
                            tracing::error!("Timed out acquiring lock retrying in 5s: {e:#}");
                            false
                        }
                    };
                if !r {
                    tracing::info!("PG migration lock already acquired by another server or worker, a migration is in progress, this may take a long time if you have many jobs and be normal, rechecking in 5s.");
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


            if let Some(migration_sql) = OVERRIDDEN_MIGRATIONS.get(&migration.version) {
                tracing::info!("Using custom migration for version {}", migration.version);
                // tracing::info!("Migration SQL: {}", migration_sql);

                self.inner
                    .execute(&**migration_sql)
                    .await?;
                let _ = sqlx::query(
                    r#"
                INSERT INTO _sqlx_migrations ( version, description, success, checksum, execution_time )
                VALUES ( $1, $2, TRUE, $3, -1 ) ON CONFLICT DO NOTHING
                            "#,
                )
                .bind(migration.version)
                .bind(&*migration.description)
                .bind(&*migration.checksum)
                .execute(&mut *self.inner)
                .await?;
                return Ok(std::time::Duration::from_secs(0));
            } else {
                let r = self.inner.apply(migration).await;
                tracing::info!("Finished applying migration {}", migration.version);
                return r;
            }
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

pub async fn migrate(
    db: &DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<Option<JoinHandle<()>>, Error> {
    let migrator = db.acquire().await?;
    let mut custom_migrator = CustomMigrator { inner: migrator };

    if let Err(err) = sqlx::query!(
        "DELETE FROM _sqlx_migrations WHERE
        version=20250131115248 OR version=20250902085503 OR version=20250201145630 OR
        version=20250201145631 OR version=20250201145632 OR version=20251006143821"
    )
    .execute(db)
    .await
    {
        tracing::info!("Could not remove sqlx migrations: {err:#}");
    }

    tokio::select! {
        _ = killpill_rx.recv() => {
            tracing::info!("Killpill received, stopping migration");
            return Ok(None);
        }
        migration_result = sqlx::migrate!("../migrations")
            .run_direct(&mut custom_migrator)
     => {
        match migration_result {
            Ok(_) => Ok(()),
            Err(sqlx::migrate::MigrateError::VersionMissing(e)) => {
                tracing::error!("Database had been applied more migrations than this container.
                This usually mean than another container on a more recent version migrated the database and this one is on an earlier version.
                Please update the container to latest. Not critical, but may cause issues if migration introduced a breaking change. Version missing: {e:#}");
                custom_migrator.unlock().await?;
                Ok(())
            }
            Err(err) => Err(err),
        }?;
        }
    }

    return crate::live_migrations::custom_migrations(&mut custom_migrator, db).await;
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq)]
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
    pub token_prefix: Option<String>,
}

impl ApiAuthed {
    pub fn to_authed_ref<'e>(&'e self) -> AuthedRef<'e> {
        AuthedRef {
            email: &self.email,
            username: &self.username,
            is_admin: &self.is_admin,
            is_operator: &self.is_operator,
            groups: &self.groups,
            folders: &self.folders,
            scopes: &self.scopes,
            token_prefix: &self.token_prefix,
        }
    }
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
            token_prefix: value.token_prefix,
        }
    }
}

impl From<Authed> for ApiAuthed {
    fn from(value: Authed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
            username_override: None, // Authed doesn't have this field, so default to None
            token_prefix: value.token_prefix,
        }
    }
}

impl ApiAuthed {
    pub fn display_username(&self) -> &str {
        self.username_override.as_ref().unwrap_or(&self.username)
    }
}

impl AuditAuthorable for ApiAuthed {
    fn username(&self) -> &str {
        self.username.as_str()
    }
    fn email(&self) -> &str {
        self.email.as_str()
    }
    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }
    fn token_prefix(&self) -> Option<&str> {
        self.token_prefix.as_deref()
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
