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
use windmill_common::ensure_instance_db_grant_options;
use windmill_common::error::{pg_error_message, Error, JsonResult, Result};
use windmill_common::query_builders::{render_db_quoted_identifier, DbType};
use windmill_common::utils::require_admin;
use windmill_common::workspaces::{
    can_use_datatable_role, get_datatable_resource_from_db_unchecked, DataTable,
    DataTableCatalogResourceType, DataTablePermissions, ADMIN_DATATABLE_ROLE,
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
}

impl PlannedStatement {
    /// Only the planner builds these, and that is the enterprise module.
    #[cfg_attr(
        not(all(feature = "private", feature = "enterprise")),
        allow(dead_code)
    )]
    pub(crate) fn plain(sql: String) -> Self {
        Self { display: sql.clone(), sql }
    }
}

#[derive(Debug)]
pub(crate) struct RolePlan {
    pub(crate) statements: Vec<PlannedStatement>,
    /// The permissions block to persist once the statements have run.
    pub(crate) permissions: DataTablePermissions,
    pub(crate) warnings: Vec<String>,
}

/// Serialize everything that changes one data table's roles or their access.
///
/// Both the role save and an ACL apply read the config, plan against it and run
/// the result on the data table's own database; two of them at once plan against
/// a state the other is leaving. Keyed per data table, and released when the
/// caller's transaction ends.
pub(crate) async fn lock_datatable_permissions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    datatable_name: &str,
) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext('datatable_permissions:' || $1), hashtext($2))",
        w_id,
        datatable_name,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
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

pub(crate) async fn read_datatable(db: &DB, w_id: &str, datatable_name: &str) -> Result<DataTable> {
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
    let Ok(datatable) = read_datatable(db, w_id, datatable_name).await else {
        return;
    };
    if datatable.database.resource_type != DataTableCatalogResourceType::Instance {
        return;
    }
    if let Err(e) = ensure_instance_db_grant_options(db, &datatable.database.resource_path).await {
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
    pub(crate) admin_pg_role: String,
    pub(crate) existing_pg_roles: HashSet<String>,
    /// Whether `PUBLIC` holds CREATE on schema `public`, i.e. every role in this
    /// database — including the ones created here — can make objects in it.
    pub(crate) public_schema_is_open: bool,
    /// The default-privilege rules already in force for this data table's roles.
    pub(crate) default_acl_rules: Vec<DefaultAclRule>,
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
pub(crate) async fn connect_as_admin(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<(tokio_postgres::Client, AdminConnection)> {
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
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

    let existing_pg_roles = client
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
        read_datatable(db, w_id, datatable_name)
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
            admin_pg_role,
            existing_pg_roles,
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
    let datatable = read_datatable(db, w_id, datatable_name).await?;
    let (client, conn) = connect_as_admin(db, w_id, datatable_name).await?;
    let plan = crate::datatable_permissions_oss::plan_role_changes(
        w_id,
        datatable_name,
        &conn.dbname,
        &conn.admin_pg_role,
        datatable.permissions.as_ref(),
        req,
        &conn.existing_pg_roles,
        conn.public_schema_is_open,
        &conn.default_acl_rules,
    )?;
    Ok((client, plan))
}

/// Drop the Postgres roles a data table leaves behind when it is removed from the
/// workspace config, giving its objects back to admin first.
///
/// Best-effort: a data table whose database is already unreachable must still be
/// removable from the config, so a failure is logged rather than propagated. Any
/// role that survives is reconciled by the next plan, which reads `pg_roles`.
pub(crate) async fn drop_roles_of_deleted_datatable(db: &DB, w_id: &str, datatable_name: &str) {
    let req = SetDatatablePermissions {
        enabled: false,
        roles: vec![],
        default_role: None,
        renames: vec![],
    };
    let res = async {
        let datatable = read_datatable(db, w_id, datatable_name).await?;
        if !datatable
            .permissions
            .as_ref()
            .is_some_and(|p| p.enabled && p.roles.len() > 1)
        {
            return Ok(());
        }
        let (mut client, plan) = build_plan(db, w_id, datatable_name, &req).await?;
        run_statements(&mut client, &plan).await
    }
    .await;
    if let Err(e) = res {
        tracing::error!(
            "Could not drop the Postgres roles of deleted data table {datatable_name} in {w_id}: {e:#}"
        );
    }
}

/// Run a plan's statements in a single transaction, so a failure part-way leaves
/// the database exactly as it was.
async fn run_statements(client: &mut tokio_postgres::Client, plan: &RolePlan) -> Result<()> {
    let pg_tx = client.transaction().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to open a transaction on the data table: {}",
            pg_error_message(&e)
        ))
    })?;
    for statement in plan.statements.iter() {
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
    let datatable = read_datatable(&db, &w_id, &datatable_name).await?;
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
    let datatable = read_datatable(db, w_id, datatable_name).await?;
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

/// List the roles `authed` may run this data table as. An unpermissioned data
/// table reports `enabled: false` and no roles, so a picker can hide itself.
async fn list_usable_datatable_roles(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<UsableDatatableRoles> {
    let datatable = read_datatable(&db, &w_id, &datatable_name).await?;
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
    // it are one operation: two saves of the same data table interleaved would
    // each plan against the state the other is leaving, and the one that
    // persists last would store roles the other already dropped. Held to commit,
    // so the whole sequence below is inside it.
    let mut tx = db.begin().await?;
    lock_datatable_permissions(&mut tx, &w_id, &datatable_name).await?;

    // The roles about to be created are handed privileges by this connection,
    // which cannot pass on what it holds without the grant option.
    ensure_instance_db_can_delegate(&db, &w_id, &datatable_name).await;

    // The plan is rebuilt here rather than trusted from the preview: the client
    // never gets to choose what runs against the database.
    let (mut client, plan) = build_plan(&db, &w_id, &datatable_name, &req).await?;

    // The roles are committed before the config: a Windmill-side failure after
    // this point leaves roles the config does not know about, which the next plan
    // reconciles (it reads `pg_roles`), whereas the reverse order would leave the
    // config naming roles that were never created.
    run_statements(&mut client, &plan).await?;

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

    Ok(format!(
        "Updated permissions of data table {datatable_name}"
    ))
}
