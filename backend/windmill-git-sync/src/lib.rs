/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use sqlx::{Pool, Postgres};
use windmill_common::workspaces::WorkspaceGitRepo;

use serde_json::json;
use windmill_common::error::{Error, Result};
use windmill_common::jobs::JobPayload;
use windmill_common::scripts::ScriptHash;
use windmill_queue::PushIsolationLevel;

pub type DB = Pool<Postgres>;

#[derive(Clone)]
pub enum DeployedObject {
    Script { hash: ScriptHash, path: String },
    Flow { path: String },
    App { path: String, version: i64 },
}

impl DeployedObject {
    pub fn get_path(&self) -> &str {
        match self {
            DeployedObject::Script { path, .. } => path,
            DeployedObject::Flow { path } => path,
            DeployedObject::App { path, .. } => path,
        }
    }
}

pub async fn handle_deployment_metadata<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    email: &str,
    created_by: &str,
    permissioned_as: &str,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
    deployment_message: Option<String>,
    rsmq: Option<R>,
) -> Result<()> {
    let skip_git_sync = if obj.get_path().starts_with("u/") {
        tracing::debug!(
            "Ignoring {} from git sync as it's in a private user folder",
            obj.get_path()
        );
        true
    } else {
        false
    };

    let workspace_git_repo_setting = sqlx::query_scalar!(
        "SELECT git_sync FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?;

    if workspace_git_repo_setting.is_none() {
        return Err(Error::InternalErr(
            "No workspace settings found for workspace ID".to_string(),
        ));
    }

    let workspace_git_repo = workspace_git_repo_setting
        .unwrap()
        .map(|conf| serde_json::from_value::<WorkspaceGitRepo>(conf).ok())
        .flatten();

    let git_sync_job_uuid = if !skip_git_sync && workspace_git_repo.is_some() {
        let workspace_git_repo = workspace_git_repo.unwrap();

        let mut args: HashMap<String, serde_json::Value> = HashMap::new();
        args.insert(
            "repo_url_resource_path".to_string(),
            json!(workspace_git_repo
                .git_repo_resource_path
                .strip_prefix("$res:")),
        );

        let message = match obj.clone() {
            DeployedObject::Script { path, .. } => {
                args.insert("path".to_string(), json!(path.to_string()));
                if deployment_message.as_ref().is_none()
                    || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                {
                    format!("Script '{}' deployed", path)
                } else {
                    deployment_message.clone().unwrap()
                }
            }
            DeployedObject::Flow { path } => {
                args.insert("path".to_string(), json!(path.to_string()));
                if deployment_message.as_ref().is_none()
                    || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                {
                    format!("Flow '{}' deployed", path)
                } else {
                    deployment_message.clone().unwrap()
                }
            }
            DeployedObject::App { path, .. } => {
                args.insert("path".to_string(), json!(path.to_string()));
                if deployment_message.as_ref().is_none()
                    || deployment_message.as_ref().is_some_and(|x| x.is_empty())
                {
                    format!("App '{}' deployed", path)
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

        let tx: PushIsolationLevel<'_, R> = PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);

        let (job_uuid, new_tx) = windmill_queue::push(
            db,
            tx,
            &w_id,
            JobPayload::DeploymentCallback { path: workspace_git_repo.script_path.clone() },
            args,
            created_by,
            &email,
            permissioned_as.to_string(),
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
        Some(job_uuid)
    } else {
        None
    };

    // We're not persisting the default commit msg as it's pretty useless. We will persist the ones manually set by users
    let job_uuids = if git_sync_job_uuid.is_some() {
        vec![git_sync_job_uuid.unwrap()]
    } else {
        vec![]
    };
    if deployment_message.is_some() || job_uuids.len() > 0 {
        // if the git sync job hasn't been triggered, and there is not custom deployment message, there's not point adding an entry to the table
        match obj.clone() {
             DeployedObject::Script { path, hash, .. } => {
                 sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, script_hash, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)",
                     w_id, path, hash.0, &job_uuids, deployment_message,
                 )
             },
             DeployedObject::Flow { path } => {
                 sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path) WHERE script_hash IS NULL AND app_version IS NULL DO UPDATE SET callback_job_ids = $3, deployment_msg = $4",
                     w_id, path, &job_uuids, deployment_message,
                 )
             }
             DeployedObject::App { path, version } => {
                 sqlx::query!(
                     "INSERT INTO deployment_metadata (workspace_id, path, app_version, callback_job_ids, deployment_msg) VALUES ($1, $2, $3, $4, $5)",
                     w_id, path, version, &job_uuids, deployment_message,
                 )
             }
         }.execute(db)
         .await?;
    }

    return Ok(());
}
