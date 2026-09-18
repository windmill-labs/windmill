/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::{PgConnection, Postgres};
use tokio::task::JoinHandle;
use windmill_common::{db::DB, error::Error};

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

    if let Err(err) = ensure_custom_instance_replication_user(migrator).await {
        tracing::error!(
            "Could not provision custom_instance_replication_user: {err:#}. Postgres triggers on \
             custom-instance datatables will not work until the role can open replication \
             connections: grant the role owning DATABASE_URL either SUPERUSER or (PG 16+) the \
             REPLICATION attribute. On AWS RDS this is handled by granting rds_replication, which \
             requires no change; other managed providers are not covered"
        );
    }

    Ok(())
}

// Converged on every boot, not once: the one-shot migration creates the role with the
// REPLICATION attribute, which managed postgres rejects outright, so instances set up before
// the provider-role fallback existed have no role at all. Scoped to instances that actually
// have a custom-instance database, so the rest never see the error.
async fn ensure_custom_instance_replication_user(
    migrator: &mut CustomMigrator,
) -> Result<(), Error> {
    let has_custom_instance_db = sqlx::query_scalar::<_, bool>(
        "SELECT COALESCE(value->'databases', '{}'::jsonb) <> '{}'::jsonb
         FROM global_settings WHERE name = 'custom_instance_pg_databases'",
    )
    .fetch_optional(migrator.connection())
    .await?
    .unwrap_or(false);
    if !has_custom_instance_db {
        return Ok(());
    }
    windmill_common::utils::ensure_custom_instance_replication_user(migrator.connection()).await
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

// Held for the whole background run so only one server does it at a time.
const BACKGROUND_MIGRATIONS_LOCK_ID: i64 = 4_931_072_518_336_401;

/// Schema changes too slow to hold server startup for, run by one server at a time after the
/// sqlx migrations. Each step is recorded in `windmill_migrations` once done; a step interrupted
/// by a restart or an error starts over on the next start and must resume safely.
pub fn spawn_background_migrations(
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::select! {
            r = run_background_migrations(&db) => {
                if let Err(err) = r {
                    tracing::error!("Background migrations stopped, retrying on the next start: {err:#}");
                }
            }
            _ = killpill_rx.recv() => {
                tracing::info!("Killpill received, stopping background migrations");
            }
        }
    })
}

async fn run_background_migrations(db: &DB) -> Result<(), Error> {
    // Detached so the pool's 5min statement_timeout, lifted here for the index builds, never
    // comes back with this connection; closing it also releases the advisory lock.
    let mut conn = db.acquire().await?.detach();
    conn.execute("SET statement_timeout = 0").await?;
    let locked = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
        .bind(BACKGROUND_MIGRATIONS_LOCK_ID)
        .fetch_one(&mut conn)
        .await?;
    if !locked {
        return Ok(());
    }

    const AUDIT_OPERATION_INDEX: &str = "audit_partitioned_workspace_operation_index";
    if !background_migration_done(&mut conn, AUDIT_OPERATION_INDEX).await? {
        create_audit_operation_index(&mut conn).await?;
        mark_background_migration_done(&mut conn, AUDIT_OPERATION_INDEX).await?;
    }
    Ok(())
}

async fn background_migration_done(conn: &mut PgConnection, name: &str) -> Result<bool, Error> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM windmill_migrations WHERE name = $1)",
    )
    .bind(name)
    .fetch_one(conn)
    .await?)
}

async fn mark_background_migration_done(conn: &mut PgConnection, name: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO windmill_migrations (name) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(name)
        .execute(conn)
        .await?;
    tracing::info!("Background migration {name} done");
    Ok(())
}

const AUDIT_OPERATION_INDEX_KEY: &str = r#"(workspace_id, operation, id DESC, "timestamp")"#;

/// A plain `CREATE INDEX` on the partitioned table holds a SHARE lock on every partition until the
/// whole build ends, blocking the audit insert each job push makes in its own transaction. Each
/// partition is built CONCURRENTLY instead and attached to a parent created `ON ONLY`, which turns
/// valid once all partitions are attached. Partitions created later get the index from the parent.
async fn create_audit_operation_index(conn: &mut PgConnection) -> Result<(), Error> {
    conn.execute(
        format!(
            "CREATE INDEX IF NOT EXISTS ix_audit_partitioned_workspace_operation \
             ON ONLY audit_partitioned {AUDIT_OPERATION_INDEX_KEY}"
        )
        .as_str(),
    )
    .await?;
    let partitions: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname::text FROM pg_inherits p JOIN pg_class c ON c.oid = p.inhrelid
         WHERE p.inhparent = 'audit_partitioned'::regclass
           AND NOT EXISTS (
               SELECT 1 FROM pg_inherits ip JOIN pg_index i ON i.indexrelid = ip.inhrelid
               WHERE ip.inhparent = 'ix_audit_partitioned_workspace_operation'::regclass
                 AND i.indrelid = c.oid)
         ORDER BY c.relname DESC",
    )
    .fetch_all(&mut *conn)
    .await?;
    let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
    for partition in partitions {
        let index = quote(&format!(
            "{partition}_workspace_id_operation_id_timestamp_idx"
        ));
        tracing::info!("Building ix_audit_partitioned_workspace_operation on {partition}");
        // An interrupted CONCURRENTLY build leaves an invalid index under this name.
        conn.execute(format!("DROP INDEX CONCURRENTLY IF EXISTS {index}").as_str())
            .await?;
        conn.execute(
            format!(
                "CREATE INDEX CONCURRENTLY {index} ON {} {AUDIT_OPERATION_INDEX_KEY}",
                quote(&partition)
            )
            .as_str(),
        )
        .await?;
        conn.execute(
            format!(
                "ALTER INDEX ix_audit_partitioned_workspace_operation ATTACH PARTITION {index}"
            )
            .as_str(),
        )
        .await?;
    }
    Ok(())
}
