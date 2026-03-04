/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_api_auth::{require_super_admin, ApiAuthed};
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
use windmill_common::users::username_to_permissioned_as;
use windmill_common::variables::{build_crypt, decrypt, encrypt, WORKSPACE_CRYPT_CACHE};
use windmill_common::worker::{to_raw_value, CLOUD_HOSTED};
#[cfg(feature = "enterprise")]
use windmill_common::workspaces::GitRepositorySettings;
#[cfg(feature = "enterprise")]
use windmill_common::workspaces::WorkspaceDeploymentUISettings;
use windmill_common::workspaces::{
    check_user_against_rule, get_datatable_resource_from_db_unchecked, DataTable,
    DataTableCatalogResourceType, ProtectionRuleKind, ProtectionRules, ProtectionRuleset,
    RuleCheckResult, WorkspaceGitSyncSettings,
};
use windmill_common::workspaces::{Ducklake, DucklakeCatalogResourceType};
use windmill_common::PgDatabase;
use windmill_common::{
    error::{Error, JsonResult, Result},
    global_settings::AUTOMATE_USERNAME_CREATION_SETTING,
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
    utils::{paginate, rd_string, require_admin, Pagination},
};
use windmill_dep_map::scoped_dependency_map::{
    DependencyDependent, DependencyMap, ScopedDependencyMap,
};
use windmill_git_sync::{handle_deployment_metadata, handle_fork_branch_creation, DeployedObject};
use windmill_types::s3::LargeFileStorage;

use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use windmill_common::oauth2::InstanceEvent;
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
        .route("/delete_invite", post(delete_invite))
        .route("/rebuild_dependency_map", post(rebuild_dependency_map))
        .route("/get_dependency_map", get(get_dependency_map))
        .route("/get_dependents/*imported_path", get(get_dependents))
        .route("/get_dependents_amounts", post(get_dependents_amounts))
        .route("/get_settings", get(get_settings))
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
        .route("/edit_datatable_config", post(edit_datatable_config))
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
            "/reset_diff_tally/:fork_workspace_id",
            post(reset_workspace_diffs),
        )
        .route("/compare/:target_workspace_id", get(compare_workspaces))
        .route("/protection_rules", get(list_protection_rules))
        .route("/protection_rules", post(create_protection_rule))
        .route(
            "/protection_rules/:rule_name",
            post(update_protection_rule).delete(delete_protection_rule),
        )
}
pub fn global_service() -> Router {
    Router::new()
        .route("/list_as_superadmin", get(list_workspaces_as_super_admin))
        .route("/list", get(list_workspaces))
        .route("/users", get(user_workspaces))
        .route("/create", post(create_workspace))
        .route("/create_fork", post(deprecated_create_workspace_fork))
        .route("/exists", post(exists_workspace))
        .route("/exists_username", post(exists_username))
        .route("/allowed_domain_auto_invite", get(is_allowed_auto_domain))
        .route("/unarchive/:workspace", post(unarchive_workspace))
        .route(
            "/delete/:workspace",
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
    authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
) -> JsonResult<bool> {
    require_admin(authed.is_admin, &authed.username)?;
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

async fn get_settings(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<WorkspaceSettings> {
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

    tx.commit().await?;

    let mut settings = not_found_if_none(settings, "workspace settings", &w_id)?;
    if !authed.is_admin {
        settings.slack_oauth_client_secret = None;
    }
    Ok(Json(settings))
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
pub const WM_FORK_PREFIX: &str = "wm-fork-";
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

async fn list_datatables(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let datatables = sqlx::query_scalar!(
        r#"
            SELECT jsonb_object_keys(ws.datatable->'datatables') AS datatable_name
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

    Ok(Json(datatables))
}

/// Compact column representation: "type" or "type?" for nullable, with "=default" suffix if has default
type CompactColumn = String;

/// Columns mapped by name to their compact type
type ColumnMap = HashMap<String, CompactColumn>;

/// Tables mapped by name to their columns
type TableMap = HashMap<String, ColumnMap>;

/// Schemas mapped by name to their tables
type SchemaMap = HashMap<String, TableMap>;

#[derive(Serialize, Debug)]
struct DataTableSchema {
    datatable_name: String,
    /// Hierarchical schema: schema_name -> table_name -> column_name -> "type[?][=default]"
    schemas: SchemaMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn list_datatable_schemas(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DataTableSchema>> {
    // Get all datatable names for this workspace
    let datatable_names: Vec<String> = sqlx::query_scalar!(
        r#"
            SELECT jsonb_object_keys(ws.datatable->'datatables') AS datatable_name
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

async fn get_datatable_schema(db: &DB, w_id: &str, datatable_name: &str) -> Result<SchemaMap> {
    // Get the datatable resource (connection credentials)
    let db_resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;

    // Parse the resource as PgDatabase
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {}", e)))?;

    // Connect to the datatable database
    let (client, connection) = pg_db.connect().await?;

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

        // Build compact type representation: "type[?][=default]"
        let mut compact = udt_name;
        if is_nullable == "YES" {
            compact.push('?');
        }
        if let Some(default) = column_default {
            // Truncate long defaults for compactness
            let short_default = if default.len() > 30 {
                format!("{}...", &default[..27])
            } else {
                default
            };
            compact.push('=');
            compact.push_str(&short_default);
        }

        schema_map
            .entry(table_schema)
            .or_default()
            .entry(table_name)
            .or_default()
            .insert(column_name, compact);
    }

    Ok(schema_map)
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

    // Check that non-superadmins are not abusing Instance databases
    if !is_superadmin {
        let old_ducklakes = sqlx::query_scalar!(
            r#"
                SELECT ws.ducklake->'ducklakes' AS ducklake_name
                FROM workspace_settings ws
                WHERE ws.workspace_id = $1
            "#,
            &w_id
        )
        .fetch_one(&db)
        .await?
        .unwrap_or(serde_json::Value::Null);
        let old_ducklakes: HashMap<String, Ducklake> =
            serde_json::from_value(old_ducklakes).unwrap_or_default();
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

    let config: serde_json::Value = serde_json::to_value(new_config.settings)
        .map_err(|err| Error::internal_err(err.to_string()))?;

    sqlx::query!(
        "UPDATE workspace_settings SET ducklake = $1 WHERE workspace_id = $2",
        config,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(format!("Edit ducklake config for workspace {}", &w_id))
}

async fn edit_datatable_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, email, .. }: ApiAuthed,
    Json(new_config): Json<EditDataTableConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let is_superadmin = require_super_admin(&db, &email).await.is_ok();

    let mut tx = db.begin().await?;

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
        let old_datatables = sqlx::query_scalar!(
            r#"
                SELECT ws.datatable->'datatables' AS datatable_name
                FROM workspace_settings ws
                WHERE ws.workspace_id = $1
            "#,
            &w_id
        )
        .fetch_one(&db)
        .await?
        .unwrap_or(serde_json::Value::Null);
        let old_datatables: HashMap<String, DataTable> =
            serde_json::from_value(old_datatables).unwrap_or_default();
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

    tx.commit().await?;

    Ok(format!("Edit datatable config for workspace {}", &w_id))
}

#[derive(Deserialize)]
pub struct EditGitSyncConfig {
    #[cfg(feature = "enterprise")]
    pub git_sync_settings: Option<WorkspaceGitSyncSettings>,
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct EditGitSyncRepository {
    pub git_repo_resource_path: String,
    pub repository: GitRepositorySettings,
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct DeleteGitSyncRepositoryRequest {
    pub git_repo_resource_path: String,
}

#[cfg(feature = "enterprise")]
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

#[cfg(feature = "enterprise")]
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
async fn edit_git_sync_config(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
    Json(_new_config): Json<EditGitSyncConfig>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Git sync is only available on Windmill Enterprise Edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
async fn edit_git_sync_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<EditGitSyncConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

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

    if let Some(mut git_sync_settings) = new_config.git_sync_settings {
        // Clean up legacy workspace-level settings if all repos are migrated
        cleanup_legacy_git_sync_settings_in_memory(&mut git_sync_settings, &w_id);

        let serialized_config = serde_json::to_value::<WorkspaceGitSyncSettings>(git_sync_settings)
            .map_err(|err| Error::internal_err(err.to_string()))?;

        sqlx::query!(
            "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
            serialized_config,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET git_sync = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

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

#[cfg(not(feature = "enterprise"))]
async fn edit_git_sync_repository(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
    Json(_new_config): Json<serde_json::Value>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Git sync is only available on Windmill Enterprise Edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
async fn edit_git_sync_repository(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(new_config): Json<EditGitSyncRepository>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    // Validate the resource path format
    validate_git_repo_resource_path(&new_config.git_repo_resource_path)?;

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
        // Update existing repository
        *existing_repo = new_config.repository;
    } else {
        // Repository doesn't exist, add it as a new repository
        git_sync_settings.repositories.push(new_config.repository);
    }

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

#[cfg(not(feature = "enterprise"))]
async fn delete_git_sync_repository(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
    Json(_request): Json<serde_json::Value>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Git sync is only available on Windmill Enterprise Edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
async fn delete_git_sync_repository(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(request): Json<DeleteGitSyncRepositoryRequest>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    // For deletion, only validate that path is not empty to allow cleanup of malformed entries
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
}
async fn get_default_app(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<WorkspaceDefaultApp> {
    let mut tx = db.begin().await?;
    let default_app_path = sqlx::query_scalar!(
        "SELECT default_app FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| Error::internal_err(format!("getting default_app: {err}")))?;
    tx.commit().await?;

    Ok(Json(WorkspaceDefaultApp { default_app_path }))
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

        let mut error_handler = serde_json::json!({
            "path": path,
        });
        if let Some(extra_args) = &ee.extra_args {
            error_handler["extra_args"] = extra_args.clone();
        }
        if ee.muted_on_cancel {
            error_handler["muted_on_cancel"] = serde_json::json!(true);
        }
        if ee.muted_on_user_path {
            error_handler["muted_on_user_path"] = serde_json::json!(true);
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

    let previous_encryption_key = build_crypt(&db, w_id.as_str()).await?;

    sqlx::query!(
        "UPDATE workspace_key SET key = $1 WHERE workspace_id = $2",
        request.new_key.clone(),
        w_id
    )
    .execute(&db)
    .await?;

    WORKSPACE_CRYPT_CACHE.remove(w_id.as_str());

    if !request.skip_reencrypt.unwrap_or(false) {
        let new_encryption_key = build_crypt(&db, w_id.as_str()).await?;

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
        .fetch_all(&db)
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
            .execute(&db)
            .await?;
        }
    }

    // Trigger git sync for encryption key changes
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Key { key_type: "encryption_key".to_string() },
        Some("Encryption key updated".to_string()),
        false,
        None,
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
    pub email_used: bool,
    pub nextcloud_used: bool,
    pub google_used: bool,
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
            EXISTS(SELECT 1 FROM email_trigger WHERE workspace_id = $1) AS "email_used!",
            EXISTS(SELECT 1 FROM native_trigger WHERE workspace_id = $1 AND service_name = 'nextcloud'::native_trigger_service) AS "nextcloud_used!",
            EXISTS(SELECT 1 FROM native_trigger WHERE workspace_id = $1 AND service_name = 'google'::native_trigger_service) AS "google_used!"
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
    require_super_admin(&db, &email).await?;
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

async fn clone_workspace_data(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
) -> Result<()> {
    // Clone workspace settings (merge with existing basic settings)
    update_workspace_settings(tx, source_workspace_id, target_workspace_id).await?;

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

    // Clone variables with re-encryption
    clone_variables(tx, source_workspace_id, target_workspace_id).await?;

    // Clone scripts with new hashes
    clone_scripts(tx, source_workspace_id, target_workspace_id).await?;

    // Clone flows with new versions
    clone_flows(tx, source_workspace_id, target_workspace_id).await?;

    // Clone flow nodes
    clone_flow_nodes(tx, source_workspace_id, target_workspace_id).await?;

    // Clone apps with new IDs and app scripts
    let _app_id_mapping = clone_apps(tx, source_workspace_id, target_workspace_id).await?;

    // Clone raw apps
    clone_raw_apps(tx, source_workspace_id, target_workspace_id).await?;

    // Clone workspace runnable dependencies and dependency map
    clone_workspace_runnable_dependencies(tx, source_workspace_id, target_workspace_id).await?;

    // Clone workspace dependencies
    clone_workspace_dependencies(tx, source_workspace_id, target_workspace_id).await?;
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
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, edited_at, created_by)
         SELECT $2, name, display_name, owners, extra_perms, summary, edited_at, created_by
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
            extra_perms, lock, lock_error_logs, language, kind, tag, draft_only,
            envs, concurrent_limit, concurrency_time_window_s, cache_ttl,
            dedicated_worker, ws_error_handler_muted, priority, timeout,
            delete_after_use, restart_unless_cancelled, concurrency_key,
            visible_to_runner_only, no_main_func, codebase, has_preprocessor,
            on_behalf_of_email, assets
        )
        SELECT
            $1, hash, path, parent_hashes, summary, description, content,
            created_by, created_at, archived, schema, deleted, is_template,
            extra_perms, lock, lock_error_logs, language, kind, tag, draft_only,
            envs, concurrent_limit, concurrency_time_window_s, cache_ttl,
            dedicated_worker, ws_error_handler_muted, priority, timeout,
            delete_after_use, restart_unless_cancelled, concurrency_key,
            visible_to_runner_only, no_main_func, codebase, has_preprocessor,
            on_behalf_of_email, assets
        FROM script
        WHERE workspace_id = $2"#,
        target_workspace_id,
        source_workspace_id
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
            archived, schema, extra_perms, dependency_job, draft_only, tag,
            ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
            concurrency_key, versions, on_behalf_of_email, lock_error_logs
        )
        SELECT $2, path, summary, description, value, edited_by, edited_at,
               archived, schema, extra_perms, NULL, draft_only, tag,
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
        "SELECT id, workspace_id, path, summary, policy, versions, extra_perms, draft_only, custom_path
         FROM app
         WHERE workspace_id = $1",
        source_workspace_id
    )
    .fetch_all(&mut **tx)
    .await?;

    let mut app_id_mapping: HashMap<i64, i64> = HashMap::new();

    // Clone apps with new IDs
    for app in apps {
        let new_app_id = sqlx::query_scalar!(
            "INSERT INTO app (workspace_id, path, summary, policy, versions, extra_perms, draft_only, custom_path)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
            target_workspace_id,
            app.path,
            app.summary,
            app.policy,
            &Vec::<i64>::new(), // Start with empty versions array
            app.extra_perms,
            app.draft_only,
            app.custom_path,
        )
        .fetch_one(&mut **tx)
        .await?;

        app_id_mapping.insert(app.id, new_app_id);
    }

    {
        // Clone app versions
        let app_versions = sqlx::query!(
            "SELECT app_id, value, created_by, created_at, raw_app
         FROM app_version
         WHERE app_id = ANY(SELECT id FROM app WHERE workspace_id = $1)
         ORDER BY app_id, created_at",
            source_workspace_id
        )
        .fetch_all(&mut **tx)
        .await?;

        for version in app_versions {
            if let Some(&new_app_id) = app_id_mapping.get(&version.app_id) {
                sqlx::query!(
                    "INSERT INTO app_version (app_id, value, created_by, created_at, raw_app)
                 VALUES ($1, $2, $3, $4, $5)",
                    new_app_id,
                    version.value,
                    version.created_by,
                    version.created_at,
                    version.raw_app,
                )
                .execute(&mut **tx)
                .await?;
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
    if *CLOUD_HOSTED {
        return Err(Error::BadRequest(format!(
            "Forking workspaces is not available on app.windmill.dev"
        )));
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

    Ok(Json(
        handle_fork_branch_creation(&authed.email, &authed.username, &db, &w_id, &nw.id).await?,
    ))
}

async fn create_workspace_fork(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(parent_workspace_id): Path<String>,
    Json(nw): Json<CreateWorkspaceFork>,
) -> Result<String> {
    if *CLOUD_HOSTED {
        return Err(Error::BadRequest(format!(
            "Forking workspaces is not available on app.windmill.dev"
        )));
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

    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    // Generate unique forked workspace ID with wm-fork prefix
    if !nw.id.starts_with(WM_FORK_PREFIX) {
        return Err(Error::BadRequest(format!(
            "The id `{}` is invalid for a forked workspace. It should be prefixed by {}",
            nw.id, WM_FORK_PREFIX
        )));
    }

    let forked_id = nw.id;

    sqlx::query!(
        "INSERT INTO workspace
            (id, name, owner, parent_workspace_id)
            VALUES ($1, $2, $3, $4)",
        forked_id,
        nw.name,
        authed.email,
        parent_workspace_id,
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

    sqlx::query!(
        "INSERT INTO usr
           (workspace_id, email, username, is_admin)
           SELECT $1, email, username, is_admin FROM usr
         WHERE workspace_id = $3 AND email = $2
        ",
        forked_id,
        authed.email,
        parent_workspace_id,
    )
    .execute(&mut *tx)
    .await?;

    // Clone all data from the parent workspace using Rust implementation
    clone_workspace_data(&mut tx, &parent_workspace_id, &forked_id).await?;

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
    Ok(format!("Created forked workspace {}", &forked_id))
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
        "DELETE FROM token WHERE workspace_id = $1 AND label IS DISTINCT FROM 'session' RETURNING token",
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

    let (schedules_count, canceled_count, deleted_tokens_count) =
        archive_workspace_impl(&db, &w_id, &authed.username).await?;

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
        Some(audit_params_refs),
    )
    .await?;
    tx.commit().await?;

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
    tx.commit().await?;

    Ok(format!("Unarchived workspace {}", &w_id))
}

async fn invite_user(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    nu.email = nu.email.to_lowercase();

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

    send_email_if_possible(
        &format!("Invited to Windmill's workspace: {w_id}"),
        &format!(
            "You have been granted access to Windmill's workspace {w_id}

If you do not have an account on {}, login with SSO or ask an admin to create an account for you.",
            BASE_URL.read().await.clone()
        ),
        &nu.email,
    );

    webhook.send_instance_event(InstanceEvent::UserInvitedWorkspace {
        email: nu.email.clone(),
        workspace: w_id,
    });

    Ok((
        StatusCode::CREATED,
        format!("user with email {} invited", nu.email),
    ))
}

async fn add_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut nu): Json<NewWorkspaceUser>,
) -> Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;
    nu.email = nu.email.to_lowercase();

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

    let username = if automate_username_creation {
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

    send_email_if_possible(
        &format!("Added to Windmill's workspace: {w_id}"),
        &format!(
            "You have been granted access to Windmill's workspace {w_id} by {}

If you do not have an account on {}, login with SSO or ask an admin to create an account for you.",
            authed.email,
            BASE_URL.read().await.clone()
        ),
        &nu.email,
    );

    webhook.send_instance_event(InstanceEvent::UserAddedWorkspace {
        workspace: w_id.clone(),
        email: nu.email.clone(),
    });

    Ok((
        StatusCode::CREATED,
        format!("user with email {} added", nu.email),
    ))
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

/// Create a new protection rule
async fn create_protection_rule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<CreateProtectionRuleRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

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
        }));
    }

    let diff_items = sqlx::query_as!(
        WorkspaceDiffRow,
        "SELECT path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork FROM workspace_diff
        WHERE source_workspace_id = $1 AND fork_workspace_id = $2",
        source_workspace_id,
        fork_workspace_id,
    )
    .fetch_all(&db)
    .await?;

    let mut confirmed_diffs = vec![];
    for item in diff_items {
        if let Some(has_changes) = item.has_changes {
            if has_changes {
                confirmed_diffs.push(item);
            }
            continue;
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
            "app" => Some(
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

    let visible_diffs = filter_visible_diffs(
        &confirmed_diffs,
        &source_workspace_id,
        &fork_workspace_id,
        user_db.begin(&authed).await?,
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
        apps_changed: visible_diffs.iter().filter(|s| s.kind == "app").count(),
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

    return Ok(Json(WorkspaceComparison {
        all_ahead_items_visible,
        all_behind_items_visible,
        skipped_comparison: false,
        diffs: visible_diffs,
        summary,
    }));
}

async fn filter_visible_diffs(
    confirmed_diffs: &[WorkspaceDiffRow],
    source_workspace_id: &str,
    fork_workspace_id: &str,
    mut tx: Transaction<'static, Postgres>,
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

    // Step 2: Batch query for each (workspace, kind) combination
    let source_visible = query_visible_items(&mut tx, source_workspace_id, &source_items).await?;
    let fork_visible = query_visible_items(&mut tx, fork_workspace_id, &fork_items).await?;

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
            "app" => {
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
            _ => vec![], // Unknown kind
        };

        for path in results {
            visible.insert((kind.to_string(), path));
        }
    }

    Ok(visible)
}

#[derive(Debug)]
struct ItemComparison {
    has_changes: bool,
    exists_in_source: bool,
    exists_in_fork: bool,
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
         WHERE app.workspace_id = $1 AND app.path = $2 AND COALESCE(app.draft_only, false) = false",
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
         WHERE app.workspace_id = $1 AND app.path = $2 AND COALESCE(app.draft_only, false) = false",
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
