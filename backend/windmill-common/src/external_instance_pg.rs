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

use std::collections::BTreeMap;

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
    pub last_setup: Option<ExternalInstancePgSetupReport>,
}

pub async fn read_external_instance_pg_config(db: &DB) -> Result<Option<ExternalInstancePg>> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        EXTERNAL_INSTANCE_PG_SETTING
    )
    .fetch_optional(db)
    .await?;
    value
        .map(|v| {
            serde_json::from_value(v).map_err(|e| {
                Error::internal_err(format!("reading {EXTERNAL_INSTANCE_PG_SETTING}: {e}"))
            })
        })
        .transpose()
}

pub async fn read_external_instance_pg_state<'c>(
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

/// Refuse to unset the cluster while Windmill still has databases on it: every data table and
/// Ducklake catalog there would stop resolving. Allowed on every edition, so a downgraded
/// instance can still clear a setting it no longer uses.
pub async fn ensure_external_instance_pg_removable(db: &DB) -> Result<()> {
    let state = read_external_instance_pg_state(db).await?;
    if state.databases.is_empty() {
        return Ok(());
    }
    let names = state
        .databases
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    Err(Error::BadRequest(format!(
        "The external instance cluster still holds databases Windmill created ({names}). Drop \
         them before removing {EXTERNAL_INSTANCE_PG_SETTING}."
    )))
}

/// Check a write to [`EXTERNAL_INSTANCE_PG_SETTING`] before it happens: `None`, null or an empty
/// string unsets it. Every writer of global settings calls this, the per-key and bulk endpoints
/// as well as the declarative sync.
pub async fn check_external_instance_pg_write(
    db: &DB,
    value: Option<&serde_json::Value>,
) -> Result<()> {
    match value {
        None | Some(serde_json::Value::Null) => ensure_external_instance_pg_removable(db).await,
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => {
            ensure_external_instance_pg_removable(db).await
        }
        Some(value) => {
            crate::external_instance_pg_oss::validate_external_instance_pg_setting(value)
        }
    }
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
