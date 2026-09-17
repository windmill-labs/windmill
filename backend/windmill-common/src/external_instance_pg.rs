/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The external Postgres cluster behind `external_instance` data tables and Ducklake catalogs.
//!
//! Windmill administers that cluster itself, logged in as the user in
//! [`EXTERNAL_INSTANCE_PG_SETTING`]. It creates `custom_instance_user` and
//! `custom_instance_replication_user` there, with passwords it generates and keeps in
//! [`EXTERNAL_INSTANCE_PG_STATE_SETTING`]. They share their names with the roles on Windmill's own
//! cluster, but they are different roles with different passwords.
//!
//! The cluster may hold data Windmill did not create. Two Windmill instances sharing one is not
//! supported: each would keep resetting the passwords the other depends on.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    global_settings::{EXTERNAL_INSTANCE_PG_SETTING, EXTERNAL_INSTANCE_PG_STATE_SETTING},
    instance_config::{CustomInstanceDb, ExternalInstancePg},
    DB,
};

/// What Windmill keeps about the external cluster. Server-managed and hidden: never part of the
/// instance config, never readable by an agent worker. No `Debug`: it carries live passwords.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ExternalInstancePgState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_pwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replication_pwd: Option<String>,
    /// The databases Windmill created on the cluster. It only ever drops one of these.
    #[serde(default)]
    pub databases: BTreeMap<String, CustomInstanceDb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_setup: Option<ExternalInstancePgSetupReport>,
    /// The cluster ([`external_instance_pg_address`]) the last successful setup converged. Databases
    /// are only created on a cluster setup succeeded on: the passwords above exist as soon as setup
    /// first runs, whether or not the cluster accepted them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_up_for: Option<String>,
}

/// What identifies the cluster a configuration points at. Other fields (admin login, sslmode) can
/// change without it becoming another cluster.
pub fn external_instance_pg_address(config: &ExternalInstancePg) -> String {
    format!(
        "{}:{}",
        config.host.trim().to_lowercase(),
        config.port.unwrap_or(5432)
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExternalInstancePgSetupReport {
    /// No step failed. Warnings leave it true.
    pub success: bool,
    pub finished_at: chrono::DateTime<chrono::Utc>,
    pub steps: Vec<ExternalInstancePgSetupStep>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExternalInstancePgSetupStep {
    pub name: String,
    pub status: SetupStepStatus,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SetupStepStatus {
    Ok,
    Warning,
    Error,
}

/// The status the settings page shows without running anything.
#[derive(Serialize, Debug)]
pub struct ExternalInstancePgStatus {
    pub configured: bool,
    pub database_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_setup: Option<ExternalInstancePgSetupReport>,
}

/// Authorization: returns the cluster's admin password and checks nothing. Callers MUST be
/// superadmin or an internal server path.
pub(crate) async fn read_external_instance_pg_config<'c>(
    executor: impl sqlx::PgExecutor<'c>,
) -> Result<Option<ExternalInstancePg>> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        EXTERNAL_INSTANCE_PG_SETTING
    )
    .fetch_optional(executor)
    .await?;
    value
        .map(|v| {
            serde_json::from_value(v).map_err(|e| {
                Error::internal_err(format!("reading {EXTERNAL_INSTANCE_PG_SETTING}: {e}"))
            })
        })
        .transpose()
}

/// Authorization: returns the passwords Windmill generated on the cluster and checks nothing.
/// Callers MUST be superadmin or an internal server path.
pub(crate) async fn read_external_instance_pg_state<'c>(
    executor: impl sqlx::PgExecutor<'c>,
) -> Result<ExternalInstancePgState> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        EXTERNAL_INSTANCE_PG_STATE_SETTING
    )
    .fetch_optional(executor)
    .await?;
    match value {
        None => Ok(ExternalInstancePgState::default()),
        Some(v) => serde_json::from_value(v).map_err(|e| {
            Error::internal_err(format!("reading {EXTERNAL_INSTANCE_PG_STATE_SETTING}: {e}"))
        }),
    }
}

pub async fn external_instance_pg_status(db: &DB) -> Result<ExternalInstancePgStatus> {
    let configured = read_external_instance_pg_config(db).await?.is_some();
    let state = read_external_instance_pg_state(db).await?;
    Ok(ExternalInstancePgStatus {
        configured,
        database_count: state.databases.len(),
        last_setup: state.last_setup,
    })
}

/// The databases Windmill created on the external cluster, without the passwords kept beside them.
///
/// Authorization: names every database across all workspaces, and the workspace each fork copy is
/// reserved for, and checks nothing. Callers MUST be superadmin or an internal authorization or
/// lifecycle path that does not return the names to a workspace caller.
pub async fn external_instance_databases(db: &DB) -> Result<BTreeMap<String, CustomInstanceDb>> {
    Ok(read_external_instance_pg_state(db).await?.databases)
}

/// The workspaces whose data tables or Ducklake catalogs name each database on the external cluster,
/// and the forks whose Ducklake metadata schemas there are still waiting to be dropped: those rows
/// outlive a settings change, and cleanup cannot drop a schema in a database that is gone. A row
/// whose schema is already dropped only waits on object storage, which needs no database.
///
/// Authorization: reads every workspace's settings and checks nothing. Callers MUST be superadmin
/// or an internal lifecycle path.
pub async fn external_instance_database_usages<'c>(
    db: impl sqlx::PgExecutor<'c>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT ws.workspace_id, entry->'database'->>'resource_path'
         FROM workspace_settings ws
         CROSS JOIN LATERAL jsonb_each(
             CASE WHEN jsonb_typeof(ws.datatable->'datatables') = 'object'
                 THEN ws.datatable->'datatables'
                 ELSE '{}'::jsonb END
         ) AS dt(k, entry)
         WHERE entry->'database'->>'resource_type' = 'external_instance'
           AND entry->'database'->>'resource_path' IS NOT NULL
         UNION ALL
         SELECT ws.workspace_id, entry->'catalog'->>'resource_path'
         FROM workspace_settings ws
         CROSS JOIN LATERAL jsonb_each(
             CASE WHEN jsonb_typeof(ws.ducklake->'ducklakes') = 'object'
                 THEN ws.ducklake->'ducklakes'
                 ELSE '{}'::jsonb END
         ) AS dl(k, entry)
         WHERE entry->'catalog'->>'resource_type' = 'external_instance'
           AND entry->'catalog'->>'resource_path' IS NOT NULL
         UNION ALL
         SELECT workspace_id, substring(catalog FROM length('external_instance:') + 1)
         FROM fork_ducklake_namespace
         WHERE catalog LIKE 'external\\_instance:%' AND NOT schema_dropped",
    )
    .fetch_all(db)
    .await?;
    let mut usages: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (workspace_id, dbname) in rows {
        usages.entry(dbname).or_default().insert(workspace_id);
    }
    Ok(usages)
}

/// Refuse to unset the cluster while Windmill still has databases on it, or a workspace still
/// points at one: every data table there would stop resolving. Allowed on every edition, so a
/// downgraded instance can still clear a setting it no longer uses.
pub async fn ensure_external_instance_pg_removable(db: &DB) -> Result<()> {
    let state = read_external_instance_pg_state(db).await?;
    let usages = external_instance_database_usages(db).await?;
    if state.databases.is_empty() && usages.is_empty() {
        return Ok(());
    }
    let names = state
        .databases
        .keys()
        .chain(usages.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    Err(Error::BadRequest(format!(
        "The external instance cluster still holds databases in use ({names}). Drop them and \
         repoint the data tables and Ducklake catalogs using them before removing {EXTERNAL_INSTANCE_PG_SETTING}."
    )))
}

/// Refuse a workspace setting that newly names an `external_instance` database on an edition
/// without them.
pub fn ensure_external_instance_available() -> Result<()> {
    crate::external_instance_pg_oss::ensure_external_instance_available()
}

/// The connection an `external_instance` database resolves to: `custom_instance_user`, or the
/// replication user, on the external cluster.
///
/// Authorization: returns live credentials and checks nothing. Callers MUST have authorized access
/// to the data table that names `dbname`.
pub async fn external_instance_connection_unchecked(
    db: &DB,
    dbname: &str,
    replication: bool,
) -> Result<crate::PgDatabase> {
    crate::external_instance_pg_oss::external_instance_connection_unchecked(db, dbname, replication)
        .await
}

/// Create `dbname` on the external cluster and register it. Refuses a name already taken there,
/// whoever took it.
///
/// Authorization: checks nothing. Callers MUST be superadmin, or be cloning a data table they may
/// fork into a `wm_fork_` database.
pub async fn create_external_instance_database_unchecked(
    db: &DB,
    dbname: &str,
    tag: &str,
    for_workspace: Option<&str>,
) -> Result<()> {
    crate::external_instance_pg_oss::create_external_instance_database_unchecked(
        db,
        dbname,
        tag,
        for_workspace,
    )
    .await
}

/// Drop `dbname` from the external cluster: only a database Windmill registered creating, and still
/// carries the mark it set there. Refused while anything uses it
/// ([`crate::workspaces::managed_database_uses`]), except the `exempt` data table entry: the fork
/// copy being cleaned up.
///
/// Authorization: checks nothing. Callers MUST be superadmin, or be deleting the fork that owns
/// this `wm_fork_` database.
pub async fn drop_external_instance_database_unchecked(
    db: &DB,
    dbname: &str,
    exempt: Option<(&str, &str)>,
) -> Result<()> {
    crate::external_instance_pg_oss::drop_external_instance_database_unchecked(db, dbname, exempt)
    .await
}

/// Serializes everything that changes which databases exist on the external cluster, or which data
/// tables name them: setup, creates, drops, and data table saves. Held until `tx` ends.
pub async fn lock_external_instance_pg_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(EXTERNAL_INSTANCE_PG_STATE_SETTING)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Refuse a data table naming `dbname` unless Windmill created it on the external cluster. Takes
/// the lock drops take, so none can remove the database before `tx`, which saves the data table,
/// commits.
pub async fn ensure_external_instance_database_registered(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    dbname: &str,
) -> Result<()> {
    lock_external_instance_pg_state(tx).await?;
    if read_external_instance_pg_state(&mut **tx)
        .await?
        .databases
        .contains_key(dbname)
    {
        return Ok(());
    }
    Err(Error::BadRequest(format!(
        "Windmill did not create a database named '{dbname}' on the external instance cluster. \
         Create it from the instance settings first."
    )))
}

/// Write [`EXTERNAL_INSTANCE_PG_SETTING`]: `None`, null or an empty string unsets it. Every writer
/// of global settings goes through this for that key — the per-key and bulk endpoints as well as
/// the declarative sync — instead of writing the row itself.
///
/// The checks and the write share one transaction holding [`lock_external_instance_pg_state`]. A
/// check taken outside it could pass while a database create still reads the old cluster, which
/// would then register a database there after the setting names another one.
///
/// Authorization: checks nothing. Callers MUST be superadmin, or the declarative instance config
/// sync, which applies what the operator deployed.
pub async fn write_external_instance_pg_setting(
    db: &DB,
    value: Option<&serde_json::Value>,
) -> Result<()> {
    let value = match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => None,
        Some(value) => Some(value),
    };
    let mut tx = db.begin().await?;
    lock_external_instance_pg_state(&mut tx).await?;
    match value {
        None => {
            ensure_external_instance_pg_removable(db).await?;
            sqlx::query("DELETE FROM global_settings WHERE name = $1")
                .bind(EXTERNAL_INSTANCE_PG_SETTING)
                .execute(&mut *tx)
                .await?;
        }
        Some(value) => {
            crate::external_instance_pg_oss::validate_external_instance_pg_setting(value)?;
            ensure_external_instance_pg_not_repointed(db, value).await?;
            sqlx::query(
                "INSERT INTO global_settings (name, value) VALUES ($1, $2)
                 ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
            )
            .bind(EXTERNAL_INSTANCE_PG_SETTING)
            .bind(value)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    tracing::info!(
        "{} global setting {EXTERNAL_INSTANCE_PG_SETTING}",
        if value.is_some() { "Set" } else { "Unset" }
    );
    Ok(())
}

/// [`write_external_instance_pg_setting`] for a settings diff: writes the key if the diff touches
/// it, and takes it out of the diff so the generic apply does not write it again.
///
/// Authorization: checks nothing. Callers MUST be superadmin, or the declarative instance config
/// sync, which applies what the operator deployed.
pub async fn write_external_instance_pg_from_diff(
    db: &DB,
    diff: &mut crate::instance_config::SettingsDiff,
) -> Result<()> {
    if let Some(value) = diff.upserts.remove(EXTERNAL_INSTANCE_PG_SETTING) {
        write_external_instance_pg_setting(db, Some(&value)).await?;
    }
    if let Some(i) = diff
        .deletes
        .iter()
        .position(|k| k == EXTERNAL_INSTANCE_PG_SETTING)
    {
        diff.deletes.remove(i);
        write_external_instance_pg_setting(db, None).await?;
    }
    Ok(())
}

/// Refuse pointing the setting at another host or port while databases live on the current one.
/// Data tables name databases, not clusters, so they would silently resolve to whatever the new
/// cluster holds under the same names. Other fields (admin login, sslmode) may change freely.
async fn ensure_external_instance_pg_not_repointed(
    db: &DB,
    value: &serde_json::Value,
) -> Result<()> {
    let Some(current) = read_external_instance_pg_config(db).await? else {
        return Ok(());
    };
    let Ok(desired) = serde_json::from_value::<ExternalInstancePg>(value.clone()) else {
        return Ok(());
    };
    if external_instance_pg_address(&current) == external_instance_pg_address(&desired) {
        return Ok(());
    }
    let state = read_external_instance_pg_state(db).await?;
    let usages = external_instance_database_usages(db).await?;
    if state.databases.is_empty() && usages.is_empty() {
        return Ok(());
    }
    Err(Error::BadRequest(format!(
        "The external instance cluster at {}:{} still holds databases in use. Drop them and repoint \
         what uses them before pointing {EXTERNAL_INSTANCE_PG_SETTING} at another cluster.",
        current.host.trim(),
        current.port.unwrap_or(5432)
    )))
}

/// Converge the external cluster on the configured login: check what it can do, create or update
/// Windmill's two roles with the stored passwords, and report anything that would get in the way.
/// With `rotate_passwords`, generate new passwords first. Safe to run again; running it again is
/// how a failed rotation is repaired.
///
/// Authorization: administers the external cluster with its admin credentials and checks nothing.
/// Callers MUST be superadmin.
pub async fn setup_external_instance_pg_unchecked(
    db: &DB,
    rotate_passwords: bool,
) -> Result<ExternalInstancePgSetupReport> {
    crate::external_instance_pg_oss::setup_external_instance_pg_unchecked(db, rotate_passwords)
        .await
}
