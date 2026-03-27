/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_common::DB;

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::{
    auth::get_groups_for_user,
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, paginate, Pagination},
};
use windmill_common::{db::UserDB, users::username_to_permissioned_as};

use serde::{Deserialize, Serialize};
use sqlx::{query_scalar, FromRow, Postgres, Transaction};
use windmill_git_sync::handle_deployment_metadata;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_groups))
        .route("/listnames", get(list_group_names))
        .route("/create", post(create_group))
        .route("/get/{name}", get(get_group))
        .route("/update/{name}", post(update_group))
        .route("/delete/{name}", delete(delete_group))
        .route("/adduser/{name}", post(add_user))
        .route("/removeuser/{name}", post(remove_user))
        .route("/is_owner/{name}", get(is_owner))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_igroups))
        .route("/list_with_workspaces", get(list_igroups_with_workspaces))
        .route("/get/{name}", get(get_igroup))
        .route("/create", post(create_igroup))
        .route("/update/{name}", post(update_igroup))
        .route("/delete/{name}", delete(delete_igroup))
        .route("/adduser/{name}", post(add_user_igroup))
        .route("/removeuser/{name}", post(remove_user_igroup))
        .route("/export", get(export_igroups))
        .route("/overwrite", post(overwrite_igroups))
}

/// Normalize group names: replace spaces with underscores and convert to lowercase
/// Used when manually creating groups and SCIM-managed groups
pub fn convert_name(name: &str) -> String {
    name.replace(" ", "_").to_lowercase()
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
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Deserialize)]
pub struct Email {
    pub email: String,
}

async fn list_groups(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<Group>> {
    let (per_page, offset) = paginate(pagination);

    let rows = sqlx::query_as!(
        Group,
        "SELECT * FROM group_ WHERE workspace_id = $1 ORDER BY name asc LIMIT $2 OFFSET $3",
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
    ApiAuthed { username, email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(QueryListGroup { only_member_of }): Query<QueryListGroup>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let rows = if !only_member_of.unwrap_or(false) {
        sqlx::query_scalar!(
            "SELECT name FROM group_ WHERE workspace_id = $1 UNION SELECT name FROM instance_group ORDER BY name asc",
            w_id
        )
        .fetch_all(&db)
        .await?
        .into_iter()
        .filter_map(|x| x)
        .collect()
    } else {
        get_groups_for_user(&w_id, &username, &email, &db).await?
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
    .fetch_one(&mut **tx)
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
    ApiAuthed { username, is_admin, groups, .. }: ApiAuthed,
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

async fn _check_nb_of_groups(db: &DB) -> Result<()> {
    let nb_groups = sqlx::query_scalar!("SELECT COUNT(*) FROM group_ WHERE name != 'all' AND name != 'error_handler' AND name != 'slack' AND name != 'wm_deployers'",)
        .fetch_one(db)
        .await?;
    if nb_groups.unwrap_or(0) >= 3 {
        return Err(Error::BadRequest(
            "You have reached the maximum number of groups (3 outside of native groups 'all', 'slack', 'error_handler' and 'wm_deployers') without an enterprise license"
                .to_string(),
        ));
    }
    return Ok(());
}

async fn create_group(
    authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ng): Json<NewGroup>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    check_name_conflict(&mut tx, &w_id, &ng.name).await?;

    #[cfg(not(feature = "enterprise"))]
    _check_nb_of_groups(&_db).await?;

    sqlx::query!(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ($1, $2, $3, $4)",
        w_id,
        ng.name,
        ng.summary,
        serde_json::json!({username_to_permissioned_as(&authed.username): true})
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        &authed.username,
        ng.name,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "group.create",
        ActionKind::Create,
        &w_id,
        Some(&ng.name.to_string()),
        None,
    )
    .await?;

    log_group_permission_change(&mut *tx, &w_id, &ng.name, &authed.username, "create", None)
        .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &_db,
        &w_id,
        windmill_git_sync::DeployedObject::Group { name: ng.name.clone() },
        Some(format!("Created group '{}'", &ng.name)),
        true,
        None,
    )
    .await?;

    Ok(format!("Created group {}", ng.name))
}

async fn create_igroup(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(ng): Json<NewGroup>,
) -> Result<String> {
    use uuid::Uuid;

    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    let normalized_name = convert_name(&ng.name);

    let id = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO instance_group (name, summary, id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        normalized_name,
        ng.summary,
        id,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "igroup.create",
        ActionKind::Create,
        "global",
        Some(&normalized_name),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!("Created group {}", normalized_name))
}

fn validate_instance_role(role: &Option<String>) -> Result<Option<String>> {
    match role.as_deref() {
        None => Ok(None),
        Some("") | Some("user") => Ok(None),
        Some("devops") => Ok(Some("devops".to_string())),
        Some("superadmin") => Ok(Some("superadmin".to_string())),
        Some(other) => Err(Error::BadRequest(format!(
            "Invalid instance_role '{}'. Must be 'devops', 'superadmin', 'user', or empty to clear",
            other
        ))),
    }
}

/// Compute the highest-precedence instance role from all groups a user belongs to.
/// superadmin > devops > none
pub async fn compute_effective_instance_role(
    email: &str,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Option<String>> {
    let roles = sqlx::query_scalar!(
        "SELECT ig.instance_role FROM email_to_igroup eig
         JOIN instance_group ig ON ig.name = eig.igroup
         WHERE eig.email = $1 AND ig.instance_role IS NOT NULL",
        email
    )
    .fetch_all(&mut **tx)
    .await?;

    let mut highest: Option<String> = None;
    for role in roles.into_iter().flatten() {
        match role.as_str() {
            "superadmin" => return Ok(Some("superadmin".to_string())),
            "devops" => highest = Some("devops".to_string()),
            _ => {}
        }
    }
    Ok(highest)
}

/// Apply computed instance role to password table and invalidate session tokens.
/// Only applies if role_source = 'instance_group' or user has no elevated role.
pub async fn apply_instance_role(
    email: &str,
    role: Option<&str>,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<()> {
    let current = sqlx::query!(
        "SELECT super_admin, devops, role_source FROM password WHERE email = $1",
        email
    )
    .fetch_optional(&mut **tx)
    .await?;

    let current = match current {
        Some(c) => c,
        None => return Ok(()), // user doesn't exist in password table
    };

    // Don't touch manually-set elevated roles — manual always wins
    if current.role_source == "manual" && (current.super_admin || current.devops) {
        return Ok(());
    }

    let (new_super_admin, new_devops) = match role {
        Some("superadmin") => (true, false),
        Some("devops") => (false, true),
        _ => (false, false),
    };

    // Only update if something actually changed
    if current.super_admin == new_super_admin && current.devops == new_devops {
        return Ok(());
    }

    sqlx::query!(
        "UPDATE password SET super_admin = $1, devops = $2, role_source = 'instance_group' WHERE email = $3",
        new_super_admin,
        new_devops,
        email
    )
    .execute(&mut **tx)
    .await?;

    // Invalidate session tokens to force re-login with new privileges
    sqlx::query!(
        "DELETE FROM token WHERE email = $1 AND label = 'session'",
        email
    )
    .execute(&mut **tx)
    .await?;

    // Update super_admin flag on non-session tokens
    sqlx::query!(
        "UPDATE token SET super_admin = $1 WHERE email = $2 AND label != 'session'",
        new_super_admin,
        email
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Recompute and apply roles for all members of a given instance group.
pub async fn propagate_instance_group_roles(
    group_name: &str,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<()> {
    let members = sqlx::query_scalar!(
        "SELECT email FROM email_to_igroup WHERE igroup = $1",
        group_name
    )
    .fetch_all(&mut **tx)
    .await?;

    for email in members {
        let effective_role = compute_effective_instance_role(&email, tx).await?;
        apply_instance_role(&email, effective_role.as_deref(), tx).await?;
    }

    Ok(())
}

#[derive(Deserialize)]
struct IGroupUpdate {
    new_summary: String,
    instance_role: Option<String>,
}

async fn update_igroup(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
    Json(igroup_update): Json<IGroupUpdate>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    let exists_opt = sqlx::query("SELECT 1 FROM instance_group WHERE name = $1")
        .bind(name.clone())
        .fetch_optional(&mut *tx)
        .await?;
    not_found_if_none(exists_opt, "instance_group", name.clone())?;

    let validated_role = validate_instance_role(&igroup_update.instance_role)?;

    // Fetch old role before updating so we can detect changes
    let old_role = if igroup_update.instance_role.is_some() {
        sqlx::query_scalar!(
            "SELECT instance_role FROM instance_group WHERE name = $1",
            &name
        )
        .fetch_one(&mut *tx)
        .await?
    } else {
        None
    };

    sqlx::query("UPDATE instance_group SET summary = $1, instance_role = $2 WHERE name = $3")
        .bind(igroup_update.new_summary)
        .bind(&validated_role)
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    // If instance_role actually changed, propagate to all group members
    if igroup_update.instance_role.is_some() && old_role != validated_role {
        propagate_instance_group_roles(&name, &mut tx).await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "igroup.updated",
        ActionKind::Update,
        "global",
        Some(&name.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!("Updated group {}", name))
}

async fn delete_igroup(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    // Fetch group's instance_role and members before deletion
    let group_role = sqlx::query_scalar!(
        "SELECT instance_role FROM instance_group WHERE name = $1",
        &name
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    let affected_members: Vec<String> = if group_role.is_some() {
        sqlx::query_scalar!("SELECT email FROM email_to_igroup WHERE igroup = $1", &name)
            .fetch_all(&mut *tx)
            .await?
    } else {
        vec![]
    };

    sqlx::query!("DELETE FROM email_to_igroup WHERE igroup = $1", name)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM instance_group WHERE name = $1", name)
        .execute(&mut *tx)
        .await?;

    // Recompute roles for affected members after deletion
    for email in &affected_members {
        let effective_role = compute_effective_instance_role(email, &mut tx).await?;
        apply_instance_role(email, effective_role.as_deref(), &mut tx).await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "igroup.delete",
        ActionKind::Delete,
        "global",
        Some(&name.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!("Deleted group {}", name))
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
    .fetch_optional(&mut **db)
    .await?;
    Ok(group_opt)
}

async fn get_group(
    authed: ApiAuthed,
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
            FROM usr_to_group LEFT JOIN usr ON usr_to_group.usr = usr.username AND usr_to_group.workspace_id = $2
            WHERE group_ = $1 AND usr.workspace_id = $2 AND usr_to_group.workspace_id = $2",
        name,
        w_id
    )
    .fetch_all(&mut *tx)
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
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    if name == "all" {
        return Err(Error::BadRequest(
            "The group 'all' is a special group that contains all users and cannot be deleted"
                .to_string(),
        ));
    }

    if !authed.is_admin {
        require_is_owner(&name, &authed.username, &authed.groups, &w_id, &db).await?;
    }
    not_found_if_none(get_group_opt(&mut tx, &w_id, &name).await?, "Group", &name)?;

    sqlx::query!(
        "DELETE FROM usr_to_group WHERE group_ = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM group_ WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "group.delete",
        ActionKind::Delete,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Group { name: name.clone() },
        Some(format!("Deleted group '{}'", &name)),
        true,
        None,
    )
    .await?;

    Ok(format!("delete group at name {}", name))
}

async fn update_group(
    authed: ApiAuthed,
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

    sqlx::query!(
        "UPDATE group_ SET summary = $1 WHERE name = $2 AND workspace_id = $3",
        eg.summary,
        &name,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "group.edit",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;

    log_group_permission_change(
        &mut *tx,
        &w_id,
        &name,
        &authed.username,
        "update_summary",
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Group { name: name.clone() },
        Some(format!("Updated group '{}'", &name)),
        true,
        None,
    )
    .await?;

    Ok(format!("Edited group {}", name))
}

async fn add_user(
    authed: ApiAuthed,
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

    let result = sqlx::query!(
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        user_username,
        name,
    )
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(format!(
            "{} is already a member of group {}",
            user_username, name
        ));
    }

    audit_log(
        &mut *tx,
        &authed,
        "group.adduser",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("user", user_username.as_str())].into()),
    )
    .await?;

    log_group_permission_change(
        &mut *tx,
        &w_id,
        &name,
        &authed.username,
        "add_member",
        Some(&user_username),
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Group { name: name.clone() },
        Some(format!("Added user to group '{}'", &name)),
        true,
        None,
    )
    .await?;

    Ok(format!("Added {} to group {}", user_username, name))
}

async fn add_user_igroup(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
    Json(Email { email }): Json<Email>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    let group_opt = sqlx::query_scalar!("SELECT name FROM instance_group WHERE name = $1", name)
        .fetch_optional(&mut *tx)
        .await?;

    not_found_if_none(group_opt, "IGroup", &name)?;

    sqlx::query!(
        "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        email,
        name,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "igroup.adduser",
        ActionKind::Update,
        "global",
        Some(&name.to_string()),
        Some([("email", email.as_str())].into()),
    )
    .await?;

    // Sync user to workspaces configured with this instance group
    #[cfg(all(feature = "private", feature = "enterprise"))]
    {
        use windmill_api_workspaces::workspaces_ee::auto_add_user;
        let workspaces = sqlx::query!(
            r#"
            SELECT workspace_id, auto_invite->'instance_groups_roles' as instance_groups_roles
            FROM workspace_settings
            WHERE auto_invite->'instance_groups' ? $1
            "#,
            &name
        )
        .fetch_all(&mut *tx)
        .await?;
        for ws in workspaces {
            let role = ws
                .instance_groups_roles
                .and_then(|r| r.get(&name).and_then(|v| v.as_str().map(String::from)))
                .unwrap_or_else(|| "developer".to_string());
            let (is_admin, is_operator) = match role.as_str() {
                "admin" => (true, false),
                "operator" => (false, true),
                _ => (false, false),
            };
            auto_add_user(
                &email,
                &ws.workspace_id,
                &is_operator,
                &mut tx,
                &authed,
                Some(serde_json::json!({"source": "instance_group", "group": &name})),
            )
            .await?;
            if is_admin {
                sqlx::query!(
                    "UPDATE usr SET is_admin = true WHERE workspace_id = $1 AND email = $2",
                    &ws.workspace_id,
                    &email
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    // Apply instance-level role from group membership
    let effective_role = compute_effective_instance_role(&email, &mut tx).await?;
    apply_instance_role(&email, effective_role.as_deref(), &mut tx).await?;

    tx.commit().await?;
    Ok(format!("Added {} to igroup {}", email, name))
}

#[derive(Serialize)]
struct IGroup {
    name: String,
    summary: Option<String>,
    emails: Option<Vec<String>>,
    instance_role: Option<String>,
}

#[derive(Serialize)]
struct IGroupWithWorkspaces {
    name: String,
    summary: Option<String>,
    emails: Option<Vec<String>>,
    instance_role: Option<String>,
    workspaces: Vec<WorkspaceInfo>,
}

#[derive(Serialize, Clone)]
struct WorkspaceInfo {
    workspace_id: String,
    workspace_name: String,
    role: String,
}
async fn list_igroups(Extension(db): Extension<DB>) -> JsonResult<Vec<IGroup>> {
    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    let groups = sqlx::query_as!(
        IGroup,
        "SELECT name, summary, array_remove(array_agg(email_to_igroup.email), null) as emails, instance_role FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup GROUP BY name, instance_role"
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    return Ok(Json(groups));
}

async fn list_igroups_with_workspaces(
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<IGroupWithWorkspaces>> {
    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    // Get all instance groups with their emails first
    let groups = sqlx::query_as!(
        IGroup,
        "SELECT name, summary, array_remove(array_agg(email_to_igroup.email), null) as emails, instance_role FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup GROUP BY name, summary, instance_role"
    )
    .fetch_all(&mut *tx)
    .await?;

    // Get all workspace mappings for instance groups in a single query
    let workspace_mappings = sqlx::query!(
        r#"
        SELECT
            ig.name as group_name,
            ws.workspace_id,
            w.name as workspace_name,
            ws.auto_invite->'instance_groups_roles'->ig.name as role
        FROM instance_group ig
        INNER JOIN workspace_settings ws ON ws.auto_invite->'instance_groups' IS NOT NULL
            AND ws.auto_invite->'instance_groups' ? ig.name
        INNER JOIN workspace w ON w.id = ws.workspace_id AND w.deleted = false
        ORDER BY ig.name, ws.workspace_id
        "#
    )
    .fetch_all(&mut *tx)
    .await?;

    // Create a map of group_name -> Vec<WorkspaceInfo>
    let mut workspaces_by_group: std::collections::HashMap<String, Vec<WorkspaceInfo>> =
        std::collections::HashMap::new();
    for mapping in workspace_mappings {
        let role = mapping
            .role
            .and_then(|r| r.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "developer".to_string());

        let workspace_info = WorkspaceInfo {
            workspace_id: mapping.workspace_id.clone(),
            workspace_name: mapping.workspace_name,
            role,
        };

        workspaces_by_group
            .entry(mapping.group_name)
            .or_insert_with(Vec::new)
            .push(workspace_info);
    }

    let mut result = Vec::new();
    for group in groups {
        let workspaces = workspaces_by_group
            .get(&group.name)
            .cloned()
            .unwrap_or_default();

        result.push(IGroupWithWorkspaces {
            name: group.name,
            summary: group.summary,
            emails: group.emails,
            instance_role: group.instance_role,
            workspaces,
        });
    }

    tx.commit().await?;
    return Ok(Json(result));
}

async fn get_igroup(
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
) -> JsonResult<IGroupWithWorkspaces> {
    let group = sqlx::query_as!(
        IGroup,
        "SELECT name, summary, array_remove(array_agg(email_to_igroup.email), null) as emails, instance_role FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup WHERE name = $1 GROUP BY name, instance_role",
        name
    )
    .fetch_optional(&db)
    .await?;
    let group = not_found_if_none(group, "IGroup", &name)?;

    let workspace_mappings = sqlx::query!(
        r#"
        SELECT
            ws.workspace_id,
            w.name as workspace_name,
            ws.auto_invite->'instance_groups_roles'->$1 as role
        FROM workspace_settings ws
        INNER JOIN workspace w ON w.id = ws.workspace_id AND w.deleted = false
        WHERE ws.auto_invite->'instance_groups' ? $1
        ORDER BY ws.workspace_id
        "#,
        &name
    )
    .fetch_all(&db)
    .await?;

    let workspaces: Vec<WorkspaceInfo> = workspace_mappings
        .into_iter()
        .map(|m| WorkspaceInfo {
            workspace_id: m.workspace_id,
            workspace_name: m.workspace_name,
            role: m
                .role
                .and_then(|r| r.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "developer".to_string()),
        })
        .collect();

    return Ok(Json(IGroupWithWorkspaces {
        name: group.name,
        summary: group.summary,
        emails: group.emails,
        instance_role: group.instance_role,
        workspaces,
    }));
}

async fn remove_user_igroup(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
    Json(Email { email }): Json<Email>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    let group_opt = sqlx::query_scalar!("SELECT name FROM instance_group WHERE name = $1", name,)
        .fetch_optional(&mut *tx)
        .await?;

    not_found_if_none(group_opt, "IGroup", &name)?;

    sqlx::query!(
        "DELETE FROM email_to_igroup WHERE email = $1 AND igroup = $2",
        email,
        name,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "igroup.removeuser",
        ActionKind::Update,
        "global",
        Some(&name.to_string()),
        Some([("email", email.as_str())].into()),
    )
    .await?;

    // Remove user from workspaces where they were added via this instance group
    #[cfg(all(feature = "private", feature = "enterprise"))]
    {
        use windmill_api_workspaces::workspaces_ee::remove_users_from_instance_group_workspaces;
        remove_users_from_instance_group_workspaces(&email, &name, &mut tx).await?;
    }

    // Recompute instance-level role after group removal
    let effective_role = compute_effective_instance_role(&email, &mut tx).await?;
    apply_instance_role(&email, effective_role.as_deref(), &mut tx).await?;

    tx.commit().await?;
    Ok(format!("Removed {} from igroup {}", email, name))
}

async fn remove_user(
    authed: ApiAuthed,
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
    sqlx::query!(
        "DELETE FROM usr_to_group WHERE usr = $1 AND group_ = $2 AND workspace_id = $3",
        user_username,
        name,
        &w_id,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "group.removeuser",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("user", user_username.as_str())].into()),
    )
    .await?;

    log_group_permission_change(
        &mut *tx,
        &w_id,
        &name,
        &authed.username,
        "remove_member",
        Some(&user_username),
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::Group { name: name.clone() },
        Some(format!("Removed user from group '{}'", &name)),
        true,
        None,
    )
    .await?;

    Ok(format!("Removed {} to group {}", user_username, name))
}

#[cfg(feature = "enterprise")]
#[derive(Serialize, Deserialize)]
struct ExportedIGroup {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scim_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emails: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance_role: Option<String>,
}

#[cfg(feature = "enterprise")]
async fn export_igroups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<ExportedIGroup>> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;
    let igroups = sqlx::query_as!(
        ExportedIGroup,
        "SELECT name, summary, array_remove(array_agg(email_to_igroup.email), null) as emails, id, scim_display_name, external_id, instance_role FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup GROUP BY name",
    ).fetch_all(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "igroups.export",
        ActionKind::Execute,
        "global",
        None,
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(igroups))
}

#[cfg(not(feature = "enterprise"))]
async fn export_igroups() -> JsonResult<String> {
    Err(Error::BadRequest(
        "This feature is only available in the enterprise version".to_string(),
    ))
}

#[cfg(feature = "enterprise")]
async fn overwrite_igroups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(igroups): Json<Vec<ExportedIGroup>>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    sqlx::query!("DELETE FROM email_to_igroup")
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM instance_group")
        .execute(&mut *tx)
        .await?;

    for igroup in igroups.iter() {
        let validated_role = validate_instance_role(&igroup.instance_role)?;
        sqlx::query!(
            "INSERT INTO instance_group (name, summary, id, scim_display_name, external_id, instance_role) VALUES ($1, $2, $3, $4, $5, $6)",
            igroup.name,
            igroup.summary,
            igroup.id,
            igroup.scim_display_name,
            igroup.external_id,
            validated_role,
        )
        .execute(&mut *tx)
        .await?;

        if let Some(emails) = &igroup.emails {
            for email in emails.iter() {
                sqlx::query!(
                    "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2)",
                    email,
                    igroup.name,
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    // Propagate instance roles for all groups that have one
    for igroup in igroups.iter() {
        if igroup.instance_role.is_some() {
            propagate_instance_group_roles(&igroup.name, &mut tx).await?;
        }
    }

    // Demote orphaned users: those whose role was set by a group that no longer
    // grants them any instance_role after the import
    let orphaned_users = sqlx::query_scalar!(
        "SELECT email FROM password
         WHERE role_source = 'instance_group' AND (super_admin = true OR devops = true)
         AND email NOT IN (
             SELECT eig.email FROM email_to_igroup eig
             JOIN instance_group ig ON ig.name = eig.igroup
             WHERE ig.instance_role IS NOT NULL
         )"
    )
    .fetch_all(&mut *tx)
    .await?;

    for email in &orphaned_users {
        apply_instance_role(email, None, &mut tx).await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "igroups.import",
        ActionKind::Create,
        "global",
        None,
        None,
    )
    .await?;

    tx.commit().await?;
    Ok("Imported igroups".to_string())
}

#[cfg(not(feature = "enterprise"))]
async fn overwrite_igroups() -> JsonResult<String> {
    Err(Error::BadRequest(
        "This feature is only available in the enterprise version".to_string(),
    ))
}

pub async fn log_group_permission_change<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    group_name: &str,
    changed_by: &str,
    change_type: &str,
    member_affected: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO group_permission_history
         (workspace_id, group_name, changed_by, change_type, member_affected)
         VALUES ($1, $2, $3, $4, $5)",
        workspace_id,
        group_name,
        changed_by,
        change_type,
        member_affected
    )
    .execute(db)
    .await?;
    Ok(())
}
