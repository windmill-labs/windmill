/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::Authed,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{Error, JsonResult, Result},
    users::owner_to_token_owner,
    utils::{not_found_if_none, paginate, Pagination},
};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_groups))
        .route("/listnames", get(list_group_names))
        .route("/create", post(create_group))
        .route("/get/:name", get(get_group))
        .route("/update/:name", post(update_group))
        .route("/delete/:name", delete(delete_group))
        .route("/adduser/:name", post(add_user))
        .route("/removeuser/:name", post(remove_user))
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct Group {
    pub workspace_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub extra_perms: serde_json::Value,
}

#[derive(Deserialize)]
pub struct NewGroup {
    pub name: String,
    pub summary: Option<String>,
}

#[derive(Serialize)]
pub struct GroupInfo {
    pub workspace_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub members: Vec<String>,
    pub extra_perms: serde_json::Value,
}

#[derive(Deserialize)]
pub struct EditGroup {
    pub summary: Option<String>,
}

#[derive(Deserialize)]
pub struct Username {
    pub username: String,
}

async fn add_user(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Username { username: user_username }): Json<Username>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;

    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3)",
        &w_id,
        user_username,
        name,
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "group.adduser",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("user", user_username.as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Added {} to group {}", user_username, name))
}

async fn remove_user(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Username { username: user_username }): Json<Username>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;
    if &name == "all" {
        return Err(Error::BadRequest(format!("Cannot delete users from all")));
    }
    sqlx::query_as!(
        Group,
        "DELETE FROM usr_to_group WHERE usr = $1 AND group_ = $2 AND workspace_id = $3",
        user_username,
        name,
        &w_id,
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;
    Ok(format!("Removed {} to group {}", user_username, name))
}
