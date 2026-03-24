use std::collections::HashMap;

use regex::Regex;
use serde::Serialize;
use uuid::Uuid;

use windmill_common::error::{Error, Result};
use windmill_common::jobs::JobPayload;
use windmill_common::users::SUPERADMIN_SYNC_EMAIL;
use windmill_common::workspaces::{GitRepositorySettings, ObjectType, WorkspaceGitSyncSettings};
use windmill_queue::PushIsolationLevel;

use crate::{DeployedObject, DB};

fn should_skip_object_type(
    object_type: ObjectType,
    include_type: &Vec<ObjectType>,
    workspace_git_repo: &GitRepositorySettings,
) -> bool {
    !include_type.iter().any(|element| *element == object_type)
        || workspace_git_repo
            .exclude_types_override
            .as_ref()
            .map(|exclude_types| exclude_types.iter().any(|element| *element == object_type))
            .unwrap_or(false)
}

#[derive(Default, Serialize)]
struct SyncItem<'a> {
    path_type: &'a str,
    path: String,
    parent_path: String,
    commit_msg: String,
}

fn insert_path_type_and_return_message<'a>(
    item: &mut SyncItem<'a>,
    deployment_message: &Option<String>,
    path: &str,
    path_type: &'a str,
    label: &str,
) -> String {
    item.path_type = path_type;
    if deployment_message.as_ref().is_none()
        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
    {
        format!("{} '{}' deployed", label, path)
    } else {
        deployment_message.clone().unwrap()
    }
}

fn get_effective_path_filters<'a>(
    repo_settings: &'a Option<windmill_common::workspaces::GitSyncSettings>,
    workspace_settings: &'a WorkspaceGitSyncSettings,
) -> (
    &'a [String],
    &'a Option<Vec<String>>,
    &'a Option<Vec<String>>,
) {
    if let Some(settings) = repo_settings {
        if !settings.include_path.is_empty() {
            (
                &settings.include_path,
                &settings.exclude_path,
                &settings.extra_include_path,
            )
        } else {
            (
                workspace_settings
                    .include_path
                    .as_ref()
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]),
                &workspace_settings.exclude_path,
                &workspace_settings.extra_include_path,
            )
        }
    } else {
        (
            workspace_settings
                .include_path
                .as_ref()
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            &workspace_settings.exclude_path,
            &workspace_settings.extra_include_path,
        )
    }
}

fn get_effective_include_types(
    repo_settings: &Option<windmill_common::workspaces::GitSyncSettings>,
    workspace_settings: &WorkspaceGitSyncSettings,
) -> Vec<ObjectType> {
    if let Some(settings) = repo_settings {
        if !settings.include_type.is_empty() {
            settings.include_type.clone()
        } else {
            workspace_settings.include_type.clone().unwrap_or_default()
        }
    } else {
        workspace_settings.include_type.clone().unwrap_or_default()
    }
}

fn path_matches_filters(
    path: &str,
    include: &[String],
    exclude: &Option<Vec<String>>,
    extra_include: &Option<Vec<String>>,
) -> bool {
    let included = one_regexp_match(path, include.to_vec());
    if !included {
        return false;
    }

    if let Some(ex) = exclude {
        if !ex.is_empty() && one_regexp_match(path, ex.clone()) {
            return false;
        }
    }

    if let Some(extra) = extra_include {
        if !extra.is_empty() {
            return one_regexp_match(path, extra.clone());
        }
    }

    true
}

async fn save_deployment_message(
    db: &DB,
    w_id: &str,
    obj: &DeployedObject,
    deployment_message: Option<String>,
) -> Result<()> {
    match obj {
        DeployedObject::Script { path, hash, .. } => {
            sqlx::query!(
                "INSERT INTO deployment_metadata (workspace_id, path, script_hash, deployment_msg) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (workspace_id, script_hash) WHERE script_hash IS NOT NULL \
                 DO UPDATE SET deployment_msg = EXCLUDED.deployment_msg",
                w_id,
                path,
                hash.0,
                deployment_message,
            )
            .execute(db)
            .await?;
        }
        DeployedObject::Flow { path, version, .. } => {
            sqlx::query!(
                "INSERT INTO deployment_metadata (workspace_id, path, flow_version, deployment_msg) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (workspace_id, path, flow_version) WHERE flow_version IS NOT NULL \
                 DO UPDATE SET deployment_msg = EXCLUDED.deployment_msg",
                w_id,
                path,
                version,
                deployment_message,
            )
            .execute(db)
            .await?;
        }
        DeployedObject::App { path, version, .. }
        | DeployedObject::RawApp { path, version, .. } => {
            sqlx::query!(
                "INSERT INTO deployment_metadata (workspace_id, path, app_version, deployment_msg) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (workspace_id, path, app_version) WHERE app_version IS NOT NULL \
                 DO UPDATE SET deployment_msg = EXCLUDED.deployment_msg",
                w_id,
                path,
                version,
                deployment_message,
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

pub async fn tally_deployed_object_changes(
    w_id: &str,
    obj: &DeployedObject,
    db: &DB,
    renamed_from: Option<&str>,
) -> Result<()> {
    if !matches!(
        obj,
        DeployedObject::Script { .. }
            | DeployedObject::Flow { .. }
            | DeployedObject::App { .. }
            | DeployedObject::RawApp { .. }
            | DeployedObject::Resource { .. }
            | DeployedObject::Variable { .. }
            | DeployedObject::Folder { .. }
            | DeployedObject::ResourceType { .. }
    ) {
        return Ok(());
    }

    let path = obj.get_path();
    let kind = obj.get_kind();

    let deploy_to_workspace = sqlx::query_scalar!(
        "SELECT deploy_to FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten();

    if let Some(deploy_to_w_id) = deploy_to_workspace {
        sqlx::query!("INSERT INTO workspace_diff (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
            VALUES ($1, $2, $3, $4, 1, 0, NULL)
            ON CONFLICT (source_workspace_id, fork_workspace_id, path, kind)
            DO UPDATE SET
                ahead = workspace_diff.ahead + 1,
                has_changes = NULL",
            deploy_to_w_id,
            w_id,
            path,
            kind,
        )
        .execute(db).await?;

        if let Some(renamed_from) = renamed_from {
            sqlx::query!("INSERT INTO workspace_diff (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
                VALUES ($1, $2, $3, $4, 1, 0, NULL)
                ON CONFLICT (source_workspace_id, fork_workspace_id, path, kind)
                DO UPDATE SET
                    ahead = workspace_diff.ahead + 1,
                    has_changes = NULL",
                deploy_to_w_id,
                w_id,
                renamed_from,
                kind,
            )
            .execute(db).await?;
        }
    }

    let forked_workspaces = sqlx::query_scalar!(
        "SELECT id FROM workspace WHERE parent_workspace_id = $1",
        w_id
    )
    .fetch_all(db)
    .await?;

    if forked_workspaces.len() > 0 {
        sqlx::query!("INSERT INTO workspace_diff (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
            SELECT $1, unnest($2::varchar[]), $3, $4, 0, 1, NULL
            ON CONFLICT (source_workspace_id, fork_workspace_id, path, kind)
            DO UPDATE SET
                behind = workspace_diff.behind + 1,
                has_changes = NULL",
            w_id,
            &forked_workspaces,
            path,
            kind
        )
        .execute(db).await?;
        if let Some(renamed_from) = renamed_from {
            sqlx::query!("INSERT INTO workspace_diff (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
                SELECT $1, unnest($2::varchar[]), $3, $4, 0, 1, NULL
                ON CONFLICT (source_workspace_id, fork_workspace_id, path, kind)
                DO UPDATE SET
                    behind = workspace_diff.behind + 1,
                    has_changes = NULL",
                w_id,
                &forked_workspaces,
                renamed_from,
                kind
            )
            .execute(db).await?;
        }
    }

    Ok(())
}

pub async fn handle_deployment_metadata<'c>(
    email: &str,
    created_by: &str,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
    deployment_message: Option<String>,
    skip_db_insert: bool,
    renamed_from: Option<&str>,
) -> Result<()> {
    let db = db.clone();
    let w_id = w_id.to_string();
    let email = email.to_string();
    let created_by = created_by.to_string();
    let deployment_message = deployment_message.clone();
    let obj = obj.clone();
    let renamed_from = renamed_from.map(|s| s.to_string());
    tokio::spawn(async move {
        if let Err(e) =
            tally_deployed_object_changes(&w_id, &obj, &db, renamed_from.as_deref()).await
        {
            tracing::error!(
                "Error tallying changes for {obj:?} on workspace {w_id}: {:?}",
                e
            );
        }
        if let Some(ref old_path) = renamed_from {
            tracing::info!(
                "Detected rename from '{}' to '{}' in workspace {}",
                old_path,
                obj.get_path(),
                w_id
            );
        }
        if let Err(e) = handle_deployment_metadata_inner(
            db,
            w_id.clone(),
            email,
            created_by,
            deployment_message,
            skip_db_insert,
            Some(obj.clone()),
            None,
            false,
            renamed_from,
        )
        .await
        {
            tracing::error!(
                "Error deploying metadata for {obj:?} on workspace {w_id}: {:?}",
                e
            );
        }
    });
    Ok(())
}

#[cfg(not(feature = "private"))]
pub async fn handle_fork_branch_creation<'c>(
    _email: &str,
    _created_by: &str,
    _db: &DB,
    _w_id: &str,
    _fork_workspace_id: &str,
) -> Result<Vec<Uuid>> {
    Ok(vec![])
}

pub async fn handle_deployment_metadata_inner(
    db: sqlx::Pool<sqlx::Postgres>,
    w_id: String,
    email: String,
    created_by: String,
    deployment_message: Option<String>,
    skip_db_insert: bool,
    obj: Option<DeployedObject>,
    created_workspace_fork_id: Option<String>,
    skip_promotion_mode_repos: bool,
    renamed_from: Option<String>,
) -> Result<Vec<Uuid>> {
    let workspace_git_repo_setting = sqlx::query_scalar!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|err| {
        Error::BadRequest(format!(
            "No workspace settings found for workspace ID: {} - Error: {}",
            w_id,
            err.to_string()
        ))
    })?;

    tracing::debug!(
        "workspace_git_repo_setting: {:#?}",
        workspace_git_repo_setting
    );

    let workspace_git_sync_settings = workspace_git_repo_setting
        .map(|conf| serde_json::from_value::<WorkspaceGitSyncSettings>(conf).ok())
        .flatten()
        .unwrap_or_default();

    // Runtime git sync access check
    let user_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM usr WHERE workspace_id = $1 AND disabled = false",
        &w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    let has_ee_license = windmill_common::ee_oss::get_license_plan().await
        == windmill_common::ee_oss::LicensePlan::Enterprise;
    let is_within_free_tier = user_count <= 3;

    if !has_ee_license && !is_within_free_tier {
        // Workspace exceeds free tier — save deployment message but skip git sync
        if !skip_db_insert {
            if let Some(ref o) = obj {
                save_deployment_message(&db, &w_id, o, deployment_message)
                    .await
                    .ok();
            }
        }
        return Ok(vec![]);
    }

    // Check if the object is draft-only (never deployed) and skip git sync if it is
    let is_draft_only = match obj.as_ref() {
        Some(DeployedObject::Script { path, .. }) => {
            sqlx::query_scalar!(
                "SELECT draft_only FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false",
                path,
                w_id
            )
            .fetch_optional(&db)
            .await?
            .flatten()
            .unwrap_or(false)
        }
        Some(DeployedObject::Flow { path, .. }) => {
            sqlx::query_scalar!(
                "SELECT draft_only FROM flow WHERE path = $1 AND workspace_id = $2 AND archived = false",
                path,
                w_id
            )
            .fetch_optional(&db)
            .await?
            .flatten()
            .unwrap_or(false)
        }
        Some(DeployedObject::App { path, .. }) => {
            sqlx::query_scalar!(
                "SELECT draft_only FROM app WHERE path = $1 AND workspace_id = $2",
                path,
                w_id
            )
            .fetch_optional(&db)
            .await?
            .flatten()
            .unwrap_or(false)
        }
        _ => false,
    };

    if is_draft_only {
        tracing::debug!(
            "Skipping git sync for draft-only object: {:?}",
            obj.map(|o| o.get_path()).unwrap_or("None".to_string())
        );
        return Ok(vec![]);
    }

    let mut git_sync_job_uuids: Vec<Uuid> = vec![];
    for (repo_index, workspace_git_repo) in
        workspace_git_sync_settings.repositories.iter().enumerate()
    {
        use windmill_common::{
            min_version::MIN_VERSION_SUPPORTS_SYNC_JOBS_DEBOUNCING,
            runnable_settings::DebouncingSettings,
        };

        // Multi-repo: only allow the first repository in CE
        if !has_ee_license && repo_index > 0 {
            tracing::debug!("Skipping secondary repository (EE feature)");
            continue;
        }

        // Promotion mode: EE only
        if !has_ee_license && workspace_git_repo.use_individual_branch.unwrap_or(false) {
            tracing::debug!("Skipping promotion mode repository (EE feature)");
            continue;
        }

        if skip_promotion_mode_repos && workspace_git_repo.use_individual_branch.unwrap_or(false) {
            continue;
        }
        // Get effective path filters for this repository
        let (inc_path, exc_path, extra_inc_path) =
            get_effective_path_filters(&workspace_git_repo.settings, &workspace_git_sync_settings);

        let obj_and_parent_path_matches = match obj.as_ref() {
            Some(obj) => {
                let path_matches = obj.get_ignore_regex_filter()
                    || path_matches_filters(
                        obj.get_path().as_str(),
                        inc_path,
                        exc_path,
                        extra_inc_path,
                    );

                let parent_path_matches = obj
                    .get_parent_path()
                    .map(|p| path_matches_filters(p.as_str(), inc_path, exc_path, extra_inc_path))
                    .unwrap_or(false);

                tracing::debug!("obj: {:#?}", obj);
                tracing::debug!("path_matches: {:#?}", path_matches);
                tracing::debug!("parent_path_matches: {:#?}", parent_path_matches);

                if !path_matches && !parent_path_matches {
                    tracing::debug!(
                        "Path {:?} does not match repo filters, skipping repo",
                        obj.get_path()
                    );
                    continue;
                }
                Some((path_matches, parent_path_matches, obj))
            }
            None => None,
        };

        // Get effective include types for this repository
        let effective_include_types =
            get_effective_include_types(&workspace_git_repo.settings, &workspace_git_sync_settings);

        if let Some(o) = obj.as_ref() {
            let skip_object_type = match o {
                DeployedObject::Script { .. } => should_skip_object_type(
                    ObjectType::Script,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Flow { .. } => should_skip_object_type(
                    ObjectType::Flow,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::App { .. } | DeployedObject::RawApp { .. } => {
                    should_skip_object_type(
                        ObjectType::App,
                        &effective_include_types,
                        &workspace_git_repo,
                    )
                }
                DeployedObject::Folder { .. } => should_skip_object_type(
                    ObjectType::Folder,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Resource { .. } => should_skip_object_type(
                    ObjectType::Resource,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Variable { .. } => should_skip_object_type(
                    ObjectType::Variable,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::ResourceType { .. } => should_skip_object_type(
                    ObjectType::ResourceType,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Schedule { .. } => should_skip_object_type(
                    ObjectType::Schedule,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::User { .. } => should_skip_object_type(
                    ObjectType::User,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Group { .. } => should_skip_object_type(
                    ObjectType::Group,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::HttpTrigger { .. }
                | DeployedObject::WebsocketTrigger { .. }
                | DeployedObject::KafkaTrigger { .. }
                | DeployedObject::NatsTrigger { .. }
                | DeployedObject::PostgresTrigger { .. }
                | DeployedObject::MqttTrigger { .. }
                | DeployedObject::SqsTrigger { .. }
                | DeployedObject::GcpTrigger { .. }
                | DeployedObject::EmailTrigger { .. } => should_skip_object_type(
                    ObjectType::Trigger,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Settings { .. } => should_skip_object_type(
                    ObjectType::Settings,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::Key { .. } => should_skip_object_type(
                    ObjectType::Key,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
                DeployedObject::WorkspaceDependencies { .. } => should_skip_object_type(
                    ObjectType::WorkspaceDependencies,
                    &effective_include_types,
                    &workspace_git_repo,
                ),
            };

            if skip_object_type {
                tracing::debug!("Skipping sync due to object type filter for this repo");
                continue;
            }

            tracing::debug!("skip_object_type: {:#?}", skip_object_type);
        }

        // continue with job creation
        let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
        args.insert(
            "repo_url_resource_path".to_string(),
            windmill_common::worker::to_raw_value(
                &workspace_git_repo
                    .git_repo_resource_path
                    .strip_prefix("$res:"),
            ),
        );

        let mut item = SyncItem::default();
        if let Some((path_matches, parent_path_matches, obj)) = obj_and_parent_path_matches {
            if path_matches {
                item.path = obj.get_path();
            }
            if parent_path_matches {
                item.parent_path = obj.get_parent_path().unwrap();
            }
        }

        args.insert(
            "workspace_id".to_string(),
            windmill_common::worker::to_raw_value(
                created_workspace_fork_id.as_ref().unwrap_or(&w_id),
            ),
        );

        let parent_workspace_id = if created_workspace_fork_id.is_none() {
            sqlx::query_scalar!(
                "SELECT parent_workspace_id FROM workspace WHERE id = $1",
                w_id
            )
            .fetch_optional(&db)
            .await?
            .flatten()
        } else {
            args.insert(
                "only_create_branch".to_string(),
                windmill_common::worker::to_raw_value(&true),
            );
            Some(w_id.to_string())
        };

        args.insert(
            "parent_workspace_id".to_string(),
            windmill_common::worker::to_raw_value(&parent_workspace_id),
        );

        let message = if let Some(o) = obj.as_ref() {
            match o.clone() {
                DeployedObject::Script { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "script",
                    "Script",
                ),
                DeployedObject::Flow { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "flow",
                    "Flow",
                ),
                DeployedObject::App { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "app",
                    "App",
                ),
                DeployedObject::RawApp { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "raw_app",
                    "Raw App",
                ),
                DeployedObject::Folder { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "folder",
                    "Folder",
                ),
                DeployedObject::Resource { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "resource",
                    "Resource",
                ),
                DeployedObject::Variable { path, .. } => {
                    item.path_type = "variable";
                    let skip_secret = !effective_include_types
                        .iter()
                        .any(|element| *element == ObjectType::Secret);
                    args.insert(
                        "skip_secret".to_string(),
                        windmill_common::worker::to_raw_value(&skip_secret),
                    );
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Variable '{}' updated", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::ResourceType { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "resourcetype",
                    "Resource Type",
                ),
                DeployedObject::Schedule { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "schedule",
                    "Schedule",
                ),
                DeployedObject::User { email } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &email,
                    "user",
                    "User",
                ),
                DeployedObject::Group { name } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &name,
                    "group",
                    "Group",
                ),
                DeployedObject::HttpTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "httptrigger",
                    "HTTP Trigger",
                ),
                DeployedObject::WebsocketTrigger { path, .. } => {
                    insert_path_type_and_return_message(
                        &mut item,
                        &deployment_message,
                        &path,
                        "websockettrigger",
                        "WebSocket Trigger",
                    )
                }
                DeployedObject::KafkaTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "kafkatrigger",
                    "Kafka Trigger",
                ),
                DeployedObject::NatsTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "natstrigger",
                    "NATS Trigger",
                ),
                DeployedObject::PostgresTrigger { path, .. } => {
                    insert_path_type_and_return_message(
                        &mut item,
                        &deployment_message,
                        &path,
                        "postgrestrigger",
                        "Postgres Trigger",
                    )
                }
                DeployedObject::MqttTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "mqtttrigger",
                    "MQTT Trigger",
                ),
                DeployedObject::SqsTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "sqstrigger",
                    "SQS Trigger",
                ),
                DeployedObject::GcpTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "gcptrigger",
                    "GCP Trigger",
                ),
                DeployedObject::EmailTrigger { path, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &path,
                    "emailtrigger",
                    "Email Trigger",
                ),
                DeployedObject::Settings { setting_type, .. } => {
                    insert_path_type_and_return_message(
                        &mut item,
                        &deployment_message,
                        &setting_type,
                        "settings",
                        "Workspace Settings",
                    )
                }
                DeployedObject::Key { key_type, .. } => insert_path_type_and_return_message(
                    &mut item,
                    &deployment_message,
                    &key_type,
                    "key",
                    "Encryption Key",
                ),
                DeployedObject::WorkspaceDependencies { path, .. } => {
                    insert_path_type_and_return_message(
                        &mut item,
                        &deployment_message,
                        &path,
                        "workspace_dependencies",
                        "Workspace Dependencies",
                    )
                }
            }
        } else {
            format!("Creating branch {created_workspace_fork_id:?}")
        };

        item.commit_msg = format!("[WM] {message}");
        if let (Some(ref old_path), Some(ref o)) = (&renamed_from, &obj) {
            if *old_path != o.get_path() {
                item.commit_msg = format!("{} (renamed from '{}')", item.commit_msg, old_path);
            }
        }
        let use_individual_branch = workspace_git_repo.use_individual_branch.unwrap_or(false);

        let debouncing_settings = if !use_individual_branch
            && workspace_git_repo.is_script_meets_min_version(28103)?
            && MIN_VERSION_SUPPORTS_SYNC_JOBS_DEBOUNCING.met().await
        {
            args.insert(
                "items".to_string(),
                windmill_common::worker::to_raw_value(&[item]),
            );
            DebouncingSettings {
                debounce_key: None,
                debounce_delay_s: Some(5),
                max_total_debouncing_time: Some(15),
                max_total_debounces_amount: None,
                debounce_args_to_accumulate: Some(vec!["items".to_owned()]),
            }
        } else {
            if !use_individual_branch {
                tracing::warn!("debouncing is disabled for sync jobs, make sure to have all workers on at least 1.602.0 and have your sync script set to latest.");
            }

            let SyncItem { path_type, path, parent_path, commit_msg } = item;

            let mut insert = |k: &str, v| {
                args.insert(k.to_owned(), windmill_common::worker::to_raw_value(&v));
            };

            insert("path_type", path_type);
            insert("path", &path);
            insert("parent_path", &parent_path);
            insert("commit_msg", &commit_msg);

            Default::default()
        };

        args.insert(
            "use_individual_branch".to_string(),
            windmill_common::worker::to_raw_value(&use_individual_branch),
        );
        args.insert(
            "group_by_folder".to_string(),
            windmill_common::worker::to_raw_value(
                &workspace_git_repo.group_by_folder.unwrap_or(false),
            ),
        );

        args.insert(
            "group_by_folder".to_string(),
            windmill_common::worker::to_raw_value(
                &workspace_git_repo.group_by_folder.unwrap_or(false),
            ),
        );

        if let Some(ref force_branch) = workspace_git_repo.force_branch {
            args.insert(
                "force_branch".to_string(),
                windmill_common::worker::to_raw_value(force_branch),
            );
        }

        let tx = PushIsolationLevel::IsolatedRoot(db.clone());

        let (job_uuid, new_tx) = windmill_queue::push(
            &db,
            tx,
            &w_id,
            JobPayload::DeploymentCallback {
                path: workspace_git_repo.effective_script_path().to_string(),
                debouncing_settings,
            },
            windmill_queue::PushArgs::from(&args),
            &created_by,
            &email,
            SUPERADMIN_SYNC_EMAIL.to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
        )
        .await?;
        new_tx.commit().await?;

        git_sync_job_uuids.push(job_uuid);
    }
    if !skip_db_insert && (deployment_message.is_some() || git_sync_job_uuids.len() > 0) {
        tracing::info!("Saving deployment metadata for {obj:?} on workspace {w_id}, with message: {deployment_message:?} and job uuids: {git_sync_job_uuids:?}");
        if let Some(o) = obj.as_ref() {
            match o {
                DeployedObject::Script { path, hash, .. } => {
                    sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, script_hash, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)
                            ON CONFLICT (workspace_id, script_hash) WHERE script_hash IS NOT NULL DO UPDATE SET callback_job_ids = EXCLUDED.callback_job_ids, deployment_msg = EXCLUDED.deployment_msg",
                     w_id, path, hash.0, &git_sync_job_uuids, deployment_message,
                 ).execute(&db)
                 .await?;
                }
                DeployedObject::Flow { path, version, .. } => {
                    sqlx::query!(
                    "INSERT INTO deployment_metadata (workspace_id, path, flow_version, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)
                            ON CONFLICT (workspace_id, path, flow_version) WHERE flow_version IS NOT NULL DO UPDATE SET callback_job_ids = EXCLUDED.callback_job_ids, deployment_msg = EXCLUDED.deployment_msg",
                    w_id, path, version, &git_sync_job_uuids, deployment_message,
                ).execute(&db)
                .await?;
                }
                DeployedObject::App { path, version, .. }
                | DeployedObject::RawApp { path, version, .. } => {
                    sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, app_version, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)
                            ON CONFLICT (workspace_id, path, app_version) WHERE app_version IS NOT NULL DO UPDATE SET callback_job_ids = EXCLUDED.callback_job_ids, deployment_msg = EXCLUDED.deployment_msg",
                     w_id, path, version, &git_sync_job_uuids, deployment_message,
                 ).execute(&db)
                 .await?;
                }
                DeployedObject::Folder { .. } => (),
                DeployedObject::Resource { .. } => (),
                DeployedObject::Variable { .. } => (),
                DeployedObject::ResourceType { .. } => (),
                DeployedObject::Schedule { .. } => (),
                DeployedObject::User { .. } => (),
                DeployedObject::Group { .. } => (),
                DeployedObject::HttpTrigger { .. } => (),
                DeployedObject::WebsocketTrigger { .. } => (),
                DeployedObject::KafkaTrigger { .. } => (),
                DeployedObject::NatsTrigger { .. } => (),
                DeployedObject::PostgresTrigger { .. } => (),
                DeployedObject::MqttTrigger { .. } => (),
                DeployedObject::SqsTrigger { .. } => (),
                DeployedObject::GcpTrigger { .. } => (),
                DeployedObject::EmailTrigger { .. } => (),
                DeployedObject::Settings { .. } => (),
                DeployedObject::Key { .. } => (),
                DeployedObject::WorkspaceDependencies { .. } => (),
            }
        }
    }
    Ok(git_sync_job_uuids)
}

fn one_regexp_match(path: &str, regexps: Vec<String>) -> bool {
    let has_match = regexps
        .iter()
        .map(|regexp| transform_regexp(regexp.to_owned()))
        .filter(|regexp| regexp.is_some())
        .map(|regexp| regexp.unwrap())
        .any(|regexp| regexp.is_match(path));
    tracing::debug!(
        "{} matches the set of regexps {:?} -> {}",
        path,
        regexps,
        has_match
    );
    return has_match;
}

fn transform_regexp(user_regexp: String) -> Option<Regex> {
    let mut regexp = user_regexp
        .replace("**", "[a-zA-Z0-9_\\-.%/]%")
        .replace("*", "[a-zA-Z0-9_\\-.%]%");
    regexp = regexp.replace("%", "*");
    regexp = if regexp.starts_with("^") {
        regexp
    } else {
        format!("^{}", regexp)
    };
    regexp = if regexp.ends_with("$") {
        regexp
    } else {
        format!("{}$", regexp)
    };
    let compiled_regexp = Regex::new(regexp.as_str()).ok();
    tracing::debug!("Compiled regexp: {:?}", compiled_regexp);
    return compiled_regexp;
}
