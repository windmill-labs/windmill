//! Admin endpoints for the fine-grained data table permissions config (EE),
//! plus the validation helpers shared with `edit_datatable_config`.

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::datatable_permissions::{
    compute_effective_grants, datatable_license_valid, datatable_permissions_enabled,
    drop_datatable_ephemeral_roles_best_effort, validate_grant_identifier,
    PERMISSIONED_AS_FOLDER_PREFIX,
};
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::users::{
    username_to_permissioned_as, PERMISSIONED_AS_GROUP_PREFIX, PERMISSIONED_AS_USER_PREFIX,
};
use windmill_common::utils::require_admin;
use windmill_common::workspaces::{
    datatable_shared_resource, get_datatable_config, DataTable, DataTableDatabase,
    DataTablePermissions,
};
use windmill_common::{PgDatabase, DB};

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/datatable_permissions/{datatable_name}",
            get(get_datatable_permissions).post(set_datatable_permissions),
        )
        .route(
            "/datatable_permissions_check/{datatable_name}",
            post(check_datatable_permissions_setup),
        )
}

async fn check_datatable_permissions_ee_license() -> Result<()> {
    if !datatable_license_valid().await {
        return Err(Error::BadRequest(
            "Data table fine-grained permissions require an Enterprise license".to_string(),
        ));
    }
    Ok(())
}

/// Save-time validation of the grant statements themselves (tenant shape,
/// folder access selector, operations, identifier sanity). Postgres-side
/// feasibility (ownership, CREATEROLE) is checked separately.
pub(crate) fn validate_permissions_config(perms: &DataTablePermissions) -> Result<()> {
    for grant in &perms.grants {
        let is_folder = grant.tenant.starts_with(PERMISSIONED_AS_FOLDER_PREFIX);
        let valid_tenant = [
            PERMISSIONED_AS_USER_PREFIX,
            PERMISSIONED_AS_GROUP_PREFIX,
            PERMISSIONED_AS_FOLDER_PREFIX,
        ]
        .iter()
        .any(|p| {
            grant
                .tenant
                .strip_prefix(p)
                .is_some_and(|rest| !rest.is_empty())
        });
        if !valid_tenant {
            return Err(Error::BadRequest(format!(
                "Invalid grant tenant '{}': expected u/<username>, g/<group> or f/<folder>",
                grant.tenant
            )));
        }
        if is_folder && grant.folder_access.is_none() {
            return Err(Error::BadRequest(format!(
                "Folder grant '{}' must specify folder_access (read or write)",
                grant.tenant
            )));
        }
        if !is_folder && grant.folder_access.is_some() {
            return Err(Error::BadRequest(format!(
                "folder_access is only valid on folder grants, not '{}'",
                grant.tenant
            )));
        }
        if grant.operations.is_empty() {
            return Err(Error::BadRequest(format!(
                "Grant for '{}' has no operations",
                grant.tenant
            )));
        }
        validate_grant_identifier("Schema", &grant.schema)?;
        for table in &grant.tables {
            validate_grant_identifier("Table", table)?;
        }
    }
    Ok(())
}

/// Enforcement is per database object while config is per data table: a second
/// config pointing at the same physical database would bypass the grants
/// entirely. Instance databases live on the single main cluster, so the check
/// spans all workspaces; for external resources, equal resource paths are the
/// best cheap approximation (two distinct resources holding the same DB
/// coordinates cannot be detected — documented limitation).
pub(crate) async fn check_shared_database_forbid(
    db: &DB,
    w_id: &str,
    name: &str,
    database: &DataTableDatabase,
    self_perms_enabled: bool,
) -> Result<()> {
    let resource_type = match database.resource_type {
        windmill_common::workspaces::DataTableCatalogResourceType::Instance => "instance",
        windmill_common::workspaces::DataTableCatalogResourceType::Postgresql => "postgresql",
    };
    let rows = sqlx::query!(
        r#"
        SELECT ws.workspace_id AS "workspace_id!", dt.key AS "name!",
               COALESCE((dt.value->'permissions'->>'enabled')::boolean, false) AS "perms_enabled!"
        FROM workspace_settings ws
        CROSS JOIN LATERAL jsonb_each(
            CASE WHEN jsonb_typeof(ws.datatable->'datatables') = 'object'
                THEN ws.datatable->'datatables'
                ELSE '{}'::jsonb END
        ) AS dt(key, value)
        WHERE dt.value->'database'->>'resource_type' = $1
          AND dt.value->'database'->>'resource_path' = $2
        "#,
        resource_type,
        &database.resource_path,
    )
    .fetch_all(db)
    .await?;

    for row in rows {
        if row.workspace_id == w_id && row.name == name {
            continue;
        }
        if self_perms_enabled || row.perms_enabled {
            return Err(Error::BadRequest(format!(
                "Data table '{}' (workspace '{}') already points at the same physical database \
                 ('{}'). Fine-grained permissions are enforced per database, so a \
                 permissions-enabled data table must be the only config using its database — \
                 otherwise access through the other config would bypass the grants.",
                row.name, row.workspace_id, database.resource_path
            )));
        }
    }
    Ok(())
}

/// The owner role creates/drops the ephemeral roles, so it needs CREATEROLE
/// (or superuser). True by construction for freshly provisioned instance
/// databases; verified here for external resources and legacy instance DBs.
pub(crate) async fn check_owner_can_create_roles(
    db: &DB,
    w_id: &str,
    config: &DataTable,
) -> Result<()> {
    let owner: PgDatabase =
        serde_json::from_value(datatable_shared_resource(db, w_id, config).await?)
            .map_err(|e| Error::internal_err(format!("parsing data table credentials: {e}")))?;
    let (client, connection) = owner.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::warn!("datatable permissions check connection error: {e:#}");
        }
    });
    let can: bool = client
        .query_one(
            "SELECT rolcreaterole OR rolsuper FROM pg_roles WHERE rolname = current_user",
            &[],
        )
        .await
        .map_err(|e| Error::internal_err(format!("checking CREATEROLE: {e:#}")))?
        .get(0);
    if !can {
        return Err(Error::BadRequest(format!(
            "The data table's database user ('{}') lacks the CREATEROLE privilege, which is \
             required to create the per-user roles that enforce fine-grained permissions. \
             Grant it with: ALTER ROLE \"{}\" CREATEROLE;",
            owner.user.as_deref().unwrap_or(""),
            owner.user.as_deref().unwrap_or("")
        )));
    }
    Ok(())
}

async fn get_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<DataTablePermissions> {
    require_admin(authed.is_admin, &authed.username)?;
    let config = get_datatable_config(&db, &w_id, &datatable_name).await?;
    Ok(Json(config.permissions.unwrap_or_default()))
}

async fn set_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(perms): Json<DataTablePermissions>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let config = get_datatable_config(&db, &w_id, &datatable_name).await?;

    // Disabling never requires a license: it is the escape hatch after a
    // license downgrade (enabled + no license fails closed for non-admins).
    if perms.enabled {
        check_datatable_permissions_ee_license().await?;
    }
    validate_permissions_config(&perms)?;
    if perms.enabled {
        check_shared_database_forbid(&db, &w_id, &datatable_name, &config.database, true).await?;
        check_owner_can_create_roles(&db, &w_id, &config).await?;
    }

    let mut tx = db.begin().await?;
    let args_for_audit = format!("{:?}", perms);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_datatable_permissions",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [
                ("datatable", datatable_name.as_str()),
                ("permissions", args_for_audit.as_str()),
            ]
            .into(),
        ),
    )
    .await?;
    let perms_json =
        serde_json::to_value(&perms).map_err(|e| Error::internal_err(e.to_string()))?;
    sqlx::query!(
        "UPDATE workspace_settings
         SET datatable = jsonb_set(datatable, ARRAY['datatables', $2, 'permissions'], $3)
         WHERE workspace_id = $1",
        &w_id,
        &datatable_name,
        perms_json,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Existing ephemeral roles were built from the previous grants; drop them
    // so nothing outlives the edit (new accesses would recreate them via the
    // perms-hash check anyway, and disabled data tables stop using them).
    drop_datatable_ephemeral_roles_best_effort(&db, &w_id, Some(&datatable_name)).await;

    Ok(format!(
        "Edited permissions of data table {datatable_name} in workspace {w_id}"
    ))
}

/// Preflight for the frontend: surfaces the shared-database and CREATEROLE
/// problems before the admin hits save.
async fn check_datatable_permissions_setup(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    check_datatable_permissions_ee_license().await?;
    let config = get_datatable_config(&db, &w_id, &datatable_name).await?;
    check_shared_database_forbid(&db, &w_id, &datatable_name, &config.database, true).await?;
    check_owner_can_create_roles(&db, &w_id, &config).await?;
    Ok("ok".to_string())
}

/// Introspection gate for the schema/table listing endpoints: metadata is
/// served through the shared role, so when permissions are enabled a
/// non-admin needs at least one effective grant (default deny); any grant
/// unlocks all metadata (accepted v0 leak).
pub(crate) async fn check_datatable_introspection_access(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    datatable_name: &str,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    let config = get_datatable_config(db, w_id, datatable_name).await?;
    if !datatable_permissions_enabled(&config) {
        return Ok(());
    }
    if !datatable_license_valid().await {
        return Err(Error::PermissionDenied(format!(
            "Data table '{datatable_name}' has fine-grained permissions enabled, which requires \
             an Enterprise license; non-admin access is denied."
        )));
    }
    let grants = config
        .permissions
        .as_ref()
        .map(|p| p.grants.as_slice())
        .unwrap_or(&[]);
    let (matched, _) = compute_effective_grants(
        db,
        w_id,
        &username_to_permissioned_as(&authed.username),
        grants,
    )
    .await?;
    if matched.is_empty() {
        return Err(Error::PermissionDenied(format!(
            "You have no permissions on data table '{datatable_name}'. Ask a workspace admin to \
             grant you access in the data table's permission settings."
        )));
    }
    Ok(())
}
