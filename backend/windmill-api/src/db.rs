/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use futures::FutureExt;
use sqlx::Executor;

use sqlx::{
    migrate::{Migrate, MigrateError},
    pool::PoolConnection,
    PgConnection, Pool, Postgres,
};
use windmill_audit::audit_ee::{AuditAuthor, AuditAuthorable};
use windmill_common::utils::generate_lock_id;
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
                        tracing::error!("Error acquiring lock: {e:#}");
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
            if migration.version == 20221207103910 {
                tracing::info!("Skipping migration 20221207103910 to avoid using md5");
                self.inner
                    .execute(include_str!(
                        "../../custom_migrations/create_workspace_without_md5.sql"
                    ))
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

pub async fn migrate(db: &DB) -> Result<(), Error> {
    let migrator = db.acquire().await?;
    let mut custom_migrator = CustomMigrator { inner: migrator };

    if let Err(err) = fix_flow_versioning_migration(&mut custom_migrator, db).await {
        tracing::error!("Could not apply flow versioning fix migration: {err:#}");
    }

    let db2 = db.clone();
    let _ = tokio::task::spawn(async move {
        if let Err(err) = fix_job_completed_index(&db2).await {
            tracing::error!("Could not apply job completed index fix migration: {err:#}");
        }
    });

    match sqlx::migrate!("../migrations")
        .run_direct(&mut custom_migrator)
        .await
    {
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

    Ok(())
}

async fn fix_flow_versioning_migration(
    migrator: &mut CustomMigrator,
    db: &DB,
) -> Result<(), Error> {
    let has_done_migration = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = 'fix_flow_versioning_2')",
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    if !has_done_migration {
        migrator.lock().await?;

        if migrator
            .list_applied_migrations()
            .await?
            .iter()
            .any(|x| x.version == 20240630102146)
        {
            let has_done_migration = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = 'fix_flow_versioning_2')",
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);

            if !has_done_migration {
                let query = include_str!("../../custom_migrations/fix_flow_versioning_2.sql");
                tracing::info!("Applying fix_flow_versioning_2.sql");
                let mut tx: sqlx::Transaction<'_, Postgres> = db.begin().await?;
                tx.execute(query).await?;
                tracing::info!("Applied fix_flow_versioning_2.sql");
                sqlx::query!(
                    "INSERT INTO windmill_migrations (name) VALUES ('fix_flow_versioning_2')"
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }
        }

        migrator.unlock().await?;
    }
    Ok(())
}

macro_rules! run_windmill_migration {
    ($migration_job_name:expr, $db:expr, $code:block) => {
        {
            let migration_job_name = $migration_job_name;
            let db: &Pool<Postgres> = $db;

            let has_done_migration = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = $1)",
                migration_job_name
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);
            if !has_done_migration {
                tracing::info!("Applying {migration_job_name} migration");
                let mut tx = db.begin().await?;
                let mut r = false;
                while !r {
                    r = sqlx::query_scalar!("SELECT pg_try_advisory_lock(4242)")
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(|e| {
                            tracing::error!("Error acquiring {migration_job_name} lock: {e:#}");
                            sqlx::migrate::MigrateError::Execute(e)
                        })?
                        .unwrap_or(false);
                    if !r {
                        tracing::info!("PG {migration_job_name} lock already acquired by another server or worker, retrying in 5s. (look for the advisory lock in pg_lock with granted = true)");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
                tracing::info!("acquired lock for {migration_job_name}");

                let has_done_migration = sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = $1)",
                    migration_job_name
                )
                .fetch_one(db)
                .await?
                .unwrap_or(false);

                if !has_done_migration {

                    $code

                    sqlx::query!(
                        "INSERT INTO windmill_migrations (name) VALUES ($1) ON CONFLICT DO NOTHING",
                        migration_job_name
                    )
                    .execute(&mut *tx)
                    .await?;
                    tracing::info!("Finished applying {migration_job_name} migration");
                } else {
                    tracing::info!("migration {migration_job_name} already done");
                }

                let _ = sqlx::query("SELECT pg_advisory_unlock(4242)")
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                tracing::info!("released lock for {migration_job_name}");
            } else {
                tracing::info!("migration {migration_job_name} already done");

            }
        }
    };
}

async fn fix_job_completed_index(db: &DB) -> Result<(), Error> {
    // let has_done_migration = sqlx::query_scalar!(
    //     "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = 'fix_job_completed_index')"
    // )
    // .fetch_one(db)
    // .await?
    // .unwrap_or(false);
    // if !has_done_migration {
    //     tracing::info!("Applying fix_job_completed_index migration");
    //     let mut tx = db.begin().await?;
    //     let mut r = false;
    //     while !r {
    //         r = sqlx::query_scalar!("SELECT pg_try_advisory_lock(4242)")
    //             .fetch_one(&mut *tx)
    //             .await
    //             .map_err(|e| {
    //                 tracing::error!("Error acquiring fix_job_completed_index lock: {e:#}");
    //                 sqlx::migrate::MigrateError::Execute(e)
    //             })?
    //             .unwrap_or(false);
    //         if !r {
    //             tracing::info!("PG fix_job_completed_index_migration lock already acquired by another server or worker, retrying in 5s. (look for the advisory lock in pg_lock with granted = true)");
    //             tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    //         }
    //     }
    //     // sqlx::query(
    //     //     "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_completed_job_workspace_id_created_at_new ON completed_job (workspace_id, job_kind, is_skipped, is_flow_step, created_at DESC, started_at DESC)"
    //     // ).execute(db).await?;

    //     sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at")
    //         .execute(db)
    //         .await?;

    //     sqlx::query!("INSERT INTO windmill_migrations (name) VALUES ('fix_job_completed_index') ON CONFLICT DO NOTHING")
    //         .execute(&mut *tx)
    //         .await?;
    //     let _ = sqlx::query("SELECT pg_advisory_unlock(4242)")
    //         .execute(&mut *tx)
    //         .await?;
    //     tx.commit().await?;
    // }

    run_windmill_migration!("fix_job_completed_index_2", &db, {
        //     sqlx::query(
        //     "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_completed_job_workspace_id_created_at_new_2 ON completed_job (workspace_id, job_kind, success, is_skipped, is_flow_step, created_at DESC)"
        // ).execute(db).await?;

        //     sqlx::query(
        //     "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_completed_job_workspace_id_started_at_new ON completed_job (workspace_id, job_kind, success, is_skipped, is_flow_step, started_at DESC)"
        // ).execute(db).await?;

        sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at")
            .execute(db)
            .await?;

        sqlx::query(
            "DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at_new",
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("fix_job_completed_index_3", &db, {
        sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS index_completed_job_on_schedule_path")
            .execute(db)
            .await?;

        sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS concurrency_limit_stats_queue")
            .execute(db)
            .await?;

        sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS root_job_index")
            .execute(db)
            .await?;

        sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS index_completed_on_created")
            .execute(db)
            .await?;
    });

    run_windmill_migration!("fix_job_completed_index_4", &db, {
        let migration_job_name = "fix_job_completed_index_4";
        let mut i = 1;
        tracing::info!("step {i} of {migration_job_name} migration");
        sqlx::query("create index concurrently  if not exists ix_completed_job_workspace_id_created_at_new_3 ON completed_job  (workspace_id,  created_at DESC)")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_workspace_id_created_at_new_8 ON completed_job  (workspace_id, created_at DESC) where job_kind in ('deploymentcallback') AND parent_job IS NULL")
            .execute(db)
            .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_workspace_id_created_at_new_9 ON completed_job  (workspace_id, created_at DESC) where job_kind in ('dependencies', 'flowdependencies', 'appdependencies') AND parent_job IS NULL")
            .execute(db)
            .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_workspace_id_created_at_new_5 ON completed_job  (workspace_id, created_at DESC) where job_kind in ('preview', 'flowpreview') AND parent_job IS NULL")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_workspace_id_created_at_new_6 ON completed_job  (workspace_id, created_at DESC) where job_kind in ('script', 'flow') AND parent_job IS NULL")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_workspace_id_created_at_new_7 ON completed_job  (workspace_id, success, created_at DESC) where job_kind in ('script', 'flow') AND parent_job IS NULL")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_workspace_id_started_at_new_2 ON completed_job  (workspace_id, started_at DESC)")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists root_job_index_by_path_2 ON completed_job (workspace_id, script_path, created_at desc) WHERE parent_job IS NULL")
                .execute(db)
                .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("create index concurrently if not exists ix_completed_job_created_at ON completed_job  (created_at DESC)")
                    .execute(db)
                    .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query(
            "DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at_new_2",
        )
        .execute(db)
        .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query(
            "DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_started_at_new",
        )
        .execute(db)
        .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query("DROP INDEX CONCURRENTLY IF EXISTS root_job_index_by_path")
            .execute(db)
            .await?;
    });

    run_windmill_migration!("fix_labeled_jobs_index", &db, {
        tracing::info!("Special migration to add index concurrently on job labels 2");
        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS labeled_jobs_on_jobs")
            .execute(db)
            .await?;
        sqlx::query!(
        "CREATE INDEX CONCURRENTLY labeled_jobs_on_jobs ON completed_job USING GIN ((result -> 'wm_labels')) WHERE result ? 'wm_labels'"
        ).execute(db).await?;
    });

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

impl From<&ApiAuthed> for AuditAuthor {
    fn from(value: &ApiAuthed) -> Self {
        Self {
            email: value.email.clone(),
            username: value.username.clone(),
            username_override: value.username_override.clone(),
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
