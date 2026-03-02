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
pub use windmill_common::db::DB;
use windmill_common::{
    error::Error,
    utils::{generate_lock_id, GIT_VERSION},
};

#[allow(unused_imports)]
pub use windmill_api_auth::{ApiAuthed, OptJobAuthed};

async fn current_database(conn: &mut PgConnection) -> Result<String, MigrateError> {
    // language=SQL
    Ok(sqlx::query_scalar("SELECT current_database()")
        .fetch_one(conn)
        .await?)
}

lazy_static::lazy_static! {
    pub static ref OVERRIDDEN_MIGRATIONS: std::collections::HashMap<i64, String> = vec![(20220123221903, include_str!(
                        "../../migrations/20220123221903_first.up.sql"
                    ).replace("create SCHEMA IF NOT exists extensions;", "")
                     .replace("create extension if not exists \"uuid-ossp\"      with schema extensions;", "")),
                    (20221207103910, include_str!(
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
                    (20260107133344, "".to_string()),
                    (20260126235947, include_str!(
                        "../../custom_migrations/lowercase_emails_safe.sql"
                    ).to_string()),
                    (20260206000000, "".to_string()),
                    (20260207000001, include_str!(
                        "../../migrations/20260207000001_concurrent_indexes_v2_job.up.sql"
                    ).replace("CREATE INDEX", "CREATE INDEX CONCURRENTLY").replace("DROP INDEX", "DROP INDEX CONCURRENTLY")),
                    (20260207000002, include_str!(
                        "../../migrations/20260207000002_concurrent_indexes_v2_job_completed.up.sql"
                    ).replace("CREATE INDEX", "CREATE INDEX CONCURRENTLY").replace("DROP INDEX", "DROP INDEX CONCURRENTLY").replace("DROP INDEX CONCURRENTLY IF EXISTS labeled_jobs_on_jobs;", "")),
                    (20260207000003, include_str!(
                        "../../migrations/20260207000003_concurrent_indexes_v2_job_queue.up.sql"
                    ).replace("CREATE INDEX", "CREATE INDEX CONCURRENTLY").replace("DROP INDEX", "DROP INDEX CONCURRENTLY")),
                    (20260207000004, include_str!(
                        "../../migrations/20260207000004_concurrent_indexes_other.up.sql"
                    ).replace("CREATE INDEX", "CREATE INDEX CONCURRENTLY").replace("DROP INDEX", "DROP INDEX CONCURRENTLY")),
                    (20260225100000, include_str!(
                        "../../migrations/20260225100000_asset_covering_index.up.sql"
                    ).replace("CREATE INDEX", "CREATE INDEX CONCURRENTLY").replace("DROP INDEX", "DROP INDEX CONCURRENTLY")),
                    (20260228000000, include_str!(
                        "../../migrations/20260228000000_v2_job_completed_failure_index.up.sql"
                    ).replace("CREATE INDEX", "CREATE INDEX CONCURRENTLY")),
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

                if migration_sql.contains("CONCURRENTLY") {
                    // CONCURRENTLY operations cannot run inside a transaction block
                    // or a multi-statement query (PostgreSQL requires top-level execution).
                    // Split into individual statements and execute each separately.
                    for stmt in migration_sql.split(';') {
                        let stmt = stmt.trim();
                        if !stmt.is_empty()
                            && stmt.lines().any(|l| {
                                let t = l.trim();
                                !t.is_empty() && !t.starts_with("--")
                            })
                        {
                            let summary: String = stmt.lines()
                                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("--"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            tracing::info!("Executing: {summary}");
                            self.inner.execute(stmt).await?;
                            tracing::info!("Done: {summary}");
                        }
                    }
                } else if !migration_sql.is_empty() {
                    self.inner.execute(&**migration_sql).await?;
                }
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

    // For migrations that were replaced (same version, new content), only delete if
    // the stored checksum doesn't match the current file — i.e., it's a stale record
    // from the old broken version. Once the new migration is applied, the checksum
    // matches and the record is kept, avoiding expensive re-application on every start.
    let migrator = sqlx::migrate!("../migrations");
    let potentially_stale: &[i64] = &[
        20260207000001,
        20260207000002,
        20260207000003,
        20260207000004,
    ];
    for m in migrator.migrations.iter() {
        if potentially_stale.contains(&m.version) {
            if let Err(err) =
                sqlx::query("DELETE FROM _sqlx_migrations WHERE version = $1 AND checksum != $2")
                    .bind(m.version)
                    .bind(&*m.checksum)
                    .execute(db)
                    .await
            {
                tracing::info!("Could not clean up stale migration {}: {err:#}", m.version);
            }
        }
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

    crate::live_migrations::custom_migrations(&mut custom_migrator, db).await?;
    Ok(None)
}

pub async fn wait_for_migrations(
    db: &DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    let migrator = sqlx::migrate!("../migrations");
    let latest_version = migrator
        .migrations
        .iter()
        .map(|m| m.version)
        .max()
        .expect("No migrations found") as i64;

    tracing::info!(
        "This worker is on Windmill version {GIT_VERSION} (migration version {latest_version}). Only servers run migrations. Waiting for a server with version >= {GIT_VERSION} to apply the migration..."
    );

    let mut attempts = 0;

    loop {
        let is_applied: Result<Option<bool>, sqlx::Error> =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1)")
                .bind(latest_version)
                .fetch_one(db)
                .await;

        match is_applied {
            Ok(Some(true)) => {
                tracing::info!(
                    "All migrations applied (version {latest_version}), continuing worker startup"
                );
                return Ok(());
            }
            Ok(_) => {
                tracing::info!(
                    "Database not up to date yet. This worker (Windmill {GIT_VERSION}, migration version {latest_version}) is waiting for a server with version >= {GIT_VERSION} to run the migration. Rechecking in 3s..."
                );
            }
            Err(e) => {
                tracing::info!(
                    "Could not check migration status (migrations table may not exist yet): {e:#}. Rechecking in 3s..."
                );
            }
        }

        tokio::select! {
            _ = killpill_rx.recv() => {
                tracing::info!("Killpill received, stopping migration wait");
                return Ok(());
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {}
        }

        attempts += 1;
        if attempts >= 10 {
            tracing::error!(
                "Timed out after 10 attempts waiting for migration version {latest_version}. Exiting."
            );
            std::process::exit(1);
        }
    }
}
