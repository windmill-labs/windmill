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

    const RETIRE_LEGACY_AUDIT: &str = "retire_legacy_audit_table";
    if !background_migration_done(&mut conn, RETIRE_LEGACY_AUDIT).await? {
        let mut attempt = 1;
        loop {
            match retire_legacy_audit_table(&mut conn).await {
                Ok(()) => break,
                Err(err) if attempt < 10 && is_lock_timeout(&err) => {
                    tracing::warn!("Retiring the legacy audit table timed out on a lock, retrying in 30s: {err:#}");
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
                Err(err) => return Err(err),
            }
        }
        mark_background_migration_done(&mut conn, RETIRE_LEGACY_AUDIT).await?;
    }
    Ok(())
}

fn is_lock_timeout(err: &Error) -> bool {
    matches!(err, Error::SqlErr { error: sqlx::Error::Database(db_err), .. }
        if db_err.code().as_deref() == Some("55P03"))
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

const AUDIT_COLUMNS: &str =
    "workspace_id, id, timestamp, username, operation, action_kind, resource, parameters, email, span";

/// Moves the last 30 days of the pre-partitioning `audit` table into daily partitions and drops
/// it with anything older. An empty `audit` view takes its place for servers still running an
/// older version, which read it in a `UNION ALL` with `audit_partitioned`.
async fn retire_legacy_audit_table(conn: &mut PgConnection) -> Result<(), Error> {
    let is_table = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_class WHERE oid = to_regclass('audit') AND relkind = 'r')",
    )
    .fetch_one(&mut *conn)
    .await?;
    if !is_table {
        return Ok(());
    }
    // Creating a partition and re-owning the sequence queue every audit insert (so every job
    // push) behind them while they wait for their own lock, e.g. on a long-running reader, and
    // the DROP queues older servers' reads the same way. Give up early and retry instead.
    conn.execute("SET lock_timeout = '5s'").await?;

    let days: Vec<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT DISTINCT timestamp::date FROM audit
         WHERE timestamp > now() - interval '30 days' ORDER BY 1",
    )
    .fetch_all(&mut *conn)
    .await?;
    for day in days {
        let next = day + chrono::Duration::days(1);
        // Created outside any transaction: a partition created inside one keeps a lock on
        // audit_partitioned that blocks every job push's audit insert until it commits.
        conn.execute(
            format!(
                "CREATE TABLE IF NOT EXISTS \"audit_{}\" PARTITION OF audit_partitioned \
                 FOR VALUES FROM ('{day}') TO ('{next}')",
                day.format("%Y%m%d")
            )
            .as_str(),
        )
        .await?;
        // One statement per day, so a row is always in exactly one of the two tables.
        let moved = sqlx::query(&format!(
            "WITH moved AS (
                 DELETE FROM audit WHERE timestamp >= $1::date AND timestamp < $1::date + 1
                 RETURNING {AUDIT_COLUMNS}
             )
             INSERT INTO audit_partitioned ({AUDIT_COLUMNS}) SELECT {AUDIT_COLUMNS} FROM moved"
        ))
        .bind(day)
        .execute(&mut *conn)
        .await?;
        tracing::info!(
            "Moved {} legacy audit rows of {day} into audit_partitioned",
            moved.rows_affected()
        );
    }

    // audit_partitioned draws its ids from this sequence, which dropping its owner would drop.
    // Not in the transaction below: its lock stops every insert's nextval until commit, and the
    // DROP can wait there on readers of the old table.
    conn.execute("ALTER SEQUENCE audit_id_seq OWNED BY audit_partitioned.id")
        .await?;
    let mut tx = conn.begin().await?;
    tx.execute("DROP TABLE audit").await?;
    tx.execute(
        format!("CREATE VIEW audit AS SELECT {AUDIT_COLUMNS} FROM audit_partitioned WHERE false")
            .as_str(),
    )
    .await?;
    tx.execute("GRANT ALL ON audit TO windmill_user, windmill_admin")
        .await?;
    tx.commit().await?;
    conn.execute("RESET lock_timeout").await?;
    tracing::info!("Retired the legacy audit table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn retire_legacy_audit_table_keeps_the_last_30_days(db: DB) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO audit (workspace_id, id, timestamp, username, operation, action_kind)
             VALUES ('w', -2, now() - interval '3 days', 'u', 'recent', 'execute'),
                    ('w', -1, now() - interval '60 days', 'u', 'old', 'execute')",
        )
        .execute(&db)
        .await?;

        retire_legacy_audit_table(&mut *db.acquire().await?).await?;

        let moved: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, operation::text FROM audit_partitioned WHERE workspace_id = 'w'",
        )
        .fetch_all(&db)
        .await?;
        assert_eq!(moved, vec![(-2, "recent".to_string())]);

        // The id sequence outlives the table that owned it.
        sqlx::query(
            "INSERT INTO audit_partitioned (workspace_id, username, operation, action_kind)
             VALUES ('w', 'u', 'after', 'execute')",
        )
        .execute(&db)
        .await?;

        // Servers on an older version still read `audit` in a UNION with audit_partitioned.
        let seen: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (SELECT * FROM audit_partitioned UNION ALL SELECT * FROM audit) a
             WHERE workspace_id = 'w'",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(seen, 2);
        Ok(())
    }
}
