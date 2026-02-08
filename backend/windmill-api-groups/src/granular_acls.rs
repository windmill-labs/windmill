/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use windmill_api_auth::require_owner_of_path;
use windmill_common::DB;
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

use windmill_api_auth::ApiAuthed;

use serde::{Deserialize, Serialize};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};

const KINDS: [&str; 18] = [
    "script",
    "group_",
    "resource",
    "schedule",
    "variable",
    "flow",
    "folder",
    "app",
    "raw_app",
    "http_trigger",
    "websocket_trigger",
    "kafka_trigger",
    "nats_trigger",
    "postgres_trigger",
    "mqtt_trigger",
    "gcp_trigger",
    "sqs_trigger",
    "email_trigger",
];

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/get/*path", get(get_granular_acls))
        .route("/add/*path", post(add_granular_acl))
        .route("/remove/*path", post(remove_granular_acl))
}

#[derive(Serialize, Deserialize)]
pub struct GranularAcl {
    pub owner: String,
    pub write: Option<bool>,
}

async fn add_granular_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(GranularAcl { owner, write }): Json<GranularAcl>,
) -> Result<String> {
    let path = path.to_path();

    let (kind, path) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path or kind".to_string()))?;

    if !KINDS.contains(&kind) {
        return Err(Error::BadRequest("Invalid kind".to_string()));
    }

    let mut tx = user_db.begin(&authed).await?;

    let identifier = if kind == "group_" || kind == "folder" {
        "name"
    } else {
        "path"
    };

    if !authed.is_admin {
        if kind == "folder" {
            crate::folders::require_is_owner(&authed, path)?;
        } else if kind == "group_" {
            crate::groups::require_is_owner(path, &authed.username, &authed.groups, &w_id, &db)
                .await?;
        } else {
            require_owner_of_path(&authed, path)?;
        }
    }

    if kind == "folder" {
        if let Some(obj) = sqlx::query_scalar!(
            "SELECT owners FROM folder WHERE name = $1 AND workspace_id = $2",
            path,
            w_id
        )
        .fetch_optional(&mut *tx)
        .await?
        {
            if obj.contains(&owner) {
                if write != Some(true) {
                    return Err(Error::BadRequest(
                        "Cannot remove write permission for folder owner".to_string(),
                    ));
                }
            }
        }
    }

    let obj_o = sqlx::query_scalar::<_, serde_json::Value>(&format!(
        "UPDATE {kind} SET extra_perms = jsonb_set(extra_perms, $1, to_jsonb($2), \
         true) WHERE {identifier} = $3 AND workspace_id = $4 RETURNING extra_perms"
    ))
    .bind(vec![owner.clone()])
    .bind(write.unwrap_or(false))
    .bind(path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    let _ = not_found_if_none(obj_o, &kind, &path)?;

    if kind == "folder" {
        let change_type = if write.unwrap_or(false) {
            "grant_read"
        } else {
            "grant_write"
        };
        crate::folders::log_folder_permission_change(
            &mut *tx,
            &w_id,
            path,
            &authed.username,
            change_type,
            Some(&owner),
        )
        .await?;
    } else if kind == "group_" {
        let change_type = if write.unwrap_or(false) {
            "grant_admin"
        } else {
            "grant_member_only"
        };
        crate::groups::log_group_permission_change(
            &mut *tx,
            &w_id,
            path,
            &authed.username,
            change_type,
            Some(&owner),
        )
        .await?;
    }

    tx.commit().await?;

    match kind {
        "folder" => {
            handle_deployment_metadata(
                &authed.email,
                &authed.username,
                &db,
                &w_id,
                DeployedObject::Folder { path: format!("f/{}", path) },
                Some(format!("Folder '{}' changed permissions", path)),
                true,
                None,
            )
            .await?
        }
        // "app" => {
        //     handle_deployment_metadata(
        //         &authed.email,
        //         &authed.username,
        //         &db,
        //         &w_id,
        //         DeployedObject::App { path: path.to_string(), parent_path: None, version: 0 },
        //         Some(format!("App '{}' changed permissions", path)),
        //         //         true,
        //     )
        //     .await?
        // }
        // "script" => {
        //     handle_deployment_metadata(
        //         &authed.email,
        //         &authed.username,
        //         &db,
        //         &w_id,
        //         DeployedObject::Script {
        //             path: path.to_string(),
        //             parent_path: None,
        //             hash: ScriptHash(0),
        //         },
        //         Some(format!("Script '{}' changed permissions", path)),
        //         //         true,
        //     )
        //     .await?
        // }
        // "flow" => {
        //     handle_deployment_metadata(
        //         &authed.email,
        //         &authed.username,
        //         &db,
        //         &w_id,
        //         DeployedObject::Flow { path: path.to_string(), parent_path: None },
        //         Some(format!("Flow '{}' changed permissions", path)),
        //         //         true,
        //     )
        //     .await?
        // }
        _ => (),
    }

    Ok("Successfully modified granular acl".to_string())
}

async fn remove_granular_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(GranularAcl { owner, .. }): Json<GranularAcl>,
) -> Result<String> {
    let path = path.to_path();

    let (kind, path) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path or kind".to_string()))?;

    if !KINDS.contains(&kind) {
        return Err(Error::BadRequest("Invalid kind".to_string()));
    }

    if !authed.is_admin {
        if kind == "folder" {
            crate::folders::require_is_owner(&authed, path)?;
        } else if kind == "group_" {
            crate::groups::require_is_owner(path, &authed.username, &authed.groups, &w_id, &db)
                .await?;
        } else {
            require_owner_of_path(&authed, path)?;
        }
    }

    let mut tx = user_db.begin(&authed).await?;

    let identifier = if kind == "group_" || kind == "folder" {
        "name"
    } else {
        "path"
    };

    if identifier == "path" {
        require_owner_of_path(&authed, path)?;
    }

    let obj_o = sqlx::query_scalar::<_, bool>(&format!(
        "WITH old AS (
            SELECT extra_perms->$1 as old_write FROM {kind}
            WHERE {identifier} = $2 AND workspace_id = $3 AND extra_perms ? $1
        )
        UPDATE {kind} SET extra_perms = extra_perms - $1
        WHERE {identifier} = $2 AND workspace_id = $3 AND extra_perms ? $1
        RETURNING (SELECT old_write FROM old)::bool"
    ))
    .bind(&owner)
    .bind(path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    // Only log if something was actually removed (obj_o is Some)
    if let Some(write) = obj_o {
        // Log permission changes for folders and groups
        if kind == "folder" {
            let change_type = if write { "revoke_write" } else { "revoke_read" };
            crate::folders::log_folder_permission_change(
                &mut *tx,
                &w_id,
                path,
                &authed.username,
                change_type,
                Some(&owner),
            )
            .await?;
        } else if kind == "group_" {
            crate::groups::log_group_permission_change(
                &mut *tx,
                &w_id,
                path,
                &authed.username,
                "revoke_admin",
                Some(&owner),
            )
            .await?;
        }

        tx.commit().await?;

        match kind {
            "folder" => {
                handle_deployment_metadata(
                    &authed.email,
                    &authed.username,
                    &db,
                    &w_id,
                    DeployedObject::Folder { path: format!("f/{}", path) },
                    Some(format!("Folder '{}' changed permissions", path)),
                    true,
                    None,
                )
                .await?
            }
            // "app" => {
            //     handle_deployment_metadata(
            //         &authed.email,
            //         &authed.username,
            //         &db,
            //         &w_id,
            //         DeployedObject::App { path: path.to_string(), parent_path: None, version: 0 },
            //         Some(format!("App '{}' changed permissions", path)),
            //         //         true,
            //     )
            //     .await?
            // }
            // "script" => {
            //     handle_deployment_metadata(
            //         &authed.email,
            //         &authed.username,
            //         &db,
            //         &w_id,
            //         DeployedObject::Script {
            //             path: path.to_string(),
            //             parent_path: None,
            //             hash: ScriptHash(0),
            //         },
            //         Some(format!("Script '{}' changed permissions", path)),
            //         //         true,
            //     )
            //     .await?
            // }
            // "flow" => {
            //     handle_deployment_metadata(
            //         &authed.email,
            //         &authed.username,
            //         &db,
            //         &w_id,
            //         DeployedObject::Flow { path: path.to_string(), parent_path: None },
            //         Some(format!("Flow '{}' changed permissions", path)),
            //         //         true,
            //     )
            //     .await?
            // }
            _ => (),
        }
    }

    Ok("Successfully removed granular acl".to_string())
}

async fn get_granular_acls(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<serde_json::Value> {
    let path = path.to_path();
    let (kind, path) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path or kind".to_string()))?;

    if !KINDS.contains(&kind) {
        return Err(Error::BadRequest("Invalid kind".to_string()));
    }

    let mut tx = user_db.begin(&authed).await?;

    let identifier = if kind == "group_" { "name" } else { "path" };
    let obj_o = sqlx::query_scalar::<_, serde_json::Value>(&format!(
        "SELECT extra_perms from {kind} WHERE {identifier} = $1 AND workspace_id = $2"
    ))
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut *tx)
    .await?;

    let obj = not_found_if_none(obj_o, &kind, &path)?;
    tx.commit().await?;

    Ok(Json(obj))
}
