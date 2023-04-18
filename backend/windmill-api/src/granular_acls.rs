/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::{require_owner_of_path, Authed},
};
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use windmill_common::{
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};

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
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(GranularAcl { owner, write }): Json<GranularAcl>,
) -> Result<String> {
    let path = path.to_path();

    let (kind, path) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path or kind".to_string()))?;
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

    let obj_o = sqlx::query_scalar::<_, serde_json::Value>(&format!(
        "UPDATE {kind} SET extra_perms = jsonb_set(extra_perms, '{{\"{owner}\"}}', to_jsonb($1), \
         true) WHERE {identifier} = $2 AND workspace_id = $3 RETURNING extra_perms"
    ))
    .bind(write.unwrap_or(false))
    .bind(path)
    .bind(&w_id)
    .fetch_optional(&mut tx)
    .await?;

    let _ = not_found_if_none(obj_o, &kind, &path)?;
    tx.commit().await?;

    Ok("Successfully modified granular acl".to_string())
}

async fn remove_granular_acl(
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(GranularAcl { owner, write: _ }): Json<GranularAcl>,
) -> Result<String> {
    let path = path.to_path();

    let (kind, path) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path or kind".to_string()))?;

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

    let obj_o = sqlx::query_scalar::<_, serde_json::Value>(&format!(
        "UPDATE {kind} SET extra_perms = extra_perms - $1 WHERE {identifier} = $2 AND \
         workspace_id = $3 RETURNING extra_perms"
    ))
    .bind(owner)
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;

    let _ = not_found_if_none(obj_o, &kind, &path)?;
    tx.commit().await?;

    Ok("Successfully removed granular acl".to_string())
}

async fn get_granular_acls(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<serde_json::Value> {
    let path = path.to_path();
    let (kind, path) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path or kind".to_string()))?;

    let mut tx = user_db.begin(&authed).await?;

    let identifier = if kind == "group_" { "name" } else { "path" };
    let obj_o = sqlx::query_scalar::<_, serde_json::Value>(&format!(
        "SELECT extra_perms from {kind} WHERE {identifier} = $1 AND workspace_id = $2"
    ))
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;

    let obj = not_found_if_none(obj_o, &kind, &path)?;
    tx.commit().await?;

    Ok(Json(obj))
}
