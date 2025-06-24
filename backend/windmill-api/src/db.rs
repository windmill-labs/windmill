/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::time::Duration;

use futures::FutureExt;
use sqlx::{
    migrate::{Migrate, MigrateError},
    pool::PoolConnection,
    Executor, PgConnection, Pool, Postgres,
};

use tokio::task::JoinHandle;
use windmill_audit::audit_oss::{AuditAuthor, AuditAuthorable};
use windmill_common::{
    db::{Authable, Authed},
    error::Error,
};
use windmill_common::{utils::generate_lock_id, worker::MIN_VERSION_IS_AT_LEAST_1_461};

pub type DB = Pool<Postgres>;

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
                    ].into_iter().collect();
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

pub async fn migrate(db: &DB) -> Result<Option<JoinHandle<()>>, Error> {
    let migrator = db.acquire().await?;
    let mut custom_migrator = CustomMigrator { inner: migrator };

    if let Err(err) = sqlx::query!("DELETE FROM _sqlx_migrations WHERE version=20250131115248")
        .execute(db)
        .await
    {
        tracing::info!("Could not remove sqlx migration with version=20250131115248: {err:#}");
    }

    // Remove the migration `v2_fix_no_runtime` in favor of `v2_fix_no_runtime_2`.
    if let Err(err) = sqlx::query!("DELETE FROM _sqlx_migrations WHERE version=20250201145632")
        .execute(db)
        .await
    {
        tracing::info!("Could not remove sqlx migration with version=20250201145632: {err:#}");
    }

    // New version of `v2_as_queue` and `v2_as_completed_job` VIEWs.
    if let Err(err) = sqlx::query!(
        "DELETE FROM _sqlx_migrations WHERE version=20250201145630 OR version=20250201145631"
    )
    .execute(db)
    .await
    {
        tracing::info!("Could not remove sqlx migration with version=[20250201145630, 20250201145631] : {err:#}");
    }

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

    if let Err(err) = fix_flow_versioning_migration(&mut custom_migrator, db).await {
        tracing::error!("Could not apply flow versioning fix migration: {err:#}");
    }

    let db2 = db.clone();
    let _ = tokio::task::spawn(async move {
        if let Err(err) = fix_job_completed_index(&db2).await {
            tracing::error!("Could not apply job completed index fix migration: {err:#}");
        }
    });

    let mut jh = None;
    if !has_done_migration(db, "v2_finalize_job_completed").await {
        let db2 = db.clone();
        let v2jh = tokio::task::spawn(async move {
            loop {
                if !*MIN_VERSION_IS_AT_LEAST_1_461.read().await {
                    tracing::info!("Waiting for all workers to be at least version 1.461 before applying v2 finalize migration, sleeping for 5s...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                if let Err(err) = v2_finalize(&db2).await {
                    tracing::error!(
                        "{err:#}: Could not apply v2 finalize migration, retry in 30s.."
                    );
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    continue;
                }
                tracing::info!("v2 finalization step successfully applied.");
                break;
            }
        });
        jh = Some(v2jh)
    }

    Ok(jh)
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

async fn has_done_migration(db: &DB, migration_job_name: &str) -> bool {
    sqlx::query_scalar!(
        "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = $1)",
        migration_job_name
    )
    .fetch_one(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(false)
}

macro_rules! run_windmill_migration {
    ($migration_job_name:expr, $db:expr, |$tx:ident| $code:block) => {
        {
            let migration_job_name = $migration_job_name;
            let db: &Pool<Postgres> = $db;

            let has_done = has_done_migration(db, migration_job_name).await;
            if !has_done {
                tracing::info!("Applying {migration_job_name} migration");
                let mut $tx = db.begin().await?;
                let mut r = false;
                while !r {
                    r = sqlx::query_scalar!("SELECT pg_try_advisory_lock(4242)")
                        .fetch_one(&mut *$tx)
                        .await
                        .map_err(|e| {
                            tracing::error!("Error acquiring {migration_job_name} lock: {e:#}");
                            sqlx::migrate::MigrateError::Execute(e)
                        })?
                        .unwrap_or(false);

                    if !r {
                        tracing::info!("PG {migration_job_name} lock already acquired by another server or worker, retrying in 5s. (look for the advisory lock in pg_lock with granted = true)");
                        drop($tx);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        $tx = db.begin().await?;
                    }
                }
                tracing::info!("acquired lock for {migration_job_name}");

                let has_done = has_done_migration(db, migration_job_name).await;

                if !has_done {

                    $code

                    sqlx::query!(
                        "INSERT INTO windmill_migrations (name) VALUES ($1) ON CONFLICT DO NOTHING",
                        migration_job_name
                    )
                    .execute(&mut *$tx)
                    .await?;
                    tracing::info!("Finished applying {migration_job_name} migration");
                } else {
                    tracing::debug!("migration {migration_job_name} already done");
                }

                let _ = sqlx::query("SELECT pg_advisory_unlock(4242)")
                    .execute(&mut *$tx)
                    .await?;
                $tx.commit().await?;
                tracing::info!("released lock for {migration_job_name}");
            } else {
                tracing::debug!("migration {migration_job_name} already done");

            }
        }
    };
}

async fn v2_finalize(db: &DB) -> Result<(), Error> {
    run_windmill_migration!("v2_finalize_disable_sync_III", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_queue IN ACCESS EXCLUSIVE MODE;
            ALTER TABLE v2_job_queue DISABLE ROW LEVEL SECURITY;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_2", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_completed IN ACCESS EXCLUSIVE MODE;
            ALTER TABLE v2_job_completed DISABLE ROW LEVEL SECURITY;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_3", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job IN ACCESS EXCLUSIVE MODE;
            DROP FUNCTION IF EXISTS v2_job_after_update CASCADE;
        "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_4", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_completed IN ACCESS EXCLUSIVE MODE;
            DROP FUNCTION IF EXISTS v2_job_completed_before_insert CASCADE;
            DROP FUNCTION IF EXISTS v2_job_completed_before_update CASCADE;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_5", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_queue IN ACCESS EXCLUSIVE MODE;
            DROP FUNCTION IF EXISTS v2_job_queue_after_insert CASCADE;
            DROP FUNCTION IF EXISTS v2_job_queue_before_insert CASCADE;
            DROP FUNCTION IF EXISTS v2_job_queue_before_update CASCADE;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_6", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_runtime IN ACCESS EXCLUSIVE MODE;
            DROP FUNCTION IF EXISTS v2_job_runtime_before_insert CASCADE;
            DROP FUNCTION IF EXISTS v2_job_runtime_before_update CASCADE;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_7", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_status IN ACCESS EXCLUSIVE MODE;
            DROP FUNCTION IF EXISTS v2_job_status_before_insert CASCADE;
            DROP FUNCTION IF EXISTS v2_job_status_before_update CASCADE;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_disable_sync_III_8", db, |tx| {
        tx.execute(
            r#"
            DROP VIEW IF EXISTS completed_job, completed_job_view, job, queue, queue_view CASCADE;
            "#,
        )
        .await?;
    });

    run_windmill_migration!("v2_finalize_job_queue", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_queue IN ACCESS EXCLUSIVE MODE;
            ALTER TABLE v2_job_queue
                DROP COLUMN IF EXISTS __parent_job CASCADE,
                DROP COLUMN IF EXISTS __created_by CASCADE,
                DROP COLUMN IF EXISTS __script_hash CASCADE,
                DROP COLUMN IF EXISTS __script_path CASCADE,
                DROP COLUMN IF EXISTS __args CASCADE,
                DROP COLUMN IF EXISTS __logs CASCADE,
                DROP COLUMN IF EXISTS __raw_code CASCADE,
                DROP COLUMN IF EXISTS __canceled CASCADE,
                DROP COLUMN IF EXISTS __last_ping CASCADE,
                DROP COLUMN IF EXISTS __job_kind CASCADE,
                DROP COLUMN IF EXISTS __env_id CASCADE,
                DROP COLUMN IF EXISTS __schedule_path CASCADE,
                DROP COLUMN IF EXISTS __permissioned_as CASCADE,
                DROP COLUMN IF EXISTS __flow_status CASCADE,
                DROP COLUMN IF EXISTS __raw_flow CASCADE,
                DROP COLUMN IF EXISTS __is_flow_step CASCADE,
                DROP COLUMN IF EXISTS __language CASCADE,
                DROP COLUMN IF EXISTS __same_worker CASCADE,
                DROP COLUMN IF EXISTS __raw_lock CASCADE,
                DROP COLUMN IF EXISTS __pre_run_error CASCADE,
                DROP COLUMN IF EXISTS __email CASCADE,
                DROP COLUMN IF EXISTS __visible_to_owner CASCADE,
                DROP COLUMN IF EXISTS __mem_peak CASCADE,
                DROP COLUMN IF EXISTS __root_job CASCADE,
                DROP COLUMN IF EXISTS __leaf_jobs CASCADE,
                DROP COLUMN IF EXISTS __concurrent_limit CASCADE,
                DROP COLUMN IF EXISTS __concurrency_time_window_s CASCADE,
                DROP COLUMN IF EXISTS __timeout CASCADE,
                DROP COLUMN IF EXISTS __flow_step_id CASCADE,
                DROP COLUMN IF EXISTS __cache_ttl CASCADE;
            "#,
        )
        .await?;
    });
    run_windmill_migration!("v2_finalize_job_completed", db, |tx| {
        tx.execute(
            r#"
            LOCK TABLE v2_job_completed IN ACCESS EXCLUSIVE MODE;
            ALTER TABLE v2_job_completed
                DROP COLUMN IF EXISTS __parent_job CASCADE,
                DROP COLUMN IF EXISTS __created_by CASCADE,
                DROP COLUMN IF EXISTS __created_at CASCADE,
                DROP COLUMN IF EXISTS __success CASCADE,
                DROP COLUMN IF EXISTS __script_hash CASCADE,
                DROP COLUMN IF EXISTS __script_path CASCADE,
                DROP COLUMN IF EXISTS __args CASCADE,
                DROP COLUMN IF EXISTS __logs CASCADE,
                DROP COLUMN IF EXISTS __raw_code CASCADE,
                DROP COLUMN IF EXISTS __canceled CASCADE,
                DROP COLUMN IF EXISTS __job_kind CASCADE,
                DROP COLUMN IF EXISTS __env_id CASCADE,
                DROP COLUMN IF EXISTS __schedule_path CASCADE,
                DROP COLUMN IF EXISTS __permissioned_as CASCADE,
                DROP COLUMN IF EXISTS __raw_flow CASCADE,
                DROP COLUMN IF EXISTS __is_flow_step CASCADE,
                DROP COLUMN IF EXISTS __language CASCADE,
                DROP COLUMN IF EXISTS __is_skipped CASCADE,
                DROP COLUMN IF EXISTS __raw_lock CASCADE,
                DROP COLUMN IF EXISTS __email CASCADE,
                DROP COLUMN IF EXISTS __visible_to_owner CASCADE,
                DROP COLUMN IF EXISTS __tag CASCADE,
                DROP COLUMN IF EXISTS __priority CASCADE;
            "#,
        )
        .await?;
    });

    Ok(())
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

    run_windmill_migration!("fix_job_completed_index_2", &db, |tx| {
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

    run_windmill_migration!("fix_job_completed_index_3", &db, |tx| {
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

    run_windmill_migration!("fix_job_index_1_II", &db, |tx| {
        let migration_job_name = "fix_job_index_1_II";
        let mut i = 1;
        tracing::info!("step {i} of {migration_job_name} migration");
        sqlx::query!("create index concurrently  if not exists ix_job_workspace_id_created_at_new_3 ON v2_job  (workspace_id,  created_at DESC)")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_job_workspace_id_created_at_new_8 ON v2_job  (workspace_id, created_at DESC) where kind in ('deploymentcallback') AND parent_job IS NULL")
            .execute(db)
            .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_job_workspace_id_created_at_new_9 ON v2_job  (workspace_id, created_at DESC) where kind in ('dependencies', 'flowdependencies', 'appdependencies') AND parent_job IS NULL")
            .execute(db)
            .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_job_workspace_id_created_at_new_5 ON v2_job  (workspace_id, created_at DESC) where kind in ('preview', 'flowpreview') AND parent_job IS NULL")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_completed_job_workspace_id_started_at_new_2 ON v2_job_completed  (workspace_id, started_at DESC)")
                .execute(db)
                .await?;
        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_job_root_job_index_by_path_2 ON v2_job (workspace_id, runnable_path, created_at desc) WHERE parent_job IS NULL")
                .execute(db)
                .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS root_job_index_by_path_2")
            .execute(db)
            .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_job_created_at ON v2_job  (created_at DESC)")
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

    run_windmill_migration!("fix_labeled_jobs_index", &db, |tx| {
        tracing::info!("Special migration to add index concurrently on job labels 2");
        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS labeled_jobs_on_jobs")
            .execute(db)
            .await?;
        sqlx::query!(
        "CREATE INDEX CONCURRENTLY labeled_jobs_on_jobs ON v2_job_completed USING GIN ((result -> 'wm_labels')) WHERE result ? 'wm_labels'"
        ).execute(db).await?;
    });

    run_windmill_migration!("v2_labeled_jobs_index", &db, |tx| {
        tracing::info!("Special migration to add index concurrently on job labels");
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY ix_v2_job_labels ON v2_job
                USING GIN (labels)
                WHERE labels IS NOT NULL"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("v2_jobs_rls", &db, |tx| {
        sqlx::query!("ALTER TABLE v2_job ENABLE ROW LEVEL SECURITY")
            .execute(db)
            .await?;
    });

    run_windmill_migration!("v2_improve_v2_job_indices_ii", &db, |tx| {
        sqlx::query!("create index concurrently if not exists ix_v2_job_workspace_id_created_at ON v2_job  (workspace_id, created_at DESC) where kind in ('script', 'flow', 'singlescriptflow') AND parent_job IS NULL")
                .execute(db)
                .await?;

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS ix_job_workspace_id_created_at_new_6")
            .execute(db)
            .await?;

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS ix_job_workspace_id_created_at_new_7")
            .execute(db)
            .await?;
    });

    run_windmill_migration!("v2_improve_v2_queued_jobs_indices", &db, |tx| {
        sqlx::query!("CREATE INDEX CONCURRENTLY IF NOT EXISTS queue_sort_v2 ON v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag) WHERE running = false")
            .execute(db)
            .await?;

        // sqlx::query!("CREATE INDEX CONCURRENTLY queue_sort_2_v2 ON v2_job_queue (tag, priority DESC NULLS LAST, scheduled_for) WHERE running = false")
        //     .execute(db)
        //     .await?;

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS queue_sort")
            .execute(db)
            .await?;

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS queue_sort_2")
            .execute(db)
            .await?;
    });

    run_windmill_migration!("audit_timestamps", db, |tx| {
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_audit_timestamps ON audit (timestamp DESC)"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("job_completed_completed_at", db, |tx| {
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_job_completed_completed_at ON v2_job_completed (completed_at DESC)"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("alerts_by_workspace", db, |tx| {
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS alerts_by_workspace ON alerts (workspace_id);"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("remove_redundant_log_file_index", db, |tx| {
        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS log_file_hostname_log_ts_idx")
            .execute(db)
            .await?;
    });

    run_windmill_migration!("v2_job_queue_suspend", db, |tx| {
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS v2_job_queue_suspend ON v2_job_queue (workspace_id, suspend) WHERE suspend > 0;"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("audit_recent_login_activities", db, |tx| {
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY idx_audit_recent_login_activities 
ON audit (timestamp, username) 
WHERE operation IN ('users.login', 'oauth.login', 'users.token.refresh');"
        )
        .execute(db)
        .await?;
    });
    Ok(())
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
