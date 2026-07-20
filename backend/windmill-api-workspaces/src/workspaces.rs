/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_api_auth::{
    build_scope_path_predicate, check_scopes, require_devops_role, require_is_writer,
    require_super_admin, ApiAuthed,
};
use windmill_api_users::users::WorkspaceInvite;
use windmill_common::email_oss::send_email_if_possible;
use windmill_common::usernames::{get_instance_username_or_create_pending, VALID_USERNAME};
use windmill_common::webhook::WebhookShared;
use windmill_common::{BASE_URL, DB};

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;

use regex::Regex;

use hex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;
use uuid::Uuid;
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::db::UserDB;
use windmill_common::global_settings::HTTP_ROUTE_WORKSPACED_ROUTE;
use windmill_common::users::username_to_permissioned_as;
use windmill_common::variables::{
    build_crypt, decrypt, encrypt, SECRET_SALT, WORKSPACE_CRYPT_CACHE,
};
use windmill_common::worker::{to_raw_value, CLOUD_HOSTED};
use windmill_common::workspaces::GitRepositorySettings;
#[cfg(feature = "enterprise")]
use windmill_common::workspaces::WorkspaceDeploymentUISettings;
use windmill_common::workspaces::{
    check_deploy_rules, check_user_against_rule, get_datatable_resource_from_db_unchecked,
    validate_dev_workspace_id, validate_fork_workspace_id, validate_workspace_name, DataTable,
    DataTableCatalogResourceType, DataTableForkBehavior, ProtectionRuleKind, ProtectionRules,
    ProtectionRuleset, RuleCheckResult, WorkspaceGitSyncSettings, DEV_WORKSPACE_LOCK_RULE_NAME,
};
use windmill_common::workspaces::{Ducklake, DucklakeCatalogResourceType};
use windmill_common::PgDatabase;
use windmill_common::{
    error::{Error, JsonResult, Result},
    global_settings::{
        AUTOMATE_USERNAME_CREATION_SETTING, DISABLE_WORKSPACE_INVITE_EMAILS_SETTING,
    },
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
    utils::{paginate, rd_string, require_admin, Pagination},
};
use windmill_dep_map::scoped_dependency_map::{
    DependencyDependent, DependencyMap, ScopedDependencyMap,
};
use windmill_git_sync::{
    handle_deployment_metadata, handle_deployment_metadata_batch, handle_fork_branch_creation,
    DeployedObject,
};
use windmill_types::s3::LargeFileStorage;

use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Row, Transaction};
use windmill_common::oauth2::InstanceEvent;
use windmill_common::secret_backend::{get_secret_backend, is_vault_backend_configured};
use windmill_common::utils::not_found_if_none;

lazy_static::lazy_static! {
    static ref WORKSPACE_KEY_REGEXP: Regex = Regex::new("^[a-zA-Z0-9]{64}$").unwrap();
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/get_as_superadmin", get(get_workspace_as_superadmin))
        .route("/list_pending_invites", get(list_pending_invites))
        .route("/update", post(edit_workspace))
        .route("/archive", post(archive_workspace))
        .route("/invite_user", post(invite_user))
        .route("/add_user", post(add_user))
        .route("/create_service_account", post(create_service_account))
        .route("/delete_invite", post(delete_invite))
        .route("/rebuild_dependency_map", post(rebuild_dependency_map))
        .route("/get_dependency_map", get(get_dependency_map))
        .route("/get_dependents/{*imported_path}", get(get_dependents))
        .route("/get_imports/{*importer_path}", get(get_imports))
        .route("/get_dependents_amounts", post(get_dependents_amounts))
        .route("/get_settings", get(get_settings))
        .route("/get_public_settings", get(get_public_settings))
        .route(
            "/get_copilot_settings_state",
            get(get_copilot_settings_state),
        )
        .route("/get_deploy_to", get(get_deploy_to))
        .route("/edit_slack_command", post(edit_slack_command))
        .route(
            "/run_slack_message_test_job",
            post(run_slack_message_test_job),
        )
        .route("/slack_oauth_config", get(get_slack_oauth_config))
        .route("/slack_oauth_config", post(set_slack_oauth_config))
        .route("/slack_oauth_config", delete(delete_slack_oauth_config))
        .route("/edit_webhook", post(edit_webhook))
        .route("/edit_auto_invite", post(edit_auto_invite))
        .route("/edit_instance_groups", post(edit_instance_groups))
        .route("/edit_deploy_to", post(edit_deploy_to))
        .route(
            "/get_secondary_storage_names",
            get(get_secondary_storage_names),
        )
        .route("/is_premium", get(is_premium))
        .route("/edit_error_handler", post(edit_error_handler))
        .route("/edit_success_handler", post(edit_success_handler))
        .route(
            "/edit_large_file_storage_config",
            post(edit_large_file_storage_config),
        )
        .route("/edit_ducklake_config", post(edit_ducklake_config))
        .route("/list_ducklakes", get(list_ducklakes))
        .route("/list_datatables", get(list_datatables))
        .route("/list_datatable_schemas", get(list_datatable_schemas))
        .route("/list_datatable_tables", get(list_datatable_tables))
        .route(
            "/get_datatable_table_schema",
            get(get_datatable_table_schema),
        )
        .route("/edit_datatable_config", post(edit_datatable_config))
        .merge(crate::datatable_migrations::routes())
        .route("/git_sync_enabled", get(get_git_sync_enabled))
        .route("/git_sync_deploy_mode", get(get_git_sync_deploy_mode))
        .route("/edit_git_sync_config", post(edit_git_sync_config))
        .route("/edit_git_sync_repository", post(edit_git_sync_repository))
        .route(
            "/delete_git_sync_repository",
            delete(delete_git_sync_repository),
        )
        .route("/edit_deploy_ui_config", post(edit_deploy_ui_config))
        .route("/edit_default_app", post(edit_default_app))
        .route("/default_app", get(get_default_app))
        .route(
            "/default_scripts",
            post(edit_default_scripts).get(get_default_scripts),
        )
        .route("/set_environment_variable", post(set_environment_variable))
        .route(
            "/encryption_key",
            get(get_encryption_key).post(set_encryption_key),
        )
        .route("/leave", post(leave_workspace))
        .route("/get_workspace_name", get(get_workspace_name))
        .route("/create_fork", post(create_workspace_fork))
        .route("/attach_dev_workspace", post(attach_dev_workspace))
        .route("/detach_dev_workspace", post(detach_dev_workspace))
        .route("/get_dev_workspace", get(get_dev_workspace))
        .route("/change_workspace_name", post(change_workspace_name))
        .route("/change_workspace_color", post(change_workspace_color))
        .route(
            "/change_workspace_id",
            post(crate::workspaces_extra::change_workspace_id),
        )
        .route("/usage", get(get_usage))
        .route("/used_triggers", get(get_used_triggers))
        .route("/public_app_rate_limit", post(edit_public_app_rate_limit))
        .route("/operator_settings", post(update_operator_settings))
        .route(
            "/create_workspace_fork_branch",
            post(create_workspace_fork_branch),
        )
        .route(
            "/reset_diff_tally/{fork_workspace_id}",
            post(reset_workspace_diffs),
        )
        .route("/compare/{target_workspace_id}", get(compare_workspaces))
        .route("/create_pg_database", post(create_pg_database))
        .route("/import_pg_database", post(import_pg_database))
        .route("/export_pg_schema", post(export_pg_schema))
        .route(
            "/drop_forked_datatable_databases",
            post(crate::workspaces_extra::drop_forked_datatable_databases),
        )
        .route(
            "/drop_forked_ducklake_namespaces",
            post(crate::workspaces_extra::drop_forked_ducklake_namespaces),
        )
        .route(
            "/get_datatable_full_schema",
            post(get_datatable_full_schema),
        )
        .route("/protection_rules", get(list_protection_rules))
        .route("/protection_rules", post(create_protection_rule))
        .route(
            "/protection_rules/{rule_name}",
            post(update_protection_rule).delete(delete_protection_rule),
        )
        .route("/log_chat", post(log_ai_chat))
        .route("/cloud_quotas", get(get_cloud_quotas))
        .route("/prune_versions", post(prune_versions))
        .route("/list_ws_specific", get(list_ws_specific))
        .route("/list_ws_specific_versions", get(list_ws_specific_versions))
        .route("/set_ws_specific", post(set_ws_specific))
}
pub fn global_service() -> Router {
    Router::new()
        .route("/list_as_superadmin", get(list_workspaces_as_super_admin))
        .route("/list", get(list_workspaces))
        .route("/users", get(user_workspaces))
        .route("/session_workspace_status", post(session_workspace_status))
        .route("/create", post(create_workspace))
        .route("/create_fork", post(deprecated_create_workspace_fork))
        .route("/exists", post(exists_workspace))
        .route("/exists_username", post(exists_username))
        .route("/allowed_domain_auto_invite", get(is_allowed_auto_domain))
        .route("/unarchive/{workspace}", post(unarchive_workspace))
        .route(
            "/delete/{workspace}",
            delete(crate::workspaces_extra::delete_workspace),
        )
        .route(
            "/create_workspace_require_superadmin",
            get(create_workspace_require_superadmin),
        )
}

#[derive(FromRow, Serialize)]
struct Workspace {
    id: String,
    name: String,
    owner: String,
    deleted: bool,
    premium: bool,
    color: Option<String>,
    parent_workspace_id: Option<String>,
}

#[derive(FromRow, Serialize, Debug)]
pub struct WorkspaceSettings {
    pub workspace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_team_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_team_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_command_script: Option<String>,
    pub teams_command_script: Option<String>,
    pub slack_email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_oauth_client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_oauth_client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_file_storage: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ducklake: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatable: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sync: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_ui: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_scripts: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute_critical_alerts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_settings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_app_installations: Option<serde_json::Value>,
    // Grouped config fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_invite: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_handler: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_app_execution_limit_per_minute: Option<i32>,
}

/// Subset of `WorkspaceSettings` that is safe to return to any workspace
/// member. Adding a field here means it will be readable by every authed user
/// in the workspace — anything sensitive (OAuth secrets, GitHub App tokens,
/// billing/customer info, integration credentials, etc.) must NOT be added.
/// The full `WorkspaceSettings` struct is admin-only via `get_settings`.
#[derive(FromRow, Serialize, Debug)]
pub struct WorkspacePublicSettings {
    pub workspace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_team_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_team_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute_critical_alerts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_ui: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_file_storage: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatable: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CopilotSettingsState {
    pub has_instance_ai_config: bool,
    pub uses_instance_ai_config: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_ai_summary: Option<InstanceAISummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InstanceAIProviderSummary {
    pub provider: String,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InstanceAIModelSummary {
    pub provider: String,
    pub model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InstanceAISummary {
    pub providers: Vec<InstanceAIProviderSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<InstanceAIModelSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_model: Option<InstanceAIModelSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_completion_model: Option<InstanceAIModelSummary>,
}

/// #[derive(sqlx::Type, Serialize, Deserialize, Debug)]
// #[sqlx(type_name = "WORKSPACE_KEY_KIND", rename_all = "lowercase")]
// pub enum WorkspaceKeyKind {
//     Cloud,
// }

#[derive(Deserialize)]
struct EditCommandScript {
    slack_command_script: Option<String>,
}

#[derive(Deserialize)]
struct RunSlackMessageTestJobRequest {
    hub_script_path: String,
    channel: String,
    test_msg: String,
}

#[derive(Serialize)]
struct RunSlackMessageTestJobResponse {
    job_uuid: String,
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
struct EditDeployTo {
    deploy_to: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct EditAutoInvite {
    pub operator: Option<bool>,
    pub invite_all: Option<bool>,
    pub auto_add: Option<bool>,
}

#[derive(Deserialize)]
struct EditWebhook {
    webhook: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct LargeFileStorageWithSecondary {
    #[serde(flatten)]
    large_file_storage: LargeFileStorage,
    #[serde(default)]
    secondary_storage: HashMap<String, LargeFileStorage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume_storage: Option<String>,
}
#[derive(Deserialize, Debug)]
struct EditLargeFileStorageConfig {
    large_file_storage: Option<LargeFileStorageWithSecondary>,
}

#[derive(Deserialize, Debug)]
struct EditDucklakeConfig {
    settings: DucklakeSettings,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeSettings {
    pub ducklakes: HashMap<String, Ducklake>,
}

#[derive(Deserialize, Debug)]
struct EditDataTableConfig {
    settings: DataTableSettings,
    // Data table renames (old -> new) and deletions, tracked client-side by a
    // stable id, so we can cascade or drop each data table's migrations.
    #[serde(default)]
    renames: Vec<crate::datatable_migrations::DatatableRename>,
    #[serde(default)]
    deleted_datatables: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataTableSettings {
    pub datatables: HashMap<String, DataTable>,
}

#[derive(Deserialize)]
struct CreateWorkspace {
    id: String,
    name: String,
    username: Option<String>,
    color: Option<String>,
}

#[derive(Deserialize)]
struct CreateWorkspaceFork {
    id: String,
    name: String,
    color: Option<String>,
    /// Datatable names that were forked. For each, the backend will update the
    /// forked workspace's datatable config to point to the new database.
    #[serde(default)]
    forked_datatables: Vec<ForkedDatatableInfo>,
    /// Lakes the user explicitly chose to SHARE with the parent (the fork then reads and
    /// writes the parent's lake directly). Every lake not listed gets the default isolated
    /// fork namespace + read-defer.
    #[serde(default)]
    shared_ducklakes: Vec<String>,
    /// Create the fork as a persistent dev workspace: the id is not required to carry the
    /// `wm-fork-` prefix, and at most one dev workspace may exist per parent.
    #[serde(default)]
    is_dev_workspace: bool,
    /// When creating a dev workspace, lock the parent ("prod") against direct deployment and/or
    /// ad-hoc forking, so edits are funneled through the dev workspace.
    #[serde(default)]
    lock_prod_deploy: bool,
    #[serde(default)]
    lock_prod_forking: bool,
    /// Copy the parent's members (usr rows + group memberships) into the fork so
    /// the team can work in it. Defaults off; the dev-workspace UI defaults it on.
    #[serde(default)]
    copy_members: bool,
    /// Cosmetic display label for the dev workspace: 'dev' | 'staging'. Purely visual (badge text +
    /// wording); ignored for non-dev forks. None defaults to 'dev'.
    #[serde(default)]
    dev_workspace_label: Option<String>,
}

#[derive(Deserialize)]
struct ForkedDatatableInfo {
    name: String,
    new_dbname: String,
}

#[derive(Deserialize)]
struct EditWorkspace {
    name: String,
    owner: String,
}

#[derive(Serialize)]
struct WorkspaceList {
    pub email: String,
    pub workspaces: Vec<UserWorkspace>,
}

#[derive(Serialize)]
struct UserWorkspace {
    pub id: String,
    pub name: String,
    pub username: String,
    pub color: Option<String>,
    pub operator_settings: Option<Option<serde_json::Value>>,
    pub parent_workspace_id: Option<String>,
    pub is_dev_workspace: bool,
    pub dev_workspace_label: Option<String>,
    /// Creator of the workspace (`workspace.owner`). On a fork it identifies the forker, who gets a
    /// narrow membership grant over it even without being an admin — the UI keys the fork members
    /// screen off this.
    pub created_by: Option<String>,
    pub disabled: bool,
}

#[derive(Deserialize)]
struct WorkspaceId {
    pub id: String,
}

#[derive(Deserialize)]
struct ValidateUsername {
    pub id: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct NewWorkspaceInvite {
    pub email: String,
    pub is_admin: bool,
    pub operator: bool,
}

#[derive(Deserialize)]
pub struct NewWorkspaceUser {
    pub email: String,
    pub username: Option<String>,
    pub is_admin: bool,
    pub operator: bool,
}

// New format for error handler (grouped)
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditErrorHandlerNew {
    pub path: Option<String>,
    pub extra_args: Option<serde_json::Value>,
    #[serde(default)]
    pub muted_on_cancel: bool,
    #[serde(default)]
    pub muted_on_user_path: bool,
}

// Legacy format for error handler (flat fields from old CLI)
#[derive(Deserialize)]
pub struct EditErrorHandlerLegacy {
    pub error_handler: Option<String>,
    pub error_handler_extra_args: Option<serde_json::Value>,
    #[serde(default)]
    pub error_handler_muted_on_cancel: bool,
}

// Accepts both old and new formats
#[derive(Deserialize)]
#[serde(untagged)]
pub enum EditErrorHandler {
    New(EditErrorHandlerNew),
    Legacy(EditErrorHandlerLegacy),
}

impl EditErrorHandler {
    pub fn into_normalized(self) -> EditErrorHandlerNew {
        match self {
            EditErrorHandler::New(new) => new,
            EditErrorHandler::Legacy(legacy) => EditErrorHandlerNew {
                path: legacy.error_handler,
                extra_args: legacy.error_handler_extra_args,
                muted_on_cancel: legacy.error_handler_muted_on_cancel,
                muted_on_user_path: false, // Old format doesn't have this field
            },
        }
    }
}

// New format for success handler (grouped)
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditSuccessHandlerNew {
    pub path: Option<String>,
    pub extra_args: Option<serde_json::Value>,
}

// Legacy format for success handler (flat fields from old CLI)
#[derive(Deserialize)]
pub struct EditSuccessHandlerLegacy {
    pub success_handler: Option<String>,
    pub success_handler_extra_args: Option<serde_json::Value>,
}

// Accepts both old and new formats
#[derive(Deserialize)]
#[serde(untagged)]
pub enum EditSuccessHandler {
    New(EditSuccessHandlerNew),
    Legacy(EditSuccessHandlerLegacy),
}

impl EditSuccessHandler {
    pub fn into_normalized(self) -> EditSuccessHandlerNew {
        match self {
            EditSuccessHandler::New(new) => new,
            EditSuccessHandler::Legacy(legacy) => EditSuccessHandlerNew {
                path: legacy.success_handler,
                extra_args: legacy.success_handler_extra_args,
            },
        }
    }
}

lazy_static::lazy_static! {
    pub static ref EMAIL_REGEXP: Regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
}

async fn list_pending_invites(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WorkspaceInvite>> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as!(
        WorkspaceInvite,
        "SELECT
            workspace_invite.workspace_id,
            workspace_invite.email,
            workspace_invite.is_admin,
            workspace_invite.operator,
            workspace.parent_workspace_id
        FROM workspace_invite JOIN workspace ON workspace_invite.workspace_id = workspace.id
        WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn is_premium(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
) -> JsonResult<bool> {
    // Any workspace member (not just admins) may read whether the workspace is on a paid plan: it's a
    // single boolean, and the frontend needs it to decide whether to surface premium-gated affordances
    // (e.g. forking) to non-admin developers too. The `_authed` extractor still enforces membership.
    #[cfg(feature = "cloud")]
    let premium = windmill_common::workspaces::get_team_plan_status(&_db, &_w_id)
        .await?
        .premium;
    #[cfg(not(feature = "cloud"))]
    let premium = false;
    Ok(Json(premium))
}

async fn exists_workspace(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Json(WorkspaceId { id }): Json<WorkspaceId>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace WHERE workspace.id = $1)",
        id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);
    tx.commit().await?;
    Ok(Json(exists))
}

/// Whether this workspace already has an active canonical dev workspace. The create-fork UI can't
/// rely on the caller's workspace list to decide this — a dev paired to this prod may exist that the
/// caller isn't a member of — so it asks the server, which sees all children.
#[derive(Serialize)]
struct DevWorkspaceInfo {
    id: String,
    name: String,
    dev_workspace_label: Option<String>,
}

/// Normalize/validate the cosmetic dev-workspace display label. None or 'dev' both render as "dev";
/// 'staging' renders as "stg". Anything else is rejected. Stored explicitly ('dev'/'staging') so it
/// round-trips, but a NULL column is treated as 'dev' on the read side too.
fn normalize_dev_workspace_label(label: Option<String>) -> Result<Option<String>> {
    match label.as_deref() {
        None | Some("dev") => Ok(Some("dev".to_string())),
        Some("staging") => Ok(Some("staging".to_string())),
        Some(other) => Err(Error::BadRequest(format!(
            "invalid dev workspace label '{other}' (expected 'dev' or 'staging')"
        ))),
    }
}

/// This workspace's active canonical dev workspace, if any. The create-fork UI and the dev-workspace
/// settings tab can't rely on the caller's workspace list — a dev paired to this prod may exist that
/// the caller isn't a member of — so they ask the server, which sees all children. Returns its id/name
/// so a prod admin who isn't a dev member can still see the pairing and detach it.
async fn get_dev_workspace(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Option<DevWorkspaceInfo>> {
    let dev = sqlx::query_as!(
        DevWorkspaceInfo,
        "SELECT id, name, dev_workspace_label FROM workspace WHERE parent_workspace_id = $1 AND is_dev_workspace AND deleted = false",
        &w_id
    )
    .fetch_optional(&db)
    .await?;
    Ok(Json(dev))
}

async fn list_workspaces(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Workspace>> {
    let mut tx = user_db.begin(&authed).await?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT workspace.id, workspace.name, workspace.owner, workspace.deleted, workspace.premium, workspace_settings.color, workspace.parent_workspace_id
         FROM workspace
         LEFT JOIN workspace_settings ON workspace.id = workspace_settings.workspace_id
         JOIN usr ON usr.workspace_id = workspace.id
         WHERE usr.email = $1 AND workspace.deleted = false",
        authed.email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(workspaces))
}

/// Strip the server-only webhook HMAC secret from a `git_sync` blob before it is
/// returned to a client. The UI never needs it; it stays (encrypted) in the DB.
fn redact_git_sync_webhook_secrets(git_sync: &mut serde_json::Value) {
    if let Some(repos) = git_sync
        .get_mut("repositories")
        .and_then(|r| r.as_array_mut())
    {
        for repo in repos {
            if let Some(auto_pull) = repo.get_mut("auto_pull").and_then(|a| a.as_object_mut()) {
                auto_pull.remove("webhook_secret");
            }
        }
    }
}

/// Zero the server-owned auto-pull fields (webhook id/secret/error, synced sha,
/// last pull status) on a client-supplied `AutoPullSettings`. The client only
/// controls `enabled` / `mode` / `poll_interval_s`; the rest is written by the
/// server (webhook creation, poller) and must never be trusted from the request —
/// otherwise a caller could inject a webhook id/secret or fake sync state.
fn clear_client_supplied_auto_pull_state(
    auto_pull: &mut windmill_common::workspaces::AutoPullSettings,
) {
    auto_pull.webhook_id = None;
    auto_pull.webhook_secret = None;
    auto_pull.webhook_error = None;
    auto_pull.last_synced_sha = std::collections::HashMap::new();
    auto_pull.last_pull_status = None;
}

/// A dev workspace deploys to a branch named after its environment label. If a
/// git-sync repository's tracked branch carries that same name, dev deploys
/// would write straight into the branch the workspace (or its prod) syncs
/// from — the CLI refuses that push, so every deploy job would fail. Reject
/// the label up front instead.
async fn reject_dev_label_matching_tracked_branch(
    db: &DB,
    label: Option<&str>,
    workspace_ids: &[&str],
) -> Result<()> {
    let label_branch = windmill_common::workspaces::dev_workspace_branch(label);
    for w_id in workspace_ids {
        let Some(settings) = sqlx::query_scalar!(
            "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
            w_id
        )
        .fetch_optional(db)
        .await?
        .flatten()
        .and_then(|v| serde_json::from_value::<WorkspaceGitSyncSettings>(v).ok()) else {
            continue;
        };
        for repo in &settings.repositories {
            let path = repo.git_repo_resource_path.trim_start_matches("$res:");
            let branch: Option<String> = sqlx::query_scalar!(
                "SELECT value->>'branch' FROM resource WHERE workspace_id = $1 AND path = $2",
                w_id,
                path
            )
            .fetch_optional(db)
            .await?
            .flatten();
            if branch.as_deref() == Some(label_branch.as_str()) {
                return Err(Error::BadRequest(format!(
                    "The environment label '{label_branch}' matches the tracked branch of git-sync \
                     repository '{path}' in workspace '{w_id}': deploys from the dev workspace go \
                     to the '{label_branch}' branch and would overwrite the branch that repository \
                     syncs from. Use the other label or change the repository's tracked branch."
                )));
            }
        }
    }
    Ok(())
}

/// Reject parent-only git-sync settings on a fork workspace. Auto-pull, fork
/// PRs, and promotion mode are all configured at the parent: repo → fork sync is
/// routed by the parent's webhook/poller (`sync_forks`), a fork-owned auto-pull
/// would register a second webhook on the same GitHub repo per fork, and a
/// fork's deploys always go to its `wm-fork/**` branch so a promotion repo could
/// never take effect there.
async fn reject_parent_only_git_sync_settings_on_fork<'a>(
    db: &DB,
    w_id: &str,
    mut repos: impl Iterator<Item = &'a windmill_common::workspaces::GitRepositorySettings>,
) -> Result<()> {
    let offending = repos.find_map(|r| {
        if r.auto_pull.as_ref().is_some_and(|a| a.enabled) {
            Some("Auto-pull")
        } else if r.use_individual_branch.unwrap_or(false) {
            Some("Promotion mode")
        } else if r.fork_open_prs {
            Some("Opening PRs for fork deploys")
        } else {
            None
        }
    });
    let Some(offending) = offending else {
        return Ok(());
    };
    let parent = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten();
    if parent.is_some() || w_id.starts_with(windmill_common::workspaces::WM_FORK_PREFIX) {
        return Err(Error::BadRequest(format!(
            "{offending} cannot be configured on a fork workspace: it is managed from the parent workspace's git sync settings"
        )));
    }
    Ok(())
}

/// Persist only the reconciled webhook fields (id/secret/error/mode) for `changed`
/// repos, one targeted JSONB update per repo (same pattern as the EE auto-pull
/// status writer). The webhook reconcile runs after the main save has committed,
/// so a read-modify-write of the whole blob would clobber a poller status write
/// or another settings save landing in the gap. `mode` is carried too: the
/// reconcile normalizes webhook -> polling for repos that can't register hooks,
/// and losing that would leave the poller skipping a webhook-mode repo that has
/// no webhook. A repo whose `auto_pull` was concurrently removed is left alone.
#[cfg(all(feature = "enterprise", feature = "private"))]
async fn persist_reconciled_webhook_fields(
    db: &DB,
    w_id: &str,
    changed: &[(String, windmill_common::workspaces::AutoPullSettings)],
) -> Result<()> {
    for (path, new_ap) in changed {
        let mut patch = serde_json::Map::new();
        patch.insert(
            "mode".to_string(),
            serde_json::to_value(&new_ap.mode).map_err(|e| Error::internal_err(e.to_string()))?,
        );
        if let Some(id) = new_ap.webhook_id {
            patch.insert("webhook_id".to_string(), serde_json::json!(id));
        }
        if let Some(secret) = &new_ap.webhook_secret {
            patch.insert("webhook_secret".to_string(), serde_json::json!(secret));
        }
        if let Some(err) = &new_ap.webhook_error {
            patch.insert("webhook_error".to_string(), serde_json::json!(err));
        }
        let patch = serde_json::Value::Object(patch);
        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET git_sync = jsonb_set(
                git_sync,
                '{repositories}',
                (SELECT jsonb_agg(
                    CASE WHEN elem->>'git_repo_resource_path' = $2
                          AND jsonb_typeof(elem->'auto_pull') = 'object'
                        THEN jsonb_set(elem, '{auto_pull}',
                             ((elem->'auto_pull') - 'webhook_id' - 'webhook_secret' - 'webhook_error') || $3)
                        ELSE elem END)
                 FROM jsonb_array_elements(git_sync->'repositories') AS elem)
            )
            WHERE workspace_id = $1
              AND jsonb_typeof(git_sync->'repositories') = 'array'
            "#,
            w_id,
            path,
            patch,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

async fn get_settings(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<WorkspaceSettings> {
    // Admin-only: this struct contains OAuth secrets, GitHub App tokens, billing
    // info, and other admin-managed integration credentials. Non-admin callers
    // should use `get_public_settings`.
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = user_db.begin(&authed).await?;
    let settings = sqlx::query_as!(
        WorkspaceSettings,
        r#"
        SELECT
            workspace_id,
            slack_team_id,
            teams_team_id,
            teams_team_name,
            teams_team_guid,
            slack_name,
            slack_command_script,
            teams_command_script,
            slack_email,
            slack_oauth_client_id,
            slack_oauth_client_secret,
            customer_id,
            plan,
            webhook,
            deploy_to,
            ai_config,
            large_file_storage,
            datatable,
            ducklake,
            git_sync,
            deploy_ui,
            default_app,
            default_scripts,
            mute_critical_alerts,
            color,
            operator_settings,
            git_app_installations,
            auto_invite,
            error_handler,
            success_handler,
            public_app_execution_limit_per_minute
        FROM
            workspace_settings
        WHERE
            workspace_id = $1
        "#,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("getting settings: {e:#}")))?;

    let mut settings = not_found_if_none(settings, "workspace settings", &w_id)?;
    tx.commit().await?;

    if let Some(git_sync) = settings.git_sync.as_mut() {
        redact_git_sync_webhook_secrets(git_sync);
    }

    Ok(Json(settings))
}

async fn get_public_settings(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<WorkspacePublicSettings> {
    let mut tx = user_db.begin(&authed).await?;
    let settings = sqlx::query_as!(
        WorkspacePublicSettings,
        r#"
        SELECT
            workspace_id,
            slack_team_id,
            slack_name,
            teams_team_id,
            teams_team_name,
            teams_team_guid,
            mute_critical_alerts,
            deploy_ui,
            large_file_storage,
            datatable
        FROM
            workspace_settings
        WHERE
            workspace_id = $1
        "#,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("getting public settings: {e:#}")))?;

    let settings = not_found_if_none(settings, "workspace settings", &w_id)?;
    tx.commit().await?;

    Ok(Json(settings))
}

#[derive(Serialize, Debug)]
pub struct GitSyncDeployMode {
    /// At least one git-sync repository is configured for this workspace.
    pub configured: bool,
    /// Some configured repository has auto-pull enabled on an Enterprise-licensed
    /// instance, so Windmill deploys pushes to its tracked branch. Pushing any
    /// other branch does not deploy — match the pushed branch against
    /// `auto_pull_branches` before recommending `git push`.
    pub deploy_on_push: bool,
    /// Tracked branches of the auto-pull repositories (deduplicated). Empty unless
    /// the instance is licensed for auto-pull. `git push` deploys the current
    /// checkout only when its branch is listed here.
    pub auto_pull_branches: Vec<String>,
}

/// Non-admin endpoint so the CLI/agent can pick the deploy path (git push vs
/// `wmill sync push`) without reading the admin-only workspace settings. Exposes
/// only whether auto-pull is active and which branches it tracks — never
/// repository resource paths, credentials, or webhook config.
async fn get_git_sync_deploy_mode(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<GitSyncDeployMode> {
    let mut tx = user_db.begin(&authed).await?;
    let git_sync = sqlx::query_scalar!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("getting git_sync settings: {e:#}")))?;
    tx.commit().await?;

    let settings = not_found_if_none(git_sync, "workspace settings", &w_id)?.and_then(|v| {
        serde_json::from_value::<WorkspaceGitSyncSettings>(v)
            .map_err(|e| {
                tracing::warn!("git_sync deploy mode: settings deserialize failed for {w_id}: {e}")
            })
            .ok()
    });

    let Some(settings) = settings else {
        return Ok(Json(GitSyncDeployMode {
            configured: false,
            deploy_on_push: false,
            auto_pull_branches: vec![],
        }));
    };

    // Auto-pull only runs on Enterprise-licensed instances (see poll_git_auto_pull),
    // so a retained `enabled` flag on a CE build or downgraded workspace must not
    // advertise deploy-on-push.
    let licensed = matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Enterprise
    );

    let mut deploy_on_push = false;
    let mut auto_pull_branches: Vec<String> = Vec::new();
    if licensed {
        for repo in &settings.repositories {
            if !repo.auto_pull.as_ref().is_some_and(|a| a.enabled) {
                continue;
            }
            deploy_on_push = true;
            // Read only the tracked branch name (not the resource secret), on the
            // plain pool so a non-admin member without resource RLS access still
            // gets an accurate answer.
            let path = repo.git_repo_resource_path.trim_start_matches("$res:");
            let branch: Option<String> = sqlx::query_scalar!(
                "SELECT value->>'branch' FROM resource WHERE workspace_id = $1 AND path = $2",
                &w_id,
                path
            )
            .fetch_optional(&db)
            .await?
            .flatten();
            if let Some(branch) = branch {
                if !auto_pull_branches.contains(&branch) {
                    auto_pull_branches.push(branch);
                }
            }
        }
    }

    Ok(Json(GitSyncDeployMode {
        configured: !settings.repositories.is_empty(),
        deploy_on_push,
        auto_pull_branches,
    }))
}

async fn get_copilot_settings_state(
    _authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
) -> JsonResult<CopilotSettingsState> {
    let workspace_ai_config = sqlx::query_scalar!(
        "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| Error::internal_err(format!("getting workspace ai settings: {e:#}")))?;
    let workspace_ai_config = not_found_if_none(workspace_ai_config, "workspace settings", &w_id)?;
    let instance_ai_config: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT value FROM global_settings WHERE name = 'ai_config'")
            .fetch_optional(&db)
            .await
            .map_err(|e| Error::internal_err(format!("getting instance ai settings: {e:#}")))?;

    Ok(Json(build_copilot_settings_state(
        has_ai_providers(workspace_ai_config.as_ref()),
        instance_ai_config.as_ref(),
    )))
}

pub fn has_ai_providers(config: Option<&serde_json::Value>) -> bool {
    config
        .and_then(|value| value.get("providers"))
        .and_then(|providers| providers.as_object())
        .map(|providers| !providers.is_empty())
        .unwrap_or(false)
}

pub fn build_copilot_settings_state(
    has_workspace_ai_config: bool,
    instance_ai_config: Option<&serde_json::Value>,
) -> CopilotSettingsState {
    let has_instance_ai_config = has_ai_providers(instance_ai_config);
    CopilotSettingsState {
        has_instance_ai_config,
        uses_instance_ai_config: !has_workspace_ai_config && has_instance_ai_config,
        instance_ai_summary: build_instance_ai_summary(instance_ai_config),
    }
}

pub fn build_instance_ai_summary(config: Option<&serde_json::Value>) -> Option<InstanceAISummary> {
    let config = config?;
    if !has_ai_providers(Some(config)) {
        return None;
    }
    let providers = config.get("providers")?.as_object()?;

    let mut provider_summaries = providers
        .iter()
        .map(|(provider, provider_config)| InstanceAIProviderSummary {
            provider: provider.clone(),
            models: provider_config
                .get("models")
                .and_then(|models| models.as_array())
                .map(|models| {
                    models
                        .iter()
                        .filter_map(|model| model.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    provider_summaries.sort_by(|left, right| left.provider.cmp(&right.provider));

    Some(InstanceAISummary {
        providers: provider_summaries,
        default_model: extract_instance_ai_model_summary(config, "default_model"),
        metadata_model: extract_instance_ai_model_summary(config, "metadata_model"),
        code_completion_model: extract_instance_ai_model_summary(config, "code_completion_model"),
    })
}

fn extract_instance_ai_model_summary(
    config: &serde_json::Value,
    key: &str,
) -> Option<InstanceAIModelSummary> {
    let model_config = config.get(key)?.as_object()?;
    Some(InstanceAIModelSummary {
        provider: model_config.get("provider")?.as_str()?.to_owned(),
        model: model_config.get("model")?.as_str()?.to_owned(),
    })
}

#[derive(Serialize)]
struct DeployTo {
    deploy_to: Option<String>,
}
async fn get_deploy_to(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<DeployTo> {
    let mut tx = user_db.begin(&authed).await?;
    let settings = sqlx::query_as!(
        DeployTo,
        "SELECT deploy_to FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("getting deploy_to: {e:#}")))?;

    tx.commit().await?;
    Ok(Json(settings))
}

async fn edit_slack_command(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(es): Json<EditCommandScript>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;

    if es.slack_command_script.is_some() {
        let exists_slack_command_with_team_id = sqlx::query_scalar!(
            r#"
                SELECT EXISTS (SELECT 1
                FROM workspace_settings
                WHERE workspace_id <> $1
                    AND slack_command_script IS NOT NULL
                    AND slack_team_id IS NOT NULL
                    AND slack_team_id = (SELECT slack_team_id FROM workspace_settings WHERE workspace_id = $1))
            "#,
            &w_id
        )
        .fetch_one(&mut *tx)
        .await?.unwrap_or(false);

        if exists_slack_command_with_team_id {
            return Err(Error::BadRequest(
                "A workspace connected to the same slack team already has a command script. Please remove it first."
                    .to_string(),
            ));
        }
    }

    sqlx::query!(
        "UPDATE workspace_settings SET slack_command_script = $1 WHERE workspace_id = $2",
        es.slack_command_script,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_command_script",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [(
                "script",
                es.slack_command_script
                    .unwrap_or("NO_SCRIPT".to_string())
                    .as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit command script {}", &w_id))
}

async fn run_slack_message_test_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<RunSlackMessageTestJobRequest>,
) -> JsonResult<RunSlackMessageTestJobResponse> {
    let mut fake_result = HashMap::new();
    fake_result.insert("error".to_string(), to_raw_value(&req.test_msg));
    fake_result.insert("success_result".to_string(), to_raw_value(&req.test_msg));

    let mut extra_args = HashMap::new();
    extra_args.insert("channel".to_string(), to_raw_value(&req.channel));
    extra_args.insert(
        "slack".to_string(),
        to_raw_value(&format!("$res:{WORKSPACE_SLACK_BOT_TOKEN_PATH}")),
    );

    let uuid = windmill_queue::push_error_handler(
        &db,
        Uuid::parse_str("00000000-0000-0000-0000-000000000000")?,
        None,
        Some("slack_message_test".to_string()),
        false,
        w_id.as_str(),
        &format!("script/{}", req.hub_script_path.as_str()),
        sqlx::types::Json(&fake_result),
        None,
        Some(Utc::now()),
        Some(sqlx::types::Json(to_raw_value(&extra_args))),
        authed.email.as_str(),
        false,
        false,
        None, // Note: we could mark it as high priority to return result quickly to the user
    )
    .await?;

    Ok(Json(RunSlackMessageTestJobResponse {
        job_uuid: uuid.to_string(),
    }))
}

#[derive(Deserialize)]
struct SetSlackOAuthConfigRequest {
    slack_oauth_client_id: String,
    slack_oauth_client_secret: String,
}

#[derive(Serialize)]
struct GetSlackOAuthConfigResponse {
    slack_oauth_client_id: Option<String>,
    slack_oauth_client_secret: Option<String>,
}

async fn get_slack_oauth_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<GetSlackOAuthConfigResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    let settings = sqlx::query!(
        "SELECT slack_oauth_client_id, slack_oauth_client_secret FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await?;

    // Mask the secret if it exists
    let masked_secret = settings
        .slack_oauth_client_secret
        .map(|_| "***".to_string());

    Ok(Json(GetSlackOAuthConfigResponse {
        slack_oauth_client_id: settings.slack_oauth_client_id,
        slack_oauth_client_secret: masked_secret,
    }))
}

async fn set_slack_oauth_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<SetSlackOAuthConfigRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    if req.slack_oauth_client_id.is_empty() || req.slack_oauth_client_secret.is_empty() {
        return Err(Error::BadRequest(
            "Both client ID and client secret are required".to_string(),
        ));
    }

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace_settings
         SET slack_oauth_client_id = $1, slack_oauth_client_secret = $2
         WHERE workspace_id = $3",
        &req.slack_oauth_client_id,
        &req.slack_oauth_client_secret,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.set_slack_oauth_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("client_id", req.slack_oauth_client_id.as_str())].into()),
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "slack_oauth_config".to_string() },
        Some("Slack OAuth config set".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Slack OAuth config set for workspace {}", &w_id))
}

async fn delete_slack_oauth_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace_settings
         SET slack_oauth_client_id = NULL, slack_oauth_client_secret = NULL
         WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.delete_slack_oauth_config",
        ActionKind::Delete,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "slack_oauth_config".to_string() },
        Some("Slack OAuth config deleted".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!(
        "Slack OAuth config deleted for workspace {}",
        &w_id
    ))
}

#[derive(Deserialize)]
struct GetSecondaryStorageNamesQuery {
    #[serde(default)]
    include_default: bool,
}

async fn get_secondary_storage_names(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<GetSecondaryStorageNamesQuery>,
) -> JsonResult<Vec<String>> {
    let mut result: Vec<String> = sqlx::query_scalar!(
        "SELECT jsonb_object_keys(large_file_storage->'secondary_storage') AS \"secondary_storage_name!: _\"
        FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_all(&db)
    .await?;

    // If include_default is true, check if primary storage is set and add "_default_"
    if query.include_default {
        let has_primary_storage: Option<bool> = sqlx::query_scalar!(
            "SELECT (large_file_storage IS NOT NULL
                     AND large_file_storage != 'null'::jsonb
                     AND jsonb_typeof(large_file_storage) = 'object') AS \"has_primary!\"
            FROM workspace_settings WHERE workspace_id = $1",
            &w_id
        )
        .fetch_optional(&db)
        .await?;

        if has_primary_storage.unwrap_or(false) {
            result.insert(0, "_default_".to_string());
        }
    }

    Ok(Json(result))
}

#[cfg(feature = "enterprise")]
async fn edit_deploy_to(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(es): Json<EditDeployTo>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace_settings SET deploy_to = $1 WHERE workspace_id = $2",
        es.deploy_to,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_deploy_to",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [(
                "script",
                es.deploy_to.unwrap_or("NO_DEPLOY_TO".to_string()).as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "deploy_to".to_string() },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!("Edit deploy to for {}", &w_id))
}

#[cfg(not(feature = "enterprise"))]
async fn edit_deploy_to() -> Result<String> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

pub const BANNED_DOMAINS: &str = include_str!("../../windmill-api/banned_domains.txt");
pub const MAX_CUSTOM_PROMPT_LENGTH: usize = 5000;

async fn is_allowed_auto_domain(ApiAuthed { email, .. }: ApiAuthed) -> JsonResult<bool> {
    let domain = email.split('@').last().unwrap();
    return Ok(Json(!BANNED_DOMAINS.contains(domain)));
}

async fn edit_auto_invite(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(ea): Json<EditAutoInvite>,
) -> Result<String> {
    crate::workspaces_oss::edit_auto_invite(authed, db, w_id, ea).await
}

#[cfg(feature = "private")]
async fn edit_instance_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(config): Json<crate::workspaces_ee::EditInstanceGroups>,
) -> Result<String> {
    crate::workspaces_ee::edit_instance_groups(authed, db, w_id, config).await
}

#[cfg(not(feature = "private"))]
async fn edit_instance_groups(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
    Json(_config): Json<serde_json::Value>,
) -> Result<String> {
    Err(Error::BadRequest(
        "Instance groups are only available on Windmill Enterprise Edition".to_string(),
    ))
}

async fn edit_webhook(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ew): Json<EditWebhook>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    if *CLOUD_HOSTED {
        return Err(Error::BadRequest(
            "Workspace webhooks are not available on cloud-hosted instances".to_string(),
        ));
    }

    let mut tx = db.begin().await?;

    if let Some(webhook) = &ew.webhook {
        sqlx::query!(
            "UPDATE workspace_settings SET webhook = $1 WHERE workspace_id = $2",
            webhook,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET webhook = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_webhook",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("webhook", &format!("{:?}", ew.webhook)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "webhook".to_string() },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!("Edit webhook for workspace {}", &w_id))
}

async fn edit_large_file_storage_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<EditLargeFileStorageConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    let args_for_audit = format!("{:?}", new_config.large_file_storage);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_large_file_storage_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("large_file_storage", args_for_audit.as_str())].into()),
    )
    .await?;

    if let Some(lfs_config) = new_config.large_file_storage {
        let serialized_lfs_config =
            serde_json::to_value::<LargeFileStorageWithSecondary>(lfs_config)
                .map_err(|err| Error::internal_err(err.to_string()))?;

        sqlx::query!(
            "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
            serialized_lfs_config,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET large_file_storage = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // Trigger git sync for large file storage changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings {
            setting_type: "large_file_storage".to_string(),
        },
        Some("Large file storage configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!(
        "Edit large file storage config for workspace {}",
        &w_id
    ))
}

async fn list_ducklakes(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let ducklakes = sqlx::query_scalar!(
        r#"
            SELECT jsonb_object_keys(ws.ducklake->'ducklakes') AS ducklake_name
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .filter_map(|s| s)
    .collect();

    Ok(Json(ducklakes))
}

#[derive(Serialize)]
struct DataTableListItem {
    name: String,
    resource_type: String,
    resource_path: String,
}

async fn list_datatables(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DataTableListItem>> {
    let config = sqlx::query_scalar!(
        "SELECT datatable->'datatables' FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await?;

    let items: Vec<DataTableListItem> = match config {
        Some(val) => {
            let map: HashMap<String, DataTable> = serde_json::from_value(val).unwrap_or_default();
            map.into_iter()
                .map(|(name, dt)| DataTableListItem {
                    name,
                    resource_type: dt.database.resource_type.as_ref().to_string(),
                    resource_path: dt.database.resource_path,
                })
                .collect()
        }
        None => vec![],
    };

    Ok(Json(items))
}

/// Compact column representation: "type" or "type?" for nullable, with "=default" suffix if has default
type CompactColumn = String;

/// Columns mapped by name to their compact type
type ColumnMap = HashMap<String, CompactColumn>;

/// Tables mapped by name to their columns
type TableMap = HashMap<String, ColumnMap>;

/// Schemas mapped by name to their tables
type SchemaMap = HashMap<String, TableMap>;

/// Schemas mapped by name to their table names
type TableListMap = HashMap<String, Vec<String>>;

#[derive(Serialize, Debug)]
struct DataTableSchema {
    datatable_name: String,
    /// Hierarchical schema: schema_name -> table_name -> column_name -> "type[?][=default]"
    schemas: SchemaMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Debug)]
struct DataTableTables {
    datatable_name: String,
    /// Hierarchical metadata: schema_name -> table_names
    schemas: TableListMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Deserialize)]
struct GetDataTableSchemaQuery {
    datatable_name: String,
    schema_name: String,
    table_name: String,
}

#[derive(Serialize, Debug)]
struct DataTableTableSchema {
    datatable_name: String,
    schema_name: String,
    table_name: String,
    columns: ColumnMap,
}

async fn list_datatable_schemas(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DataTableSchema>> {
    let datatable_names = list_datatable_names(&db, &w_id).await?;
    let mut results = Vec::new();

    for datatable_name in datatable_names {
        let schema = match get_datatable_schema(&db, &w_id, &datatable_name).await {
            Ok(schemas) => DataTableSchema { datatable_name, schemas, error: None },
            Err(e) => DataTableSchema {
                datatable_name,
                schemas: HashMap::new(),
                error: Some(e.to_string()),
            },
        };
        results.push(schema);
    }

    Ok(Json(results))
}

async fn list_datatable_tables(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DataTableTables>> {
    let datatable_names = list_datatable_names(&db, &w_id).await?;
    let mut results = Vec::new();

    for datatable_name in datatable_names {
        let tables = match get_datatable_tables(&db, &w_id, &datatable_name).await {
            Ok(schemas) => DataTableTables { datatable_name, schemas, error: None },
            Err(e) => DataTableTables {
                datatable_name,
                schemas: HashMap::new(),
                error: Some(e.to_string()),
            },
        };
        results.push(tables);
    }

    Ok(Json(results))
}

async fn get_datatable_table_schema(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<GetDataTableSchemaQuery>,
) -> JsonResult<DataTableTableSchema> {
    let columns = get_datatable_table_columns(
        &db,
        &w_id,
        &query.datatable_name,
        &query.schema_name,
        &query.table_name,
    )
    .await?;

    Ok(Json(DataTableTableSchema {
        datatable_name: query.datatable_name,
        schema_name: query.schema_name,
        table_name: query.table_name,
        columns,
    }))
}

async fn list_datatable_names(db: &DB, w_id: &str) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar!(
        r#"
            SELECT jsonb_object_keys(ws.datatable->'datatables') AS datatable_name
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        w_id
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .filter_map(|s| s)
    .collect())
}

async fn get_datatable_schema(db: &DB, w_id: &str, datatable_name: &str) -> Result<SchemaMap> {
    // Get the datatable resource (connection credentials)
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;

    // Parse the resource as PgDatabase
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;

    // Connect to the datatable database
    let (client, connection) = pg_db.connect(Some(db)).await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable connection error: {}", e);
        }
    });

    // First, get all non-system schemas (including empty ones)
    let schema_rows = client
        .query(
            r#"
            SELECT nspname::text AS schema_name
            FROM pg_namespace
            WHERE nspname NOT IN ('information_schema', 'pg_toast', 'pg_catalog')
              AND nspname NOT LIKE 'pg_%'
            ORDER BY nspname
            "#,
            &[],
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to query schemas: {}", e)))?;

    // Build hierarchical structure: schema -> table -> column -> compact_type
    let mut schema_map: SchemaMap = HashMap::new();

    // Collect schema names and initialize map
    let schema_names: Vec<String> = schema_rows
        .iter()
        .map(|row| {
            let name: String = row.get(0);
            schema_map.entry(name.clone()).or_default();
            name
        })
        .collect();

    // Query column information only for the schemas we found
    let rows = client
        .query(
            r#"
            SELECT
                table_schema::text,
                table_name::text,
                column_name::text,
                udt_name::text,
                is_nullable::text,
                column_default::text
            FROM information_schema.columns
            WHERE table_schema = ANY($1)
              AND table_name IS NOT NULL
            ORDER BY table_schema, table_name, ordinal_position
            "#,
            &[&schema_names],
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to query columns: {}", e)))?;

    for row in rows {
        let table_schema: String = row.get(0);
        let table_name: String = row.get(1);
        let column_name: String = row.get(2);
        let udt_name: String = row.get(3);
        let is_nullable: String = row.get(4);
        let column_default: Option<String> = row.get(5);

        schema_map
            .entry(table_schema)
            .or_default()
            .entry(table_name)
            .or_default()
            .insert(
                column_name,
                compact_column_type(udt_name, is_nullable, column_default),
            );
    }

    Ok(schema_map)
}

async fn get_datatable_tables(db: &DB, w_id: &str, datatable_name: &str) -> Result<TableListMap> {
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;
    let (client, connection) = pg_db.connect(Some(db)).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable connection error: {}", e);
        }
    });

    let schema_rows = client
        .query(
            r#"
            SELECT nspname::text AS schema_name
            FROM pg_namespace
            WHERE nspname NOT IN ('information_schema', 'pg_toast', 'pg_catalog')
              AND nspname NOT LIKE 'pg_%'
            ORDER BY nspname
            "#,
            &[],
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to query schemas: {}", e)))?;

    let mut table_map: TableListMap = HashMap::new();
    let schema_names: Vec<String> = schema_rows
        .iter()
        .map(|row| {
            let name: String = row.get(0);
            table_map.entry(name.clone()).or_default();
            name
        })
        .collect();

    let rows = client
        .query(
            r#"
            SELECT DISTINCT
                table_schema::text,
                table_name::text
            FROM information_schema.columns
            WHERE table_schema = ANY($1)
              AND table_name IS NOT NULL
            ORDER BY table_schema, table_name
            "#,
            &[&schema_names],
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to query tables: {}", e)))?;

    for row in rows {
        let table_schema: String = row.get(0);
        let table_name: String = row.get(1);
        table_map.entry(table_schema).or_default().push(table_name);
    }

    Ok(table_map)
}

async fn get_datatable_table_columns(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    schema_name: &str,
    table_name: &str,
) -> Result<ColumnMap> {
    if is_system_pg_schema(schema_name) {
        return Err(Error::BadRequest(format!(
            "Schema '{}' is not available for datatable schema lookup",
            schema_name
        )));
    }

    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;
    let (client, connection) = pg_db.connect(Some(db)).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable connection error: {}", e);
        }
    });

    let rows = client
        .query(
            r#"
            SELECT
                column_name::text,
                udt_name::text,
                is_nullable::text,
                column_default::text
            FROM information_schema.columns
            WHERE table_schema = $1
              AND table_name = $2
            ORDER BY ordinal_position
            "#,
            &[&schema_name, &table_name],
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to query columns: {}", e)))?;

    if rows.is_empty() {
        return Err(Error::NotFound(format!(
            "Table '{}.{}' not found in datatable '{}'",
            schema_name, table_name, datatable_name
        )));
    }

    let mut columns: ColumnMap = HashMap::new();
    for row in rows {
        let column_name: String = row.get(0);
        let udt_name: String = row.get(1);
        let is_nullable: String = row.get(2);
        let column_default: Option<String> = row.get(3);
        columns.insert(
            column_name,
            compact_column_type(udt_name, is_nullable, column_default),
        );
    }

    Ok(columns)
}

fn is_system_pg_schema(schema_name: &str) -> bool {
    // Match the datatable listing filter: PostgreSQL reserves pg_* schemas for system use.
    matches!(
        schema_name,
        "information_schema" | "pg_toast" | "pg_catalog"
    ) || schema_name.starts_with("pg_")
}

fn compact_column_type(
    udt_name: String,
    is_nullable: String,
    column_default: Option<String>,
) -> String {
    let mut compact = udt_name;
    if is_nullable == "YES" {
        compact.push('?');
    }
    if let Some(default) = column_default {
        compact.push('=');
        compact.push_str(&truncate_column_default(default));
    }
    compact
}

fn truncate_column_default(default: String) -> String {
    const MAX_DEFAULT_CHARS: usize = 30;
    const TRUNCATED_DEFAULT_CHARS: usize = 27;

    if default.chars().count() > MAX_DEFAULT_CHARS {
        format!(
            "{}...",
            default
                .chars()
                .take(TRUNCATED_DEFAULT_CHARS)
                .collect::<String>()
        )
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_column_type_truncates_multibyte_defaults_safely() {
        let default = "é".repeat(31);

        assert_eq!(
            compact_column_type("text".to_string(), "NO".to_string(), Some(default)),
            format!("text={}...", "é".repeat(27))
        );
    }

    // A real NUL can't live in Rust source, so build `{"k":"<n backslashes>u0000"}`
    // by repeating backslashes: an ODD run before `u0000` is a genuine NUL escape,
    // an EVEN run is an escaped backslash then the literal text "u0000".
    fn nul_json(backslashes: usize) -> String {
        format!(r#"{{"k":"{}u0000"}}"#, "\\".repeat(backslashes))
    }

    #[test]
    fn nul_escape_detected_only_for_odd_backslash_runs() {
        // 1 backslash: `\u0000` — a genuine NUL escape.
        assert!(json_text_has_nul_escape(&nul_json(1)));
        // 3 backslashes: escaped backslash + genuine NUL escape.
        assert!(json_text_has_nul_escape(&nul_json(3)));
        // 2 backslashes: escaped backslash then literal "u0000" (e.g. minified JS) — safe.
        assert!(!json_text_has_nul_escape(&nul_json(2)));
        // 0 backslashes: the bare token "u0000" — safe.
        assert!(!json_text_has_nul_escape(&nul_json(0)));
    }

    #[test]
    fn nul_escape_ignores_clean_values() {
        assert!(!json_text_has_nul_escape(
            r#"{"files":{"/index.tsx":"hello"}}"#
        ));
        assert!(!json_text_has_nul_escape(""));
        // A later genuine NUL is still caught even after an earlier even (safe) run.
        assert!(json_text_has_nul_escape(&format!(
            r#"{{"a":"x{b}{b}u0000y","b":"z{b}u0000"}}"#,
            b = "\\"
        )));
    }
}

/// Resolve a source string to PgDatabase credentials with user-scoped permission checks.
/// For `datatable://name`: accessible to everyone (variables are resolved internally).
/// For `$res:path`: uses UserDB (row-level security) to verify the user can see the resource,
/// then interpolates `$var:` references in the resource value.
pub(crate) async fn resolve_pg_source_checked(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    source: &str,
) -> Result<PgDatabase> {
    let db_resource = if let Some(name) = source.strip_prefix("datatable://") {
        get_datatable_resource_from_db_unchecked(db, w_id, name).await?
    } else if let Some(path) = source.strip_prefix("$res:") {
        let db_with_authed = windmill_common::db::DbWithOptAuthed::from_authed(
            authed,
            db.clone(),
            Some(user_db.clone()),
        );
        let value = windmill_store::resources::get_resource_value_interpolated_internal(
            &db_with_authed,
            w_id,
            path,
            None,
            None,
            false,
        )
        .await?;

        match value {
            Some(v) => v,
            None => {
                return Err(Error::NotAuthorized(format!(
                    "Resource '{}' not found or you do not have access to it",
                    path
                )));
            }
        }
    } else {
        return Err(Error::BadRequest(format!(
            "Invalid source format: '{}'. Expected 'datatable://name' or '$res:path'",
            source
        )));
    };

    serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))
}

/// A temporary file for pg_dump output that is automatically deleted when dropped.
pub(crate) struct DumpFile {
    pub(crate) path: std::path::PathBuf,
}

impl DumpFile {
    fn new() -> Result<Self> {
        let dir = std::path::Path::new("/tmp/windmill");
        std::fs::create_dir_all(dir)
            .map_err(|e| Error::internal_err(format!("Failed to create /tmp/windmill: {}", e)))?;
        // Set directory permissions to owner-only
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        let path = dir.join(format!("datatable_dump_{}", uuid::Uuid::new_v4()));
        // Create the file with restrictive permissions before pg_dump writes to it
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .mode(0o600)
                .open(&path)
                .map_err(|e| Error::internal_err(format!("Failed to create dump file: {}", e)))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::File::create(&path)
                .map_err(|e| Error::internal_err(format!("Failed to create dump file: {}", e)))?;
        }
        Ok(Self { path })
    }
}

impl Drop for DumpFile {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                tracing::warn!("Failed to remove dump file {:?}: {}", self.path, e);
            }
        }
    }
}

/// Run pg_dump against a PgDatabase, writing output to a temp file on disk.
/// Returns a DumpFile handle; the file is deleted when the handle is dropped.
pub(crate) async fn pg_dump_database(
    pg_db: &PgDatabase,
    schema_only: bool,
    exclude_tables: &[&str],
) -> Result<DumpFile> {
    let dump_file = DumpFile::new()?;

    let host = &pg_db.host;
    let port = pg_db.port.unwrap_or(5432).to_string();
    let user = pg_db.user.as_deref().unwrap_or("postgres");
    let dbname = &pg_db.dbname;

    let mut cmd = tokio::process::Command::new("pg_dump");
    cmd.arg("--format=plain").arg("--file").arg(&dump_file.path);
    if schema_only {
        cmd.arg("--schema-only");
    }
    for table in exclude_tables {
        cmd.arg(format!("--exclude-table={table}"));
    }
    cmd.arg("--host")
        .arg(host)
        .arg("--port")
        .arg(&port)
        .arg("--username")
        .arg(user)
        .arg(dbname);

    if let Some(ref password) = pg_db.password {
        cmd.env("PGPASSWORD", password);
    }

    if let Some(ref sslmode) = pg_db.sslmode {
        cmd.env("PGSSLMODE", sslmode);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to execute pg_dump: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::internal_err(format!("pg_dump failed: {}", stderr)));
    }

    Ok(dump_file)
}

/// Import a pg_dump file into a target database using psql.
async fn pg_import_dump(target_db: &PgDatabase, dump_file: &DumpFile) -> Result<()> {
    let host = &target_db.host;
    let port = target_db.port.unwrap_or(5432).to_string();
    let user = target_db.user.as_deref().unwrap_or("postgres");
    let dbname = &target_db.dbname;

    let mut cmd = tokio::process::Command::new("psql");
    cmd.arg("--host")
        .arg(host)
        .arg("--port")
        .arg(&port)
        .arg("--username")
        .arg(user)
        .arg("--dbname")
        .arg(dbname)
        .arg("--no-psqlrc")
        .arg("--file")
        .arg(&dump_file.path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(ref password) = target_db.password {
        cmd.env("PGPASSWORD", password);
    }

    if let Some(ref sslmode) = target_db.sslmode {
        cmd.env("PGSSLMODE", sslmode);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to execute psql: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::internal_err(format!(
            "psql import failed: {}",
            stderr
        )));
    }

    Ok(())
}

#[derive(Deserialize)]
struct CreatePgDatabaseRequest {
    /// The datatable source to determine connection info: 'datatable://name' or '$res:path'
    source: String,
    /// Name for the new database
    target_dbname: String,
}

/// Create a new PostgreSQL database. For instance datatables, creates on the Windmill PG instance.
/// For resource datatables, creates on the same server as the source.
async fn create_pg_database(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<CreatePgDatabaseRequest>,
) -> Result<String> {
    windmill_common::validate_dbname(&req.target_dbname)?;

    // Non-superadmin: restrict dbname to wm_fork_ prefix
    if !windmill_common::auth::is_super_admin_email(&db, &authed.email).await? {
        if !req.target_dbname.starts_with("wm_fork_") {
            return Err(Error::BadRequest(
                "Non-superadmin users can only create databases with names starting with 'wm_fork_'"
                    .to_string(),
            ));
        }
    }

    // Determine if this is an instance or resource-backed datatable
    let is_instance_datatable = if let Some(dt_name) = req.source.strip_prefix("datatable://") {
        let config = sqlx::query_scalar!(
            "SELECT datatable->'datatables'->$2 FROM workspace_settings WHERE workspace_id = $1",
            &w_id,
            dt_name
        )
        .fetch_optional(&db)
        .await?
        .flatten();
        config
            .and_then(|v| {
                v.get("database")
                    .and_then(|d| d.get("resource_type"))
                    .and_then(|r| r.as_str())
                    .map(|s| s == "instance")
            })
            .unwrap_or(false)
    } else {
        false
    };

    if is_instance_datatable {
        windmill_common::create_custom_instance_database(&db, &req.target_dbname, "datatable")
            .await?;
    } else {
        let source_pg =
            resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &req.source).await?;
        let (client, connection) = source_pg.connect(Some(&db)).await?;
        let join_handle = tokio::spawn(async move { connection.await });

        let row = client
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1)",
                &[&req.target_dbname],
            )
            .await
            .map_err(|e| {
                Error::internal_err(format!("Failed to check database existence: {}", e))
            })?;
        let db_exists: bool = row.get(0);

        if db_exists {
            drop(client);
            let _ = join_handle.await;
            return Err(Error::BadRequest(format!(
                "Database '{}' already exists on the resource server",
                req.target_dbname
            )));
        }

        client
            .execute(&format!("CREATE DATABASE \"{}\"", &req.target_dbname), &[])
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to create database '{}': {}",
                    req.target_dbname, e
                ))
            })?;

        drop(client);
        join_handle
            .await
            .map_err(|e| Error::internal_err(format!("join error: {}", e)))?
            .map_err(|e| Error::internal_err(format!("tokio_postgres error: {}", e)))?;
    }

    Ok(format!("Created database '{}'", req.target_dbname))
}

#[derive(Deserialize)]
struct ImportPgDatabaseRequest {
    source: String,
    target: String,
    #[serde(default)]
    target_dbname_override: Option<String>,
    fork_behavior: DataTableForkBehavior,
}

/// Import (pg_dump/pg_import) from source to target
async fn import_pg_database(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<ImportPgDatabaseRequest>,
) -> Result<String> {
    if req.fork_behavior == DataTableForkBehavior::KeepOriginal {
        return Ok("No action needed for KeepOriginal behavior".to_string());
    }

    if req.fork_behavior == DataTableForkBehavior::SchemaAndData {
        require_admin(authed.is_admin, &authed.username)?;
        if *CLOUD_HOSTED {
            return Err(Error::BadRequest(
                "Importing schema and data is not available on cloud".to_string(),
            ));
        }
    }

    let schema_only = req.fork_behavior == DataTableForkBehavior::SchemaOnly;
    let source_pg = resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &req.source).await?;
    let mut target_pg =
        resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &req.target).await?;

    if let Some(ref override_dbname) = req.target_dbname_override {
        if !windmill_common::auth::is_super_admin_email(&db, &authed.email).await? {
            if !override_dbname.starts_with("wm_fork_") {
                return Err(Error::BadRequest(
                    "Non-superadmin users can only override target dbname with names starting with 'wm_fork_'"
                        .to_string(),
                ));
            }
        }
        target_pg.dbname = override_dbname.clone();
    }
    windmill_common::validate_dbname(&target_pg.dbname)?;

    let dump_file = pg_dump_database(&source_pg, schema_only, &[]).await?;
    pg_import_dump(&target_pg, &dump_file).await?;

    Ok(format!(
        "Imported from '{}' into '{}'",
        req.source, target_pg.dbname
    ))
}

#[derive(Deserialize)]
struct ExportPgSchemaRequest {
    source: String,
}

async fn export_pg_schema(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<ExportPgSchemaRequest>,
) -> Result<String> {
    let pg = resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &req.source).await?;
    let dump_file = pg_dump_database(&pg, true, &[]).await?;
    tokio::fs::read_to_string(&dump_file.path)
        .await
        .map_err(|e| Error::internal_err(format!("Failed to read dump file: {}", e)))
}

#[derive(Deserialize)]
struct GetDatatableFullSchemaRequest {
    source: String,
}

async fn get_datatable_full_schema(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<GetDatatableFullSchemaRequest>,
) -> JsonResult<windmill_common::query_builders::FullDatabaseSchema> {
    let pg = resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &req.source).await?;
    let (client, connection) = pg.connect(Some(&db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });

    let result = windmill_common::query_builders::pg_get_full_schema(&client)
        .await
        .map_err(Error::internal_err)?;

    drop(client);
    join_handle
        .await
        .map_err(|e| Error::internal_err(format!("join error: {}", e)))?
        .map_err(|e| Error::internal_err(format!("tokio_postgres error: {}", e)))?;

    Ok(Json(result))
}

async fn edit_ducklake_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, email, .. }: ApiAuthed,
    Json(new_config): Json<EditDucklakeConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let is_superadmin = require_super_admin(&db, &email).await.is_ok();

    // Lake names end up interpolated in `ATTACH 'ducklake://<name>'`,
    // generated maintenance SQL and the reserved maintenance schedule path
    // (CHECK-constrained to [\w-]+ segments).
    for name in new_config.settings.ducklakes.keys() {
        if !windmill_common::workspaces::is_valid_ducklake_name(name) {
            return Err(Error::BadRequest(format!(
                "Invalid ducklake name '{name}': only letters, digits, '_' and '-' are allowed"
            )));
        }
    }

    let mut tx = db.begin().await?;

    let args_for_audit = format!("{:?}", new_config.settings);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_ducklake_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("ducklake", args_for_audit.as_str())].into()),
    )
    .await?;

    let old_ducklakes = sqlx::query_scalar!(
        r#"
            SELECT ws.ducklake->'ducklakes' AS ducklake_name
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(serde_json::Value::Null);
    let old_ducklakes: HashMap<String, Ducklake> =
        serde_json::from_value(old_ducklakes).unwrap_or_default();

    // Check that non-superadmins are not abusing Instance databases
    if !is_superadmin {
        for (name, dl) in new_config.settings.ducklakes.iter() {
            if dl.catalog.resource_type == DucklakeCatalogResourceType::Instance {
                let old_dl = old_ducklakes.get(name);
                if old_dl.is_none()
                    || old_dl.unwrap().catalog.resource_type
                        != DucklakeCatalogResourceType::Instance
                    || old_dl.unwrap().catalog.resource_path != dl.catalog.resource_path
                {
                    return Err(Error::BadRequest(
                        "Only superadmins can create or modify ducklakes with Instance databases"
                            .to_string(),
                    ));
                }
            }
        }
    }

    let config: serde_json::Value = serde_json::to_value(&new_config.settings)
        .map_err(|err| Error::internal_err(err.to_string()))?;

    sqlx::query!(
        "UPDATE workspace_settings SET ducklake = $1 WHERE workspace_id = $2",
        config,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    // Same tx as the settings update: a failed schedule sync/push must fail
    // the whole save — nothing reconciles a half-applied state later.
    let tx = windmill_queue::ducklake_maintenance::sync_ducklake_maintenance_schedules(
        &db,
        tx,
        &w_id,
        &new_config.settings.ducklakes,
        &old_ducklakes,
        &username,
        &email,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Edit ducklake config for workspace {}", &w_id))
}

async fn edit_datatable_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, email, .. }: ApiAuthed,
    Json(mut new_config): Json<EditDataTableConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let is_superadmin = require_super_admin(&db, &email).await.is_ok();

    let mut tx = db.begin().await?;

    let old_datatables: HashMap<String, DataTable> = serde_json::from_value(
        sqlx::query_scalar!(
            "SELECT ws.datatable->'datatables' FROM workspace_settings ws WHERE ws.workspace_id = $1",
            &w_id
        )
        .fetch_one(&db)
        .await?
        .unwrap_or(serde_json::Value::Null),
    )
    .unwrap_or_default();

    // Validate every persisted data table name and rename segment before
    // touching anything, since they become directory segments in migration
    // storage/export keys (`migrations/datatable/<name>/...`).
    for name in new_config.settings.datatables.keys() {
        crate::datatable_migrations::validate_datatable_path_segment(name)?;
    }
    for r in &new_config.renames {
        crate::datatable_migrations::validate_datatable_path_segment(&r.from)?;
        crate::datatable_migrations::validate_datatable_path_segment(&r.to)?;
    }

    // Map new name -> old name so a renamed data table inherits the previous
    // flag instead of being treated as brand new.
    let rename_src: HashMap<&str, &str> = new_config
        .renames
        .iter()
        .map(|r| (r.to.as_str(), r.from.as_str()))
        .collect();

    // Migrations opt-in is owned by the enable/disable endpoints, not this config
    // form: preserve each existing data table's flag, and default brand-new data
    // tables to enabled.
    for (name, dt) in new_config.settings.datatables.iter_mut() {
        let lookup = rename_src
            .get(name.as_str())
            .copied()
            .unwrap_or(name.as_str());
        dt.migrations_enabled = match old_datatables.get(lookup) {
            Some(old) => old.migrations_enabled,
            None => Some(true),
        };
    }

    let args_for_audit = format!("{:?}", new_config.settings);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_datatable_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("datatable", args_for_audit.as_str())].into()),
    )
    .await?;

    // Check that non-superadmins are not abusing Instance databases
    if !is_superadmin {
        for (name, dt) in new_config.settings.datatables.iter() {
            if dt.database.resource_type == DataTableCatalogResourceType::Instance {
                let old_dt = old_datatables.get(name);
                if old_dt.is_none()
                    || old_dt.unwrap().database.resource_type
                        != DataTableCatalogResourceType::Instance
                    || old_dt.unwrap().database.resource_path != dt.database.resource_path
                {
                    return Err(Error::BadRequest(
                        "Only superadmins can create or modify data tables with Instance databases"
                            .to_string(),
                    ));
                }
            }
        }
    }

    let config: serde_json::Value = serde_json::to_value(new_config.settings)
        .map_err(|err| Error::internal_err(err.to_string()))?;

    sqlx::query!(
        "UPDATE workspace_settings SET datatable = $1 WHERE workspace_id = $2",
        config,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    crate::datatable_migrations::cascade_datatable_migration_renames_and_deletes(
        &db,
        &mut tx,
        &w_id,
        &new_config.renames,
        &new_config.deleted_datatables,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Edit datatable config for workspace {}", &w_id))
}

#[derive(Deserialize)]
pub struct EditGitSyncConfig {
    pub git_sync_settings: Option<WorkspaceGitSyncSettings>,
}

#[derive(Deserialize, Debug)]
pub struct EditGitSyncRepository {
    pub git_repo_resource_path: String,
    pub repository: GitRepositorySettings,
}

#[derive(Deserialize, Debug)]
pub struct DeleteGitSyncRepositoryRequest {
    pub git_repo_resource_path: String,
}

fn validate_git_repo_resource_path(path: &str) -> Result<()> {
    // Resource paths should follow the pattern: $res:f/<folder>/<name> or $res:u/<username>/<name>
    if path.is_empty() {
        return Err(Error::BadRequest(
            "Resource path cannot be empty".to_string(),
        ));
    }

    // Must start with $res: prefix
    if !path.starts_with("$res:") {
        return Err(Error::BadRequest(
            "Resource path must start with '$res:'".to_string(),
        ));
    }

    // Extract the actual path after $res:
    let actual_path = &path[5..]; // Remove "$res:" prefix

    // Basic validation: must start with f/ or u/ and contain at least one slash
    if !actual_path.starts_with("f/") && !actual_path.starts_with("u/") {
        return Err(Error::BadRequest(
            "Resource path must start with '$res:f/' or '$res:u/'".to_string(),
        ));
    }

    // Must have at least 3 parts (type, folder/user, name)
    let parts: Vec<&str> = actual_path.split('/').collect();
    if parts.len() < 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(Error::BadRequest(
            "Invalid resource path format".to_string(),
        ));
    }

    // Resource name validation (last part)
    let resource_name = parts.last().unwrap();
    if !resource_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::BadRequest(
            "Resource name can only contain alphanumeric characters, underscores, and hyphens"
                .to_string(),
        ));
    }

    Ok(())
}

fn cleanup_legacy_git_sync_settings_in_memory(
    git_sync_settings: &mut windmill_common::workspaces::WorkspaceGitSyncSettings,
    workspace_id: &str,
) {
    // Check if all repositories are in new format (have settings field)
    let all_repos_migrated = git_sync_settings
        .repositories
        .iter()
        .all(|repo| repo.settings.is_some());

    // If all repos are migrated and we still have legacy workspace-level settings
    if all_repos_migrated
        && (git_sync_settings.include_path.is_some() || git_sync_settings.include_type.is_some())
    {
        tracing::info!(
            workspace_id = workspace_id,
            "All git sync repositories migrated to new format, cleaning up legacy workspace-level settings"
        );

        // Remove workspace-level legacy fields
        git_sync_settings.include_path = None;
        git_sync_settings.include_type = None;
    }
}

#[cfg(not(feature = "enterprise"))]
const CE_GIT_SYNC_MAX_USERS: i64 = 2;

/// Auto-pull is licensed per plan, not just per build: the poller only serves
/// Enterprise plans at runtime, so the save path must reject the setting too —
/// otherwise an EE binary without the plan could still register a webhook and
/// receive webhook-driven pulls.
#[cfg(feature = "enterprise")]
async fn check_git_sync_ee_license(feature: &str) -> Result<()> {
    if !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Enterprise
    ) {
        return Err(Error::BadRequest(format!(
            "{feature} requires an Enterprise license"
        )));
    }
    Ok(())
}

#[cfg(feature = "enterprise")]
async fn check_auto_pull_license() -> Result<()> {
    check_git_sync_ee_license("Automatic pull from git").await
}

/// In-app PR creation (promotion/fork deploy branches) drives GitHub API calls
/// from the deploy completion hook; runtime-gate it like auto-pull.
#[cfg(feature = "enterprise")]
async fn check_open_prs_license<'a>(
    mut repos: impl Iterator<Item = &'a windmill_common::workspaces::GitRepositorySettings>,
) -> Result<()> {
    if repos.any(|r| r.promotion_open_prs || r.fork_open_prs) {
        check_git_sync_ee_license("Opening pull requests from Windmill").await?;
    }
    Ok(())
}

#[cfg(feature = "enterprise")]
async fn check_git_sync_access(_db: &DB, _w_id: &str) -> Result<()> {
    Ok(())
}

// Anchor the CE-only query for `cargo sqlx prepare` (which runs with --features enterprise)
#[cfg(feature = "enterprise")]
#[allow(dead_code)]
async fn _sqlx_anchor_ce_user_count(db: &DB, w_id: &str) {
    let _ = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM usr WHERE workspace_id = $1 AND disabled = false",
        w_id
    )
    .fetch_one(db)
    .await;
}

#[cfg(not(feature = "enterprise"))]
async fn check_git_sync_access(db: &DB, w_id: &str) -> Result<()> {
    let user_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM usr WHERE workspace_id = $1 AND disabled = false",
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    if user_count > CE_GIT_SYNC_MAX_USERS {
        return Err(Error::BadRequest(format!(
            "Git sync is available for workspaces with up to {} members. \
             Upgrade to Windmill Enterprise Edition for unlimited workspace members.",
            CE_GIT_SYNC_MAX_USERS
        )));
    }
    Ok(())
}

#[cfg(feature = "enterprise")]
async fn get_git_sync_enabled(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
) -> JsonResult<serde_json::Value> {
    Ok(Json(serde_json::json!({
        "enabled": true,
        "reason": "enterprise",
        "max_repos": null,
        "user_count": null,
        "max_users": null,
    })))
}

#[cfg(not(feature = "enterprise"))]
async fn get_git_sync_enabled(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<serde_json::Value> {
    let user_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM usr WHERE workspace_id = $1 AND disabled = false",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    let enabled = user_count <= CE_GIT_SYNC_MAX_USERS;
    Ok(Json(serde_json::json!({
        "enabled": enabled,
        "reason": if enabled { Some("free_tier") } else { None::<&str> },
        "max_repos": if enabled { Some(1) } else { None::<i32> },
        "user_count": user_count,
        "max_users": CE_GIT_SYNC_MAX_USERS,
    })))
}

async fn edit_git_sync_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<EditGitSyncConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    check_git_sync_access(&db, &w_id).await?;

    let mut tx = db.begin().await?;

    let args_for_audit = format!("{:?}", new_config.git_sync_settings);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_git_sync_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("git_sync_settings", args_for_audit.as_str())].into()),
    )
    .await?;

    // The whole-config save only writes the DB below; the managed GitHub webhooks
    // are reconciled after the commit is durable (like the per-repository endpoint):
    // `post_commit` carries the saved repos to reconcile + the hooks of repos this
    // save removed, to delete.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    let post_commit: Option<(WorkspaceGitSyncSettings, Vec<(String, i64)>)>;

    if let Some(mut git_sync_settings) = new_config.git_sync_settings {
        // Client-supplied server-owned auto-pull state is never trusted: strip it up
        // front, then existing repos re-derive it from `existing` below and new repos
        // stay clean.
        for repo in git_sync_settings.repositories.iter_mut() {
            if let Some(ap) = repo.auto_pull.as_mut() {
                clear_client_supplied_auto_pull_state(ap);
            }
            repo.open_pr_error = None;
        }
        reject_parent_only_git_sync_settings_on_fork(
            &db,
            &w_id,
            git_sync_settings.repositories.iter(),
        )
        .await?;
        // Auto-pull is EE-only (see edit_git_sync_repository).
        #[cfg(not(feature = "enterprise"))]
        if git_sync_settings
            .repositories
            .iter()
            .any(|r| r.auto_pull.as_ref().is_some_and(|a| a.enabled))
        {
            return Err(Error::BadRequest(
                "Automatic pull from git is an Enterprise Edition feature".to_string(),
            ));
        }
        #[cfg(feature = "enterprise")]
        if git_sync_settings
            .repositories
            .iter()
            .any(|r| r.auto_pull.as_ref().is_some_and(|a| a.enabled))
        {
            check_auto_pull_license().await?;
        }
        #[cfg(feature = "enterprise")]
        check_open_prs_license(git_sync_settings.repositories.iter()).await?;
        // Preserve server-owned auto-pull state (webhook id/secret, synced sha, last
        // status) that the redacted GET response omits — otherwise a whole-config
        // save from the UI would drop the webhook secret (breaking delivery) or
        // clobber what the poller/webhook layer wrote.
        let existing: Option<WorkspaceGitSyncSettings> = sqlx::query_scalar!(
            "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
            &w_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .flatten()
        .and_then(|v| serde_json::from_value(v).ok());
        // Repos present before but absent from this save: their webhooks won't be
        // reconciled below (no longer listed), so capture them for deletion.
        #[cfg(all(feature = "enterprise", feature = "private"))]
        let removed_webhooks: Vec<(String, i64)> = existing
            .as_ref()
            .map(|e| {
                e.repositories
                    .iter()
                    .filter_map(|old| {
                        let hook = old.auto_pull.as_ref().and_then(|a| a.webhook_id)?;
                        // The save carries the hook forward (reconciled below) only
                        // when the repo is still present AND still has auto_pull — the
                        // preservation loop copies webhook fields only onto a Some
                        // auto_pull. Otherwise (repo dropped, or auto_pull cleared) the
                        // hook would orphan, so delete it.
                        let carried = git_sync_settings
                            .repositories
                            .iter()
                            .find(|n| n.git_repo_resource_path == old.git_repo_resource_path)
                            .map(|n| n.auto_pull.is_some())
                            .unwrap_or(false);
                        (!carried).then_some((old.git_repo_resource_path.clone(), hook))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(existing) = &existing {
            for repo in git_sync_settings.repositories.iter_mut() {
                let Some(old) = existing
                    .repositories
                    .iter()
                    .find(|r| r.git_repo_resource_path == repo.git_repo_resource_path)
                else {
                    continue;
                };
                repo.open_pr_error = old.open_pr_error.clone();
                if let (Some(new_ap), Some(old_ap)) =
                    (repo.auto_pull.as_mut(), old.auto_pull.as_ref())
                {
                    new_ap.webhook_id = old_ap.webhook_id;
                    new_ap.webhook_secret = old_ap.webhook_secret.clone();
                    new_ap.last_synced_sha = old_ap.last_synced_sha.clone();
                    new_ap.last_pull_status = old_ap.last_pull_status.clone();
                }
            }
        }

        // Clean up legacy workspace-level settings if all repos are migrated
        cleanup_legacy_git_sync_settings_in_memory(&mut git_sync_settings, &w_id);

        let serialized_config = serde_json::to_value(&git_sync_settings)
            .map_err(|err| Error::internal_err(err.to_string()))?;

        sqlx::query!(
            "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
            serialized_config,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
        #[cfg(all(feature = "enterprise", feature = "private"))]
        {
            post_commit = Some((git_sync_settings, removed_webhooks));
        }
    } else {
        // Clearing the whole config removes every repo — delete all their webhooks.
        #[cfg(all(feature = "enterprise", feature = "private"))]
        {
            let existing: Option<WorkspaceGitSyncSettings> = sqlx::query_scalar!(
                "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
                &w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            .flatten()
            .and_then(|v| serde_json::from_value(v).ok());
            let removed_webhooks: Vec<(String, i64)> = existing
                .map(|e| {
                    e.repositories
                        .iter()
                        .filter_map(|old| {
                            old.auto_pull
                                .as_ref()
                                .and_then(|a| a.webhook_id)
                                .map(|h| (old.git_repo_resource_path.clone(), h))
                        })
                        .collect()
                })
                .unwrap_or_default();
            post_commit = Some((WorkspaceGitSyncSettings::default(), removed_webhooks));
        }
        sqlx::query!(
            "UPDATE workspace_settings SET git_sync = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Post-commit: reconcile each saved repo's managed webhook to match the config
    // and delete the webhooks of repos this save removed. Best-effort — a failure
    // leaves polling on.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    if let Some((mut settings, removed_webhooks)) = post_commit {
        let mut changed: Vec<(String, windmill_common::workspaces::AutoPullSettings)> = Vec::new();
        for repo in settings.repositories.iter_mut() {
            let before = serde_json::to_value(&repo.auto_pull).ok();
            if let Err(e) = windmill_common::git_sync_ee::sync_repo_webhook(&db, &w_id, repo).await
            {
                tracing::warn!("git auto-pull: webhook sync error: {}", e);
            }
            if serde_json::to_value(&repo.auto_pull).ok() != before {
                if let Some(ap) = repo.auto_pull.as_ref() {
                    changed.push((repo.git_repo_resource_path.clone(), ap.clone()));
                }
            }
        }
        if let Err(e) = persist_reconciled_webhook_fields(&db, &w_id, &changed).await {
            tracing::warn!("git auto-pull: webhook field persist error: {}", e);
        }
        for (path, hook_id) in removed_webhooks {
            if let Ok(url) = windmill_common::git_sync_ee::resolve_repo_url(&db, &w_id, &path).await
            {
                let _ =
                    windmill_common::git_sync_ee::delete_repo_webhook(&db, &w_id, &url, hook_id)
                        .await;
            }
        }
    }

    // Trigger git sync for git sync settings changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "git_sync".to_string() },
        Some("Git sync configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Edit git sync config for workspace {}", &w_id))
}

async fn edit_git_sync_repository(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(mut new_config): Json<EditGitSyncRepository>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    check_git_sync_access(&db, &w_id).await?;

    // Validate the resource path format
    validate_git_repo_resource_path(&new_config.git_repo_resource_path)?;

    // Server-owned auto-pull state (webhook id/secret + sync status) is never
    // accepted from the client — the webhook layer and poller own it. Strip it so an
    // existing repo re-derives it from the DB (carried over below) and a new one
    // starts clean.
    if let Some(ap) = new_config.repository.auto_pull.as_mut() {
        clear_client_supplied_auto_pull_state(ap);
    }
    new_config.repository.open_pr_error = None;
    reject_parent_only_git_sync_settings_on_fork(
        &db,
        &w_id,
        std::iter::once(&new_config.repository),
    )
    .await?;

    // Auto-pull is EE-only: CE builds compile neither the poller nor the webhook
    // reconciler, so accepting the setting would silently do nothing.
    #[cfg(not(feature = "enterprise"))]
    if new_config
        .repository
        .auto_pull
        .as_ref()
        .is_some_and(|a| a.enabled)
    {
        return Err(Error::BadRequest(
            "Automatic pull from git is an Enterprise Edition feature".to_string(),
        ));
    }
    #[cfg(feature = "enterprise")]
    if new_config
        .repository
        .auto_pull
        .as_ref()
        .is_some_and(|a| a.enabled)
    {
        check_auto_pull_license().await?;
    }
    #[cfg(feature = "enterprise")]
    check_open_prs_license(std::iter::once(&new_config.repository)).await?;

    // Promotion mode: EE only
    #[cfg(not(feature = "enterprise"))]
    if new_config.repository.use_individual_branch.unwrap_or(false) {
        return Err(Error::BadRequest(
            "Promotion mode is an Enterprise Edition feature".to_string(),
        ));
    }

    let mut tx = db.begin().await?;

    // First, get the current git sync settings
    let current_settings = sqlx::query!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let mut git_sync_settings = if let Some(row) = current_settings {
        if let Some(git_sync) = row.git_sync {
            serde_json::from_value::<WorkspaceGitSyncSettings>(git_sync)
                .map_err(|err| Error::internal_err(err.to_string()))?
        } else {
            WorkspaceGitSyncSettings::default()
        }
    } else {
        WorkspaceGitSyncSettings::default()
    };

    // Multi-repo: EE only
    #[cfg(not(feature = "enterprise"))]
    {
        let is_new = !git_sync_settings
            .repositories
            .iter()
            .any(|r| r.git_repo_resource_path == new_config.git_repo_resource_path);
        if is_new && !git_sync_settings.repositories.is_empty() {
            return Err(Error::BadRequest(
                "Multiple git sync repositories is an Enterprise Edition feature".to_string(),
            ));
        }
    }

    // Audit log before we move the repository
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_git_sync_repository",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [
                (
                    "repository_path",
                    new_config.git_repo_resource_path.as_str(),
                ),
                ("repository_data", &format!("{:?}", new_config.repository)),
            ]
            .into(),
        ),
    )
    .await?;

    // Check if repository exists before modifying
    let repo_exists = git_sync_settings
        .repositories
        .iter()
        .any(|repo| repo.git_repo_resource_path == new_config.git_repo_resource_path);

    // Find and update the specific repository, or add it if it doesn't exist
    let repo_found = git_sync_settings
        .repositories
        .iter_mut()
        .find(|repo| repo.git_repo_resource_path == new_config.git_repo_resource_path);

    if let Some(existing_repo) = repo_found {
        // Update existing repository, but preserve server-owned auto-pull state
        // (synced sha, last pull status, webhook id/secret) so a settings save
        // from the UI cannot revert what the poller/webhook layer wrote.
        let mut updated = new_config.repository;
        updated.open_pr_error = existing_repo.open_pr_error.clone();
        match (updated.auto_pull.as_mut(), existing_repo.auto_pull.as_ref()) {
            (Some(new_ap), Some(old_ap)) => {
                new_ap.last_synced_sha = old_ap.last_synced_sha.clone();
                new_ap.last_pull_status = old_ap.last_pull_status.clone();
                new_ap.webhook_id = old_ap.webhook_id;
                new_ap.webhook_secret = old_ap.webhook_secret.clone();
            }
            // UI omitted auto_pull (e.g. older client): keep existing config.
            (None, Some(_)) => {
                updated.auto_pull = existing_repo.auto_pull.clone();
            }
            _ => {}
        }
        // The request-side license gate above only saw the submitted config; the
        // preservation can resurrect an enabled auto_pull (None arm), so re-check
        // the effective state before it gets written and reconciled.
        #[cfg(feature = "enterprise")]
        if updated.auto_pull.as_ref().is_some_and(|a| a.enabled) {
            check_auto_pull_license().await?;
        }
        *existing_repo = updated;
    } else {
        // Repository doesn't exist, add it as a new repository
        git_sync_settings.repositories.push(new_config.repository);
    }

    // Clean up legacy workspace-level settings if all repos are migrated
    cleanup_legacy_git_sync_settings_in_memory(&mut git_sync_settings, &w_id);

    // Save the updated configuration first, then reconcile the GitHub webhook to
    // match it *after* the commit is durable (phase 2). Reconciling before the
    // commit could leave the DB pointing at a hook that no longer matches if the
    // save then failed (e.g. a delete on disable); post-commit reconciliation
    // cannot. The pre-edit webhook id/secret are carried over above, so the
    // committed row stays consistent until the reconcile persists any change.
    let serialized_config = serde_json::to_value(&git_sync_settings)
        .map_err(|err| Error::internal_err(err.to_string()))?;

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        serialized_config,
        &w_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Post-commit: create/remove the webhook to match the saved config and persist
    // the resulting hook id/secret. Best-effort — a failure leaves polling on.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    {
        let mut changed: Vec<(String, windmill_common::workspaces::AutoPullSettings)> = Vec::new();
        if let Some(repo) = git_sync_settings
            .repositories
            .iter_mut()
            .find(|r| r.git_repo_resource_path == new_config.git_repo_resource_path)
        {
            let before = serde_json::to_value(&repo.auto_pull).ok();
            if let Err(e) = windmill_common::git_sync_ee::sync_repo_webhook(&db, &w_id, repo).await
            {
                tracing::warn!("git auto-pull: webhook sync error: {}", e);
            }
            if serde_json::to_value(&repo.auto_pull).ok() != before {
                if let Some(ap) = repo.auto_pull.as_ref() {
                    changed.push((repo.git_repo_resource_path.clone(), ap.clone()));
                }
            }
        }
        if let Err(e) = persist_reconciled_webhook_fields(&db, &w_id, &changed).await {
            tracing::warn!("git auto-pull: webhook field persist error: {}", e);
        }
    }

    // Trigger git sync for individual repository update/add
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "git_sync".to_string() },
        Some(format!(
            "Git sync repository '{}' {}",
            new_config.git_repo_resource_path,
            if repo_exists { "updated" } else { "added" }
        )),
        false,
        None,
    )
    .await?;

    Ok(format!(
        "{} git sync repository '{}' for workspace {}",
        if repo_exists { "Updated" } else { "Added" },
        new_config.git_repo_resource_path,
        &w_id
    ))
}

async fn delete_git_sync_repository(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(request): Json<DeleteGitSyncRepositoryRequest>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    // No check_git_sync_access here — admins should always be able to delete/clean up repos
    if request.git_repo_resource_path.is_empty() {
        return Err(Error::BadRequest(
            "Resource path cannot be empty".to_string(),
        ));
    }

    let mut tx = db.begin().await?;

    // First, get the current git sync settings
    let current_settings = sqlx::query!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let mut git_sync_settings = if let Some(row) = current_settings {
        if let Some(git_sync) = row.git_sync {
            serde_json::from_value::<WorkspaceGitSyncSettings>(git_sync)
                .map_err(|err| Error::internal_err(err.to_string()))?
        } else {
            WorkspaceGitSyncSettings::default()
        }
    } else {
        WorkspaceGitSyncSettings::default()
    };

    // Capture the repo's managed webhook id; the hook itself is deleted only after
    // the DB removal commits (below), so a failed save can't leave the repo pointing
    // at a hook that no longer exists. Deletion bypasses the sync_repo_webhook
    // lifecycle, so GitHub would otherwise keep delivering to an orphaned hook.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    let webhook_to_delete: Option<i64> = git_sync_settings
        .repositories
        .iter()
        .find(|r| r.git_repo_resource_path == request.git_repo_resource_path)
        .and_then(|r| r.auto_pull.as_ref())
        .and_then(|a| a.webhook_id);

    // Check if repository exists and remove it
    let original_count = git_sync_settings.repositories.len();
    git_sync_settings
        .repositories
        .retain(|repo| repo.git_repo_resource_path != request.git_repo_resource_path);

    if git_sync_settings.repositories.len() == original_count {
        return Err(Error::BadRequest(format!(
            "Repository with path '{}' not found in git sync configuration",
            request.git_repo_resource_path
        )));
    }

    // Audit log
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.delete_git_sync_repository",
        ActionKind::Delete,
        &w_id,
        Some(&authed.email),
        Some([("repository_path", request.git_repo_resource_path.as_str())].into()),
    )
    .await?;

    // Clean up legacy workspace-level settings if all repos are migrated
    cleanup_legacy_git_sync_settings_in_memory(&mut git_sync_settings, &w_id);

    // Save the updated configuration
    let serialized_config = serde_json::to_value::<WorkspaceGitSyncSettings>(git_sync_settings)
        .map_err(|err| Error::internal_err(err.to_string()))?;

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        serialized_config,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Removal is durable now — best-effort delete the GitHub webhook.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    if let Some(hook_id) = webhook_to_delete {
        if let Ok(url) = windmill_common::git_sync_ee::resolve_repo_url(
            &db,
            &w_id,
            &request.git_repo_resource_path,
        )
        .await
        {
            let _ =
                windmill_common::git_sync_ee::delete_repo_webhook(&db, &w_id, &url, hook_id).await;
        }
    }

    // Trigger git sync for repository deletion
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "git_sync".to_string() },
        Some(format!(
            "Git sync repository '{}' deleted",
            request.git_repo_resource_path
        )),
        false,
        None,
    )
    .await?;

    Ok(format!(
        "Deleted git sync repository '{}' from workspace {}",
        request.git_repo_resource_path, &w_id
    ))
}

#[cfg(feature = "enterprise")]
#[derive(Debug, Deserialize)]
struct EditDeployUIConfig {
    deploy_ui_settings: Option<WorkspaceDeploymentUISettings>,
}

#[cfg(not(feature = "enterprise"))]
async fn edit_deploy_ui_config(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Deployment UI is only available on Windmill Enterprise Edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
async fn edit_deploy_ui_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<EditDeployUIConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;
    let args_for_audit = format!("{:?}", new_config.deploy_ui_settings);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_deploy_ui_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("deployment_ui_settings", args_for_audit.as_str())].into()),
    )
    .await?;

    if let Some(deploy_ui_settings) = new_config.deploy_ui_settings {
        let serialized_config =
            serde_json::to_value::<WorkspaceDeploymentUISettings>(deploy_ui_settings)
                .map_err(|err| Error::internal_err(err.to_string()))?;

        sqlx::query!(
            "UPDATE workspace_settings SET deploy_ui = $1 WHERE workspace_id = $2",
            serialized_config,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET deploy_ui = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(format!("Edit deployment UI config for workspace {}", &w_id))
}

#[derive(Deserialize)]
pub struct EditDefaultApp {
    #[cfg(feature = "enterprise")]
    pub default_app_path: Option<String>,
}

#[cfg(not(feature = "enterprise"))]
async fn edit_default_app(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
    Json(_new_config): Json<EditDefaultApp>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Setting a workspace default app is only available on Windmill Enterprise Edition"
            .to_string(),
    ));
}

async fn edit_default_scripts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<Option<serde_json::Value>>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_default_scripts",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;

    if let Some(config) = new_config {
        sqlx::query!(
            "UPDATE workspace_settings SET default_scripts = $1 WHERE workspace_id = $2",
            config,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET default_scripts = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // Trigger git sync for default scripts changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "default_scripts".to_string() },
        Some("Default scripts configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Edit default scripts for workspace {}", &w_id))
}

async fn get_default_scripts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Option<serde_json::Value>> {
    let mut tx = db.begin().await?;
    let default_scripts = sqlx::query_scalar!(
        "SELECT default_scripts FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| Error::internal_err(format!("getting default_app: {err}")))?;
    tx.commit().await?;

    Ok(Json(default_scripts.flatten()))
}

#[cfg(feature = "enterprise")]
async fn edit_default_app(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<EditDefaultApp>,
) -> Result<String> {
    #[cfg(not(feature = "enterprise"))]
    {
        return Err(Error::BadRequest(
            "Setting a workspace default app is only available on Windmill Enterprise Edition"
                .to_string(),
        ));
    }

    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    let args_for_audit = format!("{:?}", new_config.default_app_path);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_default_app",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("default_app", args_for_audit.as_str())].into()),
    )
    .await?;

    if let Some(default_app_path) = new_config.default_app_path {
        sqlx::query!(
            "UPDATE workspace_settings SET default_app = $1 WHERE workspace_id = $2",
            default_app_path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET default_app = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // Trigger git sync for default app changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "default_app".to_string() },
        Some("Default app configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Edit default app for workspace {}", &w_id))
}

#[derive(Serialize)]
struct WorkspaceDefaultApp {
    pub default_app_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_app_raw: Option<bool>,
}
async fn get_default_app(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<WorkspaceDefaultApp> {
    let row = sqlx::query!(
        "SELECT ws.default_app AS default_app_path, av.raw_app AS \"default_app_raw: Option<bool>\"
         FROM workspace_settings ws
         LEFT JOIN app ON app.path = ws.default_app AND app.workspace_id = ws.workspace_id
         LEFT JOIN app_version av ON av.id = app.versions[array_upper(app.versions, 1)]
         WHERE ws.workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|err| Error::internal_err(format!("getting default_app: {err}")))?;

    Ok(Json(WorkspaceDefaultApp {
        default_app_path: row.default_app_path,
        default_app_raw: row.default_app_raw,
    }))
}

async fn edit_error_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ee): Json<EditErrorHandler>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    // Normalize to new format (handles both old CLI and new CLI requests)
    let ee = ee.into_normalized();

    let mut tx = db.begin().await?;

    sqlx::query_as!(
        Group,
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        w_id,
        "error_handler",
        "The group the error handler acts on behalf of",
        serde_json::json!({username_to_permissioned_as(&authed.username): true})
    )
    .execute(&mut *tx)
    .await?;

    if let Some(path) = &ee.path {
        match ee.extra_args.as_ref() {
            Some(extra_args) if extra_args.is_object() => {
                let Ok(email_recipients) = serde_json::from_value::<Option<Vec<String>>>(
                    extra_args["email_recipients"].to_owned(),
                ) else {
                    return Err(Error::BadRequest(
                        "Field `email_recipients` expected to be JSON array".to_string(),
                    ));
                };

                if let Some(email_recipients) = email_recipients {
                    for email in email_recipients {
                        if !EMAIL_REGEXP.is_match(&email) {
                            return Err(Error::BadRequest(format!(
                                "Invalid email format: {}",
                                email
                            )));
                        }
                    }
                }
            }
            None => {}
            _ => {
                return Err(Error::BadRequest(
                    "Field `extra_args` expected to be JSON object".to_string(),
                ))
            }
        }

        // Always persist `muted_on_cancel` and `muted_on_user_path` (including
        // false values) so that a YAML round-trip via `wmill sync pull && wmill
        // sync push` is stable instead of re-firing `editErrorHandler` on every
        // push (the CLI sends `false` defaults and deepEqual would otherwise
        // mismatch an omitted-on-write shape against an always-sent-by-CLI one).
        let mut error_handler = serde_json::json!({
            "path": path,
            "muted_on_cancel": ee.muted_on_cancel,
            "muted_on_user_path": ee.muted_on_user_path,
        });
        if let Some(extra_args) = &ee.extra_args {
            error_handler["extra_args"] = extra_args.clone();
        }

        sqlx::query!(
            "UPDATE workspace_settings SET error_handler = $1 WHERE workspace_id = $2",
            error_handler,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET error_handler = NULL WHERE workspace_id = $1",
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_error_handler",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("error_handler", &format!("{:?}", ee.path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    // Trigger git sync for error handler changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "error_handler".to_string() },
        Some("Error handler configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Edit error_handler for workspace {}", &w_id))
}

async fn edit_success_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(es): Json<EditSuccessHandler>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    // Normalize to new format (handles both old CLI and new CLI requests)
    let es = es.into_normalized();

    let mut tx = db.begin().await?;

    sqlx::query_as!(
        Group,
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        w_id,
        "success_handler",
        "The group the success handler acts on behalf of",
        serde_json::json!({username_to_permissioned_as(&authed.username): true})
    )
    .execute(&mut *tx)
    .await?;

    if let Some(path) = &es.path {
        let mut success_handler = serde_json::json!({
            "path": path,
        });
        if let Some(extra_args) = &es.extra_args {
            success_handler["extra_args"] = extra_args.clone();
        }

        sqlx::query!(
            "UPDATE workspace_settings SET success_handler = $1 WHERE workspace_id = $2",
            success_handler,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET success_handler = NULL WHERE workspace_id = $1",
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_success_handler",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("success_handler", &format!("{:?}", es.path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    // Trigger git sync for success handler changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "success_handler".to_string() },
        Some("Success handler configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Edit success_handler for workspace {}", &w_id))
}

#[derive(Deserialize)]
struct NewEnvironmentVariable {
    name: String,
    value: Option<String>,
}

async fn set_environment_variable(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(NewEnvironmentVariable { value, name }): Json<NewEnvironmentVariable>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = db.begin().await?;

    match value {
        Some(value) => {
            sqlx::query!(
                "INSERT INTO workspace_env (workspace_id, name, value) VALUES ($1, $2, $3) ON CONFLICT (workspace_id, name) DO UPDATE SET value = EXCLUDED.value",
                &w_id,
                name,
                value
            )
            .execute(&mut *tx)
            .await?;

            audit_log(
                &mut *tx,
                &authed,
                "workspace.set_environment_variable",
                ActionKind::Create,
                &w_id,
                Some(&authed.email),
                None,
            )
            .await?;
            tx.commit().await?;

            Ok(format!("Set environment variable {}", name))
        }
        None => {
            sqlx::query!(
                "DELETE FROM workspace_env WHERE workspace_id = $1 AND name = $2",
                &w_id,
                name
            )
            .execute(&mut *tx)
            .await?;

            audit_log(
                &mut *tx,
                &authed,
                "workspace.delete_environment_variable",
                ActionKind::Delete,
                &w_id,
                Some(&authed.email),
                None,
            )
            .await?;
            tx.commit().await?;

            Ok(format!("Deleted environment variable {}", name))
        }
    }
}

#[derive(Serialize)]
pub struct GetEncryptionKeyResponse {
    key: String,
}

async fn get_encryption_key(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<GetEncryptionKeyResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    let encryption_key_opt = sqlx::query_scalar!(
        "SELECT key FROM workspace_key WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(&db)
    .await?;

    let encryption_key = not_found_if_none(encryption_key_opt, "workspace_encryption_key", w_id)?;
    return Ok(Json(GetEncryptionKeyResponse { key: encryption_key }));
}

#[derive(Deserialize)]
struct SetEncryptionKeyRequest {
    new_key: String,
    skip_reencrypt: Option<bool>,
}

async fn set_encryption_key(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<SetEncryptionKeyRequest>,
) -> Result<()> {
    require_super_admin(&db, &authed.email).await?;

    if !WORKSPACE_KEY_REGEXP.is_match(request.new_key.as_str()) {
        return Err(Error::BadRequest(
            "Encryption key should be an alphanumeric string of 64 characters".to_string(),
        ));
    }

    // Build the previous cipher before the transaction (reads from cache/pool)
    let previous_encryption_key = build_crypt(&db, w_id.as_str()).await?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace_key SET key = $1 WHERE workspace_id = $2",
        request.new_key.clone(),
        w_id
    )
    .execute(&mut *tx)
    .await?;

    let mut reencrypted_secret_paths: Vec<String> = Vec::new();
    if !request.skip_reencrypt.unwrap_or(false) {
        // Build the new cipher directly from the key string, since the transaction
        // hasn't committed yet and build_crypt() would read the old key from the pool.
        let crypt_key = if let Some(ref salt) = SECRET_SALT.as_ref() {
            format!("{}{}", request.new_key, salt)
        } else {
            request.new_key.clone()
        };
        let new_encryption_key = magic_crypt::new_magic_crypt!(crypt_key, 256);

        let mut truncated_new_key = request.new_key.clone();
        truncated_new_key.truncate(8);
        tracing::warn!(
            "Re-encrypting all secrets for workspace {}. New key is {}***",
            w_id,
            truncated_new_key
        );

        let all_variables = sqlx::query!(
            "SELECT path, value, is_secret FROM variable WHERE workspace_id = $1",
            w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for variable in all_variables {
            if !variable.is_secret {
                continue;
            }
            let decrypted_value =
                decrypt(&previous_encryption_key, variable.value).map_err(|e| {
                    Error::internal_err(format!(
                        "Error decrypting variable {}: {}",
                        variable.path, e
                    ))
                })?;
            let new_encrypted_value = encrypt(&new_encryption_key, decrypted_value.as_str());
            sqlx::query!(
                "UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3",
                new_encrypted_value,
                w_id,
                variable.path
            )
            .execute(&mut *tx)
            .await?;
            reencrypted_secret_paths.push(variable.path);
        }
    }

    tx.commit().await?;

    // Invalidate the cache only after the transaction has committed
    WORKSPACE_CRYPT_CACHE.remove(w_id.as_str());

    // Build the batch: one event for the encryption key itself plus one per
    // re-encrypted secret variable. The batch entrypoint dispatches a single
    // git-sync job per repo carrying all items, so repos with Secrets sync
    // enabled receive the new ciphertexts in one commit.
    let mut batch: Vec<DeployedObject> = Vec::with_capacity(reencrypted_secret_paths.len() + 1);
    batch.push(DeployedObject::Key { key_type: "encryption_key".to_string() });
    for path in reencrypted_secret_paths {
        batch.push(DeployedObject::Variable { path: path.clone(), parent_path: Some(path) });
    }

    handle_deployment_metadata_batch(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        batch,
        Some("Encryption key updated".to_string()),
    )
    .await?;

    return Ok(());
}

#[derive(Serialize)]
struct UsedTriggers {
    pub websocket_used: bool,
    pub http_routes_used: bool,
    pub kafka_used: bool,
    pub nats_used: bool,
    pub postgres_used: bool,
    pub mqtt_used: bool,
    pub sqs_used: bool,
    pub gcp_used: bool,
    pub azure_used: bool,
    pub email_used: bool,
    pub nextcloud_used: bool,
    pub google_used: bool,
    pub github_used: bool,
}

async fn get_used_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<UsedTriggers> {
    let mut tx = user_db.begin(&authed).await?;
    let triggers_used = sqlx::query_as!(
        UsedTriggers,
        r#"
        SELECT
            EXISTS(SELECT 1 FROM websocket_trigger WHERE workspace_id = $1) AS "websocket_used!",
            EXISTS(SELECT 1 FROM http_trigger WHERE workspace_id = $1) AS "http_routes_used!",
            EXISTS(SELECT 1 FROM kafka_trigger WHERE workspace_id = $1) as "kafka_used!",
            EXISTS(SELECT 1 FROM nats_trigger WHERE workspace_id = $1) as "nats_used!",
            EXISTS(SELECT 1 FROM postgres_trigger WHERE workspace_id = $1) AS "postgres_used!",
            EXISTS(SELECT 1 FROM mqtt_trigger WHERE workspace_id = $1) AS "mqtt_used!",
            EXISTS(SELECT 1 FROM sqs_trigger WHERE workspace_id = $1) AS "sqs_used!",
            EXISTS(SELECT 1 FROM gcp_trigger WHERE workspace_id = $1) AS "gcp_used!",
            EXISTS(SELECT 1 FROM azure_trigger WHERE workspace_id = $1) AS "azure_used!",
            EXISTS(SELECT 1 FROM email_trigger WHERE workspace_id = $1) AS "email_used!",
            EXISTS(SELECT 1 FROM native_trigger WHERE workspace_id = $1 AND service_name = 'nextcloud'::native_trigger_service) AS "nextcloud_used!",
            EXISTS(SELECT 1 FROM native_trigger WHERE workspace_id = $1 AND service_name = 'google'::native_trigger_service) AS "google_used!",
            EXISTS(SELECT 1 FROM native_trigger WHERE workspace_id = $1 AND service_name = 'github'::native_trigger_service) AS "github_used!"
        "#,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(triggers_used))
}

async fn get_workspace_as_superadmin(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Workspace> {
    require_super_admin(&db, &authed.email).await?;
    let workspace = sqlx::query_as!(
        Workspace,
        "SELECT
            workspace.id AS \"id!\",
            workspace.name AS \"name!\",
            workspace.owner AS \"owner!\",
            workspace.deleted AS \"deleted!\",
            workspace.premium AS \"premium!\",
            workspace_settings.color AS \"color\",
            workspace.parent_workspace_id AS \"parent_workspace_id\"
        FROM workspace
        LEFT JOIN workspace_settings ON workspace.id = workspace_settings.workspace_id
        WHERE workspace.id = $1",
        w_id
    )
    .fetch_optional(&db)
    .await?;

    let workspace = not_found_if_none(workspace, "workspace", w_id)?;

    Ok(Json(workspace))
}

async fn list_workspaces_as_super_admin(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Query(pagination): Query<Pagination>,
    ApiAuthed { email, .. }: ApiAuthed,
) -> JsonResult<Vec<Workspace>> {
    require_devops_role(&db, &email).await?;
    let (per_page, offset) = paginate(pagination);

    let mut tx = user_db.begin(&authed).await?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT
            workspace.id AS \"id!\",
            workspace.name AS \"name!\",
            workspace.owner AS \"owner!\",
            workspace.deleted AS \"deleted!\",
            workspace.premium AS \"premium!\",
            workspace_settings.color AS \"color\",
            workspace.parent_workspace_id AS \"parent_workspace_id\"
        FROM workspace
        LEFT JOIN workspace_settings ON workspace.id = workspace_settings.workspace_id
         LIMIT $1 OFFSET $2",
        per_page as i32,
        offset as i32
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(workspaces))
}

async fn user_workspaces(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
) -> JsonResult<WorkspaceList> {
    let mut tx = db.begin().await?;
    let workspaces = sqlx::query_as!(
        UserWorkspace,
        "SELECT workspace.id, workspace.name, usr.username, workspace_settings.color, workspace.parent_workspace_id,
                workspace.is_dev_workspace, workspace.dev_workspace_label,
                workspace.owner AS \"created_by?\",
                CASE WHEN usr.operator THEN workspace_settings.operator_settings ELSE NULL END as operator_settings,
                usr.disabled
         FROM workspace
         JOIN usr ON usr.workspace_id = workspace.id
         JOIN workspace_settings ON workspace_settings.workspace_id = workspace.id
         WHERE usr.email = $1 AND workspace.deleted = false",
        email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(WorkspaceList { email, workspaces }))
}

#[derive(Deserialize)]
struct SessionWorkspaceStatusRequest {
    workspace_ids: Vec<String>,
}

/// Reconciliation support for client-side AI sessions, which the backend cannot touch
/// directly. The client posts the workspace ids its sessions reference and uses the
/// per-id status to keep sessions in sync with workspace lifecycle: `deleted` (no row /
/// no access → unresolvable) drops the sessions, `archived` (soft-deleted, still a
/// member) archives them, `active` restores ones previously archived-by-workspace.
/// Archived and hard-deleted workspaces are absent from `user_workspaces`, so this is the
/// only way the client learns about a change made while it was away or on another device.
async fn session_workspace_status(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
    Json(req): Json<SessionWorkspaceStatusRequest>,
) -> JsonResult<HashMap<String, String>> {
    if req.workspace_ids.len() > 1000 {
        return Err(Error::BadRequest(
            "Too many workspace ids (max 1000)".to_string(),
        ));
    }
    let rows = sqlx::query!(
        "SELECT req.id AS \"id!\",
                (CASE
                    WHEN usr.email IS NULL THEN 'deleted'
                    WHEN workspace.deleted THEN 'archived'
                    ELSE 'active'
                END) AS \"status!\"
         FROM unnest($1::text[]) AS req(id)
         LEFT JOIN workspace ON workspace.id = req.id
         LEFT JOIN usr ON usr.workspace_id = workspace.id AND usr.email = $2",
        &req.workspace_ids[..],
        email,
    )
    .fetch_all(&db)
    .await?;
    let statuses = rows.into_iter().map(|r| (r.id, r.status)).collect();
    Ok(Json(statuses))
}

pub async fn check_w_id_conflict<'c>(tx: &mut Transaction<'c, Postgres>, w_id: &str) -> Result<()> {
    if w_id == "global" {
        return Err(windmill_common::error::Error::BadRequest(
            "'global' is not allowed as a workspace ID".to_string(),
        ));
    }
    let exists = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = $1)", w_id)
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(false);
    if exists {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Workspace {} already exists",
            w_id
        )));
    }
    return Ok(());
}

/// Reject fork creation when the target workspace id is already taken,
/// distinguishing archived workspaces: archiving is a soft delete that keeps
/// the id reserved, which users frequently mistake for a permanent delete.
async fn check_fork_w_id_conflict(db: &DB, w_id: &str) -> Result<()> {
    let deleted = sqlx::query_scalar!("SELECT deleted FROM workspace WHERE id = $1", w_id)
        .fetch_optional(db)
        .await?;
    match deleted {
        Some(true) => Err(Error::BadRequest(format!(
            "Workspace '{w_id}' already exists but is archived (archiving does not free up the workspace id). \
             To reuse this id, permanently delete the archived workspace first — its owner or a superadmin \
             can do so from the fork creation dialog, the superadmin workspaces page, or the CLI \
             (`wmill workspace delete-fork`) — or choose a different fork id."
        ))),
        Some(false) => Err(Error::BadRequest(format!(
            "Workspace '{w_id}' already exists. Delete the existing fork first or choose a different fork id."
        ))),
        None => Ok(()),
    }
}

/// A fork id is reusable: it is freed when a fork is deleted and can be claimed
/// again under the same name. `workspace_diff` and `skip_workspace_diff_tally`
/// are keyed by workspace id with no FK cascade, so a freshly created fork could
/// inherit cached diff state from a previous occupant of its id — a stale skip
/// row suppresses comparison entirely, and stale workspace_diff rows produce a
/// spurious "changes not visible" warning that hides the deploy button. Clear
/// both so a new fork always starts with clean diff state, regardless of how the
/// id was freed.
async fn purge_stale_fork_diff_state(db: &DB, fork_id: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM workspace_diff WHERE source_workspace_id = $1 OR fork_workspace_id = $1",
        fork_id
    )
    .execute(db)
    .await?;
    sqlx::query!(
        "DELETE FROM skip_workspace_diff_tally WHERE workspace_id = $1",
        fork_id
    )
    .execute(db)
    .await?;
    Ok(())
}

lazy_static::lazy_static! {

    pub static ref CREATE_WORKSPACE_REQUIRE_SUPERADMIN: bool = {
        match std::env::var("CREATE_WORKSPACE_REQUIRE_SUPERADMIN") {
            Ok(val) => val == "true",
            Err(_) => true,
        }
    };

    pub static ref DISABLE_WORKSPACE_FORK: bool = {
        match std::env::var("DISABLE_WORKSPACE_FORK") {
            Ok(val) => val == "true",
            Err(_) => false,
        }
    };

    // Cloud only: how many forks a premium workspace may have per paid (developer) seat.
    pub static ref MAX_FORKS_PER_SEAT: i64 = std::env::var("MAX_FORKS_PER_SEAT")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v >= 0)
        .unwrap_or(5);

    // How deep a fork chain may nest (root = depth 0, a direct fork = depth 1). A general guardrail
    // for all builds, independent of the cloud per-seat cap: deep fork chains are a footgun and no
    // real use case needs them. Clamped to [1, 20] so it's always a real limit and can never exceed
    // the fork-walk recursion backstop (20) that billing/count resolution uses (a chain deeper than
    // the backstop would truncate and mis-resolve its root).
    pub static ref MAX_FORK_DEPTH: i64 = std::env::var("MAX_FORK_DEPTH")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 20))
        .unwrap_or(5);

}

async fn create_workspace_require_superadmin() -> String {
    format!("{}", *CREATE_WORKSPACE_REQUIRE_SUPERADMIN)
}

async fn _check_nb_of_workspaces(db: &DB) -> Result<()> {
    let nb_workspaces = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace WHERE id != 'admins' AND deleted = false",
    )
    .fetch_one(db)
    .await?;
    if nb_workspaces.unwrap_or(0) >= 2 {
        return Err(Error::BadRequest(
            "You have reached the maximum number of workspaces (2 outside of default workspace 'admins') without an enterprise license. Archive/delete another workspace to create a new one"
                .to_string(),
        ));
    }
    return Ok(());
}

async fn _check_nb_of_archived_workspaces(db: &DB) -> Result<()> {
    let nb_archived = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace WHERE id != 'admins' AND deleted = true",
    )
    .fetch_one(db)
    .await?;
    if nb_archived.unwrap_or(0) >= 1 {
        return Err(Error::BadRequest(
            "You have reached the maximum number of archived workspaces (1) without an enterprise license. Permanently delete or unarchive the existing archived workspace first"
                .to_string(),
        ));
    }
    return Ok(());
}

async fn create_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(nw): Json<CreateWorkspace>,
) -> Result<String> {
    if *CREATE_WORKSPACE_REQUIRE_SUPERADMIN {
        require_super_admin(&db, &authed.email).await?;
    }

    #[cfg(not(feature = "enterprise"))]
    _check_nb_of_workspaces(&db).await?;

    if *CLOUD_HOSTED {
        let nb_workspaces = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM workspace WHERE owner = $1",
            authed.email
        )
        .fetch_one(&db)
        .await?;
        if nb_workspaces.unwrap_or(0) >= 10 {
            return Err(Error::BadRequest(
                "You have reached the maximum number of workspaces (10) on cloud. Contact support@windmill.dev to increase the limit"
                    .to_string(),
            ));
        }
    }

    validate_workspace_name(&nw.name)?;

    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    check_w_id_conflict(&mut tx, &nw.id).await?;
    sqlx::query!(
        "INSERT INTO workspace
            (id, name, owner)
            VALUES ($1, $2, $3)",
        nw.id,
        nw.name,
        authed.email,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_settings
            (workspace_id, color)
            VALUES ($1, $2)",
        nw.id,
        nw.color,
    )
    .execute(&mut *tx)
    .await?;
    let key = rd_string(64);
    sqlx::query!(
        "INSERT INTO workspace_key
            (workspace_id, kind, key)
            VALUES ($1, 'cloud', $2)",
        nw.id,
        &key
    )
    .execute(&mut *tx)
    .await?;

    // let mc = magic_crypt::new_magic_crypt!(key, 256);
    // sqlx::query!(
    //     "INSERT INTO variable
    //         (workspace_id, path, value, is_secret, description)
    //         VALUES ($1, 'g/all/pretty_secret', $2, true, 'This item is secret'),
    //             ($3, 'g/all/not_secret', $4, false, 'This item is not secret')",
    //     nw.id,
    //     crate::variables::encrypt(&mc, "pretty secret value"),
    //     nw.id,
    //     "finland does not actually exist",
    // )
    // .execute(&mut *tx)
    // .await?;

    let automate_username_creation = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        AUTOMATE_USERNAME_CREATION_SETTING,
    )
    .fetch_optional(&mut *tx)
    .await?
    .map(|v| v.as_bool())
    .flatten()
    .unwrap_or(true);

    let username = if automate_username_creation {
        if nw.username.is_some() && nw.username.unwrap().len() > 0 {
            return Err(Error::BadRequest(
                "username is not allowed when username creation is automated".to_string(),
            ));
        }
        get_instance_username_or_create_pending(&mut tx, &authed.email).await?
    } else {
        nw.username
            .ok_or(Error::BadRequest("username is required".to_string()))?
    };

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin)
            VALUES ($1, $2, $3, true)",
        nw.id,
        authed.email,
        username,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO group_
            VALUES ($1, 'all', 'The group that always contains all users of this workspace')",
        nw.id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO group_
            VALUES ($1, 'wm_deployers', 'Members can preserve the original author when deploying to this workspace')",
        nw.id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO usr_to_group
            VALUES ($1, 'all', $2)",
        nw.id,
        username
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.create",
        ActionKind::Create,
        &nw.id,
        Some(nw.name.as_str()),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Created workspace {}", &nw.id))
}

// `authed_email` is the forker's email — `clone_drafts` only carries this
// user's per-user drafts (and the legacy NULL-email workspace draft, if any)
// across, since other users aren't added to the fork's `usr` table and
// their drafts would dangle as orphans.
async fn clone_workspace_data(
    tx: &mut Transaction<'_, Postgres>,
    db: &DB,
    source_workspace_id: &str,
    target_workspace_id: &str,
    authed_email: &str,
) -> Result<()> {
    // Clone workspace settings (merge with existing basic settings)
    update_workspace_settings(tx, source_workspace_id, target_workspace_id).await?;

    // Clone data table migration definitions (the settings above carry the data
    // table config; this carries their migration history).
    crate::datatable_migrations::clone_datatable_migrations(
        tx,
        source_workspace_id,
        target_workspace_id,
    )
    .await?;

    // Clone workspace environment variables
    clone_workspace_env(tx, source_workspace_id, target_workspace_id).await?;

    // Clone folders
    clone_folders(tx, source_workspace_id, target_workspace_id).await?;

    // Clone groups
    clone_groups(tx, source_workspace_id, target_workspace_id).await?;

    // Clone resource types
    clone_resource_types(tx, source_workspace_id, target_workspace_id).await?;

    // Clone resources
    clone_resources(tx, source_workspace_id, target_workspace_id).await?;

    // Clone variables (including external secret backend replication)
    clone_variables(tx, db, source_workspace_id, target_workspace_id).await?;

    // Clone scripts with new hashes
    clone_scripts(tx, source_workspace_id, target_workspace_id).await?;

    // Clone CI test references
    clone_ci_test_references(tx, source_workspace_id, target_workspace_id).await?;
    clone_macro_registry(tx, source_workspace_id, target_workspace_id).await?;
    clone_asset_usages_and_triggers(tx, source_workspace_id, target_workspace_id).await?;

    // Clone flows with new versions
    clone_flows(tx, source_workspace_id, target_workspace_id).await?;

    // Clone flow nodes
    clone_flow_nodes(tx, source_workspace_id, target_workspace_id).await?;

    // Clone apps with new IDs and app scripts
    let _app_id_mapping = clone_apps(tx, source_workspace_id, target_workspace_id).await?;

    // Clone raw apps
    clone_raw_apps(tx, source_workspace_id, target_workspace_id).await?;

    // Clone the forker's own per-user drafts (plus the legacy NULL-email
    // workspace draft, if any) so they keep their pending edits in the
    // fork. Other users' drafts are intentionally NOT cloned — they don't
    // own a `usr` row in the fork (see `clone_workspace_full`) so their
    // drafts would dangle and the home-page `draft_users` aggregate would
    // surface them as duplicate legacy entries.
    clone_drafts(tx, source_workspace_id, target_workspace_id, authed_email).await?;

    // Clone workspace runnable dependencies and dependency map
    clone_workspace_runnable_dependencies(tx, source_workspace_id, target_workspace_id).await?;

    // Clone workspace dependencies
    clone_workspace_dependencies(tx, source_workspace_id, target_workspace_id).await?;
    Ok(())
}

/// Clone every trigger and schedule from the parent workspace, forcing
/// `mode='disabled'` / `enabled=false`. Always runs at fork creation —
/// disabled rows have no side effects, so cloning them is safe and lets
/// users re-enable selectively in the fork. Listener identifiers
/// (group_id, replication_slot_name, subscription_name, …) are copied
/// verbatim — the runtime suffix that prevents the fork from competing with
/// the parent ships in a follow-up PR.
async fn clone_triggers_and_schedules(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    // Managed ducklake-maintenance schedules are excluded: the fork starts
    // with no ducklake config, so cloned rows could never resolve a lake and
    // the schedule API refuses mutations under the reserved prefix.
    sqlx::query!(
        r#"INSERT INTO schedule (
            workspace_id, path, edited_by, edited_at, schedule, enabled, script_path,
            args, extra_perms, is_flow, email, error, timezone, on_failure,
            on_recovery, on_failure_times, on_failure_exact, on_failure_extra_args,
            on_recovery_times, on_recovery_extra_args, ws_error_handler_muted, retry,
            summary, no_flow_overlap, tag, paused_until, on_success, on_success_extra_args,
            cron_version, description, dynamic_skip, permissioned_as, labels
        )
        SELECT
            $1, path, edited_by, edited_at, schedule, FALSE, script_path,
            args, extra_perms, is_flow, email, error, timezone, on_failure,
            on_recovery, on_failure_times, on_failure_exact, on_failure_extra_args,
            on_recovery_times, on_recovery_extra_args, ws_error_handler_muted, retry,
            summary, no_flow_overlap, tag, paused_until, on_success, on_success_extra_args,
            cron_version, description, dynamic_skip, permissioned_as, labels
        FROM schedule WHERE workspace_id = $2 AND NOT starts_with(path, $3)"#,
        target_workspace_id,
        source_workspace_id,
        windmill_common::workspaces::DUCKLAKE_MAINTENANCE_PATH_PREFIX,
    )
    .execute(&mut **tx)
    .await?;

    // Skip non-workspaced HTTP triggers: their URL has no workspace prefix, so
    // a clone would collide with the parent's row at runtime (matchit::Router
    // silently drops one of two duplicates) and `route_path_key_exists` would
    // also fail to spot the cross-workspace conflict cleanly. The instance
    // settings `CLOUD_HOSTED` and `HTTP_ROUTE_WORKSPACED_ROUTE` force every
    // route to be workspace-prefixed regardless of the column, so when either
    // is on we clone everything.
    let force_workspaced =
        *CLOUD_HOSTED || HTTP_ROUTE_WORKSPACED_ROUTE.load(std::sync::atomic::Ordering::Relaxed);
    sqlx::query!(
        r#"INSERT INTO http_trigger (
            path, route_path, route_path_key, script_path, is_flow, workspace_id,
            edited_by, edited_at, extra_perms, authentication_method, http_method,
            static_asset_config, is_static_website, workspaced_route, wrap_body,
            raw_string, authentication_resource_path, summary, description,
            error_handler_path, error_handler_args, retry, request_type, mode,
            permissioned_as, labels
        )
        SELECT
            path, route_path, route_path_key, script_path, is_flow, $1,
            edited_by, edited_at, extra_perms, authentication_method, http_method,
            static_asset_config, is_static_website, workspaced_route, wrap_body,
            raw_string, authentication_resource_path, summary, description,
            error_handler_path, error_handler_args, retry, request_type, 'disabled'::TRIGGER_MODE,
            permissioned_as, labels
        FROM http_trigger
        WHERE workspace_id = $2
            AND (workspaced_route IS TRUE OR $3)"#,
        target_workspace_id,
        source_workspace_id,
        force_workspaced,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO websocket_trigger (
            path, url, script_path, is_flow, workspace_id, edited_by, edited_at,
            extra_perms, server_id, last_server_ping, error, filters, initial_messages,
            url_runnable_args, can_return_message, error_handler_path, error_handler_args,
            retry, can_return_error_result, mode, permissioned_as, filter_logic, labels,
            heartbeat
        )
        SELECT
            path, url, script_path, is_flow, $1, edited_by, edited_at,
            extra_perms, NULL, NULL, NULL, filters, initial_messages,
            url_runnable_args, can_return_message, error_handler_path, error_handler_args,
            retry, can_return_error_result, 'disabled'::TRIGGER_MODE, permissioned_as, filter_logic, labels,
            heartbeat
        FROM websocket_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO kafka_trigger (
            path, kafka_resource_path, topics, group_id, script_path, is_flow,
            workspace_id, edited_by, edited_at, extra_perms, server_id,
            last_server_ping, error, error_handler_path, error_handler_args, retry,
            mode, filters, auto_offset_reset, reset_offset, auto_commit,
            permissioned_as, filter_logic, labels
        )
        SELECT
            path, kafka_resource_path, topics, group_id, script_path, is_flow,
            $1, edited_by, edited_at, extra_perms, NULL,
            NULL, NULL, error_handler_path, error_handler_args, retry,
            'disabled'::TRIGGER_MODE, filters, auto_offset_reset, reset_offset, auto_commit,
            permissioned_as, filter_logic, labels
        FROM kafka_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO nats_trigger (
            path, nats_resource_path, subjects, stream_name, consumer_name,
            use_jetstream, script_path, is_flow, workspace_id, edited_by, edited_at,
            extra_perms, server_id, last_server_ping, error, error_handler_path,
            error_handler_args, retry, mode, permissioned_as, labels
        )
        SELECT
            path, nats_resource_path, subjects, stream_name, consumer_name,
            use_jetstream, script_path, is_flow, $1, edited_by, edited_at,
            extra_perms, NULL, NULL, NULL, error_handler_path,
            error_handler_args, retry, 'disabled'::TRIGGER_MODE, permissioned_as, labels
        FROM nats_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO postgres_trigger (
            path, script_path, is_flow, workspace_id, edited_by, edited_at,
            extra_perms, postgres_resource_path, error, server_id, last_server_ping,
            replication_slot_name, publication_name, error_handler_path,
            error_handler_args, retry, mode, permissioned_as, labels
        )
        SELECT
            path, script_path, is_flow, $1, edited_by, edited_at,
            extra_perms, postgres_resource_path, NULL, NULL, NULL,
            replication_slot_name, publication_name, error_handler_path,
            error_handler_args, retry, 'disabled'::TRIGGER_MODE, permissioned_as, labels
        FROM postgres_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO mqtt_trigger (
            mqtt_resource_path, subscribe_topics, client_version, v5_config, v3_config,
            client_id, path, script_path, is_flow, workspace_id, edited_by, edited_at,
            extra_perms, server_id, last_server_ping, error, error_handler_path,
            error_handler_args, retry, mode, permissioned_as, labels
        )
        SELECT
            mqtt_resource_path, subscribe_topics, client_version, v5_config, v3_config,
            client_id, path, script_path, is_flow, $1, edited_by, edited_at,
            extra_perms, NULL, NULL, NULL, error_handler_path,
            error_handler_args, retry, 'disabled'::TRIGGER_MODE, permissioned_as, labels
        FROM mqtt_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO sqs_trigger (
            path, queue_url, aws_resource_path, message_attributes, script_path,
            is_flow, workspace_id, edited_by, edited_at, extra_perms, error,
            server_id, last_server_ping, aws_auth_resource_type, error_handler_path,
            error_handler_args, retry, mode, permissioned_as, labels
        )
        SELECT
            path, queue_url, aws_resource_path, message_attributes, script_path,
            is_flow, $1, edited_by, edited_at, extra_perms, NULL,
            NULL, NULL, aws_auth_resource_type, error_handler_path,
            error_handler_args, retry, 'disabled'::TRIGGER_MODE, permissioned_as, labels
        FROM sqs_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO gcp_trigger (
            gcp_resource_path, topic_id, subscription_id, delivery_type,
            delivery_config, path, script_path, is_flow, workspace_id, edited_by,
            edited_at, extra_perms, server_id, last_server_ping, error,
            subscription_mode, error_handler_path, error_handler_args, retry,
            auto_acknowledge_msg, ack_deadline, mode, permissioned_as, labels
        )
        SELECT
            gcp_resource_path, topic_id, subscription_id, delivery_type,
            delivery_config, path, script_path, is_flow, $1, edited_by,
            edited_at, extra_perms, NULL, NULL, NULL,
            subscription_mode, error_handler_path, error_handler_args, retry,
            auto_acknowledge_msg, ack_deadline, 'disabled'::TRIGGER_MODE, permissioned_as, labels
        FROM gcp_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO azure_trigger (
            azure_resource_path, azure_mode, scope_resource_id, topic_name,
            subscription_name, event_type_filters, push_auth_config, path, script_path,
            is_flow, workspace_id, edited_by, email, edited_at, extra_perms, server_id,
            last_server_ping, error, mode, permissioned_as, error_handler_path,
            error_handler_args, retry, labels
        )
        SELECT
            azure_resource_path, azure_mode, scope_resource_id, topic_name,
            subscription_name, event_type_filters, push_auth_config, path, script_path,
            is_flow, $1, edited_by, email, edited_at, extra_perms, NULL,
            NULL, NULL, 'disabled'::TRIGGER_MODE, permissioned_as, error_handler_path,
            error_handler_args, retry, labels
        FROM azure_trigger WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    // Skip non-workspaced email triggers: same shape as the non-workspaced
    // HTTP route case — a clone would share the same `local_part@domain`
    // address as the parent, and incoming mail would arbitrarily land in one
    // or the other. CLOUD_HOSTED scopes email lookup by workspace_id natively,
    // so on cloud we clone everything.
    sqlx::query!(
        r#"INSERT INTO email_trigger (
            path, local_part, workspaced_local_part, script_path, is_flow,
            workspace_id, edited_by, edited_at, extra_perms, error_handler_path,
            error_handler_args, retry, mode, permissioned_as, labels
        )
        SELECT
            path, local_part, workspaced_local_part, script_path, is_flow,
            $1, edited_by, edited_at, extra_perms, error_handler_path,
            error_handler_args, retry, 'disabled'::TRIGGER_MODE, permissioned_as, labels
        FROM email_trigger
        WHERE workspace_id = $2
            AND (workspaced_local_part IS TRUE OR $3)"#,
        target_workspace_id,
        source_workspace_id,
        *CLOUD_HOSTED,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn update_workspace_settings(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO workspace_key (workspace_id, kind, key)
        SELECT $2, kind, key FROM workspace_key WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET
            deploy_to = $1,
            ai_config = source_ws.ai_config,
            large_file_storage = source_ws.large_file_storage,
            ducklake = source_ws.ducklake,
            datatable = source_ws.datatable,
            git_app_installations = source_ws.git_app_installations
        FROM workspace_settings source_ws
        WHERE source_ws.workspace_id = $1
        AND workspace_settings.workspace_id = $2
        "#,
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    let current_git_sync_settings = sqlx::query!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        source_workspace_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let mut git_sync_settings = if let Some(row) = current_git_sync_settings {
        if let Some(git_sync) = row.git_sync {
            serde_json::from_value::<WorkspaceGitSyncSettings>(git_sync)
                .map_err(|err| Error::internal_err(err.to_string()))?
        } else {
            WorkspaceGitSyncSettings::default()
        }
    } else {
        WorkspaceGitSyncSettings::default()
    };

    // We only keep the first git sync repo that is sync mode (use_individual_branch = false), since it is considered the main one
    // Context: see WIN-1559
    git_sync_settings.repositories = git_sync_settings
        .repositories
        .into_iter()
        .filter(|r| !r.use_individual_branch.unwrap_or(false))
        .take(1)
        .map(|mut r| {
            // Auto-pull and fork PRs are parent-owned and must not be inherited:
            // the fork would otherwise carry the parent's webhook id (turning off
            // auto-pull on the fork would delete the parent's webhook). A fork
            // still inherits the push-direction config and the installation.
            // Repo → fork sync is driven by the parent's webhook/poller
            // (`sync_forks`), which routes the fork's `wm-fork/**` branch into it.
            r.auto_pull = None;
            r.fork_open_prs = false;
            r.open_pr_error = None;
            r
        })
        .collect();

    let serialized_config = serde_json::to_value::<WorkspaceGitSyncSettings>(git_sync_settings)
        .map_err(|err| Error::internal_err(err.to_string()))?;

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        serialized_config,
        target_workspace_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_workspace_env(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO workspace_env (workspace_id, name, value)
         SELECT $2, name, value
         FROM workspace_env
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_folders(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, edited_at, created_by, default_permissioned_as, labels)
         SELECT $2, name, display_name, owners, extra_perms, summary, edited_at, created_by, default_permissioned_as, labels
         FROM folder
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_groups(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms)
         SELECT $2, name, summary, extra_perms
         FROM group_
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "INSERT INTO usr_to_group (workspace_id, group_, usr)
         SELECT $2, group_, usr
         FROM usr_to_group
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Copy the source workspace's members (the `usr` rows, carrying each member's role) into the
/// target so a fork/dev can be a shared environment. Idempotent — skips members the target already
/// has. Group memberships are not handled here: the sole caller is the create-fork path, where
/// `clone_groups` already copies the source's full group structure (including `all` membership).
async fn copy_workspace_members(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO usr (workspace_id, username, email, is_admin, created_at, operator, disabled, role, is_service_account, added_via)
         SELECT $1, username, email, is_admin, created_at, operator, disabled, role, is_service_account, added_via
         FROM usr WHERE workspace_id = $2
         ON CONFLICT DO NOTHING",
        target_workspace_id,
        source_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn clone_resource_types(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO resource_type (workspace_id, name, schema, description, edited_at, created_by, format_extension, is_fileset)
         SELECT $2, name, schema, description, edited_at, created_by, format_extension, is_fileset
         FROM resource_type
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_resources(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, edited_at, created_by)
         SELECT $2, path, value, description, resource_type, extra_perms, edited_at, created_by
         FROM resource
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_variables(
    tx: &mut Transaction<'_, Postgres>,
    db: &DB,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms, account, is_oauth, expires_at)
         SELECT $2, path, value, is_secret, description, extra_perms, account, is_oauth, expires_at
         FROM variable
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    // With an external backend the secret lives in the store under (workspace_id,
    // path), so the row copy above leaves the fork pointing at keys that don't
    // exist. Replicate every secret, not just marker-valued ones: migration writes
    // to the store without rewriting `value` to a `$...:` marker.
    if is_vault_backend_configured(db).await? {
        let secret_variables = sqlx::query!(
            "SELECT path FROM variable
             WHERE workspace_id = $1 AND is_secret = true AND value != ''",
            target_workspace_id,
        )
        .fetch_all(&mut **tx)
        .await?;

        let backend = get_secret_backend(db).await?;
        for variable in secret_variables {
            match backend
                .get_secret(source_workspace_id, &variable.path)
                .await
            {
                Ok(plain_value) => {
                    backend
                        .set_secret(target_workspace_id, &variable.path, &plain_value)
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to replicate secret variable {} to the external secret backend for the forked workspace: {e}",
                                variable.path
                            ))
                        })?;
                }
                // The source secret is unreadable (e.g. deleted out-of-band from
                // the external store), so the variable is equally broken in the
                // source workspace — don't let it block forking.
                Err(e) => {
                    tracing::warn!(
                        workspace_id = %source_workspace_id,
                        path = %variable.path,
                        error = %e,
                        "Could not read secret variable from the external secret backend while forking; the forked variable will not resolve"
                    );
                }
            }
        }
    }

    Ok(())
}

async fn clone_scripts(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    // Clone all scripts directly with a single query
    sqlx::query!(
        r#"INSERT INTO script (
            workspace_id, hash, path, parent_hashes, summary, description, content,
            created_by, created_at, archived, schema, deleted, is_template,
            extra_perms, lock, lock_error_logs, language, kind, tag,
            envs, concurrent_limit, concurrency_time_window_s, cache_ttl,
            dedicated_worker, ws_error_handler_muted, priority, timeout,
            delete_after_use, delete_after_secs, restart_unless_cancelled, concurrency_key,
            visible_to_runner_only, auto_kind, codebase, has_preprocessor,
            on_behalf_of_email, assets, modules
        )
        SELECT
            $1, hash, path, parent_hashes, summary, description, content,
            created_by, created_at, archived, schema, deleted, is_template,
            extra_perms, lock, lock_error_logs, language, kind, tag,
            envs, concurrent_limit, concurrency_time_window_s, cache_ttl,
            dedicated_worker, ws_error_handler_muted, priority, timeout,
            delete_after_use, delete_after_secs, restart_unless_cancelled, concurrency_key,
            visible_to_runner_only, auto_kind, codebase, has_preprocessor,
            on_behalf_of_email, assets, modules
        FROM script
        WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_ci_test_references(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO ci_test_reference (workspace_id, test_script_path, test_script_hash, tested_item_path, tested_item_kind)
         SELECT $2, test_script_path, test_script_hash, tested_item_path, tested_item_kind
         FROM ci_test_reference WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// DuckDB macro registry + call-site edges are deploy-derived like
// ci_test_reference: without cloning them, a forked consumer script fails at
// run time until every macro library is manually redeployed in the fork.
async fn clone_macro_registry(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO macro_definition (workspace_id, name, provider_path, params, body, is_table_macro)
         SELECT $2, name, provider_path, params, body, is_table_macro
         FROM macro_definition WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO macro_usage (workspace_id, consumer_path, macro_name)
         SELECT $2, consumer_path, macro_name
         FROM macro_usage WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// Asset usage rows and `// on` subscriber triggers are deploy-derived like
// ci_test_reference / the macro registry: without cloning them the fork's
// pipeline graph has no asset nodes or lineage edges, and — worse — the asset
// dispatch cascade never fires in the fork (it reads `script_trigger`), so
// materializing an upstream node can't trigger its consumers until every
// script is manually redeployed. `job`-kind usage rows (runtime-detected,
// ephemeral) are skipped like the graph does.
async fn clone_asset_usages_and_triggers(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)
         SELECT $2, path, kind, usage_access_type, usage_path, usage_kind, columns
         FROM asset WHERE workspace_id = $1 AND usage_kind IN ('script', 'flow')",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO script_trigger (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref, join_all, debounce_s, retry_count, retry_delay_s)
         SELECT $2, runnable_kind, runnable_path, trigger_kind, trigger_ref, join_all, debounce_s, retry_count, retry_delay_s
         FROM script_trigger WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn clone_flows(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    // First, clone flows without versions
    sqlx::query!(
        "INSERT INTO flow (
            workspace_id, path, summary, description, value, edited_by, edited_at,
            archived, schema, extra_perms, dependency_job, tag,
            ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
            concurrency_key, versions, on_behalf_of_email, lock_error_logs
        )
        SELECT $2, path, summary, description, value, edited_by, edited_at,
               archived, schema, extra_perms, NULL, tag,
               ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
               concurrency_key, ARRAY[]::bigint[], on_behalf_of_email, lock_error_logs
        FROM flow
        WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    // Then clone flow versions
    let flow_versions = sqlx::query!(
        "SELECT id, workspace_id, path, value, schema, created_by, created_at
         FROM flow_version
         WHERE workspace_id = $1
         ORDER BY path, created_at",
        source_workspace_id
    )
    .fetch_all(&mut **tx)
    .await?;

    for version in flow_versions {
        let new_version_id = sqlx::query_scalar!(
            "INSERT INTO flow_version (workspace_id, path, value, schema, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id",
            target_workspace_id,
            version.path,
            version.value,
            version.schema,
            version.created_by,
            version.created_at,
        )
        .fetch_one(&mut **tx)
        .await?;

        // Update flow to include this version
        sqlx::query!(
            "UPDATE flow
             SET versions = array_append(versions, $1)
             WHERE workspace_id = $2 AND path = $3",
            new_version_id,
            target_workspace_id,
            version.path,
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn clone_flow_nodes(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO flow_node (workspace_id, hash, path, lock, code, flow, hash_v2)
         SELECT $2,
                (SELECT COALESCE(MAX(hash), 0) FROM flow_node) + row_number() OVER () AS new_hash,
                source_fn.path, source_fn.lock, source_fn.code, source_fn.flow, source_fn.hash_v2
         FROM flow_node source_fn
         WHERE source_fn.workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_apps(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<HashMap<i64, i64>> {
    // Get all apps from source workspace
    let apps = sqlx::query!(
        "SELECT id, workspace_id, path, summary, policy, versions, extra_perms, custom_path
         FROM app
         WHERE workspace_id = $1",
        source_workspace_id
    )
    .fetch_all(&mut **tx)
    .await?;

    let mut app_id_mapping: HashMap<i64, i64> = HashMap::new();
    // Only a raw app's current (last) version has a bundle worth carrying into the fork: bundles exist
    // only for raw apps, and older versions aren't viewable/runnable (the bundle secret is only ever
    // minted for `versions.last()`). Copying a bundle for every version of every app is what makes
    // forking a workspace with many app versions hang — a serial S3 round-trip per version, held inside
    // the fork transaction. Collect each app's current version here; intersect with raw versions below.
    let mut latest_version_ids: HashSet<i64> = HashSet::new();

    // Clone apps with new IDs
    for app in apps {
        if let Some(&current_version) = app.versions.last() {
            latest_version_ids.insert(current_version);
        }
        let new_app_id = sqlx::query_scalar!(
            "INSERT INTO app (workspace_id, path, summary, policy, versions, extra_perms, custom_path)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
            target_workspace_id,
            app.path,
            app.summary,
            app.policy,
            &Vec::<i64>::new(), // Start with empty versions array
            app.extra_perms,
            app.custom_path,
        )
        .fetch_one(&mut **tx)
        .await?;

        app_id_mapping.insert(app.id, new_app_id);
    }

    let mut version_id_mapping: HashMap<i64, i64> = HashMap::new();
    let mut raw_version_ids: HashSet<i64> = HashSet::new();

    {
        // Clone app versions
        let app_versions = sqlx::query!(
            "SELECT id, app_id, value, created_by, created_at, raw_app
         FROM app_version
         WHERE app_id = ANY(SELECT id FROM app WHERE workspace_id = $1)
         ORDER BY app_id, created_at",
            source_workspace_id
        )
        .fetch_all(&mut **tx)
        .await?;

        for version in app_versions {
            if let Some(&new_app_id) = app_id_mapping.get(&version.app_id) {
                if version.raw_app {
                    raw_version_ids.insert(version.id);
                }
                let new_version_id = sqlx::query_scalar!(
                    "INSERT INTO app_version (app_id, value, created_by, created_at, raw_app)
                 VALUES ($1, $2, $3, $4, $5) RETURNING id",
                    new_app_id,
                    version.value,
                    version.created_by,
                    version.created_at,
                    version.raw_app,
                )
                .fetch_one(&mut **tx)
                .await?;

                version_id_mapping.insert(version.id, new_version_id);
            }
        }
    }

    // The bundles worth cloning: each raw app's current version (latest ∩ raw). Everything else either
    // has no bundle (low-code apps) or an unreachable one (older versions), so we don't touch S3 for it.
    let bundle_version_ids: HashSet<i64> = latest_version_ids
        .intersection(&raw_version_ids)
        .copied()
        .collect();

    // Clone app bundles — only the current version of each raw app (see bundle_version_ids).
    if !bundle_version_ids.is_empty() {
        let old_ids: Vec<i64> = bundle_version_ids.iter().copied().collect();
        let bundles = sqlx::query!(
            "SELECT app_version_id, file_type, data FROM app_bundles
             WHERE app_version_id = ANY($1) AND w_id = $2",
            &old_ids,
            source_workspace_id
        )
        .fetch_all(&mut **tx)
        .await?;

        let mut cloned_from_db: std::collections::HashSet<(i64, String)> = HashSet::new();
        for bundle in &bundles {
            cloned_from_db.insert((bundle.app_version_id, bundle.file_type.clone()));
        }

        for bundle in bundles {
            if let Some(&new_version_id) = version_id_mapping.get(&bundle.app_version_id) {
                sqlx::query!(
                    "INSERT INTO app_bundles (app_version_id, w_id, file_type, data)
                     VALUES ($1, $2, $3, $4)",
                    new_version_id,
                    target_workspace_id,
                    bundle.file_type,
                    bundle.data,
                )
                .execute(&mut **tx)
                .await?;
            }
        }

        // Clone bundles from S3 for versions not found in DB
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        {
            let object_store = windmill_object_store::get_object_store().await;
            if let Some(os) = object_store {
                for (&old_version_id, &new_version_id) in &version_id_mapping {
                    if !bundle_version_ids.contains(&old_version_id) {
                        continue;
                    }
                    for file_type in &["js", "css"] {
                        if cloned_from_db.contains(&(old_version_id, file_type.to_string())) {
                            continue;
                        }
                        // Prefer a server-side copy (no bytes through the backend). A missing source —
                        // e.g. a raw app with a js bundle but no css — surfaces as NotFound and is
                        // skipped. Not every object-store provider supports server-side copy, so fall
                        // back to download+upload on any other error.
                        let src_path =
                            windmill_object_store::object_store_reexports::Path::from(format!(
                                "/app_bundles/{}/{}.{}",
                                source_workspace_id, old_version_id, file_type
                            ));
                        let dst_path =
                            windmill_object_store::object_store_reexports::Path::from(format!(
                                "/app_bundles/{}/{}.{}",
                                target_workspace_id, new_version_id, file_type
                            ));
                        match os.copy(&src_path, &dst_path).await {
                            Ok(()) => {
                                tracing::info!(
                                    "Cloned app bundle in object store: {}.{} -> {}.{}",
                                    old_version_id,
                                    file_type,
                                    new_version_id,
                                    file_type
                                );
                            }
                            Err(windmill_object_store::object_store_reexports::ObjectStoreError::NotFound { .. }) => {
                                // No bundle in the object store for this version/type, skip
                            }
                            Err(copy_err) => {
                                // Provider may not support server-side copy — fall back to get+put.
                                tracing::warn!(
                                    "object store copy failed ({copy_err:#}), falling back to get+put for app bundle {}.{}",
                                    old_version_id, file_type
                                );
                                match os.get(&src_path).await {
                                    Ok(result) => {
                                        let data = result.bytes().await.map_err(
                                            windmill_object_store::object_store_error_to_error,
                                        )?;
                                        os.put(&dst_path, data.into()).await.map_err(
                                            windmill_object_store::object_store_error_to_error,
                                        )?;
                                        tracing::info!(
                                            "Cloned app bundle via get+put fallback: {}.{} -> {}.{}",
                                            old_version_id,
                                            file_type,
                                            new_version_id,
                                            file_type
                                        );
                                    }
                                    Err(windmill_object_store::object_store_reexports::ObjectStoreError::NotFound { .. }) => {
                                        // No bundle for this version/type, skip
                                    }
                                    Err(e) => {
                                        return Err(
                                            windmill_object_store::object_store_error_to_error(e),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Update app versions arrays
    sqlx::query!(
        "UPDATE app SET versions = (
            SELECT array_agg(av.id ORDER BY av.created_at)
            FROM app_version av
            WHERE av.app_id = app.id
        ) WHERE workspace_id = $1",
        target_workspace_id
    )
    .execute(&mut **tx)
    .await?;

    // Clone app scripts with recomputed hashes
    let app_scripts = sqlx::query!(
        "SELECT app, hash, lock, code, code_sha256
         FROM app_script
         WHERE app = ANY(SELECT id FROM app WHERE workspace_id = $1)",
        source_workspace_id
    )
    .fetch_all(&mut **tx)
    .await?;

    for app_script in app_scripts {
        if let Some(&new_app_id) = app_id_mapping.get(&app_script.app) {
            // Recompute hash using app_id, code_sha256, and lock
            let mut hasher = Sha256::new();
            hasher.update(new_app_id.to_be_bytes());
            hasher.update(hex::decode(&app_script.code_sha256)?);
            if let Some(lock) = &app_script.lock {
                hasher.update(lock.as_bytes());
            }
            let new_hash = hex::encode(hasher.finalize());

            sqlx::query!(
                "INSERT INTO app_script (app, hash, lock, code, code_sha256)
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
                new_app_id,
                new_hash,
                app_script.lock,
                app_script.code,
                app_script.code_sha256,
            )
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(app_id_mapping)
}

async fn clone_raw_apps(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO raw_app (path, version, workspace_id, summary, edited_at, data, extra_perms)
         SELECT path, version, $2, summary, edited_at, data, extra_perms
         FROM raw_app
         WHERE workspace_id = $1",
        source_workspace_id,
        target_workspace_id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Clone every per-user draft (and the legacy NULL-email workspace draft,
/// if present) from the parent. The fork target is empty at create time so
/// a plain INSERT is safe — no need to UPSERT against the partial unique
/// indexes (`draft_pkey_with_user` / `draft_pkey_legacy`). `id` is the
/// BIGSERIAL synthetic PK and is regenerated by the default; we don't list
/// it in the column set. `created_at` is preserved so the per-tab
/// `last_sync` baseline the editor reads (`?get_draft=true` → overlay's
/// `draft_saved_at`) lines up with the parent's timeline — otherwise the
/// fork's first POST from any open editor would race a stale `last_sync`
/// and trip the conflict modal on every cloned draft.
// Only `email = authed_email` and the legacy NULL row are cloned — see
// `clone_workspace_data` for the rationale.
async fn clone_drafts(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
    authed_email: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO draft (workspace_id, path, typ, value, created_at, email)
         SELECT $2, path, typ, value, created_at, email
         FROM draft
         WHERE workspace_id = $1 AND (email = $3 OR email IS NULL)",
        source_workspace_id,
        target_workspace_id,
        authed_email,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_workspace_runnable_dependencies(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    // Clone workspace_runnable_dependencies
    sqlx::query!(
        "INSERT INTO workspace_runnable_dependencies (flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id, app_path)
         SELECT flow_path, runnable_path, script_hash, runnable_is_flow, $1, app_path
         FROM workspace_runnable_dependencies
         WHERE workspace_id = $2",
        target_workspace_id,
        source_workspace_id
    )
    .execute(&mut **tx)
    .await?;

    // Clone dependency_map to preserve import relationships
    sqlx::query!(
        "INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id)
         SELECT $1, importer_path, importer_kind, imported_path, importer_node_id
         FROM dependency_map
         WHERE workspace_id = $2",
        target_workspace_id,
        source_workspace_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn clone_workspace_dependencies(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    // Clone workspace_dependencies
    sqlx::query!(
        "INSERT INTO workspace_dependencies (workspace_id, language, name, description, content, archived, created_at)
         SELECT $1, language, name, description, content, archived, created_at
         FROM workspace_dependencies
         WHERE workspace_id = $2",
        target_workspace_id,
        source_workspace_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn deprecated_create_workspace_fork(_authed: ApiAuthed) -> Result<String> {
    return Err(Error::BadRequest("This API endpoint has been relocated. Your Windmill CLI version is outdated and needs to be updated.".to_string()));
}

/// Return the uuids of the git sync jobs to create the branch before creating the fork
async fn create_workspace_fork_branch(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nw): Json<CreateWorkspaceFork>,
) -> JsonResult<Vec<Uuid>> {
    // Pre-check the fork guards before creating any git branch, so we don't leave orphaned branches
    // behind when the follow-up create_workspace_fork would be rejected anyway.
    enforce_fork_depth(&db, &w_id, 0).await?;
    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED {
        enforce_cloud_fork_cap(&db, &w_id).await?;
    }

    if *DISABLE_WORKSPACE_FORK {
        require_super_admin(&db, &authed.email).await?;
    }
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableWorkspaceForking,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    // Two-phase create for git-synced workspaces: this endpoint only creates the git branch(es) and
    // validates up front; it does NOT create the workspace row. The caller follows up with
    // `create_workspace_fork`, which inserts the row and applies the dev designation + prod lock +
    // member copy. So the dev/lock/copy_members fields here are validated only — they are acted on by
    // that second call. Validating early lets a bad request fail before any branch is created.
    if nw.is_dev_workspace {
        validate_dev_workspace_id(&nw.id)?;
        // Reject a bad cosmetic label before any git branch is created (acted on in create_workspace_fork).
        let label = normalize_dev_workspace_label(nw.dev_workspace_label.clone())?;
        reject_dev_label_matching_tracked_branch(&db, label.as_deref(), &[&w_id]).await?;
        ensure_dev_parent_is_root(&db, &w_id).await?;
        // Reject before creating any git branch if the parent already has a dev workspace,
        // otherwise the deferred branch-creation job leaves a dangling branch on the synced repos.
        ensure_no_existing_dev_workspace(&db, &w_id).await?;
        // Creating the canonical dev consumes the parent's one-dev-per-prod slot (and locking the
        // parent mutates its protection rules), so require admin of the parent regardless of the lock
        // flags — mirrors attach/detach, which are prod-admin gated. Without this a non-admin forker
        // could claim the dev slot. Enforced in this first phase too so the request fails before any
        // git branch is created rather than leaving dangling branches.
        require_admin(authed.is_admin, &authed.username)?;
    } else {
        validate_fork_workspace_id(&nw.id)?;
    }
    validate_workspace_name(&nw.name)?;

    // Fail before creating any git branch so a name conflict doesn't leave a
    // dangling branch on the synced repos.
    check_fork_w_id_conflict(&db, &nw.id).await?;
    purge_stale_fork_diff_state(&db, &nw.id).await?;

    Ok(Json(
        handle_fork_branch_creation(&authed.email, &authed.username, &db, &w_id, &nw.id).await?,
    ))
}

/// Update a forked workspace's datatable config to point to the new database.
/// For instance datatables: updates resource_path in the datatable config.
/// For resource datatables: updates the resource's dbname and marks it as ws_specific.
/// Snapshot the schema from the source datatable by connecting to its database.
async fn snapshot_datatable_schema(
    db: &DB,
    parent_w_id: &str,
    dt_name: &str,
) -> Result<serde_json::Value> {
    let pg = get_datatable_resource_from_db_unchecked(db, parent_w_id, dt_name).await?;
    let pg: PgDatabase = serde_json::from_value(pg)
        .map_err(|e| Error::internal_err(format!("Failed to parse db credentials: {}", e)))?;
    let (client, connection) = pg.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });

    let schema = windmill_common::query_builders::pg_get_full_schema(&client)
        .await
        .map_err(Error::internal_err)?;

    drop(client);
    join_handle
        .await
        .map_err(|e| Error::internal_err(format!("join error: {}", e)))?
        .map_err(|e| Error::internal_err(format!("tokio_postgres error: {}", e)))?;

    serde_json::to_value(schema)
        .map_err(|e| Error::internal_err(format!("Failed to serialize schema: {}", e)))
}

async fn apply_forked_datatable(
    db: &DB,
    tx: &mut Transaction<'_, Postgres>,
    parent_w_id: &str,
    forked_w_id: &str,
    fdt: &ForkedDatatableInfo,
) -> Result<()> {
    windmill_common::validate_dbname(&fdt.new_dbname)?;
    if !fdt.new_dbname.starts_with("wm_fork_") {
        return Err(Error::BadRequest(format!(
            "Forked datatable database name '{}' must start with 'wm_fork_'",
            fdt.new_dbname
        )));
    }

    // Snapshot the schema from the source (parent) datatable
    let schema = snapshot_datatable_schema(db, parent_w_id, &fdt.name).await?;
    let forked_from = serde_json::json!({ "schema": schema });

    // Read the datatable config from the forked workspace
    let config_val = sqlx::query_scalar!(
        "SELECT datatable->'datatables'->$2 FROM workspace_settings WHERE workspace_id = $1",
        forked_w_id,
        &fdt.name
    )
    .fetch_optional(&mut **tx)
    .await?
    .flatten()
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Datatable '{}' not found in workspace '{}'",
            fdt.name, forked_w_id
        ))
    })?;

    let dt: DataTable = serde_json::from_value(config_val)
        .map_err(|e| Error::internal_err(format!("Failed to parse datatable config: {}", e)))?;

    if dt.database.resource_type == DataTableCatalogResourceType::Instance {
        // Instance: update resource_path to the new dbname
        sqlx::query!(
            r#"UPDATE workspace_settings
               SET datatable = jsonb_set(
                   jsonb_set(datatable, ARRAY['datatables', $2, 'database', 'resource_path'], to_jsonb($3::text)),
                   ARRAY['datatables', $2, 'forked_from'], $4::jsonb
               )
               WHERE workspace_id = $1"#,
            forked_w_id,
            &fdt.name,
            &fdt.new_dbname,
            forked_from,
        )
        .execute(&mut **tx)
        .await?;
    } else {
        // Resource: update the resource's dbname and mark as ws_specific
        let resource_path = &dt.database.resource_path;
        sqlx::query!(
            r#"UPDATE resource
               SET value = jsonb_set(value, '{dbname}', to_jsonb($3::text))
               WHERE workspace_id = $1 AND path = $2"#,
            forked_w_id,
            resource_path,
            &fdt.new_dbname,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            "INSERT INTO ws_specific (workspace_id, item_kind, path) VALUES ($1, 'resource', $2) ON CONFLICT DO NOTHING",
            forked_w_id,
            resource_path,
        )
        .execute(&mut **tx)
        .await?;

        // Set forked_from on the datatable config
        sqlx::query!(
            r#"UPDATE workspace_settings
               SET datatable = jsonb_set(datatable, ARRAY['datatables', $2, 'forked_from'], $3::jsonb)
               WHERE workspace_id = $1"#,
            forked_w_id,
            &fdt.name,
            forked_from,
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

/// Cloud: require the fork/dev's root (billing) workspace be premium; returns the resolved root id.
#[cfg(feature = "cloud")]
async fn require_cloud_fork_premium(db: &DB, parent_workspace_id: &str) -> Result<String> {
    let root =
        windmill_common::workspaces::get_billing_workspace_id(db, parent_workspace_id).await?;
    if !windmill_common::workspaces::get_team_plan_status(db, &root)
        .await?
        .premium
    {
        return Err(Error::BadRequest(
            "Creating a fork or dev workspace on the cloud requires a paid team plan. Upgrade the workspace first.".to_string(),
        ));
    }
    Ok(root)
}

/// Cloud: reject if adding `incoming` fork/dev workspaces would push `root`'s family over its per-seat
/// allotment. `incoming` is the number of workspaces the operation adds to the family — 1 for a plain
/// create, but `1 + candidate_subtree` for an attach whose candidate already has child forks.
#[cfg(feature = "cloud")]
async fn enforce_cloud_fork_count(db: &DB, root: &str, incoming: i64) -> Result<()> {
    let seats = windmill_common::workspaces::count_paid_seats(db, root).await?;
    let per_seat = *MAX_FORKS_PER_SEAT;
    // Any premium workspace has at least one paid seat, so floor the seat count at 1.
    let allowed = seats.max(1) * per_seat;

    let existing = windmill_common::workspaces::count_workspace_forks(db, root).await?;
    let projected = existing + incoming;
    if projected > allowed {
        return Err(Error::BadRequest(format!(
            "Fork limit reached: this would bring the workspace family to {projected} fork(s), over the cap of {allowed} ({seats} paid seat(s) × {per_seat} per seat). Delete a fork or add seats."
        )));
    }

    Ok(())
}

/// Cloud-only guard for creating a fork/dev workspace. Forks piggyback on the parent's plan (a fork
/// inherits the root's premium and meters its usage into the root's bill), so forking is limited to
/// premium workspaces and capped at `MAX_FORKS_PER_SEAT` per paid (developer) seat of the root.
#[cfg(feature = "cloud")]
async fn enforce_cloud_fork_cap(db: &DB, parent_workspace_id: &str) -> Result<()> {
    let root = require_cloud_fork_premium(db, parent_workspace_id).await?;
    enforce_cloud_fork_count(db, &root, 1).await
}

/// General guardrail (all builds): reject creating a fork/dev under `parent` when it would nest deeper
/// than `MAX_FORK_DEPTH`. `added_subtree_height` is the height of the subtree grafted below the new
/// node — 0 for a plain fork, or the candidate's own subtree height for an attach.
async fn enforce_fork_depth(
    db: &DB,
    parent_workspace_id: &str,
    added_subtree_height: i64,
) -> Result<()> {
    let parent_depth =
        windmill_common::workspaces::fork_chain_depth(db, parent_workspace_id).await?;
    // The new node sits one level below the parent; its deepest descendant adds the grafted height.
    let resulting_depth = parent_depth + 1 + added_subtree_height;
    if resulting_depth > *MAX_FORK_DEPTH {
        return Err(Error::BadRequest(format!(
            "Fork depth limit reached: forks can be nested at most {} level(s) deep, but this would create a fork at depth {}. Fork from a workspace closer to the root instead.",
            *MAX_FORK_DEPTH, resulting_depth
        )));
    }
    Ok(())
}

/// True if `raw` (the text form of a `json` value) contains a genuine `\u0000`
/// NUL escape: a `u0000` preceded by an ODD run of backslashes. Mirrors the
/// parity rule in `strip_null_chars` (windmill-api `apps.rs`) — an even run
/// (`\\u0000`) is an escaped backslash then the literal text "u0000" (common in
/// minified JS) and is jsonb-safe. A genuine NUL is exactly what the
/// `json`→`jsonb` re-encode in `clone_apps` / `clone_flows` rejects with
/// SQLSTATE 22P05.
fn json_text_has_nul_escape(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let mut search_from = 0;
    while let Some(rel) = raw[search_from..].find("u0000") {
        let at = search_from + rel;
        let mut backslashes = 0;
        let mut j = at;
        while j > 0 && bytes[j - 1] == b'\\' {
            backslashes += 1;
            j -= 1;
        }
        if backslashes % 2 == 1 {
            return true;
        }
        search_from = at + 5;
    }
    false
}

/// SQLSTATE 22P05 (`untranslatable_character`) is what Postgres raises for
/// "unsupported Unicode escape sequence" when a `json` value carrying a genuine
/// `\u0000` is re-encoded to `jsonb` — the exact failure the per-row `json`
/// clones (`clone_apps`, `clone_flows`) hit when a source item holds a NUL.
fn is_unsupported_unicode_escape(e: &Error) -> bool {
    matches!(
        e,
        Error::SqlErr { error, .. }
            if error.as_database_error().and_then(|d| d.code()).as_deref() == Some("22P05")
    )
}

/// After a fork clone aborts on a NUL escape, locate the offending source items
/// so the error can name them. Reads the committed source workspace on the pool
/// (the clone transaction is already poisoned and unusable). Only the `json`
/// columns re-encoded to `jsonb` during the clone can trigger the failure:
/// `app_version.value` (clone_apps) and `flow_version.schema` (clone_flows).
/// Best-effort — returns an empty list rather than erroring if a probe query
/// fails, so the caller can still surface a generic message.
async fn find_nul_escape_locations(db: &DB, workspace_id: &str) -> Vec<String> {
    let mut apps: std::collections::BTreeSet<String> = Default::default();
    if let Ok(rows) = sqlx::query(
        "SELECT a.path AS path, av.value::text AS value
         FROM app_version av JOIN app a ON a.id = av.app_id
         WHERE a.workspace_id = $1 AND av.value IS NOT NULL",
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await
    {
        for row in rows {
            let value: String = row.get("value");
            if json_text_has_nul_escape(&value) {
                apps.insert(row.get::<String, _>("path"));
            }
        }
    }

    let mut flows: std::collections::BTreeSet<String> = Default::default();
    if let Ok(rows) = sqlx::query(
        "SELECT fv.path AS path, fv.schema::text AS schema
         FROM flow_version fv
         WHERE fv.workspace_id = $1 AND fv.schema IS NOT NULL",
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await
    {
        for row in rows {
            let schema: String = row.get("schema");
            if json_text_has_nul_escape(&schema) {
                flows.insert(row.get::<String, _>("path"));
            }
        }
    }

    apps.into_iter()
        .map(|p| format!("app: {p}"))
        .chain(flows.into_iter().map(|p| format!("flow: {p}")))
        .collect()
}

async fn create_workspace_fork(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(parent_workspace_id): Path<String>,
    Json(nw): Json<CreateWorkspaceFork>,
) -> Result<String> {
    enforce_fork_depth(&db, &parent_workspace_id, 0).await?;
    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED {
        enforce_cloud_fork_cap(&db, &parent_workspace_id).await?;
    }

    if nw.is_dev_workspace {
        validate_dev_workspace_id(&nw.id)?;
    } else {
        validate_fork_workspace_id(&nw.id)?;
    }
    validate_workspace_name(&nw.name)?;
    // Cosmetic label only applies to dev workspaces; a non-dev fork stores NULL.
    let dev_workspace_label = if nw.is_dev_workspace {
        let label = normalize_dev_workspace_label(nw.dev_workspace_label.clone())?;
        reject_dev_label_matching_tracked_branch(&db, label.as_deref(), &[&parent_workspace_id])
            .await?;
        label
    } else {
        None
    };
    // Check the id conflict before the CE workspace-count limit so that
    // re-using a taken (possibly archived) fork id reports the actual
    // conflict instead of a misleading "maximum number of workspaces" error.
    check_fork_w_id_conflict(&db, &nw.id).await?;
    purge_stale_fork_diff_state(&db, &nw.id).await?;
    // A previously deleted fork with this id may have left ducklake namespaces behind if its
    // physical cleanup failed after the delete committed (registry rows are the durable
    // ledger — no FK, they outlive the workspace). Retry that cleanup now, and refuse to
    // proceed while any metadata schema still can't be dropped: creating the fork anyway
    // would silently reattach the deterministic namespace and its stale tables. Data-file
    // leftovers alone don't block — once the schema is gone they are inert (a deleted fork's
    // `$res:` storage resource is gone forever, so they may never resolve again), and the
    // surviving registry row has the next successful same-prefix cleanup sweep them.
    // `$res:` fallback = the workspace being forked: the deleted fork's resource clones came
    // from a parent, so the new parent is the natural donor for the retry's credentials.
    let leftover_issues = crate::workspaces_extra::drop_forked_ducklake_namespaces_impl(
        &db,
        &nw.id,
        Some(&parent_workspace_id),
    )
    .await?;
    let blocking: Vec<&str> = leftover_issues
        .iter()
        .filter(|i| i.blocking)
        .map(|i| i.msg.as_str())
        .collect();
    if !blocking.is_empty() {
        return Err(Error::BadRequest(format!(
            "a previously deleted workspace with id '{}' left ducklake namespaces that could \
             not be cleaned up: {}; retry once the catalog is reachable again",
            nw.id,
            blocking.join("; ")
        )));
    }
    for i in &leftover_issues {
        tracing::warn!(
            "creating fork {}: leftover ducklake cleanup: {}",
            nw.id,
            i.msg
        );
    }

    #[cfg(not(feature = "enterprise"))]
    _check_nb_of_workspaces(&db).await?;

    if *DISABLE_WORKSPACE_FORK {
        require_super_admin(&db, &authed.email).await?;
    }
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &parent_workspace_id,
        &ProtectionRuleKind::DisableWorkspaceForking,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    if nw.is_dev_workspace {
        ensure_dev_parent_is_root(&db, &parent_workspace_id).await?;
        // Creating the canonical dev consumes the parent's one-dev-per-prod slot (and locking prod
        // mutates its protection rules), so require admin of the parent regardless of the lock flags —
        // mirrors attach/detach, which are prod-admin gated. Without this a non-admin forker could
        // claim the dev slot (and, without member copy, prod admins might not even see it to detach).
        require_admin(authed.is_admin, &authed.username)?;
        ensure_no_existing_dev_workspace(&db, &parent_workspace_id).await?;
    }

    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    let forked_id = nw.id;

    sqlx::query!(
        "INSERT INTO workspace
            (id, name, owner, parent_workspace_id, is_dev_workspace, dev_workspace_label)
            VALUES ($1, $2, $3, $4, $5, $6)",
        forked_id,
        nw.name,
        authed.email,
        parent_workspace_id,
        nw.is_dev_workspace,
        dev_workspace_label,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO workspace_settings
            (workspace_id, color)
            VALUES ($1, $2)",
        forked_id,
        nw.color,
    )
    .execute(&mut *tx)
    .await?;

    // Optionally bring the parent's members into the fork (a shared dev env). Dev-only: it's part of the
    // dev-workspace feature (and the frontend only offers it there), so the backend enforces it rather
    // than trusting the client — copying the parent's whole team into an ordinary throwaway fork isn't
    // intended. Dev creation is already admin-gated, so this is transitively admin-only too. Done before
    // the explicit creator insert below so the creator (a parent member) is copied with full metadata
    // (operator/role/is_service_account/added_via), not the bare row the insert alone would leave.
    if nw.copy_members && nw.is_dev_workspace {
        copy_workspace_members(&mut tx, &parent_workspace_id, &forked_id).await?;
    }

    // Ensure the creator is a member of the fork even without copy_members (or if they aren't a parent
    // member). No-op when copy_members already brought their full row.
    sqlx::query!(
        "INSERT INTO usr
           (workspace_id, email, username, is_admin)
           SELECT $1, email, username, is_admin FROM usr
         WHERE workspace_id = $3 AND email = $2
         ON CONFLICT DO NOTHING
        ",
        forked_id,
        authed.email,
        parent_workspace_id,
    )
    .execute(&mut *tx)
    .await?;

    // Clone all data from the parent workspace using Rust implementation
    if let Err(e) = clone_workspace_data(
        &mut tx,
        &db,
        &parent_workspace_id,
        &forked_id,
        &authed.email,
    )
    .await
    {
        // A genuine `\u0000` in a source `json` value (`app_version.value` /
        // `flow_version.schema`) aborts the clone when it is re-encoded to jsonb:
        // Postgres raises 22P05 with only "unsupported Unicode escape sequence"
        // and no hint at which item. Pinpoint the offenders so the user can fix
        // them — re-saving strips the NUL, and the usual source is a binary file
        // (e.g. `.DS_Store`) accidentally bundled into a raw app.
        if is_unsupported_unicode_escape(&e) {
            drop(tx); // release the poisoned connection before probing on the pool
            let locations = find_nul_escape_locations(&db, &parent_workspace_id).await;
            let where_clause = if locations.is_empty() {
                "The offending item could not be pinpointed — check recently edited apps and flows."
                    .to_string()
            } else {
                format!("Offending item(s):\n  - {}", locations.join("\n  - "))
            };
            return Err(Error::BadRequest(format!(
                "Cannot fork workspace '{parent_workspace_id}': an item contains a NUL character \
                 (\\u0000) that Postgres cannot store as jsonb. Re-save the item to remove it \
                 (the editor strips NUL automatically), or delete the offending binary/character \
                 from its source (often a file like .DS_Store bundled into a raw app). {where_clause}"
            )));
        }
        return Err(e);
    }

    // Clone triggers and schedules unconditionally, always with mode='disabled' /
    // enabled=false. Disabled rows have no side effects (no listener
    // attaches, no cron fires) so this is safe by construction. The user
    // re-enables in the fork, with parent-conflict warnings on enable.
    clone_triggers_and_schedules(&mut tx, &parent_workspace_id, &forked_id).await?;

    // Update forked datatable settings to point to new databases
    for fdt in &nw.forked_datatables {
        apply_forked_datatable(&db, &mut tx, &parent_workspace_id, &forked_id, fdt).await?;
    }

    // The settings clone copies the source's ducklake config verbatim — including a parent
    // fork's own `fork_behavior` stamps. Sharing is a per-fork-creation choice, never
    // inherited: reset any cloned stamps first, then apply this fork's requested list.
    sqlx::query!(
        r#"UPDATE workspace_settings
           SET ducklake = jsonb_set(ducklake, '{ducklakes}', (
               SELECT COALESCE(jsonb_object_agg(key, value - 'fork_behavior'), '{}'::jsonb)
               FROM jsonb_each(ducklake->'ducklakes')
           ))
           WHERE workspace_id = $1 AND jsonb_typeof(ducklake->'ducklakes') = 'object'"#,
        &forked_id,
    )
    .execute(&mut *tx)
    .await?;

    // Stamp the per-lake ducklake fork choice into the fork's own settings. Only the `shared`
    // opt-out needs stamping — absent `fork_behavior` already means isolated (the default),
    // so unlisted lakes and API callers that omit the field stay safe.
    for lake in &nw.shared_ducklakes {
        let stamped = sqlx::query_scalar!(
            r#"UPDATE workspace_settings
               SET ducklake = jsonb_set(ducklake, ARRAY['ducklakes', $2, 'fork_behavior'], '"shared"')
               WHERE workspace_id = $1 AND ducklake->'ducklakes' ? $2
               RETURNING 1 AS "one!""#,
            &forked_id,
            lake,
        )
        .fetch_optional(&mut *tx)
        .await?;
        if stamped.is_none() {
            return Err(Error::BadRequest(format!(
                "cannot mark ducklake `{lake}` as shared: no such lake in the workspace settings"
            )));
        }
    }

    // Lock the parent ("prod") so edits are funneled through this dev workspace.
    let locked_prod = nw.is_dev_workspace && (nw.lock_prod_deploy || nw.lock_prod_forking);
    if locked_prod {
        lock_prod_workspace(
            &mut tx,
            &parent_workspace_id,
            nw.lock_prod_deploy,
            nw.lock_prod_forking,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.create_fork",
        ActionKind::Create,
        &forked_id,
        Some(nw.name.as_str()),
        None,
    )
    .await?;
    tx.commit().await?;

    // A pre-creation lookup could have cached an EMPTY ancestor chain for this id, which
    // would bypass ducklake fork isolation for the TTL.
    windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&forked_id);

    if locked_prod {
        windmill_common::workspaces::invalidate_protection_rules_cache(&parent_workspace_id);
    }

    Ok(format!("Created forked workspace {}", &forked_id))
}

#[derive(Deserialize)]
struct AttachDevWorkspace {
    dev_workspace_id: String,
    #[serde(default)]
    lock_prod_deploy: bool,
    #[serde(default)]
    lock_prod_forking: bool,
    /// Cosmetic display label for the attached dev workspace: 'dev' | 'staging'. None defaults to 'dev'.
    #[serde(default)]
    dev_workspace_label: Option<String>,
}

#[derive(Deserialize)]
struct DetachDevWorkspace {
    dev_workspace_id: String,
}

/// Pair an existing standalone workspace to this workspace ("prod") as its dev workspace, without
/// cloning any data (both already exist). Sets the dev's parent + deploy_to to prod and, optionally,
/// locks prod against direct deployment.
async fn attach_dev_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(prod_w_id): Path<String>,
    Json(req): Json<AttachDevWorkspace>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    // Attaching grafts the candidate (and its own fork subtree) under prod, so enforce the general
    // depth limit on the deepest resulting node.
    let candidate_height =
        windmill_common::workspaces::fork_subtree_height(&db, &req.dev_workspace_id).await?;
    enforce_fork_depth(&db, &prod_w_id, candidate_height).await?;

    // Attaching reparents a workspace under prod (one dev per prod, admin-gated) and it then draws
    // prod's plan, so hold it to the same premium requirement as creating a fork. Only enforce the
    // per-seat count when the attach actually adds a new workspace to the family: re-designating a
    // workspace already under this root as its dev doesn't increase the descendant count.
    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED {
        let root = require_cloud_fork_premium(&db, &prod_w_id).await?;
        // Only count against the cap when this attach adds workspaces to the family (candidate not
        // already under this root). The candidate may itself have child forks, so reserve slots for its
        // whole incoming subtree (the candidate + its descendants), not just one.
        if windmill_common::workspaces::get_billing_workspace_id(&db, &req.dev_workspace_id).await?
            != root
        {
            let incoming =
                1 + windmill_common::workspaces::count_workspace_forks(&db, &req.dev_workspace_id)
                    .await?;
            enforce_cloud_fork_count(&db, &root, incoming).await?;
        }
    }

    let dev_w_id = req.dev_workspace_id;
    if dev_w_id == prod_w_id {
        return Err(Error::BadRequest(
            "A workspace cannot be its own dev workspace".to_string(),
        ));
    }

    // The id is interpolated into a `wm-fork/<branch>/<id>` branch name like any fork.
    validate_dev_workspace_id(&dev_w_id)?;
    let dev_workspace_label = normalize_dev_workspace_label(req.dev_workspace_label.clone())?;
    // The attached workspace keeps its own sync repos and prod keeps its config;
    // the label branch must not collide with either side's tracked branch.
    reject_dev_label_matching_tracked_branch(
        &db,
        dev_workspace_label.as_deref(),
        &[&prod_w_id, &dev_w_id],
    )
    .await?;

    let dev = sqlx::query!(
        r#"SELECT parent_workspace_id, deleted FROM workspace WHERE id = $1"#,
        &dev_w_id
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Workspace {} not found", dev_w_id)))?;

    if dev.deleted {
        return Err(Error::BadRequest(format!(
            "Workspace {} is archived",
            dev_w_id
        )));
    }
    // A candidate that already belongs to a DIFFERENT parent can't be attached. A candidate already
    // parented to this prod is allowed: it's the recovery path after renaming a dev workspace (the
    // rename keeps the parent but drops the dev flag), and re-designating an existing fork of this
    // prod as its dev.
    if dev
        .parent_workspace_id
        .as_deref()
        .is_some_and(|p| p != prod_w_id)
    {
        return Err(Error::BadRequest(format!(
            "Workspace {} is already a fork or dev workspace of another workspace",
            dev_w_id
        )));
    }
    // The candidate can't itself be a prod with its own dev workspace (no nested dev chains).
    ensure_no_existing_dev_workspace(&db, &dev_w_id).await?;

    // Prod must be a root workspace, otherwise attaching could form a parent<->child cycle (e.g.
    // attaching A as the dev of B when B is already the dev of A), which breaks hierarchy traversal.
    let prod_has_parent = sqlx::query_scalar!(
        r#"SELECT (parent_workspace_id IS NOT NULL) AS "has_parent!" FROM workspace WHERE id = $1"#,
        &prod_w_id
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Workspace {} not found", prod_w_id)))?;
    if prod_has_parent {
        return Err(Error::BadRequest(format!(
            "Workspace {} is itself a fork or dev workspace and cannot be a prod workspace",
            prod_w_id
        )));
    }

    // The caller must be admin of the dev workspace too (or a superadmin).
    let is_admin_of_dev = sqlx::query_scalar!(
        "SELECT is_admin FROM usr WHERE workspace_id = $1 AND email = $2",
        &dev_w_id,
        &authed.email
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(false);
    if !is_admin_of_dev && !windmill_common::auth::is_super_admin_email(&db, &authed.email).await? {
        return Err(Error::PermissionDenied(format!(
            "Attaching workspace '{dev_w_id}' as a dev requires being an admin of it (or a superadmin)"
        )));
    }

    ensure_no_existing_dev_workspace(&db, &prod_w_id).await?;

    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace SET parent_workspace_id = $1, is_dev_workspace = true, dev_workspace_label = $3 WHERE id = $2",
        &prod_w_id,
        &dev_w_id,
        dev_workspace_label,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE workspace_settings SET deploy_to = $1 WHERE workspace_id = $2",
        &prod_w_id,
        &dev_w_id
    )
    .execute(&mut *tx)
    .await?;

    // The attached workspace is now parent-managed like any fork: its own
    // auto-pull (and webhook), fork PRs, and promotion repos must not stay
    // live — they'd keep pulling/pushing against its pre-attach tracked
    // branch. Mirror the fork-creation copy: keep sync repos only, strip the
    // parent-only fields, and delete any managed webhook after commit.
    #[allow(unused_mut)]
    let mut stripped_webhooks: Vec<(String, i64)> = Vec::new();
    if let Some(git_sync) = sqlx::query_scalar!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        &dev_w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten()
    {
        if let Ok(mut settings) = serde_json::from_value::<WorkspaceGitSyncSettings>(git_sync) {
            settings
                .repositories
                .retain(|r| !r.use_individual_branch.unwrap_or(false));
            for r in settings.repositories.iter_mut() {
                if let Some(hook) = r.auto_pull.as_ref().and_then(|a| a.webhook_id) {
                    stripped_webhooks.push((r.git_repo_resource_path.clone(), hook));
                }
                r.auto_pull = None;
                r.fork_open_prs = false;
                r.open_pr_error = None;
            }
            let serialized =
                serde_json::to_value(&settings).map_err(|e| Error::internal_err(e.to_string()))?;
            sqlx::query!(
                "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
                serialized,
                &dev_w_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    if req.lock_prod_deploy || req.lock_prod_forking {
        lock_prod_workspace(
            &mut tx,
            &prod_w_id,
            req.lock_prod_deploy,
            req.lock_prod_forking,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.attach_dev_workspace",
        ActionKind::Update,
        &prod_w_id,
        Some(&dev_w_id),
        None,
    )
    .await?;
    tx.commit().await?;

    // The dev workspace's parent just changed (none -> prod); drop its cached fork->parent mapping
    // so per-workspace job tags route to the prod family immediately rather than after the TTL.
    windmill_queue::tags::invalidate_fork_parent_cache(&dev_w_id);
    // Best-effort: the hooks captured before the strip above are unreachable now
    // (their auto_pull is gone), so remove them from GitHub.
    #[cfg(all(feature = "enterprise", feature = "private"))]
    for (path, hook_id) in stripped_webhooks {
        if let Ok(url) = windmill_common::git_sync_ee::resolve_repo_url(&db, &dev_w_id, &path).await
        {
            let _ =
                windmill_common::git_sync_ee::delete_repo_webhook(&db, &dev_w_id, &url, hook_id)
                    .await;
        }
    }
    // Drop the cached ancestor chains too — the workspace existed BEFORE the attach, so a
    // cached empty chain reads as "not a fork" and its ducklake jobs would write the shared
    // lake until the TTL. Descendants' chains also gained the new root.
    windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&dev_w_id);
    for id in windmill_common::workspaces::list_fork_descendants(&db, &dev_w_id).await? {
        windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&id);
    }
    // Same reparent invalidates the billing-workspace mapping so its usage meters to prod at once. The
    // candidate can bring its own fork subtree, whose descendants had resolved their (now-stale) root
    // to the candidate's old family; invalidate them too so they meter to prod without waiting out the
    // 60s TTL. Their immediate fork->parent links don't move, so the tag-routing cache needs no change.
    #[cfg(feature = "cloud")]
    {
        windmill_common::workspaces::invalidate_billing_workspace_cache(&dev_w_id);
        for id in windmill_common::workspaces::list_fork_descendants(&db, &dev_w_id).await? {
            windmill_common::workspaces::invalidate_billing_workspace_cache(&id);
        }
    }

    if req.lock_prod_deploy || req.lock_prod_forking {
        windmill_common::workspaces::invalidate_protection_rules_cache(&prod_w_id);
    }

    Ok(format!(
        "Attached {} as dev workspace of {}",
        dev_w_id, prod_w_id
    ))
}

/// Reverse [`attach_dev_workspace`] / clear the dev designation: unset the dev flag and remove the
/// prod lock. The workspace keeps its `parent_workspace_id` (it remains an ordinary fork).
async fn detach_dev_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(prod_w_id): Path<String>,
    Json(req): Json<DetachDevWorkspace>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let dev_w_id = req.dev_workspace_id;
    let is_dev_of_prod = sqlx::query_scalar!(
        r#"SELECT EXISTS(
            SELECT 1 FROM workspace
            WHERE id = $1 AND parent_workspace_id = $2 AND is_dev_workspace
        )"#,
        &dev_w_id,
        &prod_w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    if !is_dev_of_prod {
        return Err(Error::BadRequest(format!(
            "{} is not the dev workspace of {}",
            dev_w_id, prod_w_id
        )));
    }

    let mut tx = db.begin().await?;
    // A wm-fork- workspace re-designated as dev returns to being a plain fork
    // (keeps its parent); a standalone workspace that was attached returns to
    // being standalone — with the parent kept it would still classify as a
    // fork and deploy to wm-fork/** branches forever.
    sqlx::query!(
        "UPDATE workspace SET is_dev_workspace = false,
         parent_workspace_id = CASE WHEN id LIKE 'wm-fork-%' THEN parent_workspace_id ELSE NULL END
         WHERE id = $1",
        &dev_w_id
    )
    .execute(&mut *tx)
    .await?;
    // Only one dev per prod, so detaching it means prod no longer has a dev: drop the lock rule.
    sqlx::query!(
        "DELETE FROM workspace_protection_rule WHERE workspace_id = $1 AND name = $2",
        &prod_w_id,
        DEV_WORKSPACE_LOCK_RULE_NAME
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.detach_dev_workspace",
        ActionKind::Update,
        &prod_w_id,
        Some(&dev_w_id),
        None,
    )
    .await?;
    tx.commit().await?;

    windmill_common::workspaces::invalidate_protection_rules_cache(&prod_w_id);
    // The parent link may just have been cleared (standalone workspace that was
    // attached): drop the caches that resolved it, mirroring attach.
    windmill_queue::tags::invalidate_fork_parent_cache(&dev_w_id);
    windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&dev_w_id);
    for id in windmill_common::workspaces::list_fork_descendants(&db, &dev_w_id).await? {
        windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&id);
    }
    #[cfg(feature = "cloud")]
    {
        windmill_common::workspaces::invalidate_billing_workspace_cache(&dev_w_id);
        for id in windmill_common::workspaces::list_fork_descendants(&db, &dev_w_id).await? {
            windmill_common::workspaces::invalidate_billing_workspace_cache(&id);
        }
    }

    Ok(format!(
        "Detached dev workspace {} from {}",
        dev_w_id, prod_w_id
    ))
}

async fn edit_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ew): Json<EditWorkspace>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace SET name = $1, owner = $2 WHERE id = $3",
        ew.name,
        ew.owner,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.update",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Updated workspace {}", &w_id))
}

/// Archive a workspace: disable schedules, cancel jobs, and mark as deleted.
/// Returns (schedules_disabled_count, jobs_canceled_count).
pub(crate) async fn archive_workspace_impl(
    db: &DB,
    w_id: &str,
    username: &str,
    // When archiving a dev workspace, its parent prod. The pairing teardown (clear is_dev + drop the
    // prod's lock) is folded into the same transaction as `deleted = true` so it's atomic with the
    // archive — a later failure can't strand a half-archived dev that's still flagged/locked.
    dev_lock_parent: Option<&str>,
) -> Result<(usize, usize, usize)> {
    // Step 1: Disable all schedules and clear their queued jobs
    let mut tx = db.begin().await?;
    let disabled_schedules = sqlx::query_scalar!(
        "UPDATE schedule SET enabled = false WHERE workspace_id = $1 AND enabled = true RETURNING path",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let schedules_count = disabled_schedules.len();
    tracing::info!(
        "Disabled {} schedules in workspace {}",
        schedules_count,
        w_id
    );

    // Clear all schedule-related jobs using the existing clear_schedule function
    for schedule_path in &disabled_schedules {
        windmill_queue::schedule::clear_schedule(&mut tx, schedule_path, w_id).await?;
    }

    // Delete non-session tokens scoped to this workspace
    let deleted_tokens = sqlx::query_scalar!(
        "DELETE FROM token WHERE workspace_id = $1 AND label IS DISTINCT FROM 'session' RETURNING token_prefix",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    tracing::info!(
        "Deleted {} non-session tokens in workspace {}",
        deleted_tokens.len(),
        w_id
    );

    // Mark workspace as archived
    sqlx::query!("UPDATE workspace SET deleted = true WHERE id = $1", w_id)
        .execute(&mut *tx)
        .await?;

    if let Some(prod) = dev_lock_parent {
        // Dissolve the dev pairing atomically with the archive: clear the canonical-dev flag (so the
        // archived row no longer occupies the parent's one-dev slot), and drop the prod's lock unless a
        // replacement dev already holds it (NOT EXISTS sees the just-cleared flag within this tx, so the
        // row being archived doesn't count).
        sqlx::query!(
            "UPDATE workspace SET is_dev_workspace = false WHERE id = $1",
            w_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "DELETE FROM workspace_protection_rule WHERE workspace_id = $1 AND name = $2
             AND NOT EXISTS (
                 SELECT 1 FROM workspace
                 WHERE parent_workspace_id = $1 AND is_dev_workspace AND deleted = false
             )",
            prod,
            DEV_WORKSPACE_LOCK_RULE_NAME
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Step 2: Get all remaining queued jobs for this workspace (non-schedule jobs)
    let jobs_to_cancel =
        sqlx::query_scalar!("SELECT id FROM v2_job_queue WHERE workspace_id = $1", w_id)
            .fetch_all(db)
            .await?;

    let jobs_count = jobs_to_cancel.len();
    tracing::info!(
        "Found {} remaining jobs to cancel in workspace {}",
        jobs_count,
        w_id
    );

    // Step 3: Cancel all remaining jobs using the existing cancel_jobs function
    let canceled_count = if !jobs_to_cancel.is_empty() {
        let axum::Json(canceled_jobs) = windmill_api_jobs::cancel_jobs(
            jobs_to_cancel,
            db,
            username,
            w_id,
            false, // force_cancel
        )
        .await?;

        let count = canceled_jobs.len();
        tracing::info!("Canceled {} jobs in workspace {}", count, w_id);
        count
    } else {
        0
    };

    Ok((schedules_count, canceled_count, deleted_tokens.len()))
}

async fn archive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    // CE caps the number of archived (soft-deleted) workspaces so archiving can't be used to
    // stockpile hidden workspaces. Enforced here so a second archive is refused up front.
    #[cfg(not(feature = "enterprise"))]
    _check_nb_of_archived_workspaces(&db).await?;

    // If this is an attached dev workspace, archiving it leaves the prod with no active dev (the
    // unique index and user_workspaces both ignore deleted=true), so clear the prod's
    // dev_workspace_lock too. Gate it on prod-admin since it removes prod's protection rule (mirrors
    // detach/delete) — a dev-admin who isn't a prod-admin must not be able to unlock prod this way.
    let dev_lock_parent: Option<String> = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1 AND is_dev_workspace",
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    if let Some(ref prod) = dev_lock_parent {
        let is_prod_admin = sqlx::query_scalar!(
            "SELECT is_admin FROM usr WHERE workspace_id = $1 AND email = $2",
            prod,
            &authed.email
        )
        .fetch_optional(&db)
        .await?
        .unwrap_or(false);
        if !is_prod_admin
            && !windmill_common::auth::is_super_admin_email(&db, &authed.email).await?
        {
            return Err(Error::PermissionDenied(format!(
                "Archiving dev workspace '{w_id}' requires being an admin of its parent prod workspace '{prod}' (or a superadmin)"
            )));
        }
    }

    // The dev pairing teardown (clear is_dev + drop the prod lock) runs inside archive_workspace_impl's
    // transaction, atomically with `deleted = true`.
    let (schedules_count, canceled_count, deleted_tokens_count) =
        archive_workspace_impl(&db, &w_id, &authed.username, dev_lock_parent.as_deref()).await?;

    // Audit log
    let mut tx = db.begin().await?;
    let mut audit_params = HashMap::new();
    audit_params.insert("disabled_schedules", schedules_count.to_string());
    audit_params.insert("canceled_jobs", canceled_count.to_string());
    audit_params.insert("deleted_tokens", deleted_tokens_count.to_string());
    let audit_params_refs: HashMap<&str, &str> =
        audit_params.iter().map(|(k, v)| (*k, v.as_str())).collect();

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.archive",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(audit_params_refs.clone()),
    )
    .await?;
    // Also record under the instance-level "admins" workspace so superadmins can
    // discover who archived a workspace after it becomes hidden from the UI.
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.archive",
        ActionKind::Update,
        "admins",
        Some(&w_id),
        Some(audit_params_refs),
    )
    .await?;
    tx.commit().await?;

    if let Some(prod) = dev_lock_parent {
        windmill_common::workspaces::invalidate_protection_rules_cache(&prod);
    }

    Ok(format!(
        "Archived workspace {}, disabled {} schedules, canceled {} jobs and deleted {} tokens",
        &w_id, schedules_count, canceled_count, deleted_tokens_count
    ))
}

async fn leave_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    let mut tx = db.begin().await?;
    sqlx::query!(
        "DELETE FROM usr WHERE workspace_id = $1 AND email = $2",
        &w_id,
        &authed.email
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.leave",
        ActionKind::Delete,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Left workspace {}", &w_id))
}

async fn unarchive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    // Unarchiving re-activates a soft-deleted workspace, so it must respect the
    // same CE workspace-count cap as creating one. The archived workspace is
    // deleted = true and thus excluded from the count until it is restored.
    #[cfg(not(feature = "enterprise"))]
    _check_nb_of_workspaces(&db).await?;

    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = false WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.unarchive",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    // Also record under the instance-level "admins" workspace so superadmins keep
    // a durable trail of who unarchived a workspace.
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.unarchive",
        ActionKind::Update,
        "admins",
        Some(&w_id),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Unarchived workspace {}", &w_id))
}

/// Whether the instance is configured to suppress the email notifications sent
/// when a user is invited or added to a workspace. Defaults to false (emails on).
async fn workspace_invite_emails_disabled(db: &DB) -> Result<bool> {
    Ok(
        windmill_common::global_settings::load_value_from_global_settings(
            db,
            DISABLE_WORKSPACE_INVITE_EMAILS_SETTING,
        )
        .await?
        .and_then(|v| v.as_bool())
        .unwrap_or(false),
    )
}

async fn invite_user(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    #[cfg(not(feature = "enterprise"))]
    if w_id == "admins" {
        return Err(Error::BadRequest(
            "The admins workspace is reserved for superadmins. Members cannot be added to it without an enterprise license.".to_string(),
        ));
    }

    nu.email = nu.email.to_lowercase();

    #[cfg(feature = "enterprise")]
    if let Some(msg) =
        windmill_common::ee_oss::check_seat_cap_for_new_user(&db, &nu.email, nu.operator).await?
    {
        return Err(Error::BadRequest(msg));
    }

    let mut tx = db.begin().await?;

    let already_in_workspace = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
        &w_id,
        nu.email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if already_in_workspace {
        return Err(Error::BadRequest(format!(
            "user with email {} already exists in workspace {}",
            nu.email, w_id
        )));
    }

    sqlx::query!(
        "INSERT INTO workspace_invite
            (workspace_id, email, is_admin, operator)
            VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, email)
            DO UPDATE SET is_admin = EXCLUDED.is_admin, operator = EXCLUDED.operator",
        &w_id,
        nu.email,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    if !workspace_invite_emails_disabled(&db).await? {
        send_email_if_possible(
            &format!("Invited to Windmill's workspace: {w_id}"),
            &format!(
                "You have been granted access to Windmill's workspace {w_id}

If you do not have an account on {}, login with SSO or ask an admin to create an account for you.",
                (**BASE_URL.load()).clone()
            ),
            &nu.email,
        );
    }

    webhook.send_instance_event(InstanceEvent::UserInvitedWorkspace {
        email: nu.email.clone(),
        workspace: w_id,
    });

    Ok((
        StatusCode::CREATED,
        format!("user with email {} invited", nu.email),
    ))
}

/// Non-admin path for `add_user`: the creator of a fork may bring collaborators into the fork they
/// created, so a team can work on it without an admin of the fork having to step in. The grant is
/// deliberately narrow, because a fork holds a full clone of its parent (secrets included) and the
/// creator may be an ordinary developer:
/// - only on a fork they created, never on a root workspace;
/// - only as a developer, so it can never mint an admin (nor an operator, which would need the
///   workspace's operator settings to be meaningful);
/// - only for someone who is already a developer or admin of the parent, so pulling them into the
///   fork cannot widen who can read the parent's data.
///
/// Anything outside those bounds stays an admin's call. Returns the username the new member must be
/// given in the fork.
async fn authorize_fork_owner_add_user(
    db: &DB,
    w_id: &str,
    authed: &ApiAuthed,
    nu: &NewWorkspaceUser,
) -> Result<String> {
    let parent = windmill_common::workspaces::fork_owned_by(db, w_id, &authed.email)
        .await?
        .ok_or_else(|| Error::RequireAdmin(authed.username.clone()))?;

    if nu.is_admin || nu.operator {
        return Err(Error::PermissionDenied(format!(
            "as the creator of fork {w_id} you can only add members as developers; ask an admin of \
             {w_id} for any other role"
        )));
    }

    let parent_username = sqlx::query_scalar!(
        "SELECT username FROM usr
         WHERE workspace_id = $1 AND email = $2 AND NOT operator AND NOT disabled",
        parent,
        nu.email,
    )
    .fetch_optional(db)
    .await?;

    let Some(parent_username) = parent_username else {
        return Err(Error::PermissionDenied(format!(
            "as the creator of fork {w_id} you can only add developers or admins of its parent \
             workspace {parent}; {} is not one, so only an admin of {w_id} can add them",
            nu.email
        )));
    };

    // Ownership of a `u/<username>/` path is decided by the username alone, and the fork holds a
    // clone of every such path from the parent. Seating the new member on a username other than
    // their own would therefore hand them that parent user's cloned scripts, variables and secrets
    // — so their parent username is the only one they may be given here, whatever the caller asked
    // for (`add_user` otherwise lets the caller choose it when AUTOMATE_USERNAME_CREATION is off).
    if nu
        .username
        .as_deref()
        .is_some_and(|u| !u.is_empty() && u != parent_username)
    {
        return Err(Error::PermissionDenied(format!(
            "as the creator of fork {w_id} you cannot choose the username of a member you add; {} \
             joins as '{parent_username}', the username they already have in {parent}",
            nu.email
        )));
    }

    Ok(parent_username)
}

async fn add_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut nu): Json<NewWorkspaceUser>,
) -> Result<(StatusCode, String)> {
    #[cfg(not(feature = "enterprise"))]
    if w_id == "admins" {
        return Err(Error::BadRequest(
            "The admins workspace is reserved for superadmins. Members cannot be added to it without an enterprise license.".to_string(),
        ));
    }

    nu.email = nu.email.to_lowercase();

    let fork_owner_username = if !authed.is_admin {
        Some(authorize_fork_owner_add_user(&db, &w_id, &authed, &nu).await?)
    } else {
        None
    };

    #[cfg(feature = "enterprise")]
    if let Some(msg) =
        windmill_common::ee_oss::check_seat_cap_for_new_user(&db, &nu.email, nu.operator).await?
    {
        return Err(Error::BadRequest(msg));
    }

    let mut tx = db.begin().await?;

    let already_exists_email = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
        &w_id,
        nu.email,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if already_exists_email {
        return Err(Error::BadRequest(format!(
            "user with email {} already exists in workspace {}",
            nu.email, w_id
        )));
    }

    let automate_username_creation = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        AUTOMATE_USERNAME_CREATION_SETTING,
    )
    .fetch_optional(&mut *tx)
    .await?
    .map(|v| v.as_bool())
    .flatten()
    .unwrap_or(true);

    let username = if let Some(username) = fork_owner_username {
        username
    } else if automate_username_creation {
        if nu.username.is_some() && nu.username.unwrap().len() > 0 {
            return Err(Error::BadRequest(
                "username is not allowed when username creation is automated".to_string(),
            ));
        }
        get_instance_username_or_create_pending(&mut tx, &nu.email).await?
    } else {
        let username = nu
            .username
            .ok_or(Error::BadRequest("username is required".to_string()))?;

        if !VALID_USERNAME.is_match(&username) {
            return Err(windmill_common::error::Error::BadRequest(format!(
                "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
            )));
        }

        username
    };

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin, operator)
            VALUES ($1, $2, $3, $4, $5)",
        &w_id,
        nu.email,
        username,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2",
        &w_id,
        nu.email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        username,
        "all",
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.add_to_workspace",
        ActionKind::Create,
        &w_id,
        Some(&nu.email),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::User { email: nu.email.clone() },
        Some(format!("Added user '{}' to workspace", &nu.email)),
        true,
        None,
    )
    .await?;

    if !workspace_invite_emails_disabled(&db).await? {
        send_email_if_possible(
            &format!("Added to Windmill's workspace: {w_id}"),
            &format!(
                "You have been granted access to Windmill's workspace {w_id} by {}

If you do not have an account on {}, login with SSO or ask an admin to create an account for you.",
                authed.email,
                (**BASE_URL.load()).clone()
            ),
            &nu.email,
        );
    }

    webhook.send_instance_event(InstanceEvent::UserAddedWorkspace {
        workspace: w_id.clone(),
        email: nu.email.clone(),
    });

    Ok((
        StatusCode::CREATED,
        format!("user with email {} added", nu.email),
    ))
}

#[derive(Deserialize)]
pub struct NewServiceAccount {
    pub username: String,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default = "default_true")]
    pub operator: bool,
    #[serde(default)]
    pub add_to_deployers: bool,
}

fn default_true() -> bool {
    true
}

async fn create_service_account(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nu): Json<NewServiceAccount>,
) -> Result<(StatusCode, String)> {
    crate::workspaces_oss::create_service_account(authed, db, w_id, nu).await
}

async fn delete_invite(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE
        workspace_id = $1 AND email = $2 AND is_admin = $3 AND operator = $4",
        &w_id,
        nu.email,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("invite to email {} deleted", nu.email),
    ))
}

async fn exists_username(
    Extension(db): Extension<DB>,
    Json(vu): Json<ValidateUsername>,
) -> Result<String> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND workspace_id = $2)",
        vu.username,
        vu.id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);

    if exists {
        return Err(Error::BadRequest("username already taken".to_string()));
    }

    Ok("valid username".to_string())
}

async fn get_workspace_name(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let workspace = sqlx::query_scalar!("SELECT name FROM workspace WHERE id = $1", &w_id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(workspace)
}

async fn get_dependency_map(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<DependencyMap>> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;
    let dmap = sqlx::query_as!(
        DependencyMap,
        "
        SELECT workspace_id, importer_path, importer_kind::text, imported_path, importer_node_id
        FROM dependency_map WHERE workspace_id = $1",
        &w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(dmap))
}

#[axum::debug_handler]
async fn rebuild_dependency_map(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    if *CLOUD_HOSTED {
        return Err(Error::BadRequest("Disabled on Cloud".into()));
    }
    ScopedDependencyMap::rebuild_map(&w_id, &db).await
}

#[axum::debug_handler]
async fn get_dependents(
    Extension(db): Extension<DB>,
    Path((w_id, imported_path)): Path<(String, String)>,
    _authed: ApiAuthed,
) -> JsonResult<Vec<DependencyDependent>> {
    tracing::debug!(
        workspace_id = %w_id,
        imported_path = %imported_path,
        "API: Getting dependents for imported path"
    );

    let dependents = ScopedDependencyMap::get_dependents(&imported_path, &w_id, &db).await?;

    tracing::debug!(
        workspace_id = %w_id,
        imported_path = %imported_path,
        dependents_count = dependents.len(),
        "API: Found dependents: {:?}",
        dependents
    );

    Ok(Json(dependents))
}

async fn get_imports(
    Extension(db): Extension<DB>,
    Path((w_id, importer_path)): Path<(String, String)>,
    _authed: ApiAuthed,
) -> JsonResult<Vec<String>> {
    tracing::debug!(
        workspace_id = %w_id,
        importer_path = %importer_path,
        "API: Getting imports for importer path"
    );

    let imports = ScopedDependencyMap::get_imports(&importer_path, &w_id, &db).await?;

    tracing::debug!(
        workspace_id = %w_id,
        importer_path = %importer_path,
        imports_count = imports.len(),
        "API: Found imports: {:?}",
        imports
    );

    Ok(Json(imports))
}

#[derive(Serialize, Debug)]
struct DependentsAmount {
    imported_path: String,
    count: i64,
}

#[axum::debug_handler]
async fn get_dependents_amounts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(imported_paths): Json<Vec<String>>,
) -> JsonResult<Vec<DependentsAmount>> {
    tracing::debug!(
        workspace_id = %w_id,
        imported_paths = ?imported_paths,
        "API: Getting dependents amounts for imported paths"
    );

    let results = sqlx::query_as!(
        DependentsAmount,
        r#"
        SELECT
            imported_path,
            COUNT(DISTINCT importer_path) as "count!"
        FROM dependency_map
        WHERE workspace_id = $1 AND imported_path = ANY($2)
        GROUP BY imported_path
        "#,
        w_id,
        &imported_paths
    )
    .fetch_all(&db)
    .await?;

    tracing::debug!(
        workspace_id = %w_id,
        results_count = results.len(),
        "API: Found dependents amounts: {:?}",
        results
    );

    Ok(Json(results))
}

#[derive(Deserialize)]
struct ChangeWorkspaceName {
    new_name: String,
}

#[derive(Deserialize)]
struct ChangeWorkspaceColor {
    color: Option<String>,
}

async fn change_workspace_name(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
    Json(rw): Json<ChangeWorkspaceName>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace SET name = $1 WHERE id = $2",
        &rw.new_name,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspace.change_workspace_name",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;

    tx.commit().await?;

    // Trigger git sync for workspace name changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings { setting_type: "workspace_name".to_string() },
        Some(format!("Workspace name updated to {}", &rw.new_name)),
        false,
        None,
    )
    .await?;

    Ok(format!("updated workspace name to {}", &rw.new_name))
}

async fn change_workspace_color(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
    Json(rw): Json<ChangeWorkspaceColor>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace_settings SET color = $1 WHERE workspace_id = $2",
        rw.color,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "workspace_color".to_string() },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!(
        "updated workspace color to {}",
        rw.color.as_deref().unwrap_or("no color")
    ))
}

async fn get_usage(Extension(db): Extension<DB>, Path(w_id): Path<String>) -> Result<String> {
    // On cloud, a fork's executions meter against its billing root, so report the root's usage here too;
    // otherwise the free-execs indicator would show the fork's own (often 0) count while enforcement
    // applies the root's shared quota. Gated on `*CLOUD_HOSTED` (not just the `cloud` feature, which is
    // compiled into all EE builds): self-hosted doesn't meter usage this way. Off-fork it resolves to
    // `w_id` itself anyway.
    #[cfg(feature = "cloud")]
    let w_id = if *CLOUD_HOSTED {
        windmill_common::workspaces::get_billing_workspace_id(&db, &w_id).await?
    } else {
        w_id
    };
    let usage = sqlx::query_scalar!(
        "
    SELECT usage.usage FROM usage
    WHERE is_workspace = true
    AND month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)
    AND id = $1",
        w_id
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(0);
    Ok(usage.to_string())
}

#[derive(Deserialize)]
pub struct EditPublicAppRateLimitRequest {
    pub public_app_execution_limit_per_minute: Option<i32>,
}

async fn edit_public_app_rate_limit(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
    Json(req): Json<EditPublicAppRateLimitRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    sqlx::query!(
        "UPDATE workspace_settings SET public_app_execution_limit_per_minute = $1 WHERE workspace_id = $2",
        req.public_app_execution_limit_per_minute,
        &w_id
    )
    .execute(&db)
    .await?;

    // Cache is invalidated via DB trigger -> notify_event -> polling in main.rs

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "public_app_rate_limit".to_string() },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!(
        "Updated public app rate limit for workspace: {}",
        &w_id
    ))
}

// 5 minutes fallback TTL (in addition to event-based invalidation)
const PUBLIC_APP_RATE_LIMIT_CACHE_TTL_SECS: i64 = 300;

pub async fn get_public_app_rate_limit(db: &DB, w_id: &str) -> Result<Option<i32>> {
    use windmill_common::workspaces::PUBLIC_APP_RATE_LIMIT_CACHE;

    let now = Utc::now().timestamp();

    if let Some((rate_limit, cached_at)) = PUBLIC_APP_RATE_LIMIT_CACHE.get(w_id) {
        if now - cached_at < PUBLIC_APP_RATE_LIMIT_CACHE_TTL_SECS {
            return Ok(rate_limit);
        }
    }

    let result: Option<Option<i32>> = sqlx::query_scalar(
        "SELECT public_app_execution_limit_per_minute FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(w_id)
    .fetch_optional(db)
    .await?;
    let rate_limit = result.flatten();
    PUBLIC_APP_RATE_LIMIT_CACHE.insert(w_id.to_string(), (rate_limit, now));
    Ok(rate_limit)
}

#[derive(Deserialize, Serialize)]
struct ChangeOperatorSettings {
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
}

async fn update_operator_settings(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
    Json(settings): Json<ChangeOperatorSettings>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = db.begin().await?;

    let settings_json = serde_json::json!(settings);

    sqlx::query!(
        "UPDATE workspace_settings SET operator_settings = $1 WHERE workspace_id = $2",
        settings_json,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Trigger git sync for operator settings changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Settings {
            setting_type: "operator_settings".to_string(),
        },
        Some("Operator settings updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok("Operator settings updated successfully".to_string())
}

// Protection Rules API endpoints

#[derive(Deserialize)]
struct CreateProtectionRuleRequest {
    name: String,
    rules: Vec<ProtectionRuleKind>,
    bypass_groups: Vec<String>,
    bypass_users: Vec<String>,
}

#[derive(Deserialize)]
struct UpdateProtectionRuleRequest {
    rules: Vec<ProtectionRuleKind>,
    bypass_groups: Vec<String>,
    bypass_users: Vec<String>,
}

#[derive(Serialize)]
struct ProtectionRulesetResponse {
    pub workspace_id: String,
    pub name: String,
    pub rules: Vec<ProtectionRuleKind>,
    pub bypass_groups: Vec<String>,
    pub bypass_users: Vec<String>,
}

impl From<ProtectionRuleset> for ProtectionRulesetResponse {
    fn from(value: ProtectionRuleset) -> Self {
        let mut rules = vec![];

        for rule in ProtectionRuleKind::iter() {
            if value.rules.contains(rule.flag()) {
                rules.push(rule)
            }
        }

        ProtectionRulesetResponse {
            rules,
            workspace_id: value.workspace_id,
            name: value.name,
            bypass_groups: value.bypass_groups,
            bypass_users: value.bypass_users,
        }
    }
}

/// List all protection rules for a workspace
async fn list_protection_rules(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ProtectionRulesetResponse>> {
    let rules = (*windmill_common::workspaces::get_protection_rules(&w_id, &db).await?).clone();
    Ok(Json(
        rules
            .into_iter()
            .map(ProtectionRulesetResponse::from)
            .collect(),
    ))
}

/// Insert or replace a protection ruleset within an existing transaction. Unlike the
/// `create_protection_rule` handler (which rejects an existing name), this upserts, so it is safe to
/// call programmatically when designating a dev/prod pair. Callers MUST invalidate the
/// protection-rules cache (`invalidate_protection_rules_cache`) after the transaction commits.
async fn upsert_protection_rule(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    name: &str,
    rules: ProtectionRules,
    bypass_groups: &[String],
    bypass_users: &[String],
) -> Result<()> {
    sqlx::query!(
        r#"
            INSERT INTO workspace_protection_rule (workspace_id, name, rules, bypass_groups, bypass_users)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (workspace_id, name)
            DO UPDATE SET rules = EXCLUDED.rules,
                          bypass_groups = EXCLUDED.bypass_groups,
                          bypass_users = EXCLUDED.bypass_users
        "#,
        w_id,
        name,
        rules.bits(),
        bypass_groups,
        bypass_users,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Lock a prod workspace by applying the reserved dev-workspace lock rule with the selected
/// restrictions (block direct deployment and/or ad-hoc forking). Non-admins are then funneled
/// through the one dev workspace; admins bypass the rules (their existing escape hatch).
async fn lock_prod_workspace(
    tx: &mut Transaction<'_, Postgres>,
    prod_w_id: &str,
    block_deploy: bool,
    block_forking: bool,
) -> Result<()> {
    let mut rules = Vec::new();
    if block_deploy {
        rules.push(ProtectionRuleKind::DisableDirectDeployment);
    }
    if block_forking {
        rules.push(ProtectionRuleKind::DisableWorkspaceForking);
    }
    if rules.is_empty() {
        return Ok(());
    }
    upsert_protection_rule(
        tx,
        prod_w_id,
        DEV_WORKSPACE_LOCK_RULE_NAME,
        ProtectionRules::from(&rules),
        &[],
        &[],
    )
    .await
}

/// Error out if `parent_w_id` already has an active (non-archived) dev workspace. Mirrors the
/// partial unique index `workspace_canonical_dev_idx` with a friendly message.
async fn ensure_no_existing_dev_workspace(db: &DB, parent_w_id: &str) -> Result<()> {
    let existing = sqlx::query_scalar!(
        "SELECT id FROM workspace WHERE parent_workspace_id = $1 AND is_dev_workspace AND deleted = false",
        parent_w_id
    )
    .fetch_optional(db)
    .await?;
    if let Some(existing) = existing {
        return Err(Error::BadRequest(format!(
            "Workspace '{}' already has a dev workspace ('{}'). Detach it before creating another.",
            parent_w_id, existing
        )));
    }
    Ok(())
}

/// A dev workspace pairs with a root prod workspace; nesting dev workspaces (a dev of a dev) isn't
/// supported and would muddle the prod<->dev relationship.
async fn ensure_dev_parent_is_root(db: &DB, parent_w_id: &str) -> Result<()> {
    let parent_is_fork = sqlx::query_scalar!(
        r#"SELECT (parent_workspace_id IS NOT NULL) AS "is_fork!" FROM workspace WHERE id = $1"#,
        parent_w_id
    )
    .fetch_optional(db)
    .await?
    .unwrap_or(false);
    if parent_is_fork {
        return Err(Error::BadRequest(format!(
            "Cannot create a dev workspace of '{}' because it is itself a fork or dev workspace.",
            parent_w_id
        )));
    }
    Ok(())
}

/// `dev_workspace_lock` is owned by the dev-workspace feature (attach/detach/archive/delete create and
/// remove it by name). Reserve it from the public protection-rule API so a user-managed rule can't
/// collide: otherwise the feature's name-based cleanup would clobber the user's rule, or a manual edit
/// could weaken the feature's lock.
fn reject_reserved_rule_name(name: &str) -> Result<()> {
    if name == DEV_WORKSPACE_LOCK_RULE_NAME {
        return Err(Error::BadRequest(format!(
            "'{}' is a reserved protection-rule name managed by the dev workspace feature",
            DEV_WORKSPACE_LOCK_RULE_NAME
        )));
    }
    Ok(())
}

/// Create a new protection rule
async fn create_protection_rule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<CreateProtectionRuleRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    reject_reserved_rule_name(&req.name)?;

    let mut tx = db.begin().await?;

    // Check if rule with this name already exists
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace_protection_rule WHERE workspace_id = $1 AND name = $2)",
        &w_id,
        &req.name
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "Protection rule with name '{}' already exists",
            req.name
        )));
    }

    // Insert the new rule
    sqlx::query!(
        r#"
            INSERT INTO workspace_protection_rule (workspace_id, name, rules, bypass_groups, bypass_users)
            VALUES ($1, $2, $3, $4, $5)
        "#,
        &w_id,
        &req.name,
        ProtectionRules::from(&req.rules).bits(),
        &req.bypass_groups,
        &req.bypass_users,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.create_protection_rule",
        ActionKind::Create,
        &w_id,
        Some(&req.name),
        Some([("name", &req.name[..])].into()),
    )
    .await?;

    tx.commit().await?;

    // Invalidate cache
    windmill_common::workspaces::invalidate_protection_rules_cache(&w_id);

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: format!("protection_rule_{}", req.name) },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!("Created protection rule '{}'", req.name))
}

/// Update an existing protection rule
async fn update_protection_rule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, rule_name)): Path<(String, String)>,
    Json(req): Json<UpdateProtectionRuleRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    reject_reserved_rule_name(&rule_name)?;

    let mut tx = db.begin().await?;

    // Check if rule exists
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace_protection_rule WHERE workspace_id = $1 AND name = $2)",
        &w_id,
        &rule_name
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if !exists {
        return Err(Error::NotFound(format!(
            "Protection rule '{}' not found",
            rule_name
        )));
    }

    // Update the rule
    sqlx::query!(
        r#"
            UPDATE workspace_protection_rule
            SET rules = $1, bypass_groups = $2, bypass_users = $3
            WHERE workspace_id = $4 AND name = $5
        "#,
        ProtectionRules::from(&req.rules).bits(),
        &req.bypass_groups,
        &req.bypass_users,
        &w_id,
        &rule_name
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.update_protection_rule",
        ActionKind::Update,
        &w_id,
        Some(&rule_name),
        Some([("name", &rule_name[..])].into()),
    )
    .await?;

    tx.commit().await?;

    // Invalidate cache
    windmill_common::workspaces::invalidate_protection_rules_cache(&w_id);

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: format!("protection_rule_{}", rule_name) },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!("Updated protection rule '{}'", rule_name))
}

/// Delete a protection rule
async fn delete_protection_rule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, rule_name)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    reject_reserved_rule_name(&rule_name)?;

    let mut tx = db.begin().await?;

    // Delete the rule
    let result = sqlx::query!(
        "DELETE FROM workspace_protection_rule WHERE workspace_id = $1 AND name = $2",
        &w_id,
        &rule_name
    )
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!(
            "Protection rule '{}' not found",
            rule_name
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.delete_protection_rule",
        ActionKind::Delete,
        &w_id,
        Some(&rule_name),
        Some([("name", &rule_name[..])].into()),
    )
    .await?;

    tx.commit().await?;

    // Invalidate cache
    windmill_common::workspaces::invalidate_protection_rules_cache(&w_id);

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: format!("protection_rule_{}", rule_name) },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!("Deleted protection rule '{}'", rule_name))
}

#[derive(Serialize)]
pub struct WorkspaceComparison {
    pub all_ahead_items_visible: bool,
    pub all_behind_items_visible: bool,
    pub skipped_comparison: bool,
    pub diffs: Vec<WorkspaceDiffRow>,
    pub summary: CompareSummary,
    /// Items that exist in the diff but were dropped from `diffs` because they
    /// are not visible to the caller (excluded from the partial deploy). Split
    /// by direction: `hidden_ahead` lives in the fork, `hidden_behind` in the
    /// parent. `by_kind`/`total` are always populated (aggregate, no names);
    /// `items` (kind+path) is only filled for a caller who is admin of that side
    /// — never leak the paths of items the ACL is hiding from a regular user.
    pub hidden_ahead: HiddenItemsSummary,
    pub hidden_behind: HiddenItemsSummary,
}

#[derive(Serialize, Default)]
pub struct HiddenItemsSummary {
    pub total: usize,
    pub by_kind: std::collections::BTreeMap<String, usize>,
    pub items: Vec<HiddenItem>,
}

#[derive(Serialize)]
pub struct HiddenItem {
    pub kind: String,
    pub path: String,
}

#[derive(Serialize, Default)]
pub struct CompareSummary {
    pub total_diffs: usize,
    pub total_ahead: usize,
    pub total_behind: usize,
    pub scripts_changed: usize,
    pub flows_changed: usize,
    pub apps_changed: usize,
    pub resources_changed: usize,
    pub variables_changed: usize,
    pub resource_types_changed: usize,
    pub folders_changed: usize,
    pub schedules_changed: usize,
    pub triggers_changed: usize,
    pub datatable_migrations_changed: usize,
    pub conflicts: usize, // Items that are both ahead and behind
}

async fn reset_workspace_diffs(
    authed: ApiAuthed,
    Path((w_id, target_workspace_id)): Path<(String, String)>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<()> {
    // Needed to compute the has_changes: Option<bool>. Otherwise it will be None, and the query will not hit the items
    let _ = compare_workspaces(
        authed,
        Path((w_id.clone(), target_workspace_id.clone())),
        Extension(db.clone()),
        Extension(user_db),
    )
    .await?;

    sqlx::query!(
        "DELETE FROM workspace_diff WHERE has_changes = false AND (
            (source_workspace_id = $1 AND fork_workspace_id = $2)
            OR (source_workspace_id = $2 AND fork_workspace_id =$1)
        )",
        target_workspace_id,
        w_id,
    )
    .execute(&db)
    .await?;

    Ok(Json(()))
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct WorkspaceDiffRow {
    kind: String,
    path: String,
    ahead: i32,
    behind: i32,
    has_changes: Option<bool>,
    exists_in_source: Option<bool>,
    exists_in_fork: Option<bool>,
}

async fn compare_workspaces(
    authed: ApiAuthed,
    Path((source_workspace_id, fork_workspace_id)): Path<(String, String)>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<WorkspaceComparison> {
    // require_admin(authed.is_admin, &authed.username)?;

    let skipped_comparison: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM skip_workspace_diff_tally
            WHERE workspace_id = $1
        )",
    )
    .bind(&fork_workspace_id)
    .fetch_one(&db)
    .await?;

    if skipped_comparison {
        return Ok(Json(WorkspaceComparison {
            all_ahead_items_visible: true,
            all_behind_items_visible: true,
            skipped_comparison,
            diffs: vec![],
            summary: Default::default(),
            hidden_ahead: Default::default(),
            hidden_behind: Default::default(),
        }));
    }

    // Honor ws_specific at read time: a workspace-specific resource/variable keeps its own value per
    // environment, so it must never appear in the normal diff (the per-item compare suppresses it,
    // but a cached `has_changes=true` row is trusted without re-running that compare, so filter those
    // here too). Seeding the initial copy onto a side that lacks it is a separate explicit action
    // (the "Create in <other>" button on the Workspace-specific list), not part of the diff. The row
    // is left intact, so unpinning resurfaces it without a re-tally.
    let diff_items = sqlx::query_as!(
        WorkspaceDiffRow,
        "SELECT path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork FROM workspace_diff
        WHERE source_workspace_id = $1 AND fork_workspace_id = $2
        AND NOT EXISTS (
            SELECT 1 FROM ws_specific ws
            WHERE ws.path = workspace_diff.path
              AND ws.item_kind = workspace_diff.kind
              AND ws.workspace_id IN (workspace_diff.source_workspace_id, workspace_diff.fork_workspace_id)
        )",
        source_workspace_id,
        fork_workspace_id,
    )
    .fetch_all(&db)
    .await?;

    // A cached `has_changes = true` row is trusted without re-running the
    // per-kind comparison, but that verdict can go stale: an item archived or
    // deleted after it was cached still carries `exists_in_*=true` here. The
    // common offender is the old path after a rename — for lock-gen languages
    // (Python/TS/…) the `has_changes=NULL` reset is deferred to the dependency
    // job, so until that runs (or if it fails) the archived old path looks like
    // a live ahead change. Treat archived as non-existent: re-validate such rows
    // against the live tables and, if the item no longer exists on a side the
    // cache claims, re-evaluate it below so it gets corrected or removed.
    //
    // Only scripts/flows can hit this (they have `archived`; other kinds reset
    // synchronously on delete). Probe both sides in one batched query per kind
    // (mirroring `query_visible_items`) rather than per row, to keep the hot
    // compare path off an O(number of cached diffs) sequence of round trips.
    let (live_source, live_fork) = {
        let mut cached_source: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut cached_fork: HashMap<&str, Vec<&str>> = HashMap::new();
        for item in &diff_items {
            if item.has_changes == Some(true) && (item.kind == "script" || item.kind == "flow") {
                if item.exists_in_source.unwrap_or(false) {
                    cached_source
                        .entry(item.kind.as_str())
                        .or_default()
                        .push(item.path.as_str());
                }
                if item.exists_in_fork.unwrap_or(false) {
                    cached_fork
                        .entry(item.kind.as_str())
                        .or_default()
                        .push(item.path.as_str());
                }
            }
        }
        (
            existing_runnables(&db, &source_workspace_id, &cached_source).await?,
            existing_runnables(&db, &fork_workspace_id, &cached_fork).await?,
        )
    };

    let mut confirmed_diffs = vec![];
    for item in diff_items {
        if let Some(has_changes) = item.has_changes {
            if !has_changes {
                // Defensive: rows that compared equal are normally deleted, so
                // this is rarely hit. Not a diff — skip.
                continue;
            }
            // Stale only applies to script/flow (others aren't in the probed
            // sets); a row whose claimed-existing side has no live version is
            // stale and falls through to re-evaluation.
            let key = (item.kind.clone(), item.path.clone());
            let fork_stale = item.exists_in_fork.unwrap_or(false) && !live_fork.contains(&key);
            let source_stale =
                item.exists_in_source.unwrap_or(false) && !live_source.contains(&key);
            let probed = item.kind == "script" || item.kind == "flow";
            if !(probed && (fork_stale || source_stale)) {
                // Cache is still valid (or not a probed kind) — trust it.
                confirmed_diffs.push(item);
                continue;
            }
            // Stale cache: fall through to re-evaluate (and correct/delete) below.
        }

        let item_comparison = match item.kind.as_str() {
            "script" => Some(
                compare_two_scripts(&db, &source_workspace_id, &fork_workspace_id, &item.path)
                    .await?,
            ),
            "flow" => Some(
                compare_two_flows(&db, &source_workspace_id, &fork_workspace_id, &item.path)
                    .await?,
            ),
            "app" | "raw_app" => Some(
                compare_two_apps(&db, &source_workspace_id, &fork_workspace_id, &item.path).await?,
            ),
            "resource" => Some(
                compare_two_resources(&db, &source_workspace_id, &fork_workspace_id, &item.path)
                    .await?,
            ),
            "variable" => Some(
                compare_two_variables(&db, &source_workspace_id, &fork_workspace_id, &item.path)
                    .await?,
            ),
            "resource_type" => Some(
                compare_two_resource_types(
                    &db,
                    &source_workspace_id,
                    &fork_workspace_id,
                    &item.path,
                )
                .await?,
            ),
            "folder" => Some(
                compare_two_folders(&db, &source_workspace_id, &fork_workspace_id, &item.path)
                    .await?,
            ),
            "datatable_migration" => Some(
                crate::datatable_migrations::compare_two_datatable_migration(
                    &db,
                    &source_workspace_id,
                    &fork_workspace_id,
                    &item.path,
                )
                .await?,
            ),
            // Triggers and schedules are diffed against a hardcoded ignore list
            // (mode/enabled/server_id/last_server_ping/edited_at/by/error/extra_perms/permissioned_as/email)
            // so that fork-clones — which differ from the parent only in the runtime
            // mode/enabled flag — don't show as diffs.
            k if TRIGGER_OR_SCHEDULE_TABLES.contains(&k) => Some(
                compare_two_trigger_or_schedule(
                    &db,
                    item.kind.as_str(),
                    &source_workspace_id,
                    &fork_workspace_id,
                    &item.path,
                )
                .await?,
            ),
            k => {
                tracing::error!("Received unrecognized item kind `{k}` with path: `{}` while computing diff of {fork_workspace_id} and {source_workspace_id} workspaces. Skipping this item", item.path);
                None
                // Some(ItemComparison {
                //     has_changes: true,
                //     exists_in_source: true,
                //     exists_in_fork: true,
                // })
            }
        };

        if let Some(item_comparison) = item_comparison {
            if item_comparison.has_changes {
                sqlx::query!(
                    "UPDATE workspace_diff SET has_changes = true, exists_in_source = $5, exists_in_fork = $6
                    WHERE path = $3 AND kind = $4 AND (
                        (source_workspace_id = $1 AND fork_workspace_id = $2)
                        OR (source_workspace_id = $2 AND fork_workspace_id =$1)
                    )",
                    source_workspace_id,
                    fork_workspace_id,
                    item.path,
                    item.kind,
                    item_comparison.exists_in_source,
                    item_comparison.exists_in_fork,
                )
                .execute(&db)
                .await?;
                confirmed_diffs.push(WorkspaceDiffRow {
                    has_changes: Some(item_comparison.has_changes),
                    exists_in_source: Some(item_comparison.exists_in_source),
                    exists_in_fork: Some(item_comparison.exists_in_fork),
                    ..item
                });
            } else {
                sqlx::query!(
                    "DELETE FROM workspace_diff WHERE path = $3 AND kind = $4 AND (
                        (source_workspace_id = $1 AND fork_workspace_id = $2)
                        OR (source_workspace_id = $2 AND fork_workspace_id =$1)
                    )",
                    source_workspace_id,
                    fork_workspace_id,
                    item.path,
                    item.kind,
                )
                .execute(&db)
                .await?;
            }
        }
    }

    // The authed in `authed` is loaded for the source workspace (the one in the
    // URL path). Its `folders`/`groups`/`is_admin` reflect membership in the
    // source workspace only. Using it as the RLS context when querying the
    // fork's tables would hide items the user can only see via fork-specific
    // permissions (e.g. a folder the user owns in the fork but that does not
    // exist in the source), causing the spurious
    // "this fork has changes not visible to your user" warning. Build a
    // matching authed for the fork so each side's visibility check uses the
    // right RLS context.
    let fork_authed = load_workspace_authed(&db, &authed, &fork_workspace_id).await?;
    let visible_diffs = filter_visible_diffs(
        &confirmed_diffs,
        &source_workspace_id,
        &fork_workspace_id,
        &authed,
        &fork_authed,
        &user_db,
    )
    .await?;

    let summary = CompareSummary {
        total_diffs: visible_diffs.len(),
        total_ahead: visible_diffs
            .iter()
            .map(|s| s.ahead)
            .fold(0, |acc, s| acc + s.try_into().unwrap_or(0)),
        total_behind: visible_diffs
            .iter()
            .map(|s| s.behind)
            .fold(0, |acc, s| acc + s.try_into().unwrap_or(0)),
        scripts_changed: visible_diffs.iter().filter(|s| s.kind == "script").count(),
        flows_changed: visible_diffs.iter().filter(|s| s.kind == "flow").count(),
        apps_changed: visible_diffs
            .iter()
            .filter(|s| s.kind == "app" || s.kind == "raw_app")
            .count(),
        resources_changed: visible_diffs
            .iter()
            .filter(|s| s.kind == "resource")
            .count(),
        variables_changed: visible_diffs
            .iter()
            .filter(|s| s.kind == "variable")
            .count(),
        resource_types_changed: visible_diffs
            .iter()
            .filter(|s| s.kind == "resource_type")
            .count(),
        folders_changed: visible_diffs.iter().filter(|s| s.kind == "folder").count(),
        schedules_changed: visible_diffs
            .iter()
            .filter(|s| s.kind == "schedule")
            .count(),
        triggers_changed: visible_diffs
            .iter()
            .filter(|s| s.kind.ends_with("_trigger"))
            .count(),
        datatable_migrations_changed: visible_diffs
            .iter()
            .filter(|s| s.kind == "datatable_migration")
            .count(),
        conflicts: visible_diffs
            .iter()
            .filter(|s| s.ahead > 0 && s.behind > 0)
            .count(),
    };

    let all_ahead_items_visible = summary.total_ahead
        == confirmed_diffs
            .iter()
            .map(|s| s.ahead)
            .fold(0, |acc, s| acc + s.try_into().unwrap_or(0));
    let all_behind_items_visible = summary.total_behind
        == confirmed_diffs
            .iter()
            .map(|s| s.behind)
            .fold(0, |acc, s| acc + s.try_into().unwrap_or(0));

    // Blast-radius guard for the "changes not visible to your user" warning
    // (which hides the deploy button). The flag is a pure visibility guarantee —
    // the deploy re-authorizes each item against the target workspace's
    // create/update endpoints — so it is safe to force true for a caller who sees
    // every item on BOTH sides, for whom any diff the filter dropped is provably a
    // stale/phantom row, never a permission gap.
    //
    // It must be BOTH sides, not per-side: `filter_visible_diffs` keeps a modified
    // or conflict row (one that exists in the source AND the fork) only when the
    // caller can see it on both sides, so an ahead/conflict diff can be dropped for
    // a source-side visibility gap even when the caller is a fork admin. Gating the
    // ahead flag on fork-admin alone would then wrongly report "all ahead visible"
    // and let the UI deploy from an incomplete comparison. So require admin of the
    // source AND the fork (superadmin satisfies both), which guarantees full
    // visibility of every item on every side. `fork_authed.is_admin` already folds
    // in superadmin; `authed.is_admin` (source side) does not, so OR it in.
    let is_super_admin = windmill_common::auth::is_super_admin_email(&db, &authed.email).await?;
    let sees_all_items = is_super_admin || (authed.is_admin && fork_authed.is_admin);
    let all_ahead_items_visible = all_ahead_items_visible || sees_all_items;
    let all_behind_items_visible = all_behind_items_visible || sees_all_items;

    // Items dropped by the visibility filter (in confirmed_diffs but not in the
    // returned visible_diffs). Surface what the partial deploy excludes: aggregate
    // counts by kind for everyone, but kind+path only to a caller who `sees_all_items`
    // (superadmin, or admin of the source AND the fork). For them a dropped item is
    // provably a phantom/stale row, not an ACL-hidden secret, so no path leaks —
    // fork-admin alone is not enough (a fork-deleted ahead item lives only in the
    // parent, whose path a non-parent-admin must not see).
    let visible_keys: HashSet<(&str, &str)> = visible_diffs
        .iter()
        .map(|d| (d.kind.as_str(), d.path.as_str()))
        .collect();
    let mut hidden_ahead = HiddenItemsSummary::default();
    let mut hidden_behind = HiddenItemsSummary::default();
    for d in &confirmed_diffs {
        if visible_keys.contains(&(d.kind.as_str(), d.path.as_str())) {
            continue;
        }
        if d.ahead > 0 {
            hidden_ahead.total += 1;
            *hidden_ahead.by_kind.entry(d.kind.clone()).or_default() += 1;
            if sees_all_items {
                hidden_ahead
                    .items
                    .push(HiddenItem { kind: d.kind.clone(), path: d.path.clone() });
            }
        }
        if d.behind > 0 {
            hidden_behind.total += 1;
            *hidden_behind.by_kind.entry(d.kind.clone()).or_default() += 1;
            if sees_all_items {
                hidden_behind
                    .items
                    .push(HiddenItem { kind: d.kind.clone(), path: d.path.clone() });
            }
        }
    }

    return Ok(Json(WorkspaceComparison {
        all_ahead_items_visible,
        all_behind_items_visible,
        skipped_comparison: false,
        diffs: visible_diffs,
        summary,
        hidden_ahead,
        hidden_behind,
    }));
}

/// Build an `ApiAuthed` for the same user but scoped to a different workspace.
///
/// Reloads `is_admin`, `groups`, and `folders` from the target workspace's
/// `usr` / `group_` / `folder` tables (keyed by the caller's email) so the
/// returned authed can be used as the RLS context for queries against that
/// workspace. `is_admin` is OR'd with the user's superadmin status so cross-
/// workspace superadmins keep their RLS bypass.
///
/// If the user is not a member of `workspace_id`, returns an authed with no
/// folders/groups/operator/admin (except for superadmins, who stay admin) —
/// i.e. they will only see what RLS explicitly allows for unknown users.
async fn load_workspace_authed(
    db: &DB,
    base_authed: &ApiAuthed,
    workspace_id: &str,
) -> Result<ApiAuthed> {
    let mut conn = db
        .acquire()
        .await
        .map_err(|e| Error::internal_err(e.to_string()))?;

    let is_super_admin =
        windmill_common::auth::is_super_admin_email(db, &base_authed.email).await?;

    let user_row = sqlx::query!(
        "SELECT username, is_admin, operator FROM usr
         WHERE workspace_id = $1 AND email = $2 AND disabled = false",
        workspace_id,
        &base_authed.email
    )
    .fetch_optional(&mut *conn)
    .await?;

    let Some(user_row) = user_row else {
        return Ok(ApiAuthed {
            email: base_authed.email.clone(),
            username: base_authed.username.clone(),
            is_admin: is_super_admin,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes: base_authed.scopes.clone(),
            username_override: base_authed.username_override.clone(),
            token_prefix: base_authed.token_prefix.clone(),
            read_only: base_authed.read_only,
        });
    };

    let groups = windmill_common::auth::get_groups_for_user(
        workspace_id,
        &user_row.username,
        &base_authed.email,
        &mut *conn,
    )
    .await?;
    let folders = windmill_common::auth::get_folders_for_user(
        workspace_id,
        &user_row.username,
        &groups,
        &mut *conn,
    )
    .await?;

    Ok(ApiAuthed {
        email: base_authed.email.clone(),
        username: user_row.username,
        is_admin: is_super_admin || user_row.is_admin,
        is_operator: user_row.operator,
        groups,
        folders,
        scopes: base_authed.scopes.clone(),
        username_override: base_authed.username_override.clone(),
        token_prefix: base_authed.token_prefix.clone(),
        read_only: base_authed.read_only,
    })
}

async fn filter_visible_diffs(
    confirmed_diffs: &[WorkspaceDiffRow],
    source_workspace_id: &str,
    fork_workspace_id: &str,
    source_authed: &ApiAuthed,
    fork_authed: &ApiAuthed,
    user_db: &UserDB,
) -> Result<Vec<WorkspaceDiffRow>> {
    // Step 1: Group paths by (workspace, kind)
    let mut source_items: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut fork_items: HashMap<&str, Vec<&str>> = HashMap::new();

    for diff in confirmed_diffs {
        if diff.exists_in_source.unwrap_or(false) {
            source_items.entry(&diff.kind).or_default().push(&diff.path);
        }
        if diff.exists_in_fork.unwrap_or(false) {
            fork_items.entry(&diff.kind).or_default().push(&diff.path);
        }
    }

    // Step 2: Batch query for each (workspace, kind) combination, each in its
    // own transaction so RLS uses the right authed for each side. The fork's
    // authed picks up fork-only folders/groups; without this split the fork
    // queries would run with the source workspace's permissions and miss any
    // item the user can only reach through fork-specific permissions.
    let source_visible = {
        let mut tx = user_db.clone().begin(source_authed).await?;
        let visible = query_visible_items(&mut tx, source_workspace_id, &source_items).await?;
        tx.commit().await?;
        visible
    };
    let fork_visible = {
        let mut tx = user_db.clone().begin(fork_authed).await?;
        let visible = query_visible_items(&mut tx, fork_workspace_id, &fork_items).await?;
        tx.commit().await?;
        visible
    };

    // Step 3: Filter diffs based on visibility
    let visible_diffs: Vec<WorkspaceDiffRow> = confirmed_diffs
        .iter()
        .filter(|diff| {
            let v = (diff.kind.to_string(), diff.path.to_string());
            let source_ok = !diff.exists_in_source.unwrap_or(false) || source_visible.contains(&v);
            let fork_ok = !diff.exists_in_fork.unwrap_or(false) || fork_visible.contains(&v);
            source_ok && fork_ok
        })
        .cloned()
        .collect();

    Ok(visible_diffs)
}

async fn query_visible_items<'c>(
    tx: &mut Transaction<'c, Postgres>,
    workspace_id: &str,
    items_by_kind: &HashMap<&str, Vec<&str>>,
) -> Result<HashSet<(String, String)>> {
    let mut visible = HashSet::new();

    for (kind, paths) in items_by_kind {
        let paths_vec: Vec<String> = paths.iter().map(|s| s.to_string()).collect();

        let results = match *kind {
            "script" => {
                sqlx::query_scalar!(
                    "SELECT path FROM script
                     WHERE workspace_id = $1 AND path = ANY($2) AND archived = false",
                    workspace_id,
                    &paths_vec
                )
                .fetch_all(&mut **tx)
                .await?
            }
            "flow" => {
                sqlx::query_scalar!(
                    "SELECT path FROM flow
                     WHERE workspace_id = $1 AND path = ANY($2) AND archived = false",
                    workspace_id,
                    &paths_vec
                )
                .fetch_all(&mut **tx)
                .await?
            }
            "app" | "raw_app" => {
                sqlx::query_scalar!(
                    "SELECT path FROM app
                     WHERE workspace_id = $1 AND path = ANY($2)",
                    workspace_id,
                    &paths_vec
                )
                .fetch_all(&mut **tx)
                .await?
            }
            "resource" => {
                sqlx::query_scalar!(
                    "SELECT path FROM resource
                     WHERE workspace_id = $1 AND path = ANY($2)",
                    workspace_id,
                    &paths_vec
                )
                .fetch_all(&mut **tx)
                .await?
            }
            "variable" => {
                sqlx::query_scalar!(
                    "SELECT path FROM variable
                     WHERE workspace_id = $1 AND path = ANY($2)",
                    workspace_id,
                    &paths_vec
                )
                .fetch_all(&mut **tx)
                .await?
            }
            "folder" => {
                let a: Vec<String> = paths_vec
                    .iter()
                    .map(|p| p.strip_prefix("f/").unwrap_or(p.as_str()).to_string())
                    .collect();
                sqlx::query_scalar!(
                    "SELECT name FROM folder
                     WHERE workspace_id = $1 AND name = ANY($2)",
                    workspace_id,
                    &a,
                )
                .fetch_all(&mut **tx)
                .await?
                .into_iter()
                .map(|p| format!("f/{p}"))
                .collect()
            }
            "resource_type" => {
                sqlx::query_scalar!(
                    "SELECT name FROM resource_type
                     WHERE workspace_id = $1 AND name = ANY($2)",
                    workspace_id,
                    &paths_vec
                )
                .fetch_all(&mut **tx)
                .await?
            }
            "datatable_migration" => {
                // Match by (datatable, timestamp), not the full path: a migration
                // keeps its identity across a rename, so the candidate path's
                // `name` segment can differ from the stored one. Parse each
                // `<datatable>/<timestamp>_<name>` candidate, probe existence by
                // (datatable, timestamp), and return the *original* candidate path
                // so the visibility set stays keyed by the diff's path.
                let parsed: Vec<(String, i64, String)> = paths_vec
                    .iter()
                    .filter_map(|p| {
                        let (dt, rest) = p.split_once('/')?;
                        let ts = rest.split_once('_')?.0.parse::<i64>().ok()?;
                        Some((dt.to_string(), ts, p.clone()))
                    })
                    .collect();
                if parsed.is_empty() {
                    vec![]
                } else {
                    let dts: Vec<String> = parsed.iter().map(|(d, _, _)| d.clone()).collect();
                    let tss: Vec<i64> = parsed.iter().map(|(_, t, _)| *t).collect();
                    let existing: HashSet<(String, i64)> = sqlx::query!(
                        "SELECT datatable, timestamp FROM datatable_migrations \
                         WHERE workspace_id = $1 AND datatable = ANY($2) AND timestamp = ANY($3)",
                        workspace_id,
                        &dts,
                        &tss,
                    )
                    .fetch_all(&mut **tx)
                    .await?
                    .into_iter()
                    .map(|r| (r.datatable, r.timestamp))
                    .collect();
                    parsed
                        .into_iter()
                        .filter(|(d, t, _)| existing.contains(&(d.clone(), *t)))
                        .map(|(_, _, p)| p)
                        .collect()
                }
            }
            k if TRIGGER_OR_SCHEDULE_TABLES.contains(&k) => {
                // SAFETY: `kind` comes from a hardcoded allowlist
                // TRIGGER_OR_SCHEDULE_TABLES, not user input.
                let sql =
                    format!("SELECT path FROM {kind} WHERE workspace_id = $1 AND path = ANY($2)");
                sqlx::query_scalar(&sql)
                    .bind(workspace_id)
                    .bind(&paths_vec)
                    .fetch_all(&mut **tx)
                    .await?
            }
            _ => vec![], // Unknown kind
        };

        for path in results {
            visible.insert((kind.to_string(), path));
        }
    }

    Ok(visible)
}

/// Batched existence probe used to detect stale `workspace_diff` cache rows.
///
/// Given candidate paths grouped by kind, returns the set of `(kind, path)`
/// that currently have a *deployable* (non-archived) version in the workspace,
/// mirroring the existence semantics of `compare_two_scripts` /
/// `compare_two_flows`. Only scripts and flows are probed — they're the only
/// kinds with `archived`, and the only ones whose diff-row reset can lag behind
/// the actual change (deferred dependency job for lock-gen languages); other
/// kinds reset synchronously on delete, so their cache is trusted (and they're
/// never passed in). One query per kind keeps the compare path off a per-row
/// sequence of round trips. Runs on `&db` (no RLS) — this is a pure existence
/// check; authorization stays in `filter_visible_diffs` / `query_visible_items`.
async fn existing_runnables(
    db: &DB,
    workspace_id: &str,
    items_by_kind: &HashMap<&str, Vec<&str>>,
) -> Result<HashSet<(String, String)>> {
    let mut existing = HashSet::new();
    for (kind, paths) in items_by_kind {
        let paths_vec: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
        let found: Vec<String> = match *kind {
            "script" => sqlx::query_scalar!(
                "SELECT DISTINCT path FROM script WHERE workspace_id = $1 AND path = ANY($2) AND archived = false",
                workspace_id,
                &paths_vec
            )
            .fetch_all(db)
            .await?,
            "flow" => sqlx::query_scalar!(
                "SELECT path FROM flow WHERE workspace_id = $1 AND path = ANY($2) AND archived = false",
                workspace_id,
                &paths_vec
            )
            .fetch_all(db)
            .await?,
            _ => vec![],
        };
        for path in found {
            existing.insert((kind.to_string(), path));
        }
    }
    Ok(existing)
}

#[derive(Debug)]
pub(crate) struct ItemComparison {
    pub(crate) has_changes: bool,
    pub(crate) exists_in_source: bool,
    pub(crate) exists_in_fork: bool,
}

async fn compare_two_scripts(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    // Get latest script from each workspace
    let source_script = sqlx::query!(
        "SELECT hash, created_at, content, summary, description, lock, schema
         FROM script
         WHERE workspace_id = $1 AND path = $2 AND archived = false
         ORDER BY created_at DESC
         LIMIT 1",
        source_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let target_script = sqlx::query!(
        "SELECT hash, created_at, content, summary, description, lock, schema
         FROM script
         WHERE workspace_id = $1 AND path = $2 AND archived = false
         ORDER BY created_at DESC
         LIMIT 1",
        fork_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let mut has_changes = false;

    // Check metadata differences
    if let (Some(source), Some(target)) = (&source_script, &target_script) {
        if source.content != target.content
            || source.summary != target.summary
            || source.description != target.description
            || source.lock != target.lock
            || source.schema != target.schema
        {
            has_changes = true;
        }
    } else if source_script.is_some() || target_script.is_some() {
        // The script exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_script.is_some(),
        exists_in_fork: target_script.is_some(),
    });
}

async fn compare_two_flows(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    // Get latest flow from each workspace
    let source_flow = sqlx::query!(
        "SELECT value, summary, description, schema
         FROM flow
         WHERE workspace_id = $1 AND path = $2 AND archived = false",
        source_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let target_flow = sqlx::query!(
        "SELECT value, summary, description, schema
         FROM flow
         WHERE workspace_id = $1 AND path = $2 AND archived = false",
        fork_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let mut has_changes = false;

    // Check metadata differences
    if let (Some(source), Some(target)) = (&source_flow, &target_flow) {
        if source.value != target.value
            || source.summary != target.summary
            || source.description != target.description
            || source.schema != target.schema
        {
            has_changes = true;
        }
    } else if source_flow.is_some() || target_flow.is_some() {
        // The flow exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_flow.is_some(),
        exists_in_fork: target_flow.is_some(),
    });
}

async fn compare_two_apps(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    // Get app with its latest version data from source workspace
    let source_app = sqlx::query!(
        "SELECT app.summary, app.policy, app_version.value
         FROM app
         JOIN app_version
         ON app_version.id = app.versions[array_upper(app.versions, 1)]
         WHERE app.workspace_id = $1 AND app.path = $2",
        source_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let target_app = sqlx::query!(
        "SELECT app.summary, app.policy, app_version.value
         FROM app
         JOIN app_version
         ON app_version.id = app.versions[array_upper(app.versions, 1)]
         WHERE app.workspace_id = $1 AND app.path = $2",
        fork_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let mut has_changes = false;

    // Check metadata and content differences
    if let (Some(source), Some(target)) = (&source_app, &target_app) {
        if source.summary != target.summary
            || source.policy != target.policy
            || source.value != target.value
        {
            has_changes = true;
        }
    } else if source_app.is_some() || target_app.is_some() {
        // The app exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_app.is_some(),
        exists_in_fork: target_app.is_some(),
    });
}

async fn compare_two_resources(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    // Get resource from each workspace
    let source_resource = sqlx::query!(
        "SELECT value, description, resource_type
         FROM resource
         WHERE workspace_id = $1 AND path = $2",
        source_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let target_resource = sqlx::query!(
        "SELECT value, description, resource_type
         FROM resource
         WHERE workspace_id = $1 AND path = $2",
        fork_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let source_ws_specific = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2)",
        source_workspace_id,
        path
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    let target_ws_specific = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2)",
        fork_workspace_id,
        path
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    // A workspace-specific resource keeps its own value per environment, so it never appears in the
    // diff (in either direction). Seeding the initial copy onto a side that lacks it is a separate
    // explicit action ("Create in <other>"), not a diff entry.
    if source_ws_specific || target_ws_specific {
        return Ok(ItemComparison {
            has_changes: false,
            exists_in_source: source_resource.is_some(),
            exists_in_fork: target_resource.is_some(),
        });
    }

    let mut has_changes = false;

    // Check metadata differences
    if let (Some(source), Some(target)) = (&source_resource, &target_resource) {
        if source.value != target.value
            || source.description != target.description
            || source.resource_type != target.resource_type
        {
            has_changes = true;
        }
    } else if source_resource.is_some() || target_resource.is_some() {
        // The resource exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_resource.is_some(),
        exists_in_fork: target_resource.is_some(),
    });
}

async fn compare_two_variables(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    // Combine the four EXISTS checks (ws_specific × {source, fork}, variable
    // × {source, fork}) into a single round-trip; this runs per variable
    // during a workspace diff so the savings add up.
    let presence = sqlx::query!(
        r#"SELECT
            EXISTS(SELECT 1 FROM ws_specific
                   WHERE workspace_id = $1 AND item_kind = 'variable' AND path = $3) AS "src_ws!",
            EXISTS(SELECT 1 FROM ws_specific
                   WHERE workspace_id = $2 AND item_kind = 'variable' AND path = $3) AS "tgt_ws!",
            EXISTS(SELECT 1 FROM variable
                   WHERE workspace_id = $1 AND path = $3) AS "src_var!",
            EXISTS(SELECT 1 FROM variable
                   WHERE workspace_id = $2 AND path = $3) AS "tgt_var!""#,
        source_workspace_id,
        fork_workspace_id,
        path,
    )
    .fetch_one(db)
    .await?;

    // A workspace-specific variable keeps its own value per environment, so it never appears in the
    // diff. Seeding the initial copy onto a side that lacks it is a separate explicit action.
    if presence.src_ws || presence.tgt_ws {
        return Ok(ItemComparison {
            has_changes: false,
            exists_in_source: presence.src_var,
            exists_in_fork: presence.tgt_var,
        });
    }

    // Get variable from each workspace
    let source_variable = sqlx::query!(
        "SELECT value, is_secret, description
         FROM variable
         WHERE workspace_id = $1 AND path = $2",
        source_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let target_variable = sqlx::query!(
        "SELECT value, is_secret, description
         FROM variable
         WHERE workspace_id = $1 AND path = $2",
        fork_workspace_id,
        path
    )
    .fetch_optional(db)
    .await?;

    let mut has_changes = false;

    // Check metadata differences
    if let (Some(source), Some(target)) = (&source_variable, &target_variable) {
        if source.is_secret != target.is_secret
            || source.value != target.value
            || source.description != target.description
        {
            has_changes = true;
        }
    } else if source_variable.is_some() || target_variable.is_some() {
        // The variable exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_variable.is_some(),
        exists_in_fork: target_variable.is_some(),
    });
}

async fn compare_two_resource_types(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    name: &str,
) -> Result<ItemComparison> {
    // Get resource type from each workspace
    let source_resource_type = sqlx::query!(
        "SELECT schema, description, format_extension, is_fileset
         FROM resource_type
         WHERE workspace_id = $1 AND name = $2",
        source_workspace_id,
        name
    )
    .fetch_optional(db)
    .await?;

    let target_resource_type = sqlx::query!(
        "SELECT schema, description, format_extension, is_fileset
         FROM resource_type
         WHERE workspace_id = $1 AND name = $2",
        fork_workspace_id,
        name
    )
    .fetch_optional(db)
    .await?;

    let mut has_changes = false;

    // Check metadata differences
    if let (Some(source), Some(target)) = (&source_resource_type, &target_resource_type) {
        if source.schema != target.schema
            || source.description != target.description
            || source.format_extension != target.format_extension
            || source.is_fileset != target.is_fileset
        {
            has_changes = true;
        }
    } else if source_resource_type.is_some() || target_resource_type.is_some() {
        // The resource type exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_resource_type.is_some(),
        exists_in_fork: target_resource_type.is_some(),
    });
}

async fn compare_two_folders(
    db: &DB,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    name: &str,
) -> Result<ItemComparison> {
    // Get folder from each workspace
    let source_folder = sqlx::query!(
        "SELECT display_name, owners, extra_perms, summary
         FROM folder
         WHERE workspace_id = $1 AND name = $2",
        source_workspace_id,
        name.strip_prefix("f/"),
    )
    .fetch_optional(db)
    .await?;

    let target_folder = sqlx::query!(
        "SELECT display_name, owners, extra_perms, summary
         FROM folder
         WHERE workspace_id = $1 AND name = $2",
        fork_workspace_id,
        name.strip_prefix("f/"),
    )
    .fetch_optional(db)
    .await?;

    let mut has_changes = false;

    // Check metadata differences
    if let (Some(source), Some(target)) = (&source_folder, &target_folder) {
        if source.display_name != target.display_name
            || source.owners != target.owners
            || source.extra_perms != target.extra_perms
            || source.summary != target.summary
        {
            has_changes = true;
        }
    } else if source_folder.is_some() || target_folder.is_some() {
        // The folder exists in one of source or target, but not the other, this is considered as a change
        has_changes = true
    }

    return Ok(ItemComparison {
        has_changes,
        exists_in_source: source_folder.is_some(),
        exists_in_fork: target_folder.is_some(),
    });
}

/// Fields stripped before comparing two trigger or schedule rows.
///
/// `mode` and `enabled` are forced to disabled/false on fork clone so they always
/// differ between fork and parent — comparing them would mark every cloned row as
/// "changed". The rest are runtime state (`server_id`, `last_server_ping`, `error`)
/// or per-row metadata that diverges naturally (`edited_at/by`, `email`, `extra_perms`,
/// `permissioned_as`). Comparing without these answers "is this trigger/schedule
/// configured the same way?" rather than "are the rows byte-identical?".
const TRIGGER_COMPARE_IGNORE: &[&str] = &[
    "workspace_id",
    "edited_by",
    "edited_at",
    "email",
    "error",
    "enabled",
    "mode",
    "server_id",
    "last_server_ping",
    "extra_perms",
    "permissioned_as",
    // Server-managed fields that the merge feature treats as workspace-local
    // (regenerated by the deploy handler): GCP `subscription_id` is rewritten
    // to `windmill_<workspace_id>_<path>` in `CreateUpdate` mode, and Azure's
    // `push_auth_config` carries only the regenerated `secret_hash`. Without
    // stripping, GCP/Azure push triggers stay flagged as "changed" forever.
    "subscription_id",
    "push_auth_config",
];

async fn compare_two_trigger_or_schedule(
    db: &DB,
    table: &str,
    source_workspace_id: &str,
    fork_workspace_id: &str,
    path: &str,
) -> Result<ItemComparison> {
    // Whitelist guard: callers in `compare_workspaces` and `query_visible_items`
    // already match `table` against a closed set, but a stray future caller
    // could open an injection hole. Bail loudly in debug, fail safe in release.
    debug_assert!(
        TRIGGER_OR_SCHEDULE_TABLES.contains(&table),
        "compare_two_trigger_or_schedule called with unrecognized table: {table}"
    );
    if !TRIGGER_OR_SCHEDULE_TABLES.contains(&table) {
        return Ok(ItemComparison {
            has_changes: false,
            exists_in_source: false,
            exists_in_fork: false,
        });
    }

    let mut select_expr = String::from("to_jsonb(t)");
    for f in TRIGGER_COMPARE_IGNORE {
        // The `-` operator on jsonb returns the object without the named key,
        // or the unchanged object if the key is absent — so one ignore list
        // works across tables with different column sets.
        select_expr.push_str(&format!(" - '{f}'"));
    }
    // SAFETY: `table` comes from a hardcoded allowlist TRIGGER_OR_SCHEDULE_TABLES
    // (guarded by the debug_assert + runtime check above), not user input.
    // `select_expr` is built from `TRIGGER_COMPARE_IGNORE`, also a static const.
    let sql = format!("SELECT {select_expr} FROM {table} t WHERE workspace_id = $1 AND path = $2");

    let source_fut = sqlx::query_scalar::<_, serde_json::Value>(&sql)
        .bind(source_workspace_id)
        .bind(path)
        .fetch_optional(db);
    let target_fut = sqlx::query_scalar::<_, serde_json::Value>(&sql)
        .bind(fork_workspace_id)
        .bind(path)
        .fetch_optional(db);
    let (source, target) = tokio::try_join!(source_fut, target_fut)?;

    let has_changes = match (source.as_ref(), target.as_ref()) {
        (Some(s), Some(t)) => s != t,
        (None, None) => false,
        _ => true,
    };

    Ok(ItemComparison {
        has_changes,
        exists_in_source: source.is_some(),
        exists_in_fork: target.is_some(),
    })
}

const TRIGGER_OR_SCHEDULE_TABLES: &[&str] = &[
    "schedule",
    "http_trigger",
    "websocket_trigger",
    "kafka_trigger",
    "nats_trigger",
    "postgres_trigger",
    "mqtt_trigger",
    "sqs_trigger",
    "gcp_trigger",
    "azure_trigger",
    "email_trigger",
];

#[derive(Deserialize)]
struct LogAiChatPayload {
    session_id: String,
    provider: String,
    model: String,
    mode: String,
}

async fn log_ai_chat(
    Extension(db): Extension<DB>,
    Json(payload): Json<LogAiChatPayload>,
) -> Result<StatusCode> {
    sqlx::query!(
        "INSERT INTO ai_chat_usage (session_id, provider, model, mode) VALUES ($1, $2, $3, $4)
         ON CONFLICT (session_id) DO UPDATE SET message_count = ai_chat_usage.message_count + 1",
        &payload.session_id,
        &payload.provider,
        &payload.model,
        &payload.mode
    )
    .execute(&db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct QuotaInfo {
    used: i64,
    limit: i64,
    prunable: i64,
}

#[derive(Serialize)]
struct CloudQuotas {
    scripts: QuotaInfo,
    flows: QuotaInfo,
    apps: QuotaInfo,
    variables: QuotaInfo,
    resources: QuotaInfo,
    /// Fork/dev workspaces under this workspace's billing root vs the per-seat cap. `limit` is 0 for a
    /// non-premium root (forking is premium-only). Family-wide: resolves to the billing root, so it
    /// reads the same whether viewed from the root or one of its forks.
    forks: QuotaInfo,
}

async fn get_cloud_quotas(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<CloudQuotas> {
    require_admin(authed.is_admin, &authed.username)?;

    if !*CLOUD_HOSTED {
        return Err(Error::BadRequest(
            "Cloud quotas are only available on cloud-hosted instances".to_string(),
        ));
    }

    let scripts_used =
        sqlx::query_scalar!("SELECT COUNT(*) FROM script WHERE workspace_id = $1", &w_id)
            .fetch_one(&db)
            .await?
            .unwrap_or(0);

    let scripts_prunable = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM script s WHERE s.workspace_id = $1 AND s.hash NOT IN (
            SELECT DISTINCT ON (path) hash FROM script
            WHERE workspace_id = $1 AND deleted = false
            ORDER BY path, created_at DESC
        )",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    let flows_used =
        sqlx::query_scalar!("SELECT COUNT(*) FROM flow WHERE workspace_id = $1", &w_id)
            .fetch_one(&db)
            .await?
            .unwrap_or(0);

    let flows_prunable = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM flow_version fv
        JOIN flow f ON f.workspace_id = fv.workspace_id AND f.path = fv.path
        WHERE fv.workspace_id = $1 AND fv.id != f.versions[array_upper(f.versions, 1)]",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    let apps_used = sqlx::query_scalar!("SELECT COUNT(*) FROM app WHERE workspace_id = $1", &w_id)
        .fetch_one(&db)
        .await?
        .unwrap_or(0);

    let apps_prunable = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM app_version av
        JOIN app a ON a.id = av.app_id
        WHERE a.workspace_id = $1 AND av.id != a.versions[array_upper(a.versions, 1)]",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    let variables_used = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM variable WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    let resources_used = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM resource WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    // Fork/dev workspaces vs the per-seat cap, resolved to the billing root. Non-premium roots can't
    // fork, so their allowance is 0.
    #[cfg(feature = "cloud")]
    let forks = {
        use windmill_common::workspaces::{
            count_paid_seats, count_workspace_forks, get_billing_workspace_id, get_team_plan_status,
        };
        let root = get_billing_workspace_id(&db, &w_id).await?;
        let used = count_workspace_forks(&db, &root).await?;
        let limit = if get_team_plan_status(&db, &root).await?.premium {
            count_paid_seats(&db, &root).await?.max(1) * *MAX_FORKS_PER_SEAT
        } else {
            0
        };
        QuotaInfo { used, limit, prunable: 0 }
    };
    #[cfg(not(feature = "cloud"))]
    let forks = QuotaInfo { used: 0, limit: 0, prunable: 0 };

    Ok(Json(CloudQuotas {
        scripts: QuotaInfo { used: scripts_used, limit: 5000, prunable: scripts_prunable },
        flows: QuotaInfo { used: flows_used, limit: 1000, prunable: flows_prunable },
        apps: QuotaInfo { used: apps_used, limit: 1000, prunable: apps_prunable },
        variables: QuotaInfo { used: variables_used, limit: 10000, prunable: 0 },
        resources: QuotaInfo { used: resources_used, limit: 10000, prunable: 0 },
        forks,
    }))
}

#[derive(Deserialize)]
struct PruneVersionsRequest {
    resource_type: String,
}

#[derive(Serialize)]
struct PruneVersionsResponse {
    pruned: u64,
}

async fn prune_versions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<PruneVersionsRequest>,
) -> JsonResult<PruneVersionsResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    if !*CLOUD_HOSTED {
        return Err(Error::BadRequest(
            "Version pruning is only available on cloud-hosted instances".to_string(),
        ));
    }

    let pruned = match req.resource_type.as_str() {
        "scripts" => {
            let result = sqlx::query(
                "DELETE FROM script
                WHERE workspace_id = $1 AND hash NOT IN (
                    SELECT DISTINCT ON (path) hash FROM script
                    WHERE workspace_id = $1 AND deleted = false
                    ORDER BY path, created_at DESC
                )",
            )
            .bind(&w_id)
            .execute(&db)
            .await?;
            result.rows_affected()
        }
        "flows" => {
            let deleted = sqlx::query(
                "DELETE FROM flow_version fv
                USING flow f
                WHERE fv.workspace_id = f.workspace_id AND fv.path = f.path
                AND fv.workspace_id = $1
                AND fv.id != f.versions[array_upper(f.versions, 1)]",
            )
            .bind(&w_id)
            .execute(&db)
            .await?;

            sqlx::query(
                "UPDATE flow SET versions = ARRAY[versions[array_upper(versions, 1)]]
                WHERE workspace_id = $1 AND array_length(versions, 1) > 1",
            )
            .bind(&w_id)
            .execute(&db)
            .await?;

            deleted.rows_affected()
        }
        "apps" => {
            let deleted = sqlx::query(
                "DELETE FROM app_version av
                USING app a
                WHERE av.app_id = a.id AND a.workspace_id = $1
                AND av.id != a.versions[array_upper(a.versions, 1)]",
            )
            .bind(&w_id)
            .execute(&db)
            .await?;

            sqlx::query(
                "UPDATE app SET versions = ARRAY[versions[array_upper(versions, 1)]]
                WHERE workspace_id = $1 AND array_length(versions, 1) > 1",
            )
            .bind(&w_id)
            .execute(&db)
            .await?;

            deleted.rows_affected()
        }
        _ => {
            return Err(Error::BadRequest(format!(
                "Invalid resource type '{}'. Must be 'scripts', 'flows', or 'apps'",
                req.resource_type
            )));
        }
    };

    Ok(Json(PruneVersionsResponse { pruned }))
}

#[derive(Serialize)]
struct WsSpecificItem {
    item_kind: String,
    path: String,
}

async fn list_ws_specific(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WsSpecificItem>> {
    // ws_specific itself has no per-item RLS — only the workspace_id column.
    // Joining against resource/variable under user_db forces the same
    // path-based RLS policies that govern those tables (see_own / see_member /
    // see_extra_perms_* / see_folder_extra_perms_user) to also gate visibility
    // here. Without these joins, any workspace member could enumerate paths
    // in folders they lack read access to (e.g. f/finance/prod_db_creds).
    let mut tx = user_db.begin(&authed).await?;
    let items = sqlx::query_as!(
        WsSpecificItem,
        r#"
        SELECT s.item_kind, s.path
        FROM ws_specific s
        WHERE s.workspace_id = $1
          AND (
              (s.item_kind = 'resource' AND EXISTS (
                  SELECT 1 FROM resource r
                  WHERE r.workspace_id = s.workspace_id AND r.path = s.path
              ))
              OR (s.item_kind = 'variable' AND EXISTS (
                  SELECT 1 FROM variable v
                  WHERE v.workspace_id = s.workspace_id AND v.path = s.path
              ))
          )
        ORDER BY s.item_kind, s.path
        "#,
        &w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    // RLS gates membership/folder access, but a scoped API token must also be held to its read
    // scopes — mirror the resource/variable list endpoints, which filter with these predicates so a
    // token lacking `resources:read:*` / `variables:read:*` can't enumerate pinned paths it can't read.
    let resource_allowed = build_scope_path_predicate(&authed, "resources", "read");
    let variable_allowed = build_scope_path_predicate(&authed, "variables", "read");
    let items = items
        .into_iter()
        .filter(|it| match it.item_kind.as_str() {
            "resource" => resource_allowed(&it.path),
            "variable" => variable_allowed(&it.path),
            _ => false,
        })
        .collect::<Vec<_>>();
    Ok(Json(items))
}

#[derive(Deserialize)]
struct ListWsSpecificVersionsQuery {
    kind: String,
    path: String,
}

async fn list_ws_specific_versions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(q): Query<ListWsSpecificVersionsQuery>,
) -> JsonResult<Vec<String>> {
    if q.kind != "resource" && q.kind != "variable" {
        return Err(Error::BadRequest(format!(
            "Invalid kind '{}'. Must be 'resource' or 'variable'",
            q.kind
        )));
    }

    // A scoped API token must hold the read scope for this path, like the resource/variable read
    // endpoints. Without the scope, report no versions rather than leaking the path's history.
    let domain = if q.kind == "resource" {
        "resources"
    } else {
        "variables"
    };
    if !build_scope_path_predicate(&authed, domain, "read")(&q.path) {
        return Ok(Json(vec![]));
    }

    let versions: Vec<String> = sqlx::query_scalar!(
        r#"SELECT ws AS "ws!" FROM list_ws_specific_versions($1, $2, $3, $4)"#,
        &w_id,
        &authed.email,
        &q.kind,
        &q.path,
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(versions))
}

#[derive(Deserialize)]
struct SetWsSpecificBody {
    item_kind: String,
    path: String,
    value: bool,
}

/// Mark (or unmark) a single resource/variable as workspace-specific. Pinning
/// excludes it from the deploy diff so each environment keeps its own value
/// (see `compare_two_resources`/`compare_two_variables`). Set per-workspace, so
/// the compare page calls this once per side to flag both environments.
async fn set_ws_specific(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(body): Json<SetWsSpecificBody>,
) -> Result<String> {
    if body.item_kind != "resource" && body.item_kind != "variable" {
        return Err(Error::BadRequest(format!(
            "Invalid kind '{}'. Must be 'resource' or 'variable'",
            body.item_kind
        )));
    }

    // Reject a malformed path with a 400 before the auth check, which indexes the leading segments and
    // would otherwise panic (500) on a path like `u` with no segment. Accept all three shared path
    // shapes Windmill uses — `u/<user>`, `f/<folder>`, `g/<group>` (e.g. seeded `g/all/...` resources).
    let segs: Vec<&str> = body.path.split('/').collect();
    if segs.len() < 2 || !matches!(segs[0], "u" | "f" | "g") || segs[1].is_empty() {
        return Err(Error::BadRequest(format!(
            "Invalid {} path: {}",
            body.item_kind, body.path
        )));
    }

    // Authorize like the resource/variable editors' own ws_specific toggle:
    // actual write access to the item + token scope + the workspace deploy rules.
    // `require_owner_of_path` is the real write gate (the resource editor uses it);
    // `check_scopes` only constrains scoped tokens (it is a no-op for session/cookie
    // logins). Together: a non-admin who can edit the item may pin it, while a
    // read-only member is rejected and a locked workspace still blocks non-deployers.
    // `require_is_writer` matches the resource/variable editors' write semantics (owner, folder
    // writer, or item writer via extra_perms) — not owner-only.
    let writer_query = if body.item_kind == "resource" {
        "SELECT extra_perms FROM resource WHERE path = $1 AND workspace_id = $2"
    } else {
        "SELECT extra_perms FROM variable WHERE path = $1 AND workspace_id = $2"
    };
    require_is_writer(
        &authed,
        &body.path,
        &w_id,
        db.clone(),
        writer_query,
        &body.item_kind,
    )
    .await?;
    check_scopes(&authed, || {
        format!("{}s:write:{}", body.item_kind, body.path)
    })?;
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        &authed.username,
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = user_db.begin(&authed).await?;

    if body.value {
        // Existence guard keeps a dangling marker from being created for a
        // path absent in this workspace.
        if body.item_kind == "resource" {
            sqlx::query!(
                "INSERT INTO ws_specific (workspace_id, item_kind, path)
                 SELECT $1::varchar, 'resource', $2::varchar
                 WHERE EXISTS (SELECT 1 FROM resource WHERE workspace_id = $1::varchar AND path = $2::varchar)
                 ON CONFLICT DO NOTHING",
                w_id,
                body.path,
            )
            .execute(&mut *tx)
            .await?;
            // A resource owns its `$var:` secrets, so pin those too.
            windmill_store::resources::mark_linked_variables_ws_specific(
                &mut tx, &authed, &w_id, &body.path,
            )
            .await?;
        } else {
            sqlx::query!(
                "INSERT INTO ws_specific (workspace_id, item_kind, path)
                 SELECT $1::varchar, 'variable', $2::varchar
                 WHERE EXISTS (SELECT 1 FROM variable WHERE workspace_id = $1::varchar AND path = $2::varchar)
                 ON CONFLICT DO NOTHING",
                w_id,
                body.path,
            )
            .execute(&mut *tx)
            .await?;
        }
    } else {
        // Unmark only this item; linked variables stay flagged (they may be
        // referenced by other resources) — mirrors the resource-form toggle.
        sqlx::query!(
            "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = $2 AND path = $3",
            w_id,
            body.item_kind,
            body.path,
        )
        .execute(&mut *tx)
        .await?;
        // While pinned, the item's cached workspace_diff verdict was never recomputed (the compare
        // read filter excludes it), so it may now be stale in either direction. Mark it NULL so the
        // next compare re-evaluates from scratch — and the now-shared item reappears (or is dropped)
        // correctly instead of being stuck on its pre-pin verdict.
        sqlx::query!(
            "UPDATE workspace_diff SET has_changes = NULL
             WHERE path = $2 AND kind = $3
               AND ($1 IN (source_workspace_id, fork_workspace_id))",
            w_id,
            body.path,
            body.item_kind,
        )
        .execute(&mut *tx)
        .await?;
    }

    let value_str = body.value.to_string();
    audit_log(
        &mut *tx,
        &authed,
        &format!("{}s.set_ws_specific", body.item_kind),
        ActionKind::Update,
        &w_id,
        Some(&body.path),
        Some([("value", value_str.as_str())].into()),
    )
    .await?;

    tx.commit().await?;
    Ok(format!(
        "Set workspace-specific={} for {} {}",
        body.value, body.item_kind, body.path
    ))
}
