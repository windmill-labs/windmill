/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::Postgres;
use windmill_common::error::Error;

use crate::db::{CustomMigrator, DB};
use sqlx::migrate::Migrate;
use sqlx::Executor;

pub async fn custom_migrations(migrator: &mut CustomMigrator, db: &DB) -> Result<(), Error> {
    if let Err(err) = fix_flow_versioning_migration(migrator, db).await {
        tracing::error!("Could not apply flow versioning fix migration: {err:#}");
    }

    let db2 = db.clone();
    let _ = tokio::task::spawn(async move {
        if let Err(err) = fix_job_completed_index(&db2).await {
            tracing::error!("Could not apply job completed index fix migration: {err:#}");
        }
    });

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

use sqlx::Pool;

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

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS root_job_index_by_path_2")
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
        sqlx::query!("create index concurrently if not exists ix_v2_job_workspace_id_created_at ON v2_job  (workspace_id, created_at DESC) where kind in ('script', 'flow', 'singlestepflow') AND parent_job IS NULL")
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
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_recent_login_activities 
ON audit (timestamp, username) 
WHERE operation IN ('users.login', 'oauth.login', 'users.token.refresh');"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("v2_script_lock_index", db, |tx| {
        sqlx::query!(
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS script_not_archived ON script (workspace_id, path, created_at DESC) where archived = false;"
        )
        .execute(db)
        .await?;
    });

    run_windmill_migration!("v2_job_completed_completed_at_9", db, |tx| {
        let migration_job_name = "v2_job_completed_completed_at";
        let mut i = 1;
        tracing::info!("step {i} of {migration_job_name} migration");
        sqlx::query!("create index concurrently  if not exists ix_job_workspace_id_completed_at_all ON v2_job_completed  (workspace_id,  completed_at DESC)")
                .execute(db)
                .await?;
        i += 1;

        sqlx::query!("CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_job_v2_job_root_by_path_2 ON v2_job (workspace_id, runnable_path)  WHERE parent_job IS NULL;")
        .execute(db)
        .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("create index concurrently if not exists ix_job_root_job_index_by_path_2 ON v2_job (workspace_id, runnable_path, created_at desc) WHERE parent_job IS NULL")
                .execute(db)
                .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!(
            "DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_started_at_new_2"
        )
        .execute(db)
        .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS ix_job_created_at")
            .execute(db)
            .await?;

        i += 1;

        sqlx::query!("DROP INDEX CONCURRENTLY IF EXISTS ix_v2_job_root_by_path")
            .execute(db)
            .await?;

        i += 1;
        tracing::info!("step {i} of {migration_job_name} migration");
    });

    Ok(())
}
