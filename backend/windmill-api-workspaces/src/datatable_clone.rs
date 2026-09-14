/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Copying a data table's database for a fork, as one request.
//!
//! The copy is created, filled and — for a data table under roles — given the source's owners and
//! grants in one server-side operation, and a failure anywhere after the database exists drops it
//! again. The fork request then takes the copy by name ([`crate::workspaces`]'s
//! `claim_datatable_clone`), which only a copy recorded here, or by `create_pg_database`, allows.

use std::collections::BTreeSet;

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::Deserialize;

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::datatable_roles::{lock_role_catalog, read_role_catalog_tx};
use windmill_common::error::{pg_error_message, Error, Result};
use windmill_common::utils::require_admin;
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::workspaces::{
    get_datatable_resource_from_db_unchecked, parse_datatable_ref_for, DataTableForkBehavior,
    GoverningDatatable,
};
use windmill_common::{PgDatabase, DB};

use crate::datatable_acl::connect_with_notices;
use crate::workspaces::{
    create_database_on_server, ensure_datatable_is_clonable, pg_dump_database, pg_import_dump,
    record_datatable_clone, DumpFile, PgDumpOptions,
};

#[derive(Deserialize)]
pub struct ClonePgDatabaseRequest {
    /// `datatable://<name>`: the data table to copy.
    pub source: String,
    /// The database the copy lands in. This request creates it.
    pub target_dbname: String,
    pub fork_behavior: DataTableForkBehavior,
}

/// Copy data table `source` of this workspace into a new database `target_dbname`.
///
/// Who may copy is what it was before data table roles: anyone for the schema, an admin of this
/// workspace for the rows. A copy of a data table under roles is safe to hand to a fork because
/// the fork takes it governed by the source's roles, and the replay gives those roles exactly the
/// privileges they hold on the source.
pub(crate) async fn clone_pg_database(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<ClonePgDatabaseRequest>,
) -> Result<String> {
    let reference = req.source.strip_prefix("datatable://").ok_or_else(|| {
        Error::BadRequest(format!(
            "A clone copies a data table: expected 'datatable://<name>', got '{}'",
            req.source
        ))
    })?;
    let (name, role) = parse_datatable_ref_for(&db, &w_id, reference).await?;
    if role.is_some() {
        return Err(Error::BadRequest(format!(
            "A clone copies the whole data table whatever the role; name it without `?role=`: \
             'datatable://{name}'"
        )));
    }
    let schema_only = match req.fork_behavior {
        DataTableForkBehavior::KeepOriginal => {
            return Err(Error::BadRequest(
                "Keeping the original database copies nothing".to_string(),
            ))
        }
        DataTableForkBehavior::SchemaOnly => true,
        DataTableForkBehavior::SchemaAndData => {
            require_admin(authed.is_admin, &authed.username)?;
            if *CLOUD_HOSTED {
                return Err(Error::BadRequest(
                    "Cloning schema and data is not available on cloud".to_string(),
                ));
            }
            false
        }
    };
    windmill_common::validate_dbname(&req.target_dbname)?;
    if !req.target_dbname.starts_with("wm_fork_")
        && !windmill_api_auth::is_super_admin_authed(&db, &authed).await?
    {
        return Err(Error::BadRequest(
            "Non-superadmin users can only clone into databases whose names start with 'wm_fork_'"
                .to_string(),
        ));
    }
    let governing = ensure_datatable_is_clonable(&db, &w_id, &name).await?;
    if governing.datatable.permissions.is_some() {
        crate::datatable_replay_oss::ensure_replay()?;
    }

    // Detached from the request: a client that goes away mid-copy must still leave either a
    // finished copy or no database at all, and dropping the handler's future would skip the
    // cleanup.
    let clone = CloneJob { db, authed, w_id, name, target: req.target_dbname, schema_only };
    tokio::spawn(clone.run(governing))
        .await
        .map_err(|e| Error::internal_err(format!("The clone stopped unexpectedly: {e}")))?
}

struct CloneJob {
    db: DB,
    authed: ApiAuthed,
    w_id: String,
    name: String,
    target: String,
    schema_only: bool,
}

impl CloneJob {
    async fn run(self, governing: GoverningDatatable) -> Result<String> {
        let source_pg: PgDatabase = serde_json::from_value(
            get_datatable_resource_from_db_unchecked(&self.db, &self.w_id, &self.name).await?,
        )
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
        let is_instance = governing.is_instance();

        // Dumped before the database exists, so a source that cannot be read leaves nothing
        // behind. Ownership never carries over: the restore runs as the target's connection user.
        // Grants do, except on the instance, where the replay below is what reproduces them.
        let dump = pg_dump_database(
            &source_pg,
            PgDumpOptions {
                schema_only: self.schema_only,
                no_owner: true,
                no_acl: is_instance,
                ..Default::default()
            },
        )
        .await?;

        if is_instance {
            windmill_common::create_custom_instance_database(&self.db, &self.target, "datatable")
                .await?;
        } else {
            create_database_on_server(&self.db, &source_pg, &self.target).await?;
        }

        let target_pg = PgDatabase { dbname: self.target.clone(), ..source_pg.clone() };
        if let Err(e) = self.fill(&governing, &source_pg, &target_pg, &dump).await {
            let dropped = if is_instance {
                windmill_common::drop_custom_instance_database(&self.db, &self.target).await
            } else {
                drop_database_on_server(&self.db, &source_pg, &self.target).await
            };
            if let Err(drop_err) = dropped {
                tracing::error!(
                    "Could not drop '{}' after a failed clone of data table '{}': {drop_err}",
                    self.target,
                    self.name
                );
                return Err(Error::ExecutionErr(format!(
                    "{e}. The database '{}' this clone created could not be dropped: {drop_err}",
                    self.target
                )));
            }
            return Err(e);
        }
        Ok(format!(
            "Cloned data table '{}' into '{}'",
            self.name, self.target
        ))
    }

    /// Restore the dump into the new database, replay the owners and grants of a data table under
    /// roles, and record the copy for the fork to take.
    async fn fill(
        &self,
        governing: &GoverningDatatable,
        source_pg: &PgDatabase,
        target_pg: &PgDatabase,
        dump: &DumpFile,
    ) -> Result<()> {
        pg_import_dump(target_pg, dump).await?;

        // Held until the copy is recorded: a role renamed or dropped meanwhile would change what
        // the replay names, and a settings save could move the source onto another database or put
        // it under roles. Taken in the same order as the permissions save and the ACL apply.
        let mut tx = self.db.begin().await?;
        lock_role_catalog(&mut tx).await?;
        let mut rows = vec![
            governing.workspace_id.clone(),
            governing.governing_workspace_id().to_string(),
        ];
        rows.sort();
        rows.dedup();
        sqlx::query(
            "SELECT 1 FROM workspace_settings WHERE workspace_id = ANY($1)
             ORDER BY workspace_id FOR UPDATE",
        )
        .bind(&rows)
        .fetch_all(&mut *tx)
        .await?;

        // Everything so far was decided before the locks.
        let now = ensure_datatable_is_clonable(&self.db, &self.w_id, &self.name).await?;
        let same_database = match (&now.datatable.database, &governing.datatable.database) {
            (Some(now), Some(then)) => {
                now.resource_type == then.resource_type && now.resource_path == then.resource_path
            }
            _ => false,
        };
        if !same_database {
            return Err(Error::BadRequest(format!(
                "Data table '{}' moved to another database while it was being copied; clone it \
                 again",
                self.name
            )));
        }

        let replayed = now.datatable.permissions.is_some();
        let mut replay = None;
        if replayed {
            crate::datatable_replay_oss::ensure_replay()?;
            let catalog = read_role_catalog_tx(&mut tx).await?;
            let catalog_roles: BTreeSet<String> =
                catalog.values().map(|r| r.name.clone()).collect();
            let (source, _source_notices) = connect_with_notices(&self.db, source_pg).await?;
            let (target, notices) = connect_with_notices(&self.db, target_pg).await?;
            replay = Some((source, target, notices, catalog_roles));
        }

        record_datatable_clone(&mut *tx, &self.target, &self.w_id, &self.name, &self.authed)
            .await?;
        let behavior = if self.schema_only {
            "schema_only"
        } else {
            "schema_and_data"
        };
        audit_log(
            &mut *tx,
            &self.authed,
            "workspaces.clone_datatable",
            ActionKind::Create,
            &self.w_id,
            Some(&self.name),
            Some(
                [
                    ("database", self.target.as_str()),
                    ("fork_behavior", behavior),
                ]
                .into(),
            ),
        )
        .await?;

        if let Some((source, mut target, mut notices, catalog_roles)) = replay {
            // One transaction on the copy: a replay that stops halfway leaves objects owned by one
            // role and granted as another. It commits before the record does; if the record then
            // fails, the database is dropped all the same.
            let pg_tx = target.transaction().await.map_err(|e| {
                Error::internal_err(format!(
                    "Failed to open a transaction on the copy: {}",
                    pg_error_message(&e)
                ))
            })?;
            crate::datatable_replay_oss::replay_owners_and_grants(
                &source,
                &pg_tx,
                &mut notices,
                &catalog_roles,
            )
            .await?;
            pg_tx.commit().await.map_err(|e| {
                Error::internal_err(format!(
                    "Failed to commit the replayed grants: {}",
                    pg_error_message(&e)
                ))
            })?;
        }
        tx.commit().await?;

        if replayed {
            windmill_common::feature_usage::log_feature_usage(
                "datatable",
                "clone_replayed",
                behavior,
            );
        }
        Ok(())
    }
}

/// `DROP DATABASE` on the server `server` connects to, disconnecting whoever is still on it.
async fn drop_database_on_server(db: &DB, server: &PgDatabase, dbname: &str) -> Result<()> {
    windmill_common::validate_dbname(dbname)?;
    let (client, connection) = server.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });
    let result = client
        .execute(
            &format!("DROP DATABASE IF EXISTS \"{dbname}\" WITH (FORCE)"),
            &[],
        )
        .await;
    drop(client);
    let _ = windmill_common::shutdown_pg_connection(join_handle).await;
    result.map(|_| ()).map_err(|e| {
        Error::internal_err(format!(
            "Failed to drop database '{dbname}': {}",
            pg_error_message(&e)
        ))
    })
}
