/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::Postgres;
use windmill_common::error::Error;

use crate::db::CustomMigrator;
use sqlx::migrate::Migrate;
use sqlx::Acquire;
use sqlx::Executor;

pub async fn custom_migrations(migrator: &mut CustomMigrator) -> Result<(), Error> {
    if let Err(err) = fix_flow_versioning_migration(migrator).await {
        tracing::error!("Could not apply flow versioning fix migration: {err:#}");
    }

    if let Err(err) = normalize_custom_instance_user_attributes(migrator).await {
        tracing::error!("Could not normalize custom_instance_user attributes: {err:#}");
    }

    Ok(())
}

// Converged on every boot, not once: the one-shot migration swallows errors (it must not
// abort startup without superuser), and an older instance sharing the cluster can re-add
// the attribute. REPLICATION belongs only on custom_instance_replication_user.
async fn normalize_custom_instance_user_attributes(
    migrator: &mut CustomMigrator,
) -> Result<(), Error> {
    let has_replication = sqlx::query_scalar::<_, bool>(
        "SELECT rolreplication FROM pg_roles WHERE rolname = 'custom_instance_user'",
    )
    .fetch_optional(migrator.connection())
    .await?;
    if has_replication == Some(true) {
        sqlx::query("ALTER ROLE custom_instance_user NOREPLICATION")
            .execute(migrator.connection())
            .await?;
        tracing::info!("Normalized custom_instance_user attributes");
    }
    Ok(())
}

// Runs on the migrator's held connection (see CustomMigrator::connection): re-acquiring
// from the pool here would deadlock a single-connection backend.
async fn fix_flow_versioning_migration(migrator: &mut CustomMigrator) -> Result<(), Error> {
    let has_done_migration = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT name FROM windmill_migrations WHERE name = 'fix_flow_versioning_2')",
    )
    .fetch_one(migrator.connection())
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
            .fetch_one(migrator.connection())
            .await?
            .unwrap_or(false);

            if !has_done_migration {
                let query = include_str!("../../custom_migrations/fix_flow_versioning_2.sql");
                tracing::info!("Applying fix_flow_versioning_2.sql");
                let mut tx: sqlx::Transaction<'_, Postgres> = migrator.connection().begin().await?;
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
