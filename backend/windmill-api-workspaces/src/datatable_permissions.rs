/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Role-based access control for data tables: read the config, preview the SQL a
//! change plans out, and apply it.
//!
//! A permissioned data table maps each Windmill role onto a real Postgres login
//! role, so a script that runs as `analyst` connects as `analyst` and the
//! database — not Windmill — enforces what it may touch. `admin` is the exception:
//! it is the connection the data table already resolved to before permissions
//! were turned on, so it owns every existing object and is never created,
//! renamed or dropped.

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::ensure_instance_db_grant_options_unchecked;
use windmill_common::error::{pg_error_message, Error, JsonResult, Result};
use windmill_common::query_builders::{render_db_quoted_identifier, DbType};
use windmill_common::utils::require_admin;
use windmill_common::workspaces::{
    can_use_datatable_role, datatable_database_identity, get_datatable_resource_from_db_unchecked,
    DataTable, DataTableCatalogResourceType, DataTablePermissions, ADMIN_DATATABLE_ROLE,
};
use windmill_common::{PgDatabase, DB};

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/datatable_permissions/{datatable_name}",
            get(get_datatable_permissions).post(set_datatable_permissions),
        )
        .route(
            "/datatable_permissions/{datatable_name}/preview",
            post(preview_datatable_permissions),
        )
        .route(
            "/datatable_usable_roles/{datatable_name}",
            get(list_usable_datatable_roles),
        )
}

/// A data table role as the UI sees it: the generated password never leaves the
/// server, since it grants direct database access to anyone who reads it.
#[derive(Serialize, Deserialize, Debug)]
pub struct DatatableRoleInfo {
    pub name: String,
    #[serde(default)]
    pub tenants: Vec<String>,
    /// The underlying Postgres role, so grants can be written by hand against it.
    /// Absent for `admin`, which reuses the data table's own connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pg_rolename: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DatatablePermissionsInfo {
    pub enabled: bool,
    pub roles: Vec<DatatableRoleInfo>,
    /// The role a script gets when it names none.
    pub default_role: String,
}

#[derive(Deserialize, Debug)]
pub struct SetDatatablePermissions {
    pub enabled: bool,
    #[serde(default)]
    pub roles: Vec<DatatableRoleInfo>,
    /// The role a script gets when it names none. Absent means `admin`.
    #[serde(default)]
    pub default_role: Option<String>,
    /// Role renames (old -> new), tracked client-side by a stable id so a rename
    /// plans an `ALTER ROLE ... RENAME` instead of a drop plus a create — which
    /// would destroy the grants the role had accumulated.
    #[serde(default)]
    pub renames: Vec<DatatableRoleRename>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatatableRoleRename {
    pub from: String,
    pub to: String,
}

/// The roles the caller may actually run as, for pickers. Unlike the admin-only
/// permissions view this exposes no tenant lists — only what the caller can use.
#[derive(Serialize, Debug)]
pub struct UsableDatatableRoles {
    pub enabled: bool,
    pub roles: Vec<String>,
    pub default_role: String,
}

#[derive(Serialize, Debug)]
pub struct DatatablePermissionsPreview {
    pub statements: Vec<String>,
    pub warnings: Vec<String>,
}

/// One planned statement. `display` is what the preview shows: identical to
/// `sql` except where a generated password would otherwise be printed.
#[derive(Debug)]
pub(crate) struct PlannedStatement {
    pub(crate) sql: String,
    pub(crate) display: String,
    /// The Postgres role this statement is part of destroying, if any.
    ///
    /// Creating a role before the config names it leaves at worst a role the
    /// next save adopts, so those run inside the request. Dropping one is not
    /// reversible — `DROP OWNED` discards every grant it accumulated — so these
    /// wait for the config that stops naming the role to commit, and are then
    /// checked against it one role at a time.
    pub(crate) drops_role: Option<String>,
}

impl PlannedStatement {
    /// Only the planner builds these, and that is the enterprise module.
    #[cfg_attr(
        not(all(feature = "private", feature = "enterprise")),
        allow(dead_code)
    )]
    pub(crate) fn plain(sql: String) -> Self {
        Self { display: sql.clone(), sql, drops_role: None }
    }
}

#[derive(Debug)]
pub(crate) struct RolePlan {
    pub(crate) statements: Vec<PlannedStatement>,
    /// The permissions block to persist once the statements have run.
    pub(crate) permissions: DataTablePermissions,
    pub(crate) warnings: Vec<String>,
}

/// Refuse to plan a change on an enterprise binary whose plan does not cover it.
/// A build that is not enterprise has no planner at all — see
/// [`crate::datatable_permissions_oss`] — so this only has the licensed
/// editions left to tell apart.
pub(crate) async fn require_datatable_permissions_license() -> Result<()> {
    #[cfg(feature = "enterprise")]
    if !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Enterprise
    ) {
        return Err(Error::BadRequest(
            "Data table permissions require an Enterprise license".to_string(),
        ));
    }
    Ok(())
}

/// Only the planners quote identifiers, and those are the enterprise modules.
#[cfg_attr(
    not(all(feature = "private", feature = "enterprise")),
    allow(dead_code)
)]
pub(crate) fn quote_ident(ident: &str) -> String {
    render_db_quoted_identifier(ident, DbType::Postgresql)
}

/// Read a data table's config, whatever the caller is.
///
/// Authorization: performs none. What it returns is the config as stored,
/// generated role passwords included, so callers MUST have authorized the read,
/// and MUST NOT pass the value outward without
/// [`windmill_common::workspaces::redact_datatable_settings_for_export`].
pub(crate) async fn read_datatable_unchecked(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<DataTable> {
    let value = sqlx::query_scalar!(
        "SELECT ws.datatable->'datatables'->$2 FROM workspace_settings ws WHERE ws.workspace_id = $1",
        w_id,
        datatable_name,
    )
    .fetch_one(db)
    .await?
    .filter(|v| !v.is_null())
    .ok_or_else(|| Error::NotFound(format!("Data table '{datatable_name}' not found")))?;
    serde_json::from_value(value)
        .map_err(|e| Error::internal_err(format!("Invalid data table config: {e}")))
}

/// Make sure `custom_instance_user` can pass its privileges on, for a data table
/// on an instance database.
///
/// Handing privileges to the roles this feature creates means granting them, and
/// a privilege held without `WITH GRANT OPTION` cannot be granted on — Postgres
/// answers such a statement with a warning and no effect. Databases provisioned
/// before those options were part of the grants still hold them plain, and only
/// the instance's own Postgres user, which owns them, can add the options.
///
/// Idempotent, and a no-op for a data table on a user-provided resource, where
/// Windmill does not own the Postgres user. Never fatal: the caller's own work
/// is what the admin asked for, and it may well not need any of this.
pub(crate) async fn ensure_instance_db_can_delegate(db: &DB, w_id: &str, datatable_name: &str) {
    let Ok(datatable) = read_datatable_unchecked(db, w_id, datatable_name).await else {
        return;
    };
    if datatable.database.resource_type != DataTableCatalogResourceType::Instance {
        return;
    }
    if let Err(e) =
        ensure_instance_db_grant_options_unchecked(db, &datatable.database.resource_path).await
    {
        tracing::warn!(
            "Could not refresh the grant options of instance database '{}': {}. Continuing.",
            datatable.database.resource_path,
            e
        );
    }
}

/// What the plan has to be built against, probed from the data table's own
/// database rather than assumed from its config.
pub(crate) struct AdminConnection {
    pub(crate) dbname: String,
    /// The database this resolved to, as the config records it so a later
    /// resolution can tell it has not moved. `None` for an instance database,
    /// which no workspace edit can repoint.
    pub(crate) database_identity: Option<String>,
    pub(crate) admin_pg_role: String,
    pub(crate) pg_roles: PgRoleInventory,
    /// Whether `PUBLIC` holds CREATE on schema `public`, i.e. every role in this
    /// database — including the ones created here — can make objects in it.
    pub(crate) public_schema_is_open: bool,
    /// The default-privilege rules already in force for this data table's roles.
    pub(crate) default_acl_rules: Vec<DefaultAclRule>,
}

/// The `wm_` logins the cluster holds, split by whether this data table may take
/// one over.
#[derive(Default)]
/// Only the planner reads these, and that is the enterprise module.
#[cfg_attr(
    not(all(feature = "private", feature = "enterprise")),
    allow(dead_code)
)]
pub(crate) struct PgRoleInventory {
    /// Every one of them. `pg_roles` is a cluster catalog, so this spans every
    /// database and every workspace on the instance — a name in here cannot be
    /// created again, wherever it came from.
    pub(crate) existing: HashSet<String>,
    /// Those the data table's own administrative login is a member of, which is
    /// what creating a role here does. A save may finish itself by adopting one
    /// of these — a role it created before dying — and nothing else: any other
    /// occupant of a generated name belongs to someone else, and resetting its
    /// password would hand this data table their login.
    pub(crate) adoptable: HashSet<String>,
}

/// One `ALTER DEFAULT PRIVILEGES` rule as the catalog has it.
///
/// Postgres records such a rule per creating role, so a role added later is not
/// covered by any of them: a grant on "future tables" would quietly stop
/// applying to whatever that new role creates. Replaying the existing rules for
/// each new role is what keeps the policy whole.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DefaultAclRule {
    /// `None` for a rule that is not scoped to a schema.
    pub(crate) schema: Option<String>,
    /// `TABLES`, `SEQUENCES`, `FUNCTIONS` or `TYPES`.
    pub(crate) objects: String,
    /// Postgres role the privileges go to, always one of this data table's own.
    pub(crate) grantee: String,
    pub(crate) privileges: Vec<String>,
}

/// The rules this data table's own roles wrote, which are the only ones a role
/// of this data table inherits.
///
/// Scoped to `own_pg_roles` on both sides. Two data tables can point at one
/// physical database — and share its administrative login — so a rule is ours
/// only when both the role that wrote it and the role it grants to are.
async fn read_default_acl_rules(
    client: &tokio_postgres::Client,
    own_pg_roles: &[String],
) -> Result<Vec<DefaultAclRule>> {
    let rows = client
        .query(
            "SELECT n.nspname,
                    CASE d.defaclobjtype
                        WHEN 'r' THEN 'TABLES' WHEN 'S' THEN 'SEQUENCES'
                        WHEN 'f' THEN 'FUNCTIONS' ELSE 'TYPES' END,
                    pg_get_userbyid(a.grantee),
                    a.privilege_type
             FROM pg_default_acl d
             LEFT JOIN pg_namespace n ON n.oid = d.defaclnamespace,
                  aclexplode(d.defaclacl) a
             WHERE pg_get_userbyid(d.defaclrole) = ANY($1)
               AND a.grantee <> 0
               AND pg_get_userbyid(a.grantee) = ANY($1)",
            &[&own_pg_roles],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the default privileges: {}",
                pg_error_message(&e)
            ))
        })?;
    let mut folded: BTreeMap<(Option<String>, String, String), Vec<String>> = BTreeMap::new();
    for row in rows {
        folded
            .entry((row.get(0), row.get(1), row.get(2)))
            .or_default()
            .push(row.get(3));
    }
    Ok(folded
        .into_iter()
        .map(|((schema, objects, grantee), mut privileges)| {
            privileges.sort();
            privileges.dedup();
            DefaultAclRule { schema, objects, grantee, privileges }
        })
        .collect())
}

/// Connect to the data table's own database as `admin` and report the identity a
/// plan has to be built against: the database name, the role that owns the
/// existing objects, and the roles that actually exist in the cluster.
///
/// Authorization: performs none. This is the data table's own connection — it
/// owns every object in that database and can grant anything it holds — so
/// callers MUST have authorized the operation it is opened for, against the
/// identity making the request, and MUST NOT hand it to a request that has not
/// been.
pub(crate) async fn connect_as_admin_unchecked(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<(tokio_postgres::Client, AdminConnection)> {
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
    let database_identity = (read_datatable_unchecked(db, w_id, datatable_name)
        .await?
        .database
        .resource_type
        == DataTableCatalogResourceType::Postgresql)
        .then(|| datatable_database_identity(&db_resource));
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
    let dbname = pg_db.dbname.clone();
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable permissions connection error: {}", e);
        }
    });

    let admin_pg_role: String = client
        .query_one("SELECT current_user", &[])
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the data table's connection identity: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);

    let existing = client
        .query(
            "SELECT rolname FROM pg_roles WHERE rolname LIKE 'wm\\_%'",
            &[],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list existing roles: {}",
                pg_error_message(&e)
            ))
        })?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    // Membership is the mark a role of this data table carries: every one of
    // them is granted to this connection when it is created.
    let adoptable = client
        .query(
            "SELECT r.rolname
             FROM pg_auth_members m
             JOIN pg_roles r ON r.oid = m.roleid
             JOIN pg_roles a ON a.oid = m.member
             WHERE a.rolname = $1 AND r.rolname LIKE 'wm\\_%'",
            &[&admin_pg_role],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list the roles this data table owns: {}",
                pg_error_message(&e)
            ))
        })?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    // grantee 0 is PUBLIC. A NULL acl means the server default, which granted
    // PUBLIC CREATE on `public` before Postgres 15.
    let public_schema_is_open: bool = client
        .query_one(
            "SELECT COALESCE(
                 (SELECT bool_or(a.privilege_type = 'CREATE' AND a.grantee = 0)
                  FROM pg_namespace n, aclexplode(n.nspacl) a
                  WHERE n.nspname = 'public'),
                 current_setting('server_version_num')::int < 150000
             )",
            &[],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to inspect the public schema: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);

    // The roles a rule of this data table can be written by or granted to: its
    // own, plus the connection they were all created from.
    let mut own_pg_roles = vec![admin_pg_role.clone()];
    own_pg_roles.extend(
        read_datatable_unchecked(db, w_id, datatable_name)
            .await?
            .permissions
            .filter(|p| p.enabled)
            .into_iter()
            .flat_map(|p| p.roles.into_values())
            .filter_map(|role| role.pg_rolename),
    );
    let default_acl_rules = read_default_acl_rules(&client, &own_pg_roles).await?;

    Ok((
        client,
        AdminConnection {
            dbname,
            database_identity,
            admin_pg_role,
            pg_roles: PgRoleInventory { existing, adoptable },
            public_schema_is_open,
            default_acl_rules,
        },
    ))
}

async fn build_plan(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    req: &SetDatatablePermissions,
) -> Result<(tokio_postgres::Client, RolePlan)> {
    require_datatable_permissions_license().await?;
    let datatable = read_datatable_unchecked(db, w_id, datatable_name).await?;
    let (client, conn) = connect_as_admin_unchecked(db, w_id, datatable_name).await?;
    let plan = crate::datatable_permissions_oss::plan_role_changes(
        w_id,
        datatable_name,
        &conn.dbname,
        &conn.admin_pg_role,
        datatable.permissions.as_ref(),
        req,
        &conn.pg_roles,
        conn.public_schema_is_open,
        &conn.default_acl_rules,
    )?;
    let mut plan = plan;
    // Stamped from the connection the roles are about to be created through, so
    // a resolution that lands anywhere else later can refuse.
    plan.permissions.database_identity = conn.database_identity;
    Ok((client, plan))
}

/// The connection and statements that drop a deleted data table's roles, giving
/// its objects back to admin first.
///
/// Resolved separately from being run: resolving needs the data table's config,
/// which the save is about to remove, while running is irreversible and must not
/// happen until that save has committed.
pub(crate) type PlannedRoleDrop = (tokio_postgres::Client, RolePlan);

/// Plan the removal of every Postgres role of a data table that is being
/// deleted, against the config as it still stands.
///
/// A data table whose database is already unreachable must still be removable
/// from the config, so a failure here is logged and the deletion goes ahead
/// without a plan — leaving roles that only a `DROP ROLE` by hand will clear.
pub(crate) async fn plan_drop_of_deleted_datatable(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Option<PlannedRoleDrop> {
    let req = SetDatatablePermissions {
        enabled: false,
        roles: vec![],
        default_role: None,
        renames: vec![],
    };
    let res = async {
        let datatable = read_datatable_unchecked(db, w_id, datatable_name).await?;
        if !datatable
            .permissions
            .as_ref()
            .is_some_and(|p| p.enabled && p.roles.len() > 1)
        {
            return Ok(None);
        }
        build_plan(db, w_id, datatable_name, &req).await.map(Some)
    }
    .await;
    match res {
        Ok(planned) => planned,
        Err(e) => {
            tracing::error!(
                "Could not plan dropping the Postgres roles of deleted data table {datatable_name} in {w_id}: {e:#}"
            );
            None
        }
    }
}

/// The Postgres logins the workspace's config currently names.
fn pg_rolenames_in_use(settings: Option<&serde_json::Value>) -> HashSet<String> {
    settings
        .and_then(|s| s.get("datatables"))
        .and_then(|d| d.as_object())
        .map(|datatables| {
            datatables
                .values()
                .filter_map(|dt| dt.pointer("/permissions/roles")?.as_object())
                .flat_map(|roles| roles.values())
                .filter_map(|role| role.get("pg_rolename")?.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Destroy the roles a committed config stopped naming.
///
/// Asked under the settings row, one role at a time: a role the config names
/// again — a data table recreated under the same name, or a save that put the
/// role back — is left alone, and one it does not name is dropped whatever else
/// has changed in the meantime. Nobody else can be planning against these
/// between the question and the answer, since that row is what every save takes
/// first.
///
/// Best-effort: a role outliving its config is recoverable, dropping one a live
/// data table depends on is not.
async fn drop_roles_the_config_no_longer_names(
    db: &DB,
    w_id: &str,
    client: &mut tokio_postgres::Client,
    statements: &[&PlannedStatement],
) -> Result<()> {
    let mut tx = db.begin().await?;
    let settings =
        windmill_common::workspaces::lock_workspace_settings_unchecked(&mut tx, w_id).await?;
    let in_use = pg_rolenames_in_use(settings.as_ref());
    let to_run: Vec<&PlannedStatement> = statements
        .iter()
        .filter(|s| {
            s.drops_role
                .as_ref()
                .is_none_or(|role| !in_use.contains(role))
        })
        .copied()
        .collect();
    if !to_run.is_empty() {
        let mut attempted: Vec<&str> = to_run
            .iter()
            .filter_map(|s| s.drops_role.as_deref())
            .collect();
        attempted.sort();
        attempted.dedup();
        let ran = async {
            // The settings row is held for as long as these run, and they run on a
            // database this workspace does not control — a lock held there, or a
            // role with a great deal to reassign, would otherwise stall every save
            // of every data table in the workspace behind it.
            client
                .batch_execute("SET statement_timeout = '60s'")
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Failed to bound the role changes: {}",
                        pg_error_message(&e)
                    ))
                })?;
            run_statements(client, &to_run).await
        }
        .await;
        // Named here rather than by the caller, and on every way out: which of
        // them were skipped because the config names them again is only known
        // under the lock above, and whatever failed, the config that stopped
        // naming these has committed and nothing comes back for them.
        ran.map_err(|e| {
            Error::ExecutionErr(format!("{e}. Roles left behind: {}", attempted.join(", ")))
        })?;
    }
    tx.commit().await?;
    Ok(())
}

/// Drop the roles of a data table that was deleted, once the config saying so
/// has committed.
pub(crate) async fn run_planned_drop(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    (mut client, plan): PlannedRoleDrop,
) {
    let statements: Vec<&PlannedStatement> = plan.statements.iter().collect();
    if let Err(e) = drop_roles_the_config_no_longer_names(db, w_id, &mut client, &statements).await
    {
        tracing::error!(
            "Could not drop the Postgres roles of deleted data table {datatable_name} in {w_id}: {e:#}"
        );
    }
}

/// Run a plan's statements in a single transaction, so a failure part-way leaves
/// the database exactly as it was.
async fn run_statements(
    client: &mut tokio_postgres::Client,
    statements: &[&PlannedStatement],
) -> Result<()> {
    let pg_tx = client.transaction().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to open a transaction on the data table: {}",
            pg_error_message(&e)
        ))
    })?;
    for statement in statements.iter() {
        pg_tx.batch_execute(&statement.sql).await.map_err(|e| {
            Error::ExecutionErr(format!(
                "Failed to run `{}`: {}",
                statement.display,
                pg_error_message(&e)
            ))
        })?;
    }
    pg_tx.commit().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to commit the role changes: {}",
            pg_error_message(&e)
        ))
    })
}

async fn get_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<DatatablePermissionsInfo> {
    require_admin(authed.is_admin, &authed.username)?;
    let datatable = read_datatable_unchecked(&db, &w_id, &datatable_name).await?;
    let permissions = datatable.permissions.unwrap_or_default();
    let default_role = permissions.default_role().to_string();
    Ok(Json(DatatablePermissionsInfo {
        enabled: permissions.enabled,
        default_role,
        roles: permissions
            .roles
            .into_iter()
            .map(|(name, role)| DatatableRoleInfo {
                name,
                tenants: role.tenants,
                pg_rolename: role.pg_rolename,
            })
            .collect(),
    }))
}

/// Refuse a data table operation that would run as a role `authed` may not use.
///
/// The executor enforces this too, so this is not the security boundary — it is
/// what turns "the migration job failed" into an error naming the role and the
/// migration, before anything is pushed or recorded.
pub(crate) async fn ensure_can_use_datatable_role(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    role: Option<&str>,
    authed: &ApiAuthed,
    context: &str,
) -> Result<()> {
    let datatable = read_datatable_unchecked(db, w_id, datatable_name).await?;
    let Some(permissions) = datatable.permissions.filter(|p| p.enabled) else {
        // Unpermissioned: only the built-in role exists, and everyone reaches it.
        return match role {
            Some(role) if role != ADMIN_DATATABLE_ROLE => Err(Error::BadRequest(format!(
                "{context} names role '{role}', but permissions are not enabled on data table '{datatable_name}'"
            ))),
            _ => Ok(()),
        };
    };
    let role_name = role.unwrap_or_else(|| permissions.default_role());
    let entry = permissions.roles.get(role_name).ok_or_else(|| {
        Error::NotFound(format!(
            "{context} names role '{role_name}', which is not defined on data table '{datatable_name}'"
        ))
    })?;
    if !can_use_datatable_role(entry, &authed.to_authed_ref()) {
        return Err(Error::NotAuthorized(format!(
            "{context} runs as role '{role_name}' of data table '{datatable_name}', which you are not allowed to use"
        )));
    }
    Ok(())
}

/// Permissions are turned on where this workspace is the only one reaching the
/// database: no fork above it, none below it, and no other workspace's data
/// table naming the same instance database.
///
/// A fork's data table is either a copy pointing at the database of the workspace
/// it was forked from, where roles created in the fork would hold grants that
/// workspace's own config does not name, or a clone whose whole database the fork
/// can drop, taking the roles with it. In the other direction, a fork made while
/// the data table was unpermissioned carries a verbatim copy of it, and every
/// member of that fork — including members this workspace does not have — would
/// keep reaching the database through the copy's own connection, which owns
/// everything in it. Forks made after the opt-in never receive a permissioned
/// data table (see the strip in the fork creation), so refusing while any exist
/// is what closes the gap. A dev workspace detached from this one keeps such a
/// copy without being a fork any more, which is what the last check is for; it
/// is exact for an instance database, whose only credential holders are the
/// data tables naming it, and meaningless for a resource-backed one, where
/// whoever holds the resource's credentials reaches the database regardless.
///
/// Turning them off is always allowed, or a workspace carrying permissions from
/// before this rule could never be rid of them, and the roles behind them never
/// dropped.
///
/// The save calls this under the settings row lock, which fork creation takes on
/// the parent before copying its settings: a fork mid-creation has either
/// committed, and is listed here, or copies the config after the opt-in landed.
async fn refuse_enabling_permissions_over_shared_access(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    enabled: bool,
) -> Result<()> {
    if !enabled {
        return Ok(());
    }
    if crate::workspaces_extra::workspace_is_fork(db, w_id).await? {
        return Err(Error::BadRequest(
            "Data table permissions cannot be enabled from a fork workspace: a fork's data \
             table points either at the database of the workspace it was forked from, where \
             roles created here would be invisible to that workspace's own configuration, or \
             at a copy the fork can drop. Set them where the data table belongs. Disabling \
             them here is allowed."
                .to_string(),
        ));
    }
    let forks = windmill_common::workspaces::list_fork_descendants(db, w_id).await?;
    if !forks.is_empty() {
        return Err(Error::BadRequest(format!(
            "Data table permissions cannot be enabled while this workspace has forks ({}): a \
             fork holds a copy of the data table pointing at the same database, and its members \
             would keep reaching it through the data table's own connection, as every role at \
             once. Delete the forks first.",
            forks.join(", ")
        )));
    }
    let datatable = read_datatable_unchecked(db, w_id, datatable_name).await?;
    if datatable.database.resource_type == DataTableCatalogResourceType::Instance {
        let others = sqlx::query!(
            r#"SELECT ws.workspace_id AS "workspace_id!", dt.key AS "name!"
               FROM workspace_settings ws
               JOIN workspace w ON w.id = ws.workspace_id AND NOT w.deleted,
               jsonb_each(ws.datatable->'datatables') dt
               WHERE ws.workspace_id <> $1
                 AND dt.value->'database'->>'resource_type' = 'instance'
                 AND dt.value->'database'->>'resource_path' = $2"#,
            w_id,
            &datatable.database.resource_path,
        )
        .fetch_all(db)
        .await?;
        if !others.is_empty() {
            let others: Vec<String> = others
                .into_iter()
                .map(|o| format!("{} (data table '{}')", o.workspace_id, o.name))
                .collect();
            return Err(Error::BadRequest(format!(
                "Data table permissions cannot be enabled while another workspace reaches the \
                 same database: {}. Its members would keep reaching it through that data \
                 table's own connection, as every role at once. Remove that data table first.",
                others.join(", ")
            )));
        }
    }
    Ok(())
}

/// List the roles `authed` may run this data table as. An unpermissioned data
/// table reports `enabled: false` and no roles, so a picker can hide itself.
async fn list_usable_datatable_roles(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<UsableDatatableRoles> {
    let datatable = read_datatable_unchecked(&db, &w_id, &datatable_name).await?;
    let Some(permissions) = datatable.permissions.filter(|p| p.enabled) else {
        return Ok(Json(UsableDatatableRoles {
            enabled: false,
            roles: vec![],
            default_role: ADMIN_DATATABLE_ROLE.to_string(),
        }));
    };
    let authed_ref = authed.to_authed_ref();
    Ok(Json(UsableDatatableRoles {
        enabled: true,
        default_role: permissions.default_role().to_string(),
        roles: permissions
            .roles
            .iter()
            .filter(|(_, role)| can_use_datatable_role(role, &authed_ref))
            .map(|(name, _)| name.clone())
            .collect(),
    }))
}

async fn preview_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<SetDatatablePermissions>,
) -> JsonResult<DatatablePermissionsPreview> {
    require_admin(authed.is_admin, &authed.username)?;
    // Refused here too: the preview connects to the database and reads its roles,
    // and offering a plan that the save will not run is its own kind of wrong.
    refuse_enabling_permissions_over_shared_access(&db, &w_id, &datatable_name, req.enabled)
        .await?;
    let (_client, plan) = build_plan(&db, &w_id, &datatable_name, &req).await?;
    Ok(Json(DatatablePermissionsPreview {
        statements: plan.statements.into_iter().map(|s| s.display).collect(),
        warnings: plan.warnings,
    }))
}

async fn set_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<SetDatatablePermissions>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    // Reading the config, planning against it, running the plan and persisting
    // it are one operation: interleaved with another save, or with the removal
    // of a principal some role names as a tenant, this would store a block it
    // computed before the other committed. The settings row is what everything
    // touching that config takes, so taking it here is what serializes them.
    let mut tx = db.begin().await?;
    windmill_common::workspaces::lock_workspace_settings_unchecked(&mut tx, &w_id).await?;
    refuse_enabling_permissions_over_shared_access(&db, &w_id, &datatable_name, req.enabled)
        .await?;

    // The roles about to be created are handed privileges by this connection,
    // which cannot pass on what it holds without the grant option.
    ensure_instance_db_can_delegate(&db, &w_id, &datatable_name).await;

    // The plan is rebuilt here rather than trusted from the preview: the client
    // never gets to choose what runs against the database.
    let (mut client, plan) = build_plan(&db, &w_id, &datatable_name, &req).await?;

    // Creating and renaming roles is committed before the config: a Windmill-side
    // failure after this point leaves roles the config does not know about, which
    // the next plan adopts (it reads `pg_roles`), whereas the reverse order would
    // leave the config naming roles that were never created. Dropping one has no
    // such way back, so those wait below — except where this save gives the freed
    // name to another role, which only works in one order.
    let keeps_the_name = |statement: &PlannedStatement| {
        statement.drops_role.as_ref().is_some_and(|dropped| {
            plan.permissions
                .roles
                .values()
                .filter_map(|r| r.pg_rolename.as_ref())
                .any(|kept| kept == dropped)
        })
    };
    let (deferred, immediate): (Vec<&PlannedStatement>, Vec<&PlannedStatement>) = plan
        .statements
        .iter()
        .partition(|s| s.drops_role.is_some() && !keeps_the_name(s));
    run_statements(&mut client, &immediate).await?;

    let permissions = serde_json::to_value(&plan.permissions)
        .map_err(|e| Error::internal_err(format!("Failed to serialize permissions: {e}")))?;

    // Written at the permissions path only, so a concurrent edit of the data
    // table's own settings is not clobbered.
    let updated = sqlx::query_scalar!(
        "UPDATE workspace_settings
         SET datatable = jsonb_set(datatable, ARRAY['datatables', $2, 'permissions'], $3)
         WHERE workspace_id = $1 AND datatable->'datatables' ? $2
         RETURNING workspace_id",
        &w_id,
        &datatable_name,
        permissions,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if updated.is_none() {
        return Err(Error::NotFound(format!(
            "Data table '{datatable_name}' not found"
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.set_datatable_permissions",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [
                ("datatable", datatable_name.as_str()),
                ("enabled", if req.enabled { "true" } else { "false" }),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    // What the config no longer names, now that it says so. A failure here is the
    // end of the line for these logins: the config that named them has committed,
    // so no later plan diffs against them and nothing will try again. Say which
    // ones, since dropping them is now a database administrator's job.
    if !deferred.is_empty() {
        drop_roles_the_config_no_longer_names(&db, &w_id, &mut client, &deferred)
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!(
                    "Permissions of data table {datatable_name} were saved, but the Postgres \
                     logins they no longer name could not be removed: {e}. Saving again will not \
                     retry them — the config no longer names them, so they have to be dropped by \
                     hand."
                ))
            })?;
    }

    Ok(format!(
        "Updated permissions of data table {datatable_name}"
    ))
}
