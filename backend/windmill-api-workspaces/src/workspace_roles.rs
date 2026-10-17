//! The workspace's roles and its overrides of them. See `windmill_common::roles`.

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use windmill_api_auth::ApiAuthed;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    error::{Error, JsonResult, Result},
    roles::{
        self, instance_roles, load_role_overrides, EffectiveRole, RoleCapabilities, RoleOverrides,
        RoleSource, OPERATOR_ROLE,
    },
    utils::require_admin,
    DB,
};
use windmill_git_sync::handle_deployment_metadata;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/roles", get(list_roles))
        .route("/role_overrides", post(update_role_overrides))
}

#[derive(Serialize)]
struct WorkspaceRoles {
    roles: Vec<EffectiveRole>,
    overrides: RoleOverrides,
    /// Whether this workspace may add roles of its own.
    custom_roles_enabled: bool,
    /// The caller's role, `None` for a caller with no membership (a superadmin, a guest).
    my_role: Option<String>,
    my_capabilities: RoleCapabilities,
}

async fn list_roles(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
) -> JsonResult<WorkspaceRoles> {
    let roles = roles::workspace_roles(&db, &w_id).await?;
    let overrides = load_role_overrides(&db, &w_id).await?;
    let my_role = sqlx::query!(
        "SELECT workspace_role, operator FROM usr WHERE workspace_id = $1 AND username = $2",
        &w_id,
        &authed.username
    )
    .fetch_optional(&db)
    .await?
    .map(|r| roles::resolve_role_name(r.workspace_role, r.operator));
    // Not `caller_capabilities`: its fast path answers FULL for an unrestricted member, which
    // drops the visibility a role may still withhold.
    let my_capabilities = match &my_role {
        _ if authed.is_admin => roles::RoleCapabilities::FULL,
        Some(role) => roles::role_capabilities_by_name(&db, &w_id, role).await?,
        None => roles::RoleCapabilities::NONE,
    };
    Ok(Json(WorkspaceRoles {
        roles: roles.as_ref().clone(),
        overrides,
        custom_roles_enabled: roles::custom_roles_enabled(),
        my_role,
        my_capabilities,
    }))
}

async fn update_role_overrides(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
    Json(overrides): Json<RoleOverrides>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    write_role_overrides(&db, &authed, &w_id, overrides, None).await?;
    Ok("Role overrides updated".to_string())
}

/// Replaces the workspace's role overrides with `overrides`. Refuses removing a workspace-only role
/// anything still names, and a change that pushes an offline license over its seat cap.
/// `operator_settings` is the legacy `/operator_settings` payload, mirrored into its column for
/// git sync and for servers that predate roles.
pub(crate) async fn write_role_overrides(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    overrides: RoleOverrides,
    operator_settings: Option<serde_json::Value>,
) -> Result<()> {
    let instance = instance_roles(db).await?;
    roles::validate_overrides(&instance, &overrides)?;

    let previous = roles::workspace_roles(db, w_id).await?;
    let removed: Vec<String> = previous
        .iter()
        .filter(|r| r.source == RoleSource::Workspace && !overrides.contains_key(&r.name))
        .map(|r| r.name.clone())
        .collect();
    for name in &removed {
        roles::ensure_role_unused(db, Some(w_id), name).await?;
    }

    let overrides_json = serde_json::to_value(&overrides)
        .map_err(|e| Error::internal_err(format!("serializing role overrides: {e:#}")))?;
    #[cfg(feature = "enterprise")]
    let seat_baseline = windmill_common::ee_oss::seat_cap_baseline(db).await?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace_settings SET role_overrides = $1 WHERE workspace_id = $2",
        if overrides.is_empty() {
            None
        } else {
            Some(&overrides_json)
        },
        w_id
    )
    .execute(&mut *tx)
    .await?;
    if let Some(settings) = &operator_settings {
        sqlx::query!(
            "UPDATE workspace_settings
             SET operator_settings = COALESCE(operator_settings, '{}'::jsonb) || $1
             WHERE workspace_id = $2",
            settings,
            w_id
        )
        .execute(&mut *tx)
        .await?;
    }
    roles::invalidate_workspace_roles_cache(w_id);
    roles::sync_role_seats(&mut tx, Some(w_id)).await?;

    #[cfg(feature = "enterprise")]
    if let Some(msg) =
        windmill_common::ee_oss::check_seat_cap_for_role_change(&seat_baseline, &mut tx).await?
    {
        return Err(Error::BadRequest(msg));
    }

    audit_log(
        &mut *tx,
        authed,
        if operator_settings.is_some() {
            "workspaces.update_operator_settings"
        } else {
            "workspaces.update_role_overrides"
        },
        ActionKind::Update,
        w_id,
        None,
        Some([("role_overrides", overrides_json.to_string().as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    roles::invalidate_workspace_roles_cache(w_id);

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        db,
        w_id,
        windmill_git_sync::DeployedObject::Settings {
            setting_type: if operator_settings.is_some() {
                "operator_settings".to_string()
            } else {
                "role_overrides".to_string()
            },
        },
        Some("Workspace roles updated".to_string()),
        false,
        None,
    )
    .await?;
    Ok(())
}

/// The legacy `/operator_settings` payload. Visibility flags always serialize; the rights are
/// `Option` because git-sync posts whole files that may predate them, and an absent key must
/// leave the stored value alone.
#[derive(Deserialize, Serialize)]
pub(crate) struct ChangeOperatorSettings {
    #[serde(default)]
    runs: bool,
    #[serde(default)]
    schedules: bool,
    #[serde(default)]
    resources: bool,
    #[serde(default)]
    variables: bool,
    #[serde(default)]
    assets: bool,
    #[serde(default)]
    triggers: bool,
    #[serde(default)]
    audit_logs: bool,
    #[serde(default)]
    groups: bool,
    #[serde(default)]
    folders: bool,
    #[serde(default)]
    workers: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    builder_flows: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    builder_apps: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    manage_schedules: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    manage_triggers: Option<bool>,
}

/// Applies a legacy operator settings payload as an override of the `operator` role. A key equal
/// to what the instance gives the role is dropped rather than pinned, so it keeps following the
/// instance definition.
pub(crate) async fn update_operator_settings(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
    Json(settings): Json<ChangeOperatorSettings>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let instance = instance_roles(&db).await?;
    let Some(base) = roles::effective_roles(&instance, &RoleOverrides::new())?
        .into_iter()
        .find(|r| r.name == OPERATOR_ROLE)
    else {
        return Err(Error::BadRequest(
            "this instance defines no `operator` role: edit the workspace roles instead"
                .to_string(),
        ));
    };
    let base = base.capabilities;

    let mut overrides = load_role_overrides(&db, &w_id).await?;
    let current = roles::workspace_roles(&db, &w_id)
        .await?
        .iter()
        .find(|r| r.name == OPERATOR_ROLE)
        .map(|r| r.capabilities)
        .unwrap_or(base);

    let mut target = current;
    let v = &mut target.visibility;
    (v.runs, v.schedules, v.resources, v.variables, v.assets) = (
        settings.runs,
        settings.schedules,
        settings.resources,
        settings.variables,
        settings.assets,
    );
    (v.triggers, v.audit_logs, v.groups, v.folders, v.workers) = (
        settings.triggers,
        settings.audit_logs,
        settings.groups,
        settings.folders,
        settings.workers,
    );
    let compose_or_keep = |cur: roles::AuthorLevel, grant: Option<bool>| match grant {
        Some(true) if cur == roles::AuthorLevel::None => roles::AuthorLevel::Compose,
        Some(false) => roles::AuthorLevel::None,
        _ => cur,
    };
    target.flows = compose_or_keep(current.flows, settings.builder_flows);
    target.apps = compose_or_keep(current.apps, settings.builder_apps);
    if let Some(b) = settings.manage_schedules {
        target.manage_schedules = b;
    }
    if let Some(b) = settings.manage_triggers {
        target.manage_triggers = b;
    }

    let entry = overrides.entry(OPERATOR_ROLE.to_string()).or_default();
    entry.capabilities = roles::diff_capabilities(&base, &target);
    if entry.label.is_none() && entry.capabilities == Default::default() {
        overrides.remove(OPERATOR_ROLE);
    }

    let settings_json = serde_json::json!(settings);
    write_role_overrides(&db, &authed, &w_id, overrides, Some(settings_json)).await?;
    Ok("Operator settings updated successfully".to_string())
}
