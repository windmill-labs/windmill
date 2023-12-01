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

pub async fn run_workspace_repo_git_callback<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    mut tx: PushIsolationLevel<'c, R>,
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
) -> Result<PushIsolationLevel<'c, R>> {
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

    if workspace_git_repo.is_none() {
        return Ok(tx);
    }
    let workspace_git_repo = workspace_git_repo.unwrap();

    let mut args: HashMap<String, serde_json::Value> = HashMap::new();
    args.insert(
        "repo_url_resource_path".to_string(),
        json!(workspace_git_repo
            .git_repo_resource_path
            .strip_prefix("$res:")),
    );

    let commit_msg: String; // for now auto-generate a commit message
    match obj.clone() {
        DeployedObject::Script { path, .. } => {
            args.insert("path".to_string(), json!(path.to_string()));
            commit_msg = format!("Script '{}' deployed", path); // for now auto-generate a commit message
        }
        DeployedObject::Flow { path } => {
            args.insert("path".to_string(), json!(path.to_string()));
            commit_msg = format!("Flow '{}' deployed", path);
        }
        DeployedObject::App { path, .. } => {
            args.insert("path".to_string(), json!(path.to_string()));
            commit_msg = format!("App '{}' deployed", path);
        }
    }
    args.insert("commit_msg".to_string(), json!(commit_msg));

    let (job_uuid, mut new_tx) = windmill_queue::push(
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

    // We're not persisting the default commit msg as it's pretty useless. We will persist the ones manually set by users
    match obj.clone() {
        DeployedObject::Script { path, hash, .. } => {
            sqlx::query!(
                "INSERT INTO deployment_metadata (workspace_id, path, script_hash, callback_job_ids) VALUES ($1, $2, $3, $4)",
                w_id, path, hash.0, &[job_uuid]
            )
        },
        DeployedObject::Flow { path } => {
            sqlx::query!(
                "INSERT INTO deployment_metadata (workspace_id, path, callback_job_ids) VALUES ($1, $2, $3) ON CONFLICT (workspace_id, path) WHERE script_hash IS NULL AND app_version IS NULL DO UPDATE SET callback_job_ids = $3",
                w_id, path, &[job_uuid]
            )
        }
        DeployedObject::App { path, version } => {
            sqlx::query!(
                "INSERT INTO deployment_metadata (workspace_id, path, app_version, callback_job_ids) VALUES ($1, $2, $3, $4)",
                w_id, path, version, &[job_uuid]
            )
        }
    }.execute(&mut new_tx)
    .await?;

    tx = PushIsolationLevel::Transaction(new_tx);
    return Ok(tx);
}
