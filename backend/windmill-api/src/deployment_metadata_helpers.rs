/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::db::ApiAuthed;
use crate::db::DB;
use crate::workspaces::{WorkspaceGitRepo, WorkspaceSettings};

use serde_json::json;
use windmill_common::error::{Error, Result};
use windmill_common::jobs::JobPayload;
use windmill_common::scripts::ScriptHash;
use windmill_common::users::username_to_permissioned_as;
use windmill_queue::PushIsolationLevel;

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
    mut tx: PushIsolationLevel<'c, R>,
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
    deployment_message: Option<String>,
) -> Result<PushIsolationLevel<'c, R>> {
    let skip_git_sync = if obj.get_path().starts_with("u/") {
        tracing::debug!(
            "Ignoring {} from git sync as it's in a private user folder",
            obj.get_path()
        );
        true
    } else {
        false
    };

    let workspace_git_repo_setting = sqlx::query_as::<_, WorkspaceSettings>(
        "SELECT * FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(&w_id)
    .fetch_optional(db)
    .await?;

    if workspace_git_repo_setting.is_none() {
        return Err(Error::InternalErr(
            "No workspace settings found for workspace ID".to_string(),
        ));
    }

    let workspace_git_repo = workspace_git_repo_setting
        .unwrap()
        .git_sync
        .map(|conf| serde_json::from_value::<WorkspaceGitRepo>(conf).ok())
        .flatten();

    let (git_sync_job_uuid, mut new_tx) = if !skip_git_sync && workspace_git_repo.is_some() {
        let workspace_git_repo = workspace_git_repo.unwrap();

        let mut args: HashMap<String, serde_json::Value> = HashMap::new();
        args.insert(
            "repo_url_resource_path".to_string(),
            json!(workspace_git_repo
                .git_repo_resource_path
                .strip_prefix("$res:")),
        );

        let default_commit_msg: String;
        match obj.clone() {
            DeployedObject::Script { path, .. } => {
                args.insert("path".to_string(), json!(path.to_string()));
                default_commit_msg = format!("Script '{}' deployed", path);
            }
            DeployedObject::Flow { path } => {
                args.insert("path".to_string(), json!(path.to_string()));
                default_commit_msg = format!("Flow '{}' deployed", path);
            }
            DeployedObject::App { path, .. } => {
                args.insert("path".to_string(), json!(path.to_string()));
                default_commit_msg = format!("App '{}' deployed", path);
            }
        }
        args.insert(
            "commit_msg".to_string(),
            json!(deployment_message.clone().unwrap_or(default_commit_msg)),
        );
        args.insert(
            "use_individual_branch".to_string(),
            json!(workspace_git_repo.use_individual_branch.unwrap_or(false)),
        );

        let (job_uuid, new_tx) = windmill_queue::push(
            &db,
            tx,
            &w_id,
            JobPayload::DeploymentCallback { path: workspace_git_repo.script_path.clone() },
            args,
            &authed.username,
            &authed.email,
            username_to_permissioned_as(&authed.username),
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
        (Some(job_uuid), new_tx)
    } else {
        let new_tx = match tx {
            PushIsolationLevel::Isolated(user_db, authed, rsmq) => {
                (rsmq, user_db.begin(&authed).await?).into()
            }
            PushIsolationLevel::IsolatedRoot(db, rsmq) => (rsmq, db.begin().await?).into(),
            PushIsolationLevel::Transaction(tx) => tx,
        };
        (None, new_tx)
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
        }.execute(&mut new_tx)
        .await?;
    }

    tx = PushIsolationLevel::Transaction(new_tx);
    return Ok(tx);
}
