/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::atomic::Ordering;

use sqlx::Postgres;
use windmill_common::error::Error;
use windmill_common::min_version::MIN_VERSION_IS_AT_LEAST_1_647;
use windmill_common::FAST_FILTER_INDEXED;

use crate::db::{CustomMigrator, DB};
use sqlx::migrate::Migrate;
use sqlx::Executor;

pub async fn custom_migrations(migrator: &mut CustomMigrator, db: &DB) -> Result<(), Error> {
    if let Err(err) = fix_flow_versioning_migration(migrator, db).await {
        tracing::error!("Could not apply flow versioning fix migration: {err:#}");
    }

    if let Err(err) = fast_filter_migration(db).await {
        tracing::error!("Could not start fast_filter migration: {err:#}");
    }

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

async fn fast_filter_migration(db: &DB) -> Result<(), Error> {
    let has_done_migration = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = 'fast_filter_indexed')",
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    if has_done_migration {
        FAST_FILTER_INDEXED.store(true, Ordering::Relaxed);
        return Ok(());
    }

    let db = db.clone();
    tokio::spawn(async move {
        if let Err(e) = fast_filter_migration_inner(&db).await {
            tracing::error!("fast_filter background migration failed: {e:#}");
        }
    });

    Ok(())
}

async fn fast_filter_migration_inner(db: &DB) -> Result<(), Error> {
    // Wait until all workers are at least version 1.647 so new jobs write fast_filter
    let skip_version_check = std::env::var("SKIP_MIN_VERSION_CHECK")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    if !skip_version_check {
        loop {
            if MIN_VERSION_IS_AT_LEAST_1_647.met().await {
                break;
            }
            tracing::info!("fast_filter migration: waiting for all workers to be >= 1.647");
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    } else {
        tracing::info!(
            "fast_filter migration: skipping version check (SKIP_MIN_VERSION_CHECK=true)"
        );
    }

    // Backfill existing rows in batches (skip skipped jobs — they stay NULL)
    loop {
        let rows_updated = sqlx::query_scalar!(
            "WITH batch AS (
                SELECT c.id FROM v2_job_completed c
                JOIN v2_job j ON c.id = j.id
                WHERE j.parent_job IS NULL AND c.fast_filter IS NULL AND c.status != 'skipped'
                    AND j.kind IN ('script', 'flow', 'singlestepflow')
                LIMIT 50000
            )
            UPDATE v2_job_completed SET fast_filter =
                CASE WHEN v2_job_completed.status = 'success' THEN 1::smallint ELSE 2::smallint END
            FROM batch WHERE v2_job_completed.id = batch.id"
        )
        .execute(db)
        .await
        .map_err(|e| Error::internal_err(format!("fast_filter backfill failed: {e:#}")))?
        .rows_affected();

        tracing::info!("fast_filter migration: backfilled {rows_updated} rows");

        if rows_updated == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Create partial indexes concurrently (must be outside a transaction)
    tracing::info!("fast_filter migration: creating indexes concurrently");

    sqlx::query(
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_v2_job_completed_fast_filter_not_null
         ON v2_job_completed (workspace_id, completed_at DESC) WHERE fast_filter IS NOT NULL",
    )
    .execute(db)
    .await
    .map_err(|e| Error::internal_err(format!("fast_filter index creation failed: {e:#}")))?;

    sqlx::query(
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_v2_job_completed_fast_filter_failure
         ON v2_job_completed (workspace_id, completed_at DESC) WHERE fast_filter = 2",
    )
    .execute(db)
    .await
    .map_err(|e| Error::internal_err(format!("fast_filter index creation failed: {e:#}")))?;

    tracing::info!("fast_filter migration: indexes created, marking migration as done");

    sqlx::query!("INSERT INTO windmill_migrations (name) VALUES ('fast_filter_indexed')")
        .execute(db)
        .await
        .map_err(|e| {
            Error::internal_err(format!("fast_filter migration flag insert failed: {e:#}"))
        })?;

    FAST_FILTER_INDEXED.store(true, Ordering::Relaxed);
    tracing::info!("fast_filter migration: complete");

    Ok(())
}
