/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::users::SUPERADMIN_SYNC_EMAIL;
use windmill_common::workspaces::{ObjectType, WorkspaceGitSyncSettings};

use regex::Regex;
use serde_json::json;
use windmill_common::error::{Error, Result};
use windmill_common::jobs::JobPayload;
use windmill_common::scripts::ScriptHash;
use windmill_queue::PushIsolationLevel;

pub type DB = Pool<Postgres>;

#[derive(Clone)]
pub enum DeployedObject {
    Script { hash: ScriptHash, path: String, parent_path: Option<String> },
    Flow { path: String, parent_path: Option<String> },
    App { path: String, version: i64, parent_path: Option<String> },
    Folder { path: String },
    Resource { path: String, parent_path: Option<String> },
    Variable { path: String, parent_path: Option<String> },
    Schedule { path: String },
    ResourceType { path: String },
}

impl DeployedObject {
    pub fn get_path(&self) -> &str {
        match self {
            DeployedObject::Script { path, .. } => path,
            DeployedObject::Flow { path, .. } => path,
            DeployedObject::App { path, .. } => path,
            DeployedObject::Folder { path, .. } => path,
            DeployedObject::Resource { path, .. } => path,
            DeployedObject::Variable { path, .. } => path,
            DeployedObject::Schedule { path, .. } => path,
            DeployedObject::ResourceType { path, .. } => path,
        }
    }

    pub fn get_parent_path(&self) -> Option<String> {
        match self {
            DeployedObject::Script { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Flow { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::App { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Folder { .. } => None,
            DeployedObject::Resource { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Variable { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Schedule { .. } => None,
            DeployedObject::ResourceType { .. } => None,
        }
    }
}

pub async fn handle_deployment_metadata<'c, R: rsmq_async::RsmqConnection + Send + Clone + 'c>(
    email: &str,
    created_by: &str,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
    deployment_message: Option<String>,
    rsmq: Option<R>,
    skip_db_insert: bool,
) -> Result<()> {
    let workspace_git_repo_setting = sqlx::query_scalar!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|err| {
        Error::BadRequest(format!(
            "No workspace settings found for workspace ID: {} - Error: {}",
            w_id,
            err.to_string()
        ))
    })?;

    let workspace_git_sync_settings = workspace_git_repo_setting
        .map(|conf| serde_json::from_value::<WorkspaceGitSyncSettings>(conf).ok())
        .flatten()
        .unwrap_or_default();

    let obj_path = if one_regexp_match(
        obj.get_path(),
        workspace_git_sync_settings.include_path.clone(),
    ) {
        Some(obj.get_path())
    } else {
        None
    };
    let obj_parent_path = if obj
        .get_parent_path()
        .map(|p| one_regexp_match(p.as_str(), workspace_git_sync_settings.include_path))
        .unwrap_or(false)
    {
        obj.get_parent_path()
    } else {
        None
    };

    let skip_git_sync = if obj_path.is_none() && obj_parent_path.is_none() {
        tracing::debug!(
            "Ignoring {} from git sync as it's in a private user folder",
            obj.get_path()
        );
        true
    } else {
        false
    };
    tracing::debug!("Skipping git sync for {:?} -> {}", obj_path, skip_git_sync);

    let mut git_sync_job_uuids: Vec<Uuid> = vec![];
    if !skip_git_sync {
        for workspace_git_repo in workspace_git_sync_settings.repositories {
            // check that this type of object should indeed be synced
            let skip_object_type = match obj {
                DeployedObject::Script { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Script)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::Script)
                }
                DeployedObject::Flow { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Flow)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::Flow)
                }
                DeployedObject::App { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::App)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::App)
                }
                DeployedObject::Folder { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Folder)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::Folder)
                }
                DeployedObject::Resource { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Resource)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::Resource)
                }
                DeployedObject::Variable { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Variable)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::Variable)
                }
                DeployedObject::ResourceType { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::ResourceType)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::ResourceType)
                }
                DeployedObject::Schedule { .. } => {
                    !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Schedule)
                        || workspace_git_repo
                            .exclude_types_override
                            .unwrap_or_default()
                            .iter()
                            .any(|element| *element == ObjectType::Schedule)
                }
            };

            if skip_object_type {
                tracing::debug!("Skipping sync due to objec type filter");
                continue;
            }

            // if yes, sync it
            let mut args: HashMap<String, serde_json::Value> = HashMap::new();
            args.insert(
                "repo_url_resource_path".to_string(),
                json!(workspace_git_repo
                    .git_repo_resource_path
                    .strip_prefix("$res:")),
            );

            if let Some(path) = obj_path {
                args.insert("path".to_string(), json!(path));
            }
            if let Some(parent_path) = obj_parent_path.clone() {
                args.insert("parent_path".to_string(), json!(parent_path));
            }
            args.insert("workspace_id".to_string(), json!(w_id));

            let message = match obj.clone() {
                DeployedObject::Script { path, .. } => {
                    args.insert("path_type".to_string(), json!("script"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Script '{}' deployed", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::Flow { path, .. } => {
                    args.insert("path_type".to_string(), json!("flow"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Flow '{}' deployed", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::App { path, .. } => {
                    args.insert("path_type".to_string(), json!("app"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("App '{}' deployed", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::Folder { path, .. } => {
                    args.insert("path_type".to_string(), json!("folder"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Folder '{}' updated", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::Resource { path, .. } => {
                    args.insert("path_type".to_string(), json!("resource"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Resource '{}' updated", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::Variable { path, .. } => {
                    args.insert("path_type".to_string(), json!("variable"));
                    let skip_secret = !workspace_git_sync_settings
                        .include_type
                        .iter()
                        .any(|element| *element == ObjectType::Secret);
                    args.insert("skip_secret".to_string(), json!(skip_secret));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Variable '{}' updated", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::ResourceType { path, .. } => {
                    args.insert("path_type".to_string(), json!("resourcetype"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Resource Type '{}' updated", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
                DeployedObject::Schedule { path, .. } => {
                    args.insert("path_type".to_string(), json!("schedule"));
                    if deployment_message.as_ref().is_none()
                        || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                    {
                        format!("Schedule '{}' updated", path)
                    } else {
                        deployment_message.clone().unwrap()
                    }
                }
            };

            args.insert("commit_msg".to_string(), json!(message));
            args.insert(
                "use_individual_branch".to_string(),
                json!(workspace_git_repo.use_individual_branch.unwrap_or(false)),
            );

            let tx: PushIsolationLevel<'_, R> =
                PushIsolationLevel::IsolatedRoot(db.clone(), rsmq.clone());

            let (job_uuid, new_tx) = windmill_queue::push(
                db,
                tx,
                &w_id,
                JobPayload::DeploymentCallback { path: workspace_git_repo.script_path.clone() },
                args,
                created_by,
                &email,
                SUPERADMIN_SYNC_EMAIL.to_string(),
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
            )
            .await?;
            new_tx.commit().await?;

            git_sync_job_uuids.push(job_uuid);
        }
    }

    // We're not persisting the default commit msg as it's pretty useless. We will persist the ones manually set by users
    if !skip_db_insert && (deployment_message.is_some() || git_sync_job_uuids.len() > 0) {
        // if the git sync job hasn't been triggered, and there is not custom deployment message, there's not point adding an entry to the table
        match obj.clone() {
            DeployedObject::Script { path, hash, .. } => {
                sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, script_hash, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)",
                     w_id, path, hash.0, &git_sync_job_uuids, deployment_message,
                 ).execute(db)
                 .await?;
            }
            DeployedObject::Flow { path, .. } => {
                sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path) WHERE script_hash IS NULL AND app_version IS NULL DO UPDATE SET callback_job_ids = $3, deployment_msg = $4",
                     w_id, path, &git_sync_job_uuids, deployment_message,
                 ).execute(db)
                 .await?;
            }
            DeployedObject::App { path, version, .. } => {
                sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, app_version, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)",
                     w_id, path, version, &git_sync_job_uuids, deployment_message,
                 ).execute(db)
                 .await?;
            }
            DeployedObject::Folder { .. } => (),
            DeployedObject::Resource { .. } => (),
            DeployedObject::Variable { .. } => (),
            DeployedObject::ResourceType { .. } => (),
            DeployedObject::Schedule { .. } => (),
        }
    }

    return Ok(());
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
    // this is annoying b/c we want to replace ** with [a-zA-Z0-9_.*/]* AND THEN * with [a-zA-Z0-9_.*]* - but b/c the
    // first replacement string contains itself '*', we can't do it naively. So, the hack is to use a different
    // character in the replacement string, instead of '*' we use here '%', and finally we replace all '%' with '*'
    let mut regexp = user_regexp
        .replace("**", "[a-zA-Z0-9_.%/]%")
        .replace("*", "[a-zA-Z0-9_.%]%");
    regexp = regexp.replace("%", "*");
    // Then we add ^ at the beginning and '$' at the end to match the entire string, not just a substring
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
