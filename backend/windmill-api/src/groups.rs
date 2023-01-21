/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::{get_groups_for_user, Authed},
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::users::username_to_permissioned_as;
use windmill_common::{
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, paginate, Pagination},
};

use serde::{Deserialize, Serialize};
use sqlx::{query_scalar, FromRow, Postgres, Transaction};
use windmill_queue::CLOUD_HOSTED;

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
        .route("/is_owner", get(is_owner))
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

async fn list_groups(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<Group>> {
    let (per_page, offset) = paginate(pagination);

    let rows = sqlx::query_as!(
        Group,
        "SELECT * FROM group_ WHERE workspace_id = $1 ORDER BY name desc LIMIT $2 OFFSET $3",
        w_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

#[derive(Deserialize)]
struct QueryListGroup {
    pub only_member_of: Option<bool>,
}
async fn list_group_names(
    Authed { username, .. }: Authed,
    Extension(db): Extension<DB>,
    Query(QueryListGroup { only_member_of }): Query<QueryListGroup>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let rows = if !only_member_of.unwrap_or(false) {
        sqlx::query_scalar!(
            "SELECT name FROM group_ WHERE workspace_id = $1 ORDER BY name desc",
            w_id
        )
        .fetch_all(&db)
        .await?
    } else {
        get_groups_for_user(&w_id, &username, &db).await?
    };

    Ok(Json(rows))
}

async fn check_name_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    name: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM group_ WHERE name = $1 AND workspace_id = $2)",
        name,
        w_id
    )
    .fetch_one(tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Group {} already exists",
            name
        )));
    }
    return Ok(());
}

pub async fn is_owner(
    Authed { username, is_admin, groups, .. }: Authed,
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<bool> {
    if is_admin {
        Ok(Json(true))
    } else {
        Ok(Json(
            require_is_owner(&name, &username, &groups, &w_id, &db)
                .await
                .is_ok(),
        ))
    }
}

pub async fn require_is_owner(
    group_name: &str,
    username: &str,
    groups: &Vec<String>,
    w_id: &str,
    db: &DB,
) -> Result<()> {
    let is_owner = query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM group_ WHERE (group_.extra_perms ->> CONCAT('u/', $1::text))::boolean AND name = $2 AND workspace_id = $4) OR exists(
            SELECT 1 FROM group_ g, jsonb_each_text(g.extra_perms) f 
    WHERE $2 = g.name AND $4 = g.workspace_id AND SPLIT_PART(key, '/', 1) = 'g' AND key = ANY($3::text[])
    AND value::boolean)",
        username,
        group_name,
        groups,
        w_id,
    ).fetch_one(db)
    .await?
    .unwrap_or(false);
    if !is_owner {
        Err(Error::BadRequest(format!(
            "{} is not an owner of {} and hence is not authorized to perform this operation",
            username, group_name
        )))
    } else {
        Ok(())
    }
}

async fn create_group(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ng): Json<NewGroup>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    check_name_conflict(&mut tx, &w_id, &ng.name).await?;

    sqlx::query_as!(
        Group,
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ($1, $2, $3, $4)",
        w_id,
        ng.name,
        ng.summary,
        serde_json::json!({username_to_permissioned_as(&authed.username): true})
    )
    .execute(&mut tx)
    .await?;

    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        &authed.username,
        ng.name,
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "group.create",
        ActionKind::Create,
        &w_id,
        Some(&ng.name.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!("Created group {}", ng.name))
}

pub async fn get_group_opt<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    name: &str,
) -> Result<Option<Group>> {
    let group_opt = sqlx::query_as!(
        Group,
        "SELECT * FROM group_ WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_optional(db)
    .await?;
    Ok(group_opt)
}

async fn get_group(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<GroupInfo> {
    if *CLOUD_HOSTED && w_id == "demo" && name == "all" && !authed.is_admin {
        return Ok(Json(GroupInfo {
            workspace_id: w_id,
            name: name,
            summary: Some("The group that contains all users".to_string()),
            members: vec!["redacted_in_demo_workspace".to_string()],
            extra_perms: serde_json::json!({}),
        }));
    }

    let mut tx = user_db.begin(&authed).await?;

    let group = not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;

    let members = sqlx::query_scalar!(
        "SELECT  usr.username  
            FROM usr_to_group LEFT JOIN usr ON usr_to_group.usr = usr.username 
            WHERE group_ = $1 AND usr.workspace_id = $2 AND usr_to_group.workspace_id = $2",
        name,
        w_id
    )
    .fetch_all(&mut tx)
    .await?;

    tx.commit().await?;
    Ok(Json(GroupInfo {
        workspace_id: group.workspace_id,
        name: group.name,
        summary: group.summary,
        members,
        extra_perms: group.extra_perms,
    }))
}

async fn delete_group(
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    if !authed.is_admin {
        require_is_owner(&name, &authed.username, &authed.groups, &w_id, &db).await?;
    }
    not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;

    sqlx::query!(
        "DELETE FROM usr_to_group WHERE group_ = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut tx)
    .await?;
    sqlx::query!(
        "DELETE FROM group_ WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "group.delete",
        ActionKind::Delete,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("delete group at name {}", name))
}

async fn update_group(
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(eg): Json<EditGroup>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    if !authed.is_admin {
        require_is_owner(&name, &authed.username, &authed.groups, &w_id, &db).await?;
    }
    not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;

    sqlx::query_as!(
        Group,
        "UPDATE group_ SET summary = $1 WHERE name = $2 AND workspace_id = $3",
        eg.summary,
        &name,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "group.edit",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Edited group {}", name))
}

async fn add_user(
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Username { username: user_username }): Json<Username>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    if !authed.is_admin {
        require_is_owner(&name, &authed.username, &authed.groups, &w_id, &db).await?;
    }

    not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;

    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
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
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Username { username: user_username }): Json<Username>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    if !authed.is_admin {
        require_is_owner(&name, &authed.username, &authed.groups, &w_id, &db).await?;
    }

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

    audit_log(
        &mut tx,
        &authed.username,
        "group.removeuser",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("user", user_username.as_str())].into()),
    )
    .await?;

    tx.commit().await?;
    Ok(format!("Removed {} to group {}", user_username, name))
}
