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
//! database — not Windmill — enforces what it may touch. `root` is the exception:
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
use windmill_common::error::{pg_error_message, Error, JsonResult, Result};
use windmill_common::query_builders::{render_db_quoted_identifier, DbType};
use windmill_common::utils::{rd_string, require_admin};
use windmill_common::workspaces::{
    can_use_datatable_role, datatable_pg_role_name, get_datatable_resource_from_db_unchecked,
    DataTable, DataTablePermissions, DataTableRole, DATATABLE_TENANT_WILDCARD, ROOT_DATATABLE_ROLE,
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
    /// Absent for `root`, which reuses the data table's own connection.
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
    /// The role a script gets when it names none. Absent means `root`.
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
struct PlannedStatement {
    sql: String,
    display: String,
}

impl PlannedStatement {
    fn plain(sql: String) -> Self {
        Self { display: sql.clone(), sql }
    }
}

#[derive(Debug)]
struct RolePlan {
    statements: Vec<PlannedStatement>,
    /// The permissions block to persist once the statements have run.
    permissions: DataTablePermissions,
    warnings: Vec<String>,
}

fn quote_ident(ident: &str) -> String {
    render_db_quoted_identifier(ident, DbType::Postgresql)
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn validate_role_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 63
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::BadRequest(format!(
            "Invalid role name '{name}': must be 1-63 characters of letters, digits, '_' or '-'"
        )));
    }
    Ok(())
}

fn validate_tenant(tenant: &str) -> Result<()> {
    if tenant == DATATABLE_TENANT_WILDCARD {
        return Ok(());
    }
    match tenant.split_once('/') {
        Some(("u" | "g" | "f", rest)) if !rest.is_empty() => Ok(()),
        _ => Err(Error::BadRequest(format!(
            "Invalid tenant '{tenant}': expected '*', u/<user>, g/<group> or f/<folder>"
        ))),
    }
}

/// Plan the SQL that takes the data table's Postgres roles from `old` to `req`.
///
/// `existing_pg_roles` is the set of role names that actually exist in the
/// cluster, so the plan reconciles against reality rather than against what the
/// config claims: a role the config lost track of is still dropped, and one that
/// somehow already exists has its password reset instead of failing the CREATE.
fn plan_role_changes(
    w_id: &str,
    datatable: &str,
    dbname: &str,
    root_pg_role: &str,
    old: Option<&DataTablePermissions>,
    req: &SetDatatablePermissions,
    existing_pg_roles: &HashSet<String>,
) -> Result<RolePlan> {
    // A disabled data table has no Postgres roles, whatever its config says, so
    // re-enabling always plans every role as a creation.
    let old_roles: BTreeMap<String, DataTableRole> = match old {
        Some(p) if p.enabled => p.roles.clone(),
        _ => BTreeMap::new(),
    };

    let mut statements = Vec::new();
    let mut warnings = Vec::new();

    let mut dropped_pg_roles: HashSet<String> = HashSet::new();
    let drop_role =
        |statements: &mut Vec<PlannedStatement>, dropped: &mut HashSet<String>, pg_role: &str| {
            if !existing_pg_roles.contains(pg_role) {
                return;
            }
            dropped.insert(pg_role.to_string());
            let q = quote_ident(pg_role);
            // Give the objects back to root before dropping, else the DROP fails on
            // anything the role still owns. DROP OWNED then clears what is left:
            // privileges granted to it and its default-privilege entries.
            statements.push(PlannedStatement::plain(format!(
                "REASSIGN OWNED BY {q} TO {};",
                quote_ident(root_pg_role)
            )));
            statements.push(PlannedStatement::plain(format!("DROP OWNED BY {q};")));
            statements.push(PlannedStatement::plain(format!("DROP ROLE {q};")));
        };

    if !req.enabled {
        for (name, role) in old_roles.iter() {
            if name == ROOT_DATATABLE_ROLE {
                continue;
            }
            if let Some(pg_role) = role.pg_rolename.as_deref() {
                drop_role(&mut statements, &mut dropped_pg_roles, pg_role);
            }
        }
        // Opting out drops the roles, so keeping their definitions would leave
        // the config describing roles that no longer exist.
        return Ok(RolePlan {
            statements,
            permissions: DataTablePermissions {
                enabled: false,
                roles: BTreeMap::new(),
                default_role: None,
            },
            warnings,
        });
    }

    let mut requested: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for role in req.roles.iter() {
        validate_role_name(&role.name)?;
        for tenant in role.tenants.iter() {
            validate_tenant(tenant)?;
        }
        if requested
            .insert(role.name.clone(), role.tenants.clone())
            .is_some()
        {
            return Err(Error::BadRequest(format!(
                "Duplicate role name '{}'",
                role.name
            )));
        }
    }
    if !requested.contains_key(ROOT_DATATABLE_ROLE) {
        return Err(Error::BadRequest(format!(
            "The '{ROOT_DATATABLE_ROLE}' role cannot be removed"
        )));
    }

    // A default naming a role that is not being saved would leave every script
    // that names no role failing to resolve.
    let default_role = req.default_role.as_deref().unwrap_or(ROOT_DATATABLE_ROLE);
    if !requested.contains_key(default_role) {
        return Err(Error::BadRequest(format!(
            "Default role '{default_role}' is not one of the submitted roles"
        )));
    }

    // new name -> old name, so a renamed role keeps its Postgres role (and the
    // grants on it) instead of being planned as a drop plus a create.
    let mut rename_src: BTreeMap<&str, &str> = BTreeMap::new();
    for r in req.renames.iter() {
        validate_role_name(&r.from)?;
        validate_role_name(&r.to)?;
        if r.from == ROOT_DATATABLE_ROLE || r.to == ROOT_DATATABLE_ROLE {
            return Err(Error::BadRequest(format!(
                "The '{ROOT_DATATABLE_ROLE}' role cannot be renamed"
            )));
        }
        if r.from == r.to {
            continue;
        }
        if !old_roles.contains_key(&r.from) {
            return Err(Error::BadRequest(format!(
                "Cannot rename unknown role '{}'",
                r.from
            )));
        }
        if !requested.contains_key(&r.to) {
            return Err(Error::BadRequest(format!(
                "Renamed role '{}' is missing from the submitted roles",
                r.to
            )));
        }
        if rename_src.insert(&r.to, &r.from).is_some() {
            return Err(Error::BadRequest(format!(
                "Two roles were renamed to '{}'",
                r.to
            )));
        }
    }

    let renamed_away: HashSet<&str> = rename_src.values().copied().collect();

    // Which old role each requested role continues: its rename source if it was
    // renamed, else the old role of the same name. Matching on the requested name
    // alone would spare an old role that a *different* role is being renamed onto,
    // leaving the name occupied when the rename runs.
    let claimed: HashSet<&str> = requested
        .keys()
        .map(|name| {
            rename_src
                .get(name.as_str())
                .copied()
                .unwrap_or(name.as_str())
        })
        .collect();

    // Drops come first so a name freed in this save can be reused by a rename or
    // a create in the same save: the other order aborts on "role already exists".
    for (name, role) in old_roles.iter() {
        if name == ROOT_DATATABLE_ROLE || claimed.contains(name.as_str()) {
            continue;
        }
        if let Some(pg_role) = role.pg_rolename.as_deref() {
            drop_role(&mut statements, &mut dropped_pg_roles, pg_role);
        }
    }

    let mut roles: BTreeMap<String, DataTableRole> = BTreeMap::new();
    // Renames run before creates so a name freed by a rename can be taken by a new
    // role in the same save; both run after the drops, for the same reason.
    // Postgres names this save frees, by dropping the role or renaming it away.
    // A name in here is occupied now but free by the time the creates run, so a new
    // role may take it — treating it as taken would silently reuse the old role
    // (and reset its password) instead of creating the new one.
    let vacated_pg_roles: HashSet<String> = dropped_pg_roles
        .iter()
        .cloned()
        .chain(rename_src.iter().filter_map(|(to, from)| {
            let old_pg = old_roles.get(*from)?.pg_rolename.clone()?;
            let new_pg = datatable_pg_role_name(w_id, datatable, to);
            (old_pg != new_pg).then_some(old_pg)
        }))
        .collect();

    let mut pending_renames: Vec<(String, String, String)> = Vec::new();
    let mut creates_sql: Vec<PlannedStatement> = Vec::new();

    for (name, tenants) in requested.iter() {
        if name == ROOT_DATATABLE_ROLE {
            roles.insert(
                name.clone(),
                DataTableRole { pg_rolename: None, pg_password: None, tenants: tenants.clone() },
            );
            continue;
        }

        let pg_rolename = datatable_pg_role_name(w_id, datatable, name);
        let previous = rename_src
            .get(name.as_str())
            .and_then(|from| old_roles.get(*from).map(|r| (*from, r)))
            .or_else(|| {
                // An old role of the same name that is being renamed away is not
                // this role's predecessor: the name is being freed and taken by a
                // brand new role, which has to be created rather than inherited.
                (!renamed_away.contains(name.as_str()))
                    .then(|| old_roles.get(name).map(|r| (name.as_str(), r)))
                    .flatten()
            });

        match previous {
            Some((from, old_role)) if old_role.pg_rolename.is_some() => {
                let old_pg = old_role.pg_rolename.clone().unwrap();
                let password = old_role
                    .pg_password
                    .clone()
                    .unwrap_or_else(|| rd_string(32));
                if old_pg != pg_rolename {
                    if existing_pg_roles.contains(&old_pg) {
                        pending_renames.push((
                            old_pg.clone(),
                            pg_rolename.clone(),
                            password.clone(),
                        ));
                    } else {
                        warnings.push(format!(
                            "Role '{from}' was expected to exist in the database as '{old_pg}' but does not; it will be created as '{pg_rolename}'."
                        ));
                        creates_sql.push(create_role_statement(&pg_rolename, &password));
                        creates_sql.push(grant_connect_statement(&pg_rolename, dbname));
                    }
                }
                roles.insert(
                    name.clone(),
                    DataTableRole {
                        pg_rolename: Some(pg_rolename),
                        pg_password: Some(password),
                        tenants: tenants.clone(),
                    },
                );
            }
            _ => {
                let password = rd_string(32);
                if existing_pg_roles.contains(&pg_rolename)
                    && !vacated_pg_roles.contains(&pg_rolename)
                {
                    warnings.push(format!(
                        "A Postgres role named '{pg_rolename}' already exists; it will be reused and its password reset."
                    ));
                    creates_sql.push(PlannedStatement {
                        sql: format!(
                            "ALTER ROLE {} WITH LOGIN PASSWORD {};",
                            quote_ident(&pg_rolename),
                            quote_literal(&password)
                        ),
                        display: format!(
                            "ALTER ROLE {} WITH LOGIN PASSWORD '<generated>';",
                            quote_ident(&pg_rolename)
                        ),
                    });
                } else {
                    creates_sql.push(create_role_statement(&pg_rolename, &password));
                }
                creates_sql.push(grant_connect_statement(&pg_rolename, dbname));
                roles.insert(
                    name.clone(),
                    DataTableRole {
                        pg_rolename: Some(pg_rolename),
                        pg_password: Some(password),
                        tenants: tenants.clone(),
                    },
                );
            }
        }
    }

    statements.extend(order_renames(
        pending_renames,
        existing_pg_roles,
        &dropped_pg_roles,
    )?);
    statements.extend(creates_sql);

    Ok(RolePlan {
        statements,
        permissions: DataTablePermissions {
            enabled: true,
            roles,
            default_role: (default_role != ROOT_DATATABLE_ROLE).then(|| default_role.to_string()),
        },
        warnings,
    })
}

/// Order the planned renames so each runs only once its target name is free, and
/// emit them.
///
/// A rename whose target is still occupied aborts the whole transaction with a
/// bare "role already exists", so the ordering is part of the plan rather than
/// something the admin has to discover: `a -> b` where the old `b` is being
/// dropped or itself renamed away is a legitimate save, it just has to run in the
/// right order. A true cycle (swapping two names) cannot be ordered at all and is
/// reported as such instead of failing in Postgres.
fn order_renames(
    renames: Vec<(String, String, String)>,
    existing_pg_roles: &HashSet<String>,
    dropped_pg_roles: &HashSet<String>,
) -> Result<Vec<PlannedStatement>> {
    let mut pending = renames;
    let mut vacated: HashSet<String> = dropped_pg_roles.clone();
    let mut statements = Vec::new();

    while !pending.is_empty() {
        let ready = pending
            .iter()
            .position(|(_, to, _)| !existing_pg_roles.contains(to) || vacated.contains(to));
        let Some(i) = ready else {
            let blocked: Vec<&str> = pending.iter().map(|(from, _, _)| from.as_str()).collect();
            return Err(Error::BadRequest(format!(
                "Renaming {} would swap Postgres role names, which cannot be done in one \
                 transaction. Apply the renames in two separate saves.",
                blocked.join(", ")
            )));
        };
        let (from, to, password) = pending.remove(i);
        statements.push(PlannedStatement::plain(format!(
            "ALTER ROLE {} RENAME TO {};",
            quote_ident(&from),
            quote_ident(&to)
        )));
        // RENAME discards an md5-hashed password, so the stored one would stop
        // working; re-setting it is a no-op under scram-sha-256 and a repair under
        // md5.
        statements.push(PlannedStatement {
            sql: format!(
                "ALTER ROLE {} PASSWORD {};",
                quote_ident(&to),
                quote_literal(&password)
            ),
            display: format!("ALTER ROLE {} PASSWORD '<unchanged>';", quote_ident(&to)),
        });
        vacated.insert(from);
    }
    Ok(statements)
}

fn create_role_statement(pg_rolename: &str, password: &str) -> PlannedStatement {
    PlannedStatement {
        sql: format!(
            "CREATE ROLE {} LOGIN PASSWORD {};",
            quote_ident(pg_rolename),
            quote_literal(password)
        ),
        display: format!(
            "CREATE ROLE {} LOGIN PASSWORD '<generated>';",
            quote_ident(pg_rolename)
        ),
    }
}

/// New roles are created bare — privileges are granted additively afterwards —
/// but they do need to reach the database. CONNECT is usually already theirs
/// through PUBLIC, and the data table's own role often cannot grant it (it is
/// rarely the database owner), so the grant is conditional rather than
/// unconditional: it stays a no-op in the common case instead of failing the
/// whole transaction.
fn grant_connect_statement(pg_rolename: &str, dbname: &str) -> PlannedStatement {
    let role_lit = quote_literal(pg_rolename);
    let db_lit = quote_literal(dbname);
    // The names are passed to `format(%I)` rather than interpolated as quoted
    // identifiers: this identifier sits inside a single-quoted EXECUTE string, so
    // a `'` in the database name — which comes from a user-editable postgres
    // resource — would otherwise close the literal and run as plpgsql of its own.
    PlannedStatement::plain(format!(
        "DO $$ BEGIN\n  IF NOT has_database_privilege({role_lit}, {db_lit}, 'CONNECT') THEN\n    EXECUTE format('GRANT CONNECT ON DATABASE %I TO %I', {db_lit}, {role_lit});\n  END IF;\nEND $$;"
    ))
}

async fn read_datatable(db: &DB, w_id: &str, datatable_name: &str) -> Result<DataTable> {
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

/// Connect to the data table's own database as `root` and report the identity
/// the plan has to be built against: the database name, the role that owns the
/// existing objects, and the roles that actually exist in the cluster.
async fn connect_as_root(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<(tokio_postgres::Client, String, String, HashSet<String>)> {
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

    let root_pg_role: String = client
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

    Ok((client, dbname, root_pg_role, existing_pg_roles))
}

async fn build_plan(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    req: &SetDatatablePermissions,
) -> Result<(tokio_postgres::Client, RolePlan)> {
    let datatable = read_datatable(db, w_id, datatable_name).await?;
    let (client, dbname, root_pg_role, existing_pg_roles) =
        connect_as_root(db, w_id, datatable_name).await?;
    let plan = plan_role_changes(
        w_id,
        datatable_name,
        &dbname,
        &root_pg_role,
        datatable.permissions.as_ref(),
        req,
        &existing_pg_roles,
    )?;
    Ok((client, plan))
}

/// Drop the Postgres roles a data table leaves behind when it is removed from the
/// workspace config, giving its objects back to root first.
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
            default_role: ROOT_DATATABLE_ROLE.to_string(),
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

    let mut tx = db.begin().await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const W_ID: &str = "acme";
    const DT: &str = "main";
    const DB_NAME: &str = "wm_acme_main";
    const ROOT_PG: &str = "custom_instance_user";

    fn role(name: &str, tenants: &[&str]) -> DatatableRoleInfo {
        DatatableRoleInfo {
            name: name.to_string(),
            tenants: tenants.iter().map(|t| t.to_string()).collect(),
            pg_rolename: None,
        }
    }

    fn enabled_with(roles: &[&str]) -> DataTablePermissions {
        let mut map = BTreeMap::new();
        map.insert(ROOT_DATATABLE_ROLE.to_string(), DataTableRole::default());
        for name in roles {
            map.insert(
                name.to_string(),
                DataTableRole {
                    pg_rolename: Some(datatable_pg_role_name(W_ID, DT, name)),
                    pg_password: Some("kept-password".to_string()),
                    tenants: vec![],
                },
            );
        }
        DataTablePermissions { enabled: true, roles: map, default_role: None }
    }

    fn plan(
        old: Option<&DataTablePermissions>,
        req: &SetDatatablePermissions,
        existing: &[&str],
    ) -> Result<RolePlan> {
        plan_role_changes(
            W_ID,
            DT,
            DB_NAME,
            ROOT_PG,
            old,
            req,
            &existing.iter().map(|r| r.to_string()).collect(),
        )
    }

    fn sql(plan: &RolePlan) -> Vec<&str> {
        plan.statements.iter().map(|s| s.sql.as_str()).collect()
    }

    #[test]
    fn adding_a_role_creates_it_and_stores_its_credentials() {
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[]), role("analyst", &["u/alice", "g/devs"])],
            default_role: None,
            renames: vec![],
        };
        let plan = plan(None, &req, &[]).unwrap();

        let pg_role = datatable_pg_role_name(W_ID, DT, "analyst");
        assert!(sql(&plan)[0].starts_with(&format!("CREATE ROLE \"{pg_role}\" LOGIN PASSWORD ")));
        assert!(sql(&plan)[1].contains("has_database_privilege"));

        let stored = &plan.permissions.roles["analyst"];
        assert_eq!(stored.pg_rolename.as_deref(), Some(pg_role.as_str()));
        assert!(stored.pg_password.as_ref().is_some_and(|p| p.len() == 32));
        assert_eq!(stored.tenants, vec!["u/alice", "g/devs"]);
        // root reuses the data table's own connection, so it never gets one.
        assert!(plan.permissions.roles[ROOT_DATATABLE_ROLE]
            .pg_rolename
            .is_none());
    }

    #[test]
    fn renaming_a_role_keeps_its_postgres_role_and_password() {
        let old = enabled_with(&["analyst"]);
        let old_pg = datatable_pg_role_name(W_ID, DT, "analyst");
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[]), role("reader", &[])],
            default_role: None,
            renames: vec![DatatableRoleRename {
                from: "analyst".to_string(),
                to: "reader".to_string(),
            }],
        };
        let plan = plan(Some(&old), &req, &[old_pg.as_str()]).unwrap();

        let new_pg = datatable_pg_role_name(W_ID, DT, "reader");
        assert_eq!(
            sql(&plan)[0],
            format!("ALTER ROLE \"{old_pg}\" RENAME TO \"{new_pg}\";")
        );
        // The rename must not be planned as a drop plus a create: that would
        // silently discard every grant the role had accumulated.
        assert!(!sql(&plan).iter().any(|s| s.contains("DROP ROLE")));
        assert!(!sql(&plan).iter().any(|s| s.contains("CREATE ROLE")));
        assert_eq!(
            plan.permissions.roles["reader"].pg_password.as_deref(),
            Some("kept-password")
        );
    }

    #[test]
    fn removing_a_role_gives_its_objects_back_to_root_before_dropping_it() {
        let old = enabled_with(&["analyst"]);
        let pg_role = datatable_pg_role_name(W_ID, DT, "analyst");
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[])],
            default_role: None,
            renames: vec![],
        };
        let plan = plan(Some(&old), &req, &[pg_role.as_str()]).unwrap();

        assert_eq!(
            sql(&plan),
            vec![
                format!("REASSIGN OWNED BY \"{pg_role}\" TO \"{ROOT_PG}\";"),
                format!("DROP OWNED BY \"{pg_role}\";"),
                format!("DROP ROLE \"{pg_role}\";"),
            ]
        );
        assert!(!plan.permissions.roles.contains_key("analyst"));
    }

    #[test]
    fn opting_out_drops_every_role_and_clears_the_definitions() {
        let old = enabled_with(&["analyst", "writer"]);
        let existing: Vec<String> = ["analyst", "writer"]
            .iter()
            .map(|r| datatable_pg_role_name(W_ID, DT, r))
            .collect();
        let req = SetDatatablePermissions {
            enabled: false,
            roles: vec![],
            default_role: None,
            renames: vec![],
        };
        let plan = plan(
            Some(&old),
            &req,
            &existing.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .unwrap();

        assert_eq!(
            sql(&plan)
                .iter()
                .filter(|s| s.starts_with("DROP ROLE"))
                .count(),
            2
        );
        assert!(!plan.permissions.enabled);
        assert!(plan.permissions.roles.is_empty());
    }

    /// A name freed by a delete must be reusable by a rename in the same save,
    /// which only holds if the drops are planned first.
    #[test]
    fn a_name_freed_in_the_same_save_can_be_reused() {
        let old = enabled_with(&["analyst", "reader"]);
        let existing: Vec<String> = ["analyst", "reader"]
            .iter()
            .map(|r| datatable_pg_role_name(W_ID, DT, r))
            .collect();
        // Delete `reader`, rename `analyst` into its place.
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[]), role("reader", &[])],
            default_role: None,
            renames: vec![DatatableRoleRename {
                from: "analyst".to_string(),
                to: "reader".to_string(),
            }],
        };
        let plan = plan(
            Some(&old),
            &req,
            &existing.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .unwrap();

        let statements = sql(&plan);
        let drop_at = statements
            .iter()
            .position(|s| s.starts_with("DROP ROLE"))
            .expect("the freed role is dropped");
        let rename_at = statements
            .iter()
            .position(|s| s.starts_with("ALTER ROLE") && s.contains("RENAME TO"))
            .expect("the surviving role is renamed");
        assert!(
            drop_at < rename_at,
            "drop must precede the rename that reuses the name: {statements:?}"
        );
    }

    /// Freeing a name by renaming its holder away and giving it to a brand new
    /// role in the same save: the new role must actually be created, and only
    /// after the rename has vacated the name.
    #[test]
    fn a_name_freed_by_a_rename_can_be_taken_by_a_new_role() {
        let old = enabled_with(&["analyst"]);
        let old_pg = datatable_pg_role_name(W_ID, DT, "analyst");
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[]), role("reader", &[]), role("analyst", &[])],
            default_role: None,
            renames: vec![DatatableRoleRename {
                from: "analyst".to_string(),
                to: "reader".to_string(),
            }],
        };
        let plan = plan(Some(&old), &req, &[old_pg.as_str()]).unwrap();

        let statements = sql(&plan);
        let rename_at = statements
            .iter()
            .position(|s| s.starts_with("ALTER ROLE") && s.contains("RENAME TO"))
            .expect("the old role is renamed away");
        let create_at = statements
            .iter()
            .position(|s| s.starts_with("CREATE ROLE"))
            .expect("the reused name is created fresh, not silently inherited");
        assert!(
            rename_at < create_at,
            "the rename must vacate the name before the create takes it: {statements:?}"
        );
        // The new role is a different Postgres role from the one that was renamed.
        assert_eq!(
            plan.permissions.roles["analyst"].pg_rolename.as_deref(),
            Some(old_pg.as_str())
        );
        assert_ne!(
            plan.permissions.roles["reader"].pg_rolename.as_deref(),
            Some(old_pg.as_str())
        );
    }

    /// Swapping two role names cannot be ordered into a working sequence, so it
    /// is refused with an actionable message rather than aborting in Postgres.
    #[test]
    fn swapping_two_role_names_is_refused_with_an_explanation() {
        let old = enabled_with(&["a", "b"]);
        let existing: Vec<String> = ["a", "b"]
            .iter()
            .map(|r| datatable_pg_role_name(W_ID, DT, r))
            .collect();
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[]), role("a", &[]), role("b", &[])],
            default_role: None,
            renames: vec![
                DatatableRoleRename { from: "a".to_string(), to: "b".to_string() },
                DatatableRoleRename { from: "b".to_string(), to: "a".to_string() },
            ],
        };
        let err = plan(
            Some(&old),
            &req,
            &existing.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("separate saves"), "{err}");
    }

    #[test]
    fn a_role_the_config_lost_track_of_is_not_dropped() {
        let old = enabled_with(&["analyst"]);
        let req = SetDatatablePermissions {
            enabled: false,
            roles: vec![],
            default_role: None,
            renames: vec![],
        };
        // The Postgres role is already gone, so planning its drop would fail the
        // whole transaction and wedge the opt-out.
        let plan = plan(Some(&old), &req, &[]).unwrap();
        assert!(sql(&plan).is_empty());
    }

    #[test]
    fn the_default_role_must_be_one_of_the_saved_roles() {
        let mut req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &[]), role("analyst", &[])],
            default_role: Some("analyst".to_string()),
            renames: vec![],
        };
        let planned = plan(None, &req, &[]).unwrap();
        assert_eq!(planned.permissions.default_role(), "analyst");

        // A default naming a role that is not being saved would leave every
        // script that names no role failing to resolve.
        req.default_role = Some("ghost".to_string());
        assert!(plan(None, &req, &[]).is_err());

        // Absent means root, and is stored as absent rather than spelled out.
        req.default_role = None;
        let planned = plan(None, &req, &[]).unwrap();
        assert_eq!(planned.permissions.default_role(), ROOT_DATATABLE_ROLE);
        assert!(planned.permissions.default_role.is_none());
    }

    #[test]
    fn root_cannot_be_dropped_or_renamed() {
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("analyst", &[])],
            default_role: None,
            renames: vec![],
        };
        assert!(plan(None, &req, &[]).is_err());

        let old = enabled_with(&[]);
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("owner", &[])],
            default_role: None,
            renames: vec![DatatableRoleRename {
                from: ROOT_DATATABLE_ROLE.to_string(),
                to: "owner".to_string(),
            }],
        };
        assert!(plan(Some(&old), &req, &[]).is_err());
    }

    #[test]
    fn invalid_role_names_and_tenants_are_rejected() {
        for bad_role in ["", "bad name", "a;b", "drop\"role"] {
            let req = SetDatatablePermissions {
                enabled: true,
                roles: vec![role("root", &[]), role(bad_role, &[])],
                default_role: None,
                renames: vec![],
            };
            assert!(
                plan(None, &req, &[]).is_err(),
                "{bad_role} should be rejected"
            );
        }
        // The wildcard is the one tenant with no prefix.
        let req = SetDatatablePermissions {
            enabled: true,
            roles: vec![role("root", &["*"])],
            default_role: None,
            renames: vec![],
        };
        assert!(plan(None, &req, &[]).is_ok());

        for bad_tenant in ["alice", "x/alice", "u/", "", "*/alice", "**"] {
            let req = SetDatatablePermissions {
                enabled: true,
                roles: vec![role("root", &[bad_tenant])],
                default_role: None,
                renames: vec![],
            };
            assert!(
                plan(None, &req, &[]).is_err(),
                "{bad_tenant} should be rejected"
            );
        }
    }
}
