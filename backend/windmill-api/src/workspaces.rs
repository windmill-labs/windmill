/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::ai::{AiResource, AI_KEY_CACHE};
use crate::db::ApiAuthed;
use crate::users_ee::send_email_if_possible;
use crate::utils::get_instance_username_or_create_pending;
use crate::BASE_URL;
use crate::{
    db::DB,
    users::{WorkspaceInvite, VALID_USERNAME},
    utils::require_super_admin,
    webhook_util::WebhookShared,
};

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;

use regex::Regex;

use uuid::Uuid;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::db::UserDB;
use windmill_common::s3_helpers::LargeFileStorage;
use windmill_common::users::username_to_permissioned_as;
use windmill_common::variables::build_crypt;
use windmill_common::worker::to_raw_value;
#[cfg(feature = "enterprise")]
use windmill_common::workspaces::WorkspaceDeploymentUISettings;
#[cfg(feature = "enterprise")]
use windmill_common::workspaces::WorkspaceGitSyncSettings;
use windmill_common::{
    error::{Error, JsonResult, Result},
    global_settings::AUTOMATE_USERNAME_CREATION_SETTING,
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
    utils::{paginate, rd_string, require_admin, Pagination},
};
use windmill_git_sync::handle_deployment_metadata;

#[cfg(feature = "enterprise")]
use windmill_common::utils::require_admin_or_devops;

use crate::variables::{decrypt, encrypt};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use windmill_common::oauth2::InstanceEvent;
use windmill_common::utils::not_found_if_none;

lazy_static::lazy_static! {
    static ref WORKSPACE_KEY_REGEXP: Regex = Regex::new("^[a-zA-Z0-9]{64}$").unwrap();
}

pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/list_pending_invites", get(list_pending_invites))
        .route("/update", post(edit_workspace))
        .route("/archive", post(archive_workspace))
        .route("/invite_user", post(invite_user))
        .route("/add_user", post(add_user))
        .route("/delete_invite", post(delete_invite))
        .route("/get_settings", get(get_settings))
        .route("/get_deploy_to", get(get_deploy_to))
        .route("/edit_slack_command", post(edit_slack_command))
        .route(
            "/run_slack_message_test_job",
            post(run_slack_message_test_job),
        )
        .route("/edit_webhook", post(edit_webhook))
        .route("/edit_auto_invite", post(edit_auto_invite))
        .route("/edit_deploy_to", post(edit_deploy_to))
        .route("/tarball", get(crate::workspaces_export::tarball_workspace))
        .route("/is_premium", get(is_premium))
        .route("/edit_copilot_config", post(edit_copilot_config))
        .route("/get_copilot_info", get(get_copilot_info))
        .route("/edit_error_handler", post(edit_error_handler))
        .route(
            "/edit_large_file_storage_config",
            post(edit_large_file_storage_config),
        )
        .route("/edit_git_sync_config", post(edit_git_sync_config))
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
        .route("/change_workspace_name", post(change_workspace_name))
        .route(
            "/change_workspace_id",
            post(crate::workspaces_extra::change_workspace_id),
        )
        .route("/usage", get(get_usage))
        .route("/used_triggers", get(get_used_triggers))
        .route("/critical_alerts", get(get_critical_alerts))
        .route(
            "/critical_alerts/:id/acknowledge",
            post(acknowledge_critical_alert),
        )
        .route(
            "/critical_alerts/acknowledge_all",
            post(acknowledge_all_critical_alerts),
        )
        .route("/critical_alerts/mute", post(mute_critical_alerts));

    #[cfg(feature = "stripe")]
    {
        crate::stripe_ee::add_stripe_routes(router)
    }

    #[cfg(not(feature = "stripe"))]
    router
}
pub fn global_service() -> Router {
    Router::new()
        .route("/list_as_superadmin", get(list_workspaces_as_super_admin))
        .route("/list", get(list_workspaces))
        .route("/users", get(user_workspaces))
        .route("/create", post(create_workspace))
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
}

#[derive(FromRow, Serialize, Debug)]
pub struct WorkspaceSettings {
    pub workspace_id: String,
    pub slack_team_id: Option<String>,
    pub slack_name: Option<String>,
    pub slack_command_script: Option<String>,
    pub slack_email: String,
    pub auto_invite_domain: Option<String>,
    pub auto_invite_operator: Option<bool>,
    pub auto_add: Option<bool>,
    pub customer_id: Option<String>,
    pub plan: Option<String>,
    pub webhook: Option<String>,
    pub deploy_to: Option<String>,
    pub ai_resource: Option<serde_json::Value>,
    pub code_completion_enabled: bool,
    pub error_handler: Option<String>,
    pub error_handler_extra_args: Option<serde_json::Value>,
    pub error_handler_muted_on_cancel: Option<bool>,
    pub large_file_storage: Option<serde_json::Value>, // effectively: DatasetsStorage
    pub git_sync: Option<serde_json::Value>,           // effectively: WorkspaceGitSyncSettings
    pub deploy_ui: Option<serde_json::Value>,          // effectively: WorkspaceDeploymentUISettings
    pub default_app: Option<String>,
    pub automatic_billing: bool,
    pub default_scripts: Option<serde_json::Value>,
    pub mute_critical_alerts: Option<bool>,
}

#[derive(FromRow, Serialize, Debug)]
pub struct Usage {
    pub workspace_id: String,
    pub slack_team_id: Option<String>,
    pub slack_name: Option<String>,
    pub slack_command_script: Option<String>,
    pub slack_email: String,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "WORKSPACE_KEY_KIND", rename_all = "lowercase")]
pub enum WorkspaceKeyKind {
    Cloud,
}

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

#[derive(Deserialize)]
struct EditCopilotConfig {
    ai_resource: Option<serde_json::Value>,
    code_completion_enabled: bool,
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

#[derive(Deserialize)]
struct CreateWorkspace {
    id: String,
    name: String,
    username: Option<String>,
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

#[derive(Deserialize)]
pub struct EditErrorHandler {
    pub error_handler: Option<String>,
    pub error_handler_extra_args: Option<serde_json::Value>,
    pub error_handler_muted_on_cancel: Option<bool>,
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
        "SELECT * from workspace_invite WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn is_premium(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<bool> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query_scalar!(
        "SELECT premium FROM workspace WHERE workspace.id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(row))
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
        "SELECT workspace.* FROM workspace, usr WHERE usr.workspace_id = workspace.id AND \
         usr.email = $1 AND deleted = false",
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
        "SELECT * FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting settings: {e:#}")))?;

    tx.commit().await?;
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
    .map_err(|e| Error::InternalErr(format!("getting deploy_to: {e:#}")))?;

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

    Ok(format!("Edit deploy to for {}", &w_id))
}

#[cfg(not(feature = "enterprise"))]
async fn edit_deploy_to() -> Result<String> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

pub const BANNED_DOMAINS: &str = include_str!("../banned_domains.txt");

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
    crate::workspaces_ee::edit_auto_invite(authed, db, w_id, ea).await
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

    Ok(format!("Edit webhook for workspace {}", &w_id))
}

async fn edit_copilot_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(eo): Json<EditCopilotConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    if let Some(ai_resource) = &eo.ai_resource {
        let path = serde_json::from_value::<AiResource>(ai_resource.clone())
            .map_err(|e| Error::BadRequest(e.to_string()))?
            .path;
        sqlx::query!(
            "UPDATE workspace_settings SET ai_resource = $1, code_completion_enabled = $2 WHERE workspace_id = $3",
            ai_resource,
            eo.code_completion_enabled,
            &w_id
        )
        .execute(&mut *tx)
        .await?;

        if let Some(cached) = AI_KEY_CACHE.get(&w_id) {
            if cached.path != path {
                AI_KEY_CACHE.remove(&w_id);
            }
        }
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET ai_resource = NULL, code_completion_enabled = $1 WHERE workspace_id = $2",
            eo.code_completion_enabled,
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
        AI_KEY_CACHE.remove(&w_id);
    }
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_copilot_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [
                ("ai_resource", &format!("{:?}", eo.ai_resource)[..]),
                (
                    "code_completion_enabled",
                    &format!("{:?}", eo.code_completion_enabled)[..],
                ),
            ]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit copilot config for workspace {}", &w_id))
}

#[derive(Serialize)]
struct CopilotInfo {
    pub ai_provider: String,
    pub exists_ai_resource: bool,
    pub code_completion_enabled: bool,
}
async fn get_copilot_info(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<CopilotInfo> {
    let mut tx = db.begin().await?;
    let record = sqlx::query!(
        "SELECT ai_resource, code_completion_enabled FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting ai_resource and code_completion_enabled: {e:#}")))?;
    tx.commit().await?;

    let (ai_provider, exists_ai_resource) = if let Some(ai_resource) = record.ai_resource {
        let ai_resource = serde_json::from_value::<AiResource>(ai_resource);
        let exist = ai_resource.is_ok();
        (
            if exist {
                ai_resource.unwrap().provider
            } else {
                "".to_string()
            },
            exist,
        )
    } else {
        ("".to_string(), false)
    };

    Ok(Json(CopilotInfo {
        ai_provider,
        exists_ai_resource,
        code_completion_enabled: record.code_completion_enabled,
    }))
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
                .map_err(|err| Error::InternalErr(err.to_string()))?;

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

    Ok(format!(
        "Edit large file storage config for workspace {}",
        &w_id
    ))
}

#[derive(Deserialize)]
pub struct EditGitSyncConfig {
    #[cfg(feature = "enterprise")]
    pub git_sync_settings: Option<WorkspaceGitSyncSettings>,
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

    if let Some(git_sync_settings) = new_config.git_sync_settings {
        let serialized_config = serde_json::to_value::<WorkspaceGitSyncSettings>(git_sync_settings)
            .map_err(|err| Error::InternalErr(err.to_string()))?;

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

    Ok(format!("Edit git sync config for workspace {}", &w_id))
}

#[derive(Deserialize)]
struct EditDeployUIConfig {
    #[cfg(feature = "enterprise")]
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
                .map_err(|err| Error::InternalErr(err.to_string()))?;

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
    .map_err(|err| Error::InternalErr(format!("getting default_app: {err}")))?;
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
    .map_err(|err| Error::InternalErr(format!("getting default_app: {err}")))?;
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

    if let Some(error_handler) = &ee.error_handler {
        sqlx::query!(
            "UPDATE workspace_settings SET error_handler = $1, error_handler_extra_args = $2, error_handler_muted_on_cancel = $3 WHERE workspace_id = $4",
            error_handler,
            ee.error_handler_extra_args,
            ee.error_handler_muted_on_cancel.unwrap_or(false),
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET error_handler = NULL, error_handler_extra_args = NULL WHERE workspace_id = $1",
            &w_id,
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
        Some([("error_handler", &format!("{:?}", ee.error_handler)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit error_handler for workspace {}", &w_id))
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
                "INSERT INTO workspace_env (workspace_id, name, value) VALUES ($1, $2, $3) ON CONFLICT (workspace_id, name) DO UPDATE SET value = $3",
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
    require_super_admin(&db, &authed.email).await?;

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
            let decrypted_value = decrypt(&previous_encryption_key, variable.value)?;
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

    return Ok(());
}

#[derive(Serialize)]
struct UsedTriggers {
    pub websocket_used: bool,
    pub http_routes_used: bool,
    pub kafka_used: bool,
}

async fn get_used_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<UsedTriggers> {
    let mut tx = user_db.begin(&authed).await?;
    let websocket_used = sqlx::query_as!(
        UsedTriggers,
        r#"SELECT 
            EXISTS(SELECT 1 FROM websocket_trigger WHERE workspace_id = $1) as "websocket_used!", 
            EXISTS(SELECT 1 FROM http_trigger WHERE workspace_id = $1) as "http_routes_used!",
            EXISTS(SELECT 1 FROM kafka_trigger WHERE workspace_id = $1) as "kafka_used!""#,
        w_id,
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(websocket_used))
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
        "SELECT * FROM workspace LIMIT $1 OFFSET $2",
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
        "SELECT workspace.id, workspace.name, usr.username
     FROM workspace, usr WHERE usr.workspace_id = workspace.id AND usr.email = $1 AND deleted = \
         false",
        email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(WorkspaceList { email, workspaces }))
}

async fn check_name_conflict<'c>(tx: &mut Transaction<'c, Postgres>, w_id: &str) -> Result<()> {
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

    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    check_name_conflict(&mut tx, &nw.id).await?;
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
            (workspace_id)
            VALUES ($1)",
        nw.id
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
    .unwrap_or(false);

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
        "INSERT INTO usr_to_group
            VALUES ($1, 'all', $2)",
        nw.id,
        username
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by, edited_at) VALUES ($1, 'app_themes', 'App Themes', ARRAY[]::TEXT[], '{\"g/all\": false}', $2, now()) ON CONFLICT DO NOTHING",
        nw.id,
        username,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by, edited_at) VALUES ($1, 'app_custom', 'App Custom Components', ARRAY[]::TEXT[], '{\"g/all\": false}', $2, now()) ON CONFLICT DO NOTHING",
        nw.id,
        username,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by, edited_at) VALUES ($1, 'app_groups', 'App Groups', ARRAY[]::TEXT[], '{\"g/all\": false}', $2, now()) ON CONFLICT DO NOTHING",
        nw.id,
        username,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO resource (workspace_id, path, value, description, resource_type, created_by, edited_at) VALUES ($1, 'f/app_themes/theme_0', '{\"name\": \"Default Theme\", \"value\": \"\"}', 'The default app theme', 'app_theme', $2, now()) ON CONFLICT DO NOTHING",
        nw.id,
        username,
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

async fn archive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = true WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.archive",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Archived workspace {}", &w_id))
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
            DO UPDATE SET is_admin = $3, operator = $4",
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
    .unwrap_or(false);

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

#[derive(Deserialize)]
struct ChangeWorkspaceName {
    new_name: String,
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

    Ok(format!("updated workspace name to {}", &rw.new_name))
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

#[cfg(feature = "enterprise")]
pub async fn get_critical_alerts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
    Query(params): Query<crate::utils::AlertQueryParams>,
) -> JsonResult<serde_json::Value> {
    require_admin_or_devops(authed.is_admin, &authed.username, &authed.email, &db).await?;

    crate::utils::get_critical_alerts(db, params, Some(w_id)).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn get_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_critical_alert(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i32)>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin_or_devops(authed.is_admin, &authed.username, &authed.email, &db).await?;
    crate::utils::acknowledge_critical_alert(db, Some(w_id), id).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_critical_alert() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_all_critical_alerts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    crate::utils::acknowledge_all_critical_alerts(db, Some(w_id)).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_all_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
pub struct MuteCriticalAlertRequest {
    pub mute_critical_alerts: Option<bool>,
}

#[cfg(feature = "enterprise")]
async fn mute_critical_alerts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(m_r): Json<MuteCriticalAlertRequest>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mute_alerts = m_r.mute_critical_alerts.unwrap_or(false);

    if mute_alerts {
        sqlx::query!(
            "UPDATE alerts SET acknowledged_workspace = true, acknowledged = true WHERE workspace_id = $1",
            &w_id
        )
    .execute(&db)
    .await?;
    }

    sqlx::query!(
        "UPDATE workspace_settings SET mute_critical_alerts = $1 WHERE workspace_id = $2",
        mute_alerts,
        &w_id
    )
    .execute(&db)
    .await?;

    Ok(format!(
        "Updated mute criticital alert ui settings for workspace: {}",
        &w_id
    ))
}

#[cfg(not(feature = "enterprise"))]
pub async fn mute_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}
