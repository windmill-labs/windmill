/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Data table SQL migrations: CRUD endpoints, run/rollback execution, opt-in
//! management, and the workspace-merge diff helper. Split out of `workspaces.rs`
//! to keep that file focused on core workspace configuration.

use crate::workspaces::{pg_dump_database, ItemComparison};

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use std::collections::{HashMap, HashSet};

use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_api_jobs::run_wait_result_internal;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::db::UserDB;
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::runnable_settings::{ConcurrencySettingsWithCustom, DebouncingSettings};
use windmill_common::scripts::ScriptLang;
use windmill_common::users::username_to_permissioned_as;
use windmill_common::worker::to_raw_value;
use windmill_common::workspaces::get_datatable_resource_from_db_unchecked;
use windmill_common::{PgDatabase, DB};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, PushArgs, PushIsolationLevel};

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/run_datatable_migrations/{datatable_name}",
            post(run_datatable_migrations),
        )
        .route(
            "/rollback_datatable_migrations/{datatable_name}",
            post(rollback_datatable_migrations),
        )
        .route("/list_datatable_migrations", get(list_datatable_migrations))
        .route(
            "/datatable_migrations_status/{datatable_name}",
            get(datatable_migrations_status),
        )
        .route(
            "/enable_datatable_migrations/{datatable_name}",
            post(enable_datatable_migrations),
        )
        .route(
            "/disable_datatable_migrations/{datatable_name}",
            post(disable_datatable_migrations),
        )
        .route(
            "/create_datatable_migration/{datatable_name}",
            post(create_datatable_migration),
        )
        .route(
            "/delete_datatable_migration/{datatable_name}/{timestamp}",
            delete(delete_datatable_migration),
        )
        .route(
            "/upsert_datatable_migration/{datatable_name}",
            post(upsert_datatable_migration),
        )
        .route(
            "/generate_initial_datatable_migration/{datatable_name}",
            post(generate_initial_datatable_migration),
        )
}

#[derive(Serialize)]
struct AppliedMigration {
    version: i64,
    name: String,
}

#[derive(Serialize)]
struct RunDatatableMigrationsResult {
    applied: Vec<AppliedMigration>,
}

#[derive(Deserialize)]
struct RunDatatableMigrationsQuery {
    /// When set, only apply pending migrations up to and including this version.
    up_to: Option<i64>,
    /// When set, apply only this specific migration version (if not already
    /// applied), ignoring any other pending migrations. Takes precedence over
    /// `up_to`.
    only: Option<i64>,
}

/// Build the `database` argument for a migration job. Both resource-backed and
/// instance data tables pass a `datatable://<name>` reference; the pg executor
/// resolves it to real credentials server-side at run time. It must never be
/// resolved here: the resolved instance credentials include a single
/// instance-wide Postgres password, and the job's `args` are readable by the —
/// possibly non-admin — user who ran the migration.
async fn datatable_database_arg(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<Box<serde_json::value::RawValue>> {
    // Fail fast with a clear error if the data table doesn't exist.
    sqlx::query_scalar!(
        "SELECT ws.datatable->'datatables'->$2 FROM workspace_settings ws WHERE ws.workspace_id = $1",
        w_id,
        datatable_name,
    )
    .fetch_one(db)
    .await?
    .ok_or_else(|| Error::internal_err(format!("datatable {datatable_name} not found")))?;

    Ok(to_raw_value(&format!("datatable://{datatable_name}")))
}

/// Run a migration's SQL as a normal Windmill `postgresql` job, permissioned as
/// the requesting user and labelled `datatable_migration` for traceability, then
/// wait for it. Errors if the job fails.
async fn run_datatable_migration_job(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    database_arg: &Box<serde_json::value::RawValue>,
    sql: &str,
) -> Result<()> {
    let mut args = HashMap::new();
    args.insert("database".to_string(), database_arg.clone());
    let push_args = PushArgs { extra: None, args: &args };

    let (uuid, mut tx) = push(
        db,
        PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into()),
        w_id,
        JobPayload::Code(RawCode {
            content: sql.to_string(),
            path: Some("datatable_migration".to_string()),
            hash: None,
            language: ScriptLang::Postgresql,
            lock: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            tag: None,
            concurrency_settings: ConcurrencySettingsWithCustom::default(),
            debouncing_settings: DebouncingSettings::default(),
            modules: None,
        }),
        push_args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;

    // Tag the job so migration runs are easy to find in the run history.
    sqlx::query!(
        "UPDATE v2_job SET labels = (
                SELECT array_agg(DISTINCT l)
                FROM unnest(coalesce(labels, ARRAY[]::TEXT[]) || $2) l
            ) WHERE id = $1",
        uuid,
        &vec!["datatable_migration".to_string()],
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let (result, success) =
        run_wait_result_internal(db, uuid, w_id, None, false, &authed.username).await?;
    if !success {
        // On failure the job result is `{"error": {"name", "message", ...}}`;
        // surface the executor's message (the Postgres error, e.g. `relation
        // "foo" does not exist`) instead of the raw JSON envelope.
        let detail = serde_json::from_str::<serde_json::Value>(result.get())
            .ok()
            .as_ref()
            .and_then(|v| v.get("error"))
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| result.get().to_string());
        return Err(Error::internal_err(detail));
    }
    Ok(())
}

/// Ensure the `_wm_migrations` bookkeeping table exists. Migration versions are
/// only unique per data table, but several data-table configs can point at one
/// physical database, so it is keyed by `(datatable, version)` — a version-only
/// key would let one data table's migration mark another's same-version
/// migration as already applied (and rollback could touch the wrong row).
async fn ensure_wm_migrations_schema(client: &tokio_postgres::Client) -> Result<()> {
    client
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS _wm_migrations (\
                datatable TEXT NOT NULL, \
                version BIGINT NOT NULL, \
                installed_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
                PRIMARY KEY (datatable, version))",
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!("Failed to ensure _wm_migrations table: {}", e))
        })?;
    Ok(())
}

/// Open a connection to a data table's own database and hold the session-level
/// advisory lock that serializes migration runs/rollbacks. The lock is released
/// when the returned client is dropped, so callers must keep it in scope for the
/// whole critical section.
///
/// Runs, rollbacks *and* definition rewrites/deletes all take this lock: a run
/// snapshots a migration's `code_up` from `datatable_migrations` and only records
/// its version in `_wm_migrations` after the job succeeds, so an unserialized edit
/// could rewrite the definition in that window and leave `_wm_migrations` pointing
/// at SQL that was never applied. `_wm_migrations` is per-database, so a single
/// key is sufficient.
async fn lock_datatable_migration_runs(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<tokio_postgres::Client> {
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable connection error: {}", e);
        }
    });
    client
        .batch_execute("SELECT pg_advisory_lock(hashtext('windmill_datatable_migrations')::int8)")
        .await
        .map_err(|e| Error::internal_err(format!("Failed to acquire migration lock: {}", e)))?;
    Ok(client)
}

/// Read the versions recorded as applied in a data table's `_wm_migrations`,
/// scoped to that data table, using an existing connection. An absent table
/// (`42P01`) means nothing has been migrated yet.
async fn read_applied_versions_on_client(
    client: &tokio_postgres::Client,
    datatable_name: &str,
) -> Result<HashSet<i64>> {
    match client
        .query(
            "SELECT version FROM _wm_migrations WHERE datatable = $1",
            &[&datatable_name],
        )
        .await
    {
        Ok(rows) => Ok(rows.iter().map(|row| row.get::<_, i64>(0)).collect()),
        Err(e) if e.as_db_error().map(|d| d.code().code()) == Some("42P01") => Ok(HashSet::new()),
        Err(e) => Err(Error::internal_err(format!(
            "Failed to read _wm_migrations: {}",
            e
        ))),
    }
}

/// Apply the workspace's pending data table migrations to a given data table.
/// Each migration runs as a normal Windmill `postgresql` job (permissioned as
/// the requester, labelled `datatable_migration`); applied versions are then
/// recorded in the data table's own `_wm_migrations` table, so only migrations
/// not recorded there are run, in ascending `timestamp` order.
async fn run_datatable_migrations(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Query(query): Query<RunDatatableMigrationsQuery>,
) -> JsonResult<RunDatatableMigrationsResult> {
    audit_log(
        &db,
        &authed,
        "workspaces.run_datatable_migrations",
        ActionKind::Update,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    let database_arg = datatable_database_arg(&db, &w_id, &datatable_name).await?;

    // Take the run-serialization lock before snapshotting the migration
    // definitions: a concurrent definition rewrite/delete takes the same lock, so
    // the `code_up` we read here can't change between now and when we record its
    // version below. The lock is held until `client` drops at return.
    let client = lock_datatable_migration_runs(&db, &w_id, &datatable_name).await?;

    let migrations = sqlx::query!(
        "SELECT timestamp, name, code_up FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 ORDER BY timestamp ASC",
        &w_id,
        &datatable_name,
    )
    .fetch_all(&db)
    .await?;

    ensure_wm_migrations_schema(&client).await?;

    let applied_versions = read_applied_versions_on_client(&client, &datatable_name).await?;

    let mut applied = Vec::new();
    for m in migrations {
        if let Some(only) = query.only {
            // Run a single specific migration, skipping every other one.
            if m.timestamp != only {
                continue;
            }
        } else if query.up_to.is_some_and(|up_to| m.timestamp > up_to) {
            // Migrations are ordered ascending, so once we pass `up_to` we're done.
            break;
        }
        if applied_versions.contains(&m.timestamp) {
            continue;
        }
        run_datatable_migration_job(&db, &user_db, &authed, &w_id, &database_arg, &m.code_up)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to apply migration {} ({}): {}",
                    m.timestamp, m.name, e
                ))
            })?;
        // Record the migration as installed once its job has succeeded.
        client
            .execute(
                "INSERT INTO _wm_migrations (datatable, version) VALUES ($1, $2) \
                 ON CONFLICT (datatable, version) DO NOTHING",
                &[&datatable_name, &m.timestamp],
            )
            .await
            .map_err(|e| Error::internal_err(format!("Failed to record migration: {}", e)))?;
        applied.push(AppliedMigration { version: m.timestamp, name: m.name });
    }

    Ok(Json(RunDatatableMigrationsResult { applied }))
}

#[derive(Serialize)]
struct RolledBackMigration {
    version: i64,
    name: String,
}

#[derive(Serialize)]
struct RollbackDatatableMigrationsResult {
    rolled_back: Vec<RolledBackMigration>,
}

#[derive(Deserialize)]
struct RollbackDatatableMigrationsQuery {
    /// When set, roll back this specific applied migration version instead of
    /// the most recently applied one.
    only: Option<i64>,
}

/// Roll back a migration on a given data table: run its `code_down` as a normal
/// Windmill `postgresql` job (permissioned as the requester, labelled
/// `datatable_migration`) then drop its `_wm_migrations` row. Without `only` this
/// targets the most recently applied migration (one step); with `only` it
/// targets that specific applied version.
async fn rollback_datatable_migrations(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Query(query): Query<RollbackDatatableMigrationsQuery>,
) -> JsonResult<RollbackDatatableMigrationsResult> {
    audit_log(
        &db,
        &authed,
        "workspaces.rollback_datatable_migrations",
        ActionKind::Update,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    // The data table's `_wm_migrations` bookkeeping is read here and the version
    // dropped after the job succeeds; the down SQL itself runs in the job. The
    // lock is held (until `client` drops at return) so a concurrent run or
    // definition rewrite can't interleave with this rollback.
    let client = lock_datatable_migration_runs(&db, &w_id, &datatable_name).await?;

    ensure_wm_migrations_schema(&client).await?;

    // Resolve which applied version to roll back: a specific one when `only` is
    // given (and actually applied), otherwise the most recently applied. Scoped
    // to this data table so a shared physical database can't surface another
    // data table's version.
    let target = match query.only {
        Some(only) => client
            .query_opt(
                "SELECT version FROM _wm_migrations WHERE datatable = $1 AND version = $2",
                &[&datatable_name, &only],
            )
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read _wm_migrations: {}", e)))?,
        None => client
            .query_opt(
                "SELECT version FROM _wm_migrations WHERE datatable = $1 \
                 ORDER BY version DESC LIMIT 1",
                &[&datatable_name],
            )
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read _wm_migrations: {}", e)))?,
    };

    let version: i64 = match target {
        Some(row) => row.get::<_, i64>(0),
        None => {
            return Ok(Json(RollbackDatatableMigrationsResult {
                rolled_back: vec![],
            }))
        }
    };

    let definition = sqlx::query!(
        "SELECT name, code_down FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3",
        &w_id,
        &datatable_name,
        version
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| {
        Error::BadRequest(format!(
            "Cannot roll back migration {version}: its definition no longer exists"
        ))
    })?;

    let code_down = definition.code_down.ok_or_else(|| {
        Error::BadRequest(format!(
            "Cannot roll back migration {} ({}): it has no down migration",
            version, definition.name
        ))
    })?;

    let database_arg = datatable_database_arg(&db, &w_id, &datatable_name).await?;
    run_datatable_migration_job(&db, &user_db, &authed, &w_id, &database_arg, &code_down)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to roll back migration {} ({}): {}",
                version, definition.name, e
            ))
        })?;

    // Forget the version once its down job has succeeded.
    client
        .execute(
            "DELETE FROM _wm_migrations WHERE datatable = $1 AND version = $2",
            &[&datatable_name, &version],
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to drop migration record: {}", e)))?;

    Ok(Json(RollbackDatatableMigrationsResult {
        rolled_back: vec![RolledBackMigration { version, name: definition.name }],
    }))
}

#[derive(Serialize, Deserialize)]
pub struct DatatableMigration {
    pub datatable: String,
    pub timestamp: i64,
    pub name: String,
    pub code_up: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_down: Option<String>,
}

async fn list_datatable_migrations(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DatatableMigration>> {
    let migrations = sqlx::query_as!(
        DatatableMigration,
        "SELECT datatable, timestamp, name, code_up, code_down FROM datatable_migrations \
         WHERE workspace_id = $1 ORDER BY datatable, timestamp ASC",
        &w_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(migrations))
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum DatatableMigrationRunStatus {
    /// Recorded in the data table's `_wm_migrations` table.
    Ran,
    /// Defined but not yet applied.
    NotRun,
    /// Applied status could not be determined (connection failure).
    Unknown,
}

#[derive(Serialize)]
struct DatatableMigrationWithStatus {
    timestamp: i64,
    name: String,
    code_up: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code_down: Option<String>,
    status: DatatableMigrationRunStatus,
}

#[derive(Serialize)]
struct DatatableMigrationsStatusResult {
    /// Whether the migrations feature is opted in for this data table.
    enabled: bool,
    migrations: Vec<DatatableMigrationWithStatus>,
    /// Set when the applied status couldn't be read from the data table.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Whether the SQL-migrations feature is enabled for a data table. Honors the
/// explicit `migrations_enabled` flag; when unset (data tables predating the
/// feature) it is considered enabled only if migrations already exist.
async fn datatable_migrations_enabled(db: &DB, w_id: &str, datatable_name: &str) -> Result<bool> {
    let flag: Option<bool> = sqlx::query_scalar!(
        "SELECT (ws.datatable->'datatables'->$2->>'migrations_enabled')::boolean \
         FROM workspace_settings ws WHERE ws.workspace_id = $1",
        w_id,
        datatable_name,
    )
    .fetch_optional(db)
    .await?
    .flatten();

    match flag {
        Some(v) => Ok(v),
        None => Ok(sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM datatable_migrations \
             WHERE workspace_id = $1 AND datatable = $2)",
            w_id,
            datatable_name,
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false)),
    }
}

/// Reject the request when migrations are not enabled for the data table.
async fn ensure_datatable_migrations_enabled(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<()> {
    if !datatable_migrations_enabled(db, w_id, datatable_name).await? {
        return Err(Error::BadRequest(format!(
            "Migrations are not enabled for data table '{}'. Enable them first.",
            datatable_name
        )));
    }
    Ok(())
}

/// Read the versions recorded in a data table's `_wm_migrations` table. A
/// missing table means nothing has been applied yet (empty set, not an error).
async fn read_applied_datatable_versions(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<HashSet<i64>> {
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable connection error: {}", e);
        }
    });

    // Read-only status path: don't create the table here, and don't take the run
    // lock — a stale-by-a-moment applied set is fine for display.
    read_applied_versions_on_client(&client, datatable_name).await
}

/// List a data table's migrations annotated with whether each has been applied.
async fn datatable_migrations_status(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<DatatableMigrationsStatusResult> {
    let enabled = datatable_migrations_enabled(&db, &w_id, &datatable_name).await?;
    if !enabled {
        return Ok(Json(DatatableMigrationsStatusResult {
            enabled: false,
            migrations: vec![],
            error: None,
        }));
    }

    let defs = sqlx::query!(
        "SELECT timestamp, name, code_up, code_down FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 ORDER BY timestamp ASC",
        &w_id,
        &datatable_name,
    )
    .fetch_all(&db)
    .await?;

    let (applied, error) = match read_applied_datatable_versions(&db, &w_id, &datatable_name).await
    {
        Ok(set) => (Some(set), None),
        Err(e) => (None, Some(e.to_string())),
    };

    let migrations = defs
        .into_iter()
        .map(|m| {
            let status = match &applied {
                Some(set) if set.contains(&m.timestamp) => DatatableMigrationRunStatus::Ran,
                Some(_) => DatatableMigrationRunStatus::NotRun,
                None => DatatableMigrationRunStatus::Unknown,
            };
            DatatableMigrationWithStatus {
                timestamp: m.timestamp,
                name: m.name,
                code_up: m.code_up,
                code_down: m.code_down,
                status,
            }
        })
        .collect();

    Ok(Json(DatatableMigrationsStatusResult {
        enabled: true,
        migrations,
        error,
    }))
}

/// Only workspace admins and super admins may opt a data table in or out of
/// migrations.
async fn require_datatable_migrations_manager(db: &DB, authed: &ApiAuthed) -> Result<()> {
    if authed.is_admin || require_super_admin(db, &authed).await.is_ok() {
        Ok(())
    } else {
        Err(Error::BadRequest(
            "Only workspace admins and super admins can enable or disable data table migrations"
                .to_string(),
        ))
    }
}

async fn enable_datatable_migrations(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> Result<String> {
    require_datatable_migrations_manager(&db, &authed).await?;

    let updated = sqlx::query_scalar!(
        "UPDATE workspace_settings \
         SET datatable = jsonb_set(datatable, ARRAY['datatables', $2::text, 'migrations_enabled'], 'true'::jsonb) \
         WHERE workspace_id = $1 AND jsonb_exists(datatable->'datatables', $2) \
         RETURNING 1",
        &w_id,
        &datatable_name,
    )
    .fetch_optional(&db)
    .await?;
    if updated.is_none() {
        return Err(Error::NotFound(format!(
            "data table {datatable_name} not found"
        )));
    }

    audit_log(
        &db,
        &authed,
        "workspaces.enable_datatable_migrations",
        ActionKind::Update,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    Ok(format!(
        "Enabled migrations for data table {datatable_name}"
    ))
}

/// Opt a data table out of migrations. Deletes ALL of its migration definitions.
async fn disable_datatable_migrations(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> Result<String> {
    require_datatable_migrations_manager(&db, &authed).await?;

    let mut tx = db.begin().await?;

    let updated = sqlx::query_scalar!(
        "UPDATE workspace_settings \
         SET datatable = jsonb_set(datatable, ARRAY['datatables', $2::text, 'migrations_enabled'], 'false'::jsonb) \
         WHERE workspace_id = $1 AND jsonb_exists(datatable->'datatables', $2) \
         RETURNING 1",
        &w_id,
        &datatable_name,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if updated.is_none() {
        return Err(Error::NotFound(format!(
            "data table {datatable_name} not found"
        )));
    }

    // Capture the deleted definitions so each removal is tallied as a deployed
    // object (like single-migration deletion), keeping workspace comparison and
    // git-sync callbacks in sync when a fork opts back out of migrations.
    let deleted = sqlx::query!(
        "DELETE FROM datatable_migrations WHERE workspace_id = $1 AND datatable = $2 \
         RETURNING timestamp, name",
        &w_id,
        &datatable_name,
    )
    .fetch_all(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.disable_datatable_migrations",
        ActionKind::Delete,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    for m in deleted {
        record_datatable_migration_deployment(
            &authed,
            &db,
            &w_id,
            &datatable_name,
            m.timestamp,
            &m.name,
        )
        .await?;
    }

    Ok(format!(
        "Disabled migrations for data table {datatable_name} and deleted its migrations"
    ))
}

#[derive(Deserialize)]
pub struct CreateDatatableMigration {
    pub name: String,
    pub code_up: String,
    #[serde(default)]
    pub code_down: Option<String>,
}

/// Migration names map onto on-disk file names and the `_wm_migrations` record,
/// so keep them to a safe path-segment charset (matches the CLI scaffold).
fn validate_migration_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::BadRequest(format!(
            "Invalid migration name '{name}': use only letters, digits, '_' and '-'"
        )));
    }
    Ok(())
}

/// The data table name becomes a directory segment in the sync export
/// (`migrations/datatable/<datatable>/...`); reject anything that could escape it.
pub(crate) fn validate_datatable_path_segment(datatable: &str) -> Result<()> {
    if datatable.is_empty()
        || datatable.contains('/')
        || datatable.contains('\\')
        || datatable.contains("..")
    {
        return Err(Error::BadRequest(format!(
            "Invalid data table name '{datatable}': must not contain '/', '\\' or '..'"
        )));
    }
    Ok(())
}

/// Record a data table migration change as a deployed object so it is tallied
/// into `workspace_diff` and shows up as a `datatable_migration` item in the
/// workspace-merge diff. The diff path is `<datatable>/<timestamp>_<name>`,
/// matching `parse_datatable_migration_diff_path`.
async fn record_datatable_migration_deployment(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    datatable: &str,
    timestamp: i64,
    name: &str,
) -> Result<()> {
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        db,
        w_id,
        DeployedObject::DatatableMigration { path: format!("{datatable}/{timestamp}_{name}") },
        Some(format!(
            "Data table migration {name} ({timestamp}) on {datatable}"
        )),
        false,
        None,
    )
    .await
}

/// Allocate the next version for a data table and insert the migration
/// definition, in one transaction. A per-(workspace, data table) advisory lock
/// serializes concurrent version allocation so two creates can't read the same
/// `MAX(timestamp)` and collide on the `(workspace_id, datatable, timestamp)`
/// primary key. The version is the current UTC `YYYYMMDDHHMMSS`, bumped past any
/// existing version to stay unique and monotonically increasing.
async fn insert_datatable_migration_def(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    datatable: &str,
    name: &str,
    code_up: &str,
    code_down: Option<&str>,
) -> Result<i64> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))",
        w_id,
        datatable,
    )
    .execute(&mut **tx)
    .await?;

    let now_ts: i64 = Utc::now()
        .format("%Y%m%d%H%M%S")
        .to_string()
        .parse()
        .map_err(|e| Error::internal_err(format!("Failed to build migration version: {}", e)))?;
    let max_existing: Option<i64> = sqlx::query_scalar!(
        "SELECT MAX(timestamp) FROM datatable_migrations WHERE workspace_id = $1 AND datatable = $2",
        w_id,
        datatable,
    )
    .fetch_one(&mut **tx)
    .await?;
    let timestamp = match max_existing {
        Some(m) if m >= now_ts => m + 1,
        _ => now_ts,
    };

    sqlx::query!(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up, code_down) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        w_id,
        datatable,
        timestamp,
        name,
        code_up,
        code_down,
    )
    .execute(&mut **tx)
    .await?;

    Ok(timestamp)
}

/// Mark a version as already installed in a data table's `_wm_migrations` table
/// (ensuring the table exists first).
async fn mark_datatable_version_installed(
    db: &DB,
    pg_db: &PgDatabase,
    datatable: &str,
    version: i64,
) -> Result<()> {
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable connection error: {}", e);
        }
    });
    ensure_wm_migrations_schema(&client).await?;
    client
        .execute(
            "INSERT INTO _wm_migrations (datatable, version) VALUES ($1, $2) \
             ON CONFLICT (datatable, version) DO NOTHING",
            &[&datatable, &version],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!("Failed to mark initial migration installed: {}", e))
        })?;
    Ok(())
}

/// Create a single migration for a data table. The version is generated
/// server-side (current UTC `YYYYMMDDHHMMSS`), bumped past any existing version
/// so it stays unique and monotonically increasing.
async fn create_datatable_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(payload): Json<CreateDatatableMigration>,
) -> JsonResult<DatatableMigration> {
    validate_datatable_path_segment(&datatable_name)?;
    validate_migration_name(&payload.name)?;
    ensure_datatable_migrations_enabled(&db, &w_id, &datatable_name).await?;

    let mut tx = db.begin().await?;
    let timestamp = insert_datatable_migration_def(
        &mut tx,
        &w_id,
        &datatable_name,
        &payload.name,
        &payload.code_up,
        payload.code_down.as_deref(),
    )
    .await?;
    tx.commit().await?;

    audit_log(
        &db,
        &authed,
        "workspaces.create_datatable_migration",
        ActionKind::Create,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    record_datatable_migration_deployment(
        &authed,
        &db,
        &w_id,
        &datatable_name,
        timestamp,
        &payload.name,
    )
    .await?;

    Ok(Json(DatatableMigration {
        datatable: datatable_name,
        timestamp,
        name: payload.name,
        code_up: payload.code_up,
        code_down: payload.code_down,
    }))
}

/// Delete a single migration definition from a data table.
async fn delete_datatable_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name, timestamp)): Path<(String, String, i64)>,
) -> Result<String> {
    // Hold the run-serialization lock across the applied-check and the delete: a
    // run snapshots a migration's SQL before recording its version, so an
    // unserialized delete could race it and leave `_wm_migrations` pointing at a
    // definition that no longer exists (breaking rollback and hiding the applied
    // version). Held until the handler returns. Fail closed if we can't verify.
    let unreachable = |e| {
        Error::internal_err(format!(
            "Cannot verify whether migration {} on data table '{}' has already been applied \
             (its database is unreachable: {}). Refusing to delete it; retry once the database \
             is reachable.",
            timestamp, datatable_name, e
        ))
    };
    let lock_client = lock_datatable_migration_runs(&db, &w_id, &datatable_name)
        .await
        .map_err(unreachable)?;
    let applied = read_applied_versions_on_client(&lock_client, &datatable_name)
        .await
        .map_err(unreachable)?;
    if applied.contains(&timestamp) {
        return Err(Error::BadRequest(format!(
            "Migration {} on data table '{}' has already been applied and cannot be deleted. \
             Revert it first.",
            timestamp, datatable_name
        )));
    }

    let deleted_name = sqlx::query_scalar!(
        "DELETE FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3 \
         RETURNING name",
        &w_id,
        &datatable_name,
        timestamp,
    )
    .fetch_optional(&db)
    .await?;
    // The definition is removed; runs may resume (audit/deploy metadata below
    // don't need the lock).
    drop(lock_client);

    audit_log(
        &db,
        &authed,
        "workspaces.delete_datatable_migration",
        ActionKind::Delete,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    // Only tally a change if a migration was actually deleted.
    if let Some(name) = deleted_name {
        record_datatable_migration_deployment(
            &authed,
            &db,
            &w_id,
            &datatable_name,
            timestamp,
            &name,
        )
        .await?;
    }

    Ok(format!(
        "Deleted migration {} from {}",
        timestamp, datatable_name
    ))
}

#[derive(Deserialize)]
pub struct UpsertDatatableMigration {
    pub timestamp: i64,
    pub name: String,
    pub code_up: String,
    #[serde(default)]
    pub code_down: Option<String>,
}

/// Insert or update a single migration at an explicit version. Used by
/// `wmill sync` to push a `migrations/datatable/<dt>/<version>_<name>.up.sql`
/// (and `.down.sql`) file as the source of truth.
async fn upsert_datatable_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(payload): Json<UpsertDatatableMigration>,
) -> Result<String> {
    validate_datatable_path_segment(&datatable_name)?;
    validate_migration_name(&payload.name)?;
    ensure_datatable_migrations_enabled(&db, &w_id, &datatable_name).await?;

    // Guard against silently rewriting a migration that has already run in the
    // data table's database: its `_wm_migrations` record would no longer match
    // its SQL, so a later `migrate up` would skip it and a rollback would run a
    // `down` that doesn't correspond to what was applied. Only an actual change
    // to an existing migration is guarded; unchanged re-pushes (e.g.
    // `wmill sync push`) always proceed.
    let existing = sqlx::query!(
        "SELECT name, code_up, code_down FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3",
        &w_id,
        &datatable_name,
        payload.timestamp,
    )
    .fetch_optional(&db)
    .await?;
    // When modifying an existing definition, hold the run-serialization lock
    // across the applied-check and the write below so an in-flight run can't
    // record a version for the SQL we're about to overwrite. Held until the end
    // of the handler (well past the write); a new/unchanged upsert needs no lock.
    let _run_lock = match existing {
        Some(existing)
            if !(existing.name == payload.name
                && existing.code_up == payload.code_up
                && existing.code_down == payload.code_down) =>
        {
            // Fail closed: if we can't lock/read the applied set (e.g. the
            // data-table database is temporarily unreachable), refuse the change
            // rather than risk overwriting a migration that has already run.
            let unreachable = |e| {
                Error::internal_err(format!(
                    "Cannot verify whether migration {} on data table '{}' has already been \
                     applied (its database is unreachable: {}). Refusing to modify it; retry \
                     once the database is reachable.",
                    payload.timestamp, datatable_name, e
                ))
            };
            let client = lock_datatable_migration_runs(&db, &w_id, &datatable_name)
                .await
                .map_err(unreachable)?;
            let applied = read_applied_versions_on_client(&client, &datatable_name)
                .await
                .map_err(unreachable)?;
            if applied.contains(&payload.timestamp) {
                return Err(Error::BadRequest(format!(
                    "Migration {} on data table '{}' has already been applied and cannot be modified. \
                     Revert it first, or add a new migration instead.",
                    payload.timestamp, datatable_name
                )));
            }
            Some(client)
        }
        _ => None,
    };

    sqlx::query!(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up, code_down) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (workspace_id, datatable, timestamp) DO UPDATE \
         SET name = EXCLUDED.name, code_up = EXCLUDED.code_up, code_down = EXCLUDED.code_down",
        &w_id,
        &datatable_name,
        payload.timestamp,
        &payload.name,
        &payload.code_up,
        payload.code_down.as_deref(),
    )
    .execute(&db)
    .await?;
    // The definition is written; runs may resume (audit/deploy metadata below
    // don't need the lock).
    drop(_run_lock);

    audit_log(
        &db,
        &authed,
        "workspaces.upsert_datatable_migration",
        ActionKind::Update,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    record_datatable_migration_deployment(
        &authed,
        &db,
        &w_id,
        &datatable_name,
        payload.timestamp,
        &payload.name,
    )
    .await?;

    Ok(format!(
        "Upserted migration {} in {}",
        payload.timestamp, datatable_name
    ))
}

/// Generate the first migration for a data table by snapshotting its current
/// schema with `pg_dump`. The migration is recorded as already installed (the
/// definition is written first, then its version is marked in the data table's
/// `_wm_migrations`, so it ends up considered applied and is never re-run) and
/// has no down migration.
async fn generate_initial_datatable_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<DatatableMigration> {
    validate_datatable_path_segment(&datatable_name)?;
    ensure_datatable_migrations_enabled(&db, &w_id, &datatable_name).await?;

    // The initial snapshot only makes sense on a data table with no migrations
    // yet; reject otherwise so repeated calls don't pile up duplicate "initial"
    // definitions (each at a distinct timestamp, each marked installed).
    let has_migrations: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM datatable_migrations WHERE workspace_id = $1 AND datatable = $2)",
        &w_id,
        &datatable_name,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    if has_migrations {
        return Err(Error::BadRequest(format!(
            "Data table '{datatable_name}' already has migrations; the initial migration can only be generated when there are none."
        )));
    }

    let db_resource = get_datatable_resource_from_db_unchecked(&db, &w_id, &datatable_name).await?;
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;

    // Snapshot the schema, excluding Windmill's own migration bookkeeping table.
    let dump_file = pg_dump_database(&pg_db, true, &["_wm_migrations"]).await?;
    let raw_dump = tokio::fs::read_to_string(&dump_file.path)
        .await
        .map_err(|e| Error::internal_err(format!("Failed to read schema dump: {}", e)))?;
    // pg_dump emits psql meta-commands (\restrict / \unrestrict) that aren't
    // valid SQL; drop them so the migration body can run via a plain query.
    let code_up: String = raw_dump
        .lines()
        .filter(|line| !line.trim_start().starts_with('\\'))
        .collect::<Vec<_>>()
        .join("\n");

    // Record the definition first, then mark it installed. If marking fails we
    // delete the definition, so a failure leaves no phantom "initial" (rather
    // than a `_wm_migrations` version with no definition that the UI can't
    // clear). The narrow window where it briefly shows "not run" is benign:
    // running it would just no-op/fail harmlessly against the existing schema.
    let mut tx = db.begin().await?;
    let timestamp =
        insert_datatable_migration_def(&mut tx, &w_id, &datatable_name, "initial", &code_up, None)
            .await?;
    tx.commit().await?;

    if let Err(e) = mark_datatable_version_installed(&db, &pg_db, &datatable_name, timestamp).await
    {
        let _ = sqlx::query!(
            "DELETE FROM datatable_migrations \
             WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3",
            &w_id,
            &datatable_name,
            timestamp,
        )
        .execute(&db)
        .await;
        return Err(e);
    }

    audit_log(
        &db,
        &authed,
        "workspaces.generate_initial_datatable_migration",
        ActionKind::Create,
        &w_id,
        Some(datatable_name.as_str()),
        None,
    )
    .await?;

    record_datatable_migration_deployment(
        &authed,
        &db,
        &w_id,
        &datatable_name,
        timestamp,
        "initial",
    )
    .await?;

    Ok(Json(DatatableMigration {
        datatable: datatable_name,
        timestamp,
        name: "initial".to_string(),
        code_up,
        code_down: None,
    }))
}

/// A datatable migration diff item has path `<datatable>/<timestamp>_<name>`.
/// Parse out the (datatable, timestamp) needed to look it up.
fn parse_datatable_migration_diff_path(path: &str) -> Option<(String, i64)> {
    let (datatable, file) = path.split_once('/')?;
    let ts_str: String = file.chars().take_while(|c| c.is_ascii_digit()).collect();
    let timestamp = ts_str.parse::<i64>().ok()?;
    Some((datatable.to_string(), timestamp))
}

pub(crate) async fn compare_two_datatable_migration(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    let (datatable, timestamp) = match parse_datatable_migration_diff_path(path) {
        Some(v) => v,
        None => {
            return Ok(ItemComparison {
                has_changes: false,
                exists_in_source: false,
                exists_in_fork: false,
            })
        }
    };

    let source = sqlx::query!(
        "SELECT name, code_up, code_down FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3",
        source_workspace_id,
        datatable,
        timestamp,
    )
    .fetch_optional(db)
    .await?;
    let target = sqlx::query!(
        "SELECT name, code_up, code_down FROM datatable_migrations \
         WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3",
        fork_workspace_id,
        datatable,
        timestamp,
    )
    .fetch_optional(db)
    .await?;

    let has_changes = match (&source, &target) {
        (Some(s), Some(t)) => {
            s.name != t.name || s.code_up != t.code_up || s.code_down != t.code_down
        }
        _ => source.is_some() || target.is_some(),
    };

    Ok(ItemComparison {
        has_changes,
        exists_in_source: source.is_some(),
        exists_in_fork: target.is_some(),
    })
}

#[derive(Deserialize, Debug)]
pub(crate) struct DatatableRename {
    pub(crate) from: String,
    pub(crate) to: String,
}

async fn resolve_datatable_pg(db: &DB, w_id: &str, datatable: &str) -> Result<PgDatabase> {
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable).await?;
    serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))
}

/// Tolerate a data table whose database has no `_wm_migrations` yet (42P01 =
/// undefined_table): it has never run a migration, so nothing to rename or forget.
fn ignore_missing_wm_migrations(e: tokio_postgres::Error) -> Result<()> {
    match e.as_db_error().map(|d| d.code().code()) {
        Some("42P01") => Ok(()),
        _ => Err(Error::internal_err(format!(
            "Failed to update _wm_migrations: {}",
            e
        ))),
    }
}

/// Drop a data table's rows from its own database's `_wm_migrations`.
async fn remote_forget_datatable_migrations(db: &DB, w_id: &str, datatable: &str) -> Result<()> {
    let pg_db = resolve_datatable_pg(db, w_id, datatable).await?;
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    client
        .execute(
            "DELETE FROM _wm_migrations WHERE datatable = $1",
            &[&datatable],
        )
        .await
        .map(|_| ())
        .or_else(ignore_missing_wm_migrations)
}

/// Relabel a data table's rows in its own database's `_wm_migrations`. `resolve_by`
/// names the config entry used to find the database (the old name, still present
/// pre-commit); `from`/`to` are the `datatable` column values to move between.
async fn remote_rename_datatable_migrations(
    db: &DB,
    w_id: &str,
    resolve_by: &str,
    from: &str,
    to: &str,
) -> Result<()> {
    let pg_db = resolve_datatable_pg(db, w_id, resolve_by).await?;
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    client
        .execute(
            "UPDATE _wm_migrations SET datatable = $2 WHERE datatable = $1",
            &[&from, &to],
        )
        .await
        .map(|_| ())
        .or_else(ignore_missing_wm_migrations)
}

/// Keep migration bookkeeping in sync when data tables are renamed or deleted in
/// the workspace config: the control table `datatable_migrations` (this database,
/// in `tx`) and each data table's own `_wm_migrations` (its own database, keyed
/// by data table name — see [`ensure_wm_migrations_schema`]).
///
/// Renames are applied in two phases through a temporary key so that a rename
/// chain or a swap (A->B, B->A) can't transiently collide on the
/// (datatable, ...) uniqueness mid-update.
///
/// The `_wm_migrations` updates are best-effort: they run just before `tx`
/// commits (resolved via the pool, which still exposes the old names), and a
/// temporarily unreachable data-table database is logged rather than failing the
/// whole config edit. If one is missed, the next run re-applies its migrations
/// against the existing schema.
pub(crate) async fn cascade_datatable_migration_renames_and_deletes(
    db: &DB,
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    renames: &[DatatableRename],
    deleted_datatables: &[String],
) -> Result<()> {
    if !deleted_datatables.is_empty() {
        sqlx::query!(
            "DELETE FROM datatable_migrations WHERE workspace_id = $1 AND datatable = ANY($2::text[])",
            w_id,
            deleted_datatables
        )
        .execute(&mut **tx)
        .await?;
    }

    for (i, r) in renames.iter().enumerate() {
        let tmp = format!("__wm_rename_tmp/{i}");
        sqlx::query!(
            "UPDATE datatable_migrations SET datatable = $3 WHERE workspace_id = $1 AND datatable = $2",
            w_id,
            &r.from,
            &tmp
        )
        .execute(&mut **tx)
        .await?;
    }
    for (i, r) in renames.iter().enumerate() {
        let tmp = format!("__wm_rename_tmp/{i}");
        sqlx::query!(
            "UPDATE datatable_migrations SET datatable = $3 WHERE workspace_id = $1 AND datatable = $2",
            w_id,
            &tmp,
            &r.to
        )
        .execute(&mut **tx)
        .await?;
    }

    for name in deleted_datatables {
        if let Err(e) = remote_forget_datatable_migrations(db, w_id, name).await {
            tracing::warn!("Failed to clear _wm_migrations for deleted data table {name}: {e}");
        }
    }
    for (i, r) in renames.iter().enumerate() {
        let tmp = format!("__wm_rename_tmp/{i}");
        if let Err(e) = remote_rename_datatable_migrations(db, w_id, &r.from, &r.from, &tmp).await {
            tracing::warn!(
                "Failed to stage _wm_migrations rename {} -> {}: {e}",
                r.from,
                r.to
            );
        }
    }
    for (i, r) in renames.iter().enumerate() {
        let tmp = format!("__wm_rename_tmp/{i}");
        if let Err(e) = remote_rename_datatable_migrations(db, w_id, &r.from, &tmp, &r.to).await {
            tracing::warn!(
                "Failed to finish _wm_migrations rename {} -> {}: {e}",
                r.from,
                r.to
            );
        }
    }

    Ok(())
}

/// Copy a workspace's migration definitions to another workspace, so a fork
/// inherits the same per-data-table migration history as its parent.
pub(crate) async fn clone_datatable_migrations(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up, code_down)
         SELECT $2, datatable, timestamp, name, code_up, code_down
         FROM datatable_migrations WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_migration_name_accepts_safe_names() {
        for name in ["initial", "add_index_to_customers", "fix-bug_2", "ABC123"] {
            assert!(
                validate_migration_name(name).is_ok(),
                "{name} should be valid"
            );
        }
    }

    #[test]
    fn validate_migration_name_rejects_unsafe_names() {
        for name in [
            "",
            "add index",
            "a/b",
            "a\\b",
            "a..b",
            "a.b",
            "naïve",
            "a/../b",
        ] {
            assert!(
                validate_migration_name(name).is_err(),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn validate_datatable_path_segment_accepts_and_rejects() {
        for ok in ["mydt", "my-dt", "main", "a b"] {
            assert!(
                validate_datatable_path_segment(ok).is_ok(),
                "{ok} should be ok"
            );
        }
        for bad in ["", "a/b", "a\\b", "..", "a..b", "../etc", "x/.."] {
            assert!(
                validate_datatable_path_segment(bad).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn parse_datatable_migration_diff_path_roundtrips() {
        assert_eq!(
            parse_datatable_migration_diff_path("mydt/20260101000001_create_users.up.sql"),
            Some(("mydt".to_string(), 20260101000001))
        );
        // up and down map to the same (datatable, timestamp) record.
        assert_eq!(
            parse_datatable_migration_diff_path("mydt/20260101000001_create_users.down.sql"),
            Some(("mydt".to_string(), 20260101000001))
        );
        // datatable names may themselves be hyphenated.
        assert_eq!(
            parse_datatable_migration_diff_path("my-dt/42_x.up.sql"),
            Some(("my-dt".to_string(), 42))
        );
    }

    #[test]
    fn parse_datatable_migration_diff_path_rejects_malformed() {
        // no slash → not a migration path
        assert_eq!(parse_datatable_migration_diff_path("nofile"), None);
        // filename not starting with digits → no timestamp
        assert_eq!(
            parse_datatable_migration_diff_path("mydt/create_users.up.sql"),
            None
        );
        // empty filename
        assert_eq!(parse_datatable_migration_diff_path("mydt/"), None);
    }

    async fn seed_migration(pool: &DB, w_id: &str, datatable: &str, timestamp: i64, name: &str) {
        sqlx::query(
            "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up) \
             VALUES ($1, $2, $3, $4, 'select 1;')",
        )
        .bind(w_id)
        .bind(datatable)
        .bind(timestamp)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn migration_keys(pool: &DB, w_id: &str) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT datatable, name FROM datatable_migrations WHERE workspace_id = $1 \
             ORDER BY datatable, timestamp",
        )
        .bind(w_id)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    // The cascade keeps each data table's migrations attached to its name when a
    // data table is renamed, drops them when it is deleted, and survives a swap
    // (A->B, B->A) at a shared timestamp without a primary-key collision.
    #[sqlx::test(migrations = "../migrations")]
    async fn cascade_renames_and_deletes_datatable_migrations(pool: DB) {
        let w_id = format!("dtmig{}", uuid::Uuid::new_v4().simple());
        sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test@windmill.dev')")
            .bind(&w_id)
            .execute(&pool)
            .await
            .unwrap();

        // rename a -> a2, delete d, and swap sa <-> sb (both at timestamp 5000)
        seed_migration(&pool, &w_id, "a", 1, "a_mig").await;
        seed_migration(&pool, &w_id, "d", 1, "d_mig").await;
        seed_migration(&pool, &w_id, "sa", 5000, "sa_mig").await;
        seed_migration(&pool, &w_id, "sb", 5000, "sb_mig").await;

        let mut tx = pool.begin().await.unwrap();
        cascade_datatable_migration_renames_and_deletes(
            &pool,
            &mut tx,
            &w_id,
            &[
                DatatableRename { from: "a".to_string(), to: "a2".to_string() },
                DatatableRename { from: "sa".to_string(), to: "sb".to_string() },
                DatatableRename { from: "sb".to_string(), to: "sa".to_string() },
            ],
            &["d".to_string()],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(
            migration_keys(&pool, &w_id).await,
            vec![
                ("a2".to_string(), "a_mig".to_string()),
                ("sa".to_string(), "sb_mig".to_string()),
                ("sb".to_string(), "sa_mig".to_string()),
            ]
        );
    }

    // A fork inherits its parent's migration definitions unchanged.
    #[sqlx::test(migrations = "../migrations")]
    async fn clone_copies_datatable_migrations_to_target(pool: DB) {
        let src = format!("src{}", uuid::Uuid::new_v4().simple());
        let dst = format!("dst{}", uuid::Uuid::new_v4().simple());
        for w in [&src, &dst] {
            sqlx::query(
                "INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test@windmill.dev')",
            )
            .bind(w)
            .execute(&pool)
            .await
            .unwrap();
        }
        seed_migration(&pool, &src, "customers", 1, "create_customers").await;
        seed_migration(&pool, &src, "customers", 2, "add_index").await;
        seed_migration(&pool, &src, "orders", 3, "create_orders").await;

        let mut tx = pool.begin().await.unwrap();
        clone_datatable_migrations(&mut tx, &src, &dst)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // the target ends up with an identical set, and the source is untouched.
        let expected = vec![
            ("customers".to_string(), "create_customers".to_string()),
            ("customers".to_string(), "add_index".to_string()),
            ("orders".to_string(), "create_orders".to_string()),
        ];
        assert_eq!(migration_keys(&pool, &dst).await, expected);
        assert_eq!(migration_keys(&pool, &src).await, expected);
    }
}
