/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Who may connect to a data table as which role.
//!
//! The decision lives on the data table entry of the workspace that governs it, which is not
//! necessarily the workspace asking: a fork's entry points at its parent's, and everything here
//! resolves through that pointer first. Nothing in this module runs SQL against the data table —
//! a save is tenant lists and a default, and the Postgres roles themselves are the instance
//! catalog's business.

use std::collections::BTreeMap;

use axum::{
    extract::{Extension, Path},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::datatable_roles::{read_role_catalog, ADMIN_DATATABLE_ROLE};
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::workspaces::{
    can_use_datatable_role_in_governing_workspace, resolve_governing_datatable,
    DataTableCatalogResourceType, DataTablePermissions, DataTableRoleTenants, DatatableAccess,
    GoverningDatatable, DATATABLE_TENANT_WILDCARD,
};
use windmill_common::DB;

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/datatable_permissions/{datatable_name}",
            get(get_datatable_permissions).post(set_datatable_permissions),
        )
        .route(
            "/datatable_usable_roles/{datatable_name}",
            get(list_usable_datatable_roles),
        )
}

/// One row of the permissions drawer: an instance role (or the reserved `admin`) and who may
/// connect as it here.
#[derive(Serialize, Deserialize, Debug)]
pub struct DatatableRoleTenantsInfo {
    /// The instance catalog id, or `admin`.
    pub id: String,
    /// The role's current name, for display. Absent when the catalog no longer has the id — a
    /// role deleted out from under this data table, which the drawer shows so it can be removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub tenants: Vec<String>,
}

#[derive(Serialize)]
struct DatatablePermissionsInfo {
    /// Whether this data table can be put under roles at all — only one on the instance database
    /// can, since a role is a login on that cluster.
    supported: bool,
    /// Whether the data table is under roles at all.
    permissioned: bool,
    default_role: String,
    roles: Vec<DatatableRoleTenantsInfo>,
    /// The workspace whose entry this is, when it is not the one asking.
    #[serde(skip_serializing_if = "Option::is_none")]
    governing_workspace_id: Option<String>,
    /// Whether this caller may save. False from a fork, and for a non-admin.
    editable: bool,
    /// Every instance role the instance defines, to pick from.
    available_roles: Vec<AvailableRole>,
    /// Other workspaces whose own entry reaches the same database without being governed by this
    /// one — a legacy fork's copy, or a second entry a superadmin pointed here. They keep their own
    /// access, so a save here does not reach them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ungoverned_reachers: Vec<UngovernedReacher>,
}

#[derive(Serialize)]
struct AvailableRole {
    id: String,
    name: String,
    enabled: bool,
}

#[derive(Serialize)]
struct UngovernedReacher {
    workspace_id: String,
    datatable: String,
}

#[derive(Deserialize)]
pub struct SetDatatablePermissions {
    /// False clears the block: the data table goes back to everyone connecting as `admin`.
    pub permissioned: bool,
    #[serde(default)]
    pub default_role: Option<String>,
    #[serde(default)]
    pub roles: Vec<DatatableRoleTenantsInfo>,
}

#[derive(Serialize)]
struct UsableDatatableRoles {
    permissioned: bool,
    /// Names, not ids: this is what a caller writes in `-- role <name>`.
    roles: Vec<String>,
    default_role: String,
}

/// Administering a data table — its permissions, its migrations that declare no role, its exports
/// — is for the admins of the workspace that governs it. A fork can use the data table; it never
/// administers it.
pub(crate) async fn ensure_governs_datatable(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    governing: &GoverningDatatable,
) -> Result<()> {
    if governing.workspace_id == w_id && authed.is_admin {
        return Ok(());
    }
    if require_super_admin(db, authed).await.is_ok() {
        return Ok(());
    }
    Err(Error::NotAuthorized(format!(
        "Data table '{}' is governed by workspace '{}'; this is for its admins.",
        governing.name, governing.workspace_id
    )))
}

/// Refuse a caller that no tenant of this data table covers.
///
/// The bookkeeping endpoints below open the data table's `admin` connection to read or create
/// `_wm_migrations` before they know which migration will run — so without this, someone covered
/// by no role at all can still force admin-backed reads and writes on a database they may not
/// touch. It asks only "may you reach this data table as anything"; which role a given migration
/// runs as is still decided per migration, and by the executor after that.
pub(crate) async fn ensure_reaches_datatable(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    authed: &ApiAuthed,
) -> Result<()> {
    let governing = resolve_governing_datatable(db, w_id, datatable_name).await?;
    let Some(permissions) = governing.datatable.permissions.as_ref() else {
        return Ok(());
    };
    let catalog = read_role_catalog(db).await?;
    let access = DatatableAccess::Authed(authed.to_authed_ref());
    for (id, tenants) in &permissions.roles {
        // A role the instance no longer defines, or has disabled, cannot be connected as, so being
        // tenanted into it is not reach.
        if id != ADMIN_DATATABLE_ROLE && !catalog.get(id).is_some_and(|r| r.enabled) {
            continue;
        }
        if can_use_datatable_role_in_governing_workspace(
            db,
            &governing.workspace_id,
            w_id,
            tenants,
            &access,
        )
        .await?
        {
            return Ok(());
        }
    }
    Err(Error::NotAuthorized(format!(
        "Not allowed to use data table '{datatable_name}': no role of it covers you"
    )))
}

fn validate_tenant(tenant: &str) -> Result<()> {
    if tenant == DATATABLE_TENANT_WILDCARD {
        return Ok(());
    }
    match tenant.split_once('/') {
        Some(("u" | "g" | "f", rest)) if !rest.is_empty() => Ok(()),
        _ => Err(Error::BadRequest(format!(
            "Invalid tenant '{tenant}': expected 'u/<user>', 'g/<group>', 'f/<folder>' or '*'"
        ))),
    }
}

/// Entries in other workspaces that reach the same instance database without pointing at this one.
///
/// They exist by construction and are allowed: a fork created before data table roles holds a
/// literal copy of its parent's entry, and a superadmin can point a second workspace at any
/// instance database. Each governs its own access, so a save here leaves them untouched — which is
/// exactly why they are worth naming at the moment someone turns roles on.
async fn ungoverned_reachers(
    db: &DB,
    governing: &GoverningDatatable,
) -> Result<Vec<UngovernedReacher>> {
    let Some(database) = governing.datatable.database.as_ref() else {
        return Ok(vec![]);
    };
    if database.resource_type != DataTableCatalogResourceType::Instance {
        return Ok(vec![]);
    }
    let rows = sqlx::query!(
        r#"
        SELECT ws.workspace_id AS "workspace_id!", dt.key AS "datatable!"
        FROM workspace_settings ws
        JOIN workspace w ON w.id = ws.workspace_id AND w.deleted = false
        CROSS JOIN LATERAL jsonb_each(COALESCE(ws.datatable->'datatables', '{}'::jsonb)) dt
        WHERE ws.workspace_id <> $1
          AND dt.value->'database'->>'resource_type' = 'instance'
          AND dt.value->'database'->>'resource_path' = $2
        ORDER BY ws.workspace_id, dt.key
        "#,
        &governing.workspace_id,
        &database.resource_path,
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| UngovernedReacher { workspace_id: r.workspace_id, datatable: r.datatable })
        .collect())
}

async fn get_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<DatatablePermissionsInfo> {
    let governing = resolve_governing_datatable(&db, &w_id, &datatable_name).await?;
    let catalog = read_role_catalog(&db).await?;
    let editable = ensure_governs_datatable(&db, &authed, &w_id, &governing)
        .await
        .is_ok();

    let permissions = governing.datatable.permissions.as_ref();
    let roles = permissions
        .map(|p| {
            p.roles
                .iter()
                .map(|(id, tenants)| DatatableRoleTenantsInfo {
                    id: id.clone(),
                    name: if id == ADMIN_DATATABLE_ROLE {
                        Some(ADMIN_DATATABLE_ROLE.to_string())
                    } else {
                        catalog.get(id).map(|r| r.name.clone())
                    },
                    // Tenants name users, groups and folders of the governing workspace, so they
                    // are for the people who set them. Someone reading from a fork gets the shape
                    // of the decision, not the parent's membership; what they may use themselves
                    // is what `datatable_usable_roles` answers.
                    tenants: if editable {
                        tenants.tenants.clone()
                    } else {
                        vec![]
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(DatatablePermissionsInfo {
        supported: governing.is_instance(),
        permissioned: permissions.is_some(),
        default_role: permissions
            .map(|p| p.default_role().to_string())
            .unwrap_or_else(|| ADMIN_DATATABLE_ROLE.to_string()),
        roles,
        governing_workspace_id: (governing.workspace_id != w_id)
            .then(|| governing.workspace_id.clone()),
        editable,
        // The instance's role names are only of use to someone who can pick from them, and
        // enumerating them is the first step of anything that wants to name one it shouldn't.
        available_roles: if editable {
            catalog
                .iter()
                .map(|(id, role)| AvailableRole {
                    id: id.clone(),
                    name: role.name.clone(),
                    enabled: role.enabled,
                })
                .collect()
        } else {
            vec![]
        },
        ungoverned_reachers: if editable {
            ungoverned_reachers(&db, &governing).await?
        } else {
            vec![]
        },
    }))
}

async fn set_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<SetDatatablePermissions>,
) -> Result<String> {
    let governing = resolve_governing_datatable(&db, &w_id, &datatable_name).await?;
    ensure_governs_datatable(&db, &authed, &w_id, &governing).await?;

    // A data table role is a login on Windmill's own cluster; a resource-backed data table dials a
    // host the workspace admin chose, so it has no business naming one.
    if req.permissioned && !governing.is_instance() {
        return Err(Error::BadRequest(format!(
            "Data table '{}' is backed by a Postgres resource. Data table roles are logins on the \
             Windmill instance's own Postgres, so only a data table on the instance database can \
             use them.",
            governing.name
        )));
    }

    // One transaction for the whole save, holding both locks the decision depends on: the role
    // catalog, so a role cannot be deleted between validating an id and writing it back, and the
    // workspace settings row, so a concurrent settings save cannot carry a stale copy of this
    // block forward over what is written here.
    let mut tx = db.begin().await?;
    windmill_common::datatable_roles::lock_role_catalog(&mut tx).await?;
    sqlx::query!(
        "SELECT 1 AS one FROM workspace_settings WHERE workspace_id = $1 FOR UPDATE",
        &governing.workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    // Everything above was decided on a read taken before the locks. A settings save committing in
    // between could have moved this data table onto a PostgreSQL resource — recreating the exact
    // state the transition guard refuses — or renamed it, in which case the write below would
    // target a key that no longer exists and report success having changed nothing. Re-resolve and
    // re-check on the locked state; the earlier pass stays because it is what refuses without
    // taking locks at all.
    let governing = resolve_governing_datatable(&db, &w_id, &datatable_name).await?;
    ensure_governs_datatable(&db, &authed, &w_id, &governing).await?;
    if req.permissioned && !governing.is_instance() {
        return Err(Error::BadRequest(format!(
            "Data table '{}' is backed by a Postgres resource. Data table roles are logins on the \
             Windmill instance's own Postgres, so only a data table on the instance database can \
             use them.",
            governing.name
        )));
    }

    // Turning roles on is refused while a replication stream reads this data table. One already
    // under roles cannot have any: the listener refuses to open a stream on it.
    if req.permissioned && governing.datatable.permissions.is_none() {
        windmill_common::datatable_roles::lock_datatable_streams(&mut *tx, true).await?;
        ensure_no_streams_reaching(&db, &governing).await?;
    }

    let permissions = if req.permissioned {
        let catalog = windmill_common::datatable_roles::read_role_catalog_tx(&mut tx).await?;
        let mut roles: BTreeMap<String, DataTableRoleTenants> = BTreeMap::new();
        for role in req.roles {
            if role.id != ADMIN_DATATABLE_ROLE && !catalog.contains_key(&role.id) {
                return Err(Error::BadRequest(format!(
                    "'{}' is not a data table role of this instance",
                    role.name.unwrap_or(role.id)
                )));
            }
            for tenant in &role.tenants {
                validate_tenant(tenant)?;
            }
            roles.insert(role.id, DataTableRoleTenants { tenants: role.tenants });
        }
        // `admin` is always a row: it is the connection every object in the database is owned by,
        // and a save that dropped it would leave the data table with no way back in.
        roles.entry(ADMIN_DATATABLE_ROLE.to_string()).or_default();

        let default_role = req
            .default_role
            .unwrap_or_else(|| ADMIN_DATATABLE_ROLE.to_string());
        if !roles.contains_key(&default_role) {
            return Err(Error::BadRequest(format!(
                "The default role '{default_role}' is not among the data table's roles"
            )));
        }
        Some(DataTablePermissions { default_role: Some(default_role), roles })
    } else {
        None
    };

    let value = match &permissions {
        Some(p) => serde_json::to_value(p).map_err(|e| Error::internal_err(e.to_string()))?,
        None => serde_json::Value::Null,
    };
    // Written straight onto the governing workspace's entry rather than through the settings form,
    // which deliberately carries this block across untouched.
    sqlx::query!(
        r#"UPDATE workspace_settings
           SET datatable = CASE WHEN $3::jsonb = 'null'::jsonb
               THEN datatable #- ARRAY['datatables', $2, 'permissions']
               ELSE jsonb_set(datatable, ARRAY['datatables', $2, 'permissions'], $3::jsonb)
           END
           WHERE workspace_id = $1"#,
        &governing.workspace_id,
        &governing.name,
        value,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.set_datatable_permissions",
        ActionKind::Update,
        &governing.workspace_id,
        Some(&authed.email),
        Some([("datatable", governing.name.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    // An instance database provisioned before data table roles existed has neither the grant
    // options the admin connection needs to delegate privileges, nor a CONNECT grant for any role
    // — so a role would be refused at login however its tenants read. Repair it here, at the one
    // moment someone is deciding this data table's roles. Best-effort: neither is worth failing a
    // tenant edit over, and both converge again on the next save.
    //
    // Runs after the commit: it opens its own connections to other databases, which has no place
    // inside a transaction holding two locks.
    if permissions.is_some() {
        if let Some(database) = governing.datatable.database.as_ref() {
            if database.resource_type == DataTableCatalogResourceType::Instance {
                let dbname = &database.resource_path;
                if let Err(e) =
                    windmill_common::ensure_instance_db_grant_options_unchecked(&db, dbname).await
                {
                    tracing::warn!("Could not refresh grant options on '{dbname}': {e}");
                }
                if let Err(e) =
                    windmill_common::datatable_roles::converge_connect_grants(&db, dbname).await
                {
                    tracing::warn!("Could not refresh CONNECT grants on '{dbname}': {e}");
                }
            }
        }
    }

    windmill_common::feature_usage::log_feature_usage(
        "datatable",
        "roles_toggled",
        if permissions.is_some() { "on" } else { "off" },
    );

    Ok(if permissions.is_some() {
        format!("Updated the roles of data table '{}'", governing.name)
    } else {
        format!("Data table '{}' is no longer under roles", governing.name)
    })
}

/// Refuse to put a data table under roles while a Postgres trigger or capture streams it. A
/// replication stream reads every row whatever the roles grant, so a data table carries one or the
/// other; the listener side refuses a data table already under roles.
async fn ensure_no_streams_reaching(db: &DB, governing: &GoverningDatatable) -> Result<()> {
    // Every workspace holding an entry that resolves here, under the name it calls it: the
    // governing one, plus each fork pointing at it. A fork's trigger names its own local entry, so
    // looking in the governing workspace alone would miss every stream a fork opened.
    let mut reached = vec![(governing.workspace_id.clone(), governing.name.clone())];
    let pointers = sqlx::query!(
        r#"SELECT ws.workspace_id AS "workspace_id!", dt.key AS "datatable!"
           FROM workspace_settings ws
           CROSS JOIN LATERAL jsonb_each(COALESCE(ws.datatable->'datatables', '{}'::jsonb)) dt
           WHERE dt.value->'reference'->>'workspace_id' = $1
             AND dt.value->'reference'->>'datatable' = $2"#,
        &governing.workspace_id,
        &governing.name,
    )
    .fetch_all(db)
    .await?;
    reached.extend(pointers.into_iter().map(|r| (r.workspace_id, r.datatable)));

    let mut streams = Vec::new();
    for (w_id, name) in reached {
        let reference = format!("datatable://{name}");
        let with_query = format!("{reference}?");
        // A suspended trigger keeps its listener, and a capture streams while its client pings. A
        // listener also outlives its trigger being disabled, or its capture's client going quiet,
        // until its next heartbeat notices; one that pinged within the 15 seconds a server holds a
        // listener for may still be dispatching.
        streams.extend(
            sqlx::query_scalar::<_, String>(
                r#"SELECT workspace_id || '/' || path FROM postgres_trigger
                   WHERE workspace_id = $1
                     AND (mode <> 'disabled'::TRIGGER_MODE
                          OR last_server_ping > now() - interval '15 seconds')
                     AND (postgres_resource_path = $2 OR starts_with(postgres_resource_path, $3))
                   UNION ALL
                   SELECT workspace_id || '/' || path || ' (capture)' FROM capture_config
                   WHERE workspace_id = $1 AND trigger_kind = 'postgres'
                     AND (last_client_ping > now() - interval '10 seconds'
                          OR last_server_ping > now() - interval '15 seconds')
                     AND (trigger_config->>'postgres_resource_path' = $2
                          OR starts_with(trigger_config->>'postgres_resource_path', $3))"#,
            )
            .bind(&w_id)
            .bind(&reference)
            .bind(&with_query)
            .fetch_all(db)
            .await?,
        );
    }
    if !streams.is_empty() {
        return Err(Error::BadRequest(format!(
            "Data table '{}' cannot be put under roles while a Postgres trigger or capture streams \
             it: a replication stream reads every row whatever the roles grant. Disable them, then \
             allow their listeners up to 15 seconds to stop: {}",
            governing.name,
            streams.join(", ")
        )));
    }
    Ok(())
}

/// The roles this caller may connect as, by name, plus the one they get without asking. Drives the
/// role pickers; an empty list means the data table is not under roles.
async fn list_usable_datatable_roles(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<UsableDatatableRoles> {
    let governing = resolve_governing_datatable(&db, &w_id, &datatable_name).await?;
    let Some(permissions) = governing.datatable.permissions.as_ref() else {
        return Ok(Json(UsableDatatableRoles {
            permissioned: false,
            roles: vec![],
            default_role: ADMIN_DATATABLE_ROLE.to_string(),
        }));
    };
    let catalog = read_role_catalog(&db).await?;
    let access = DatatableAccess::Authed(authed.to_authed_ref());

    let mut roles = Vec::new();
    for (id, tenants) in &permissions.roles {
        let name = if id == ADMIN_DATATABLE_ROLE {
            ADMIN_DATATABLE_ROLE.to_string()
        } else {
            match catalog.get(id).filter(|r| r.enabled) {
                Some(role) => role.name.clone(),
                // Deleted or disabled instance-side: it cannot be connected as, so it is not
                // offered, even to someone the tenants cover.
                None => continue,
            }
        };
        if can_use_datatable_role_in_governing_workspace(
            &db,
            &governing.workspace_id,
            &w_id,
            tenants,
            &access,
        )
        .await?
        {
            roles.push(name);
        }
    }

    let default_role = permissions.default_role();
    Ok(Json(UsableDatatableRoles {
        permissioned: true,
        roles,
        default_role: if default_role == ADMIN_DATATABLE_ROLE {
            ADMIN_DATATABLE_ROLE.to_string()
        } else {
            catalog
                .get(default_role)
                .map(|r| r.name.clone())
                .unwrap_or_else(|| default_role.to_string())
        },
    }))
}
