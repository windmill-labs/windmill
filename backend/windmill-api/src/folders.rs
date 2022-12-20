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
    error::{JsonResult, Result},
    users::owner_to_token_owner,
    utils::{not_found_if_none, paginate, Pagination},
};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_folders))
        .route("/listnames", get(list_foldernames))
        .route("/create", post(create_folder))
        .route("/get/:name", get(get_folder))
        .route("/getusage/:name", get(get_folder_usage))
        .route("/delete/:name", delete(delete_folder))
        .route("/addowner/:name", post(add_owner))
        .route("/removeowner/:name", post(remove_owner))
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct Folder {
    pub workspace_id: String,
    pub name: String,
    pub display_name: String,
    pub owners: Vec<String>,
    pub extra_perms: serde_json::Value,
}

#[derive(Deserialize)]
pub struct NewFolder {
    pub name: String,
    pub display_name: Option<String>,
    pub owners: Option<Vec<String>>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct Owner {
    pub owner: String,
}

async fn list_folders(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<Folder>> {
    let (per_page, offset) = paginate(pagination);
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_as!(
        Folder,
        "SELECT workspace_id, name, display_name, owners, extra_perms FROM folder WHERE workspace_id = $1 ORDER BY name desc LIMIT $2 OFFSET $3",
        w_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(Json(rows))
}
async fn list_foldernames(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<String>> {
    let (per_page, offset) = paginate(pagination);
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_scalar!(
        "SELECT name FROM folder WHERE workspace_id = $1 ORDER BY name desc LIMIT $2 OFFSET $3",
        w_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn create_folder(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ng): Json<NewFolder>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let owner = owner_to_token_owner(&authed.username, false);
    sqlx::query_as!(
        Folder,
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES ($1, $2, $3, $4, $5)",
        w_id,
        ng.name,
        ng.display_name.unwrap_or(ng.name.clone()),
        &ng.owners.unwrap_or(vec![owner.clone()]),
        ng.extra_perms.unwrap_or(serde_json::json!({owner: true}))
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "folder.create",
        ActionKind::Create,
        &w_id,
        Some(&ng.name.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!("Created folder {}", ng.name))
}

pub async fn get_folderopt<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    name: &str,
) -> Result<Option<Folder>> {
    let folderopt = sqlx::query_as!(
        Folder,
        "SELECT workspace_id, name, display_name, owners, extra_perms FROM folder WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_optional(db)
    .await?;
    Ok(folderopt)
}

async fn get_folder(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<Folder> {
    let mut tx = user_db.begin(&authed).await?;

    let folder = not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    tx.commit().await?;
    Ok(Json(folder))
}

#[derive(Serialize)]
struct FolderUsage {
    pub scripts: i64,
    pub schedules: i64,
    pub flows: i64,
    pub apps: i64,
    pub resources: i64,
    pub variables: i64,
}
async fn get_folder_usage(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<FolderUsage> {
    let mut tx = user_db.begin(&authed).await?;

    let scripts = sqlx::query_scalar!(
        "SELECT count(path) FROM script WHERE path LIKE 'f/' || $1 || '%' AND archived IS false AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(0);

    let schedules = sqlx::query_scalar!(
        "SELECT count(path) FROM schedule WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(0);

    let flows = sqlx::query_scalar!(
        "SELECT count(path) FROM flow WHERE path LIKE 'f/' || $1 || '%' AND archived IS false AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(0);

    let apps = sqlx::query_scalar!(
        "SELECT count(path) FROM app WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(0);

    let resources = sqlx::query_scalar!(
        "SELECT count(path) FROM resource WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(0);

    let variables = sqlx::query_scalar!(
        "SELECT count(path) FROM variable WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(0);

    tx.commit().await?;
    Ok(Json(FolderUsage {
        scripts,
        flows,
        schedules,
        apps,
        resources,
        variables,
    }))
}

async fn delete_folder(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    sqlx::query!(
        "DELETE FROM folder WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "folder.delete",
        ActionKind::Delete,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("delete folder at name {}", name))
}

async fn add_owner(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Owner { owner }): Json<Owner>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    sqlx::query!(
        "UPDATE folder SET owners = array_append(owners, $1) WHERE name = $2 AND workspace_id = $3 AND NOT $1 = ANY(owners) RETURNING name",
        owner,
        name,
        &w_id,
    )
    .fetch_optional(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "folder.add_owner",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("owner", owner.as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Added {} to folder {}", owner, name))
}

pub async fn get_folders_for_user(
    w_id: &str,
    username: &str,
    groups: &[String],
    db: &DB,
) -> Result<Vec<(String, bool)>> {
    let mut perms = groups
        .into_iter()
        .map(|x| format!("g/{}", x))
        .collect::<Vec<_>>();
    perms.insert(0, format!("u/{}", username));
    let folders = sqlx::query!(
        "SELECT name, (EXISTS (SELECT 1 FROM (SELECT key, value FROM jsonb_each_text(extra_perms) WHERE key = ANY($1)) t  WHERE value::boolean IS true)) as write  FROM folder
        WHERE extra_perms ?| $1  AND workspace_id = $2",
        &perms[..],
        w_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|x| (x.name, x.write.unwrap_or(false)))
    .collect();

    Ok(folders)
}

async fn remove_owner(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Owner { owner }): Json<Owner>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    sqlx::query!(
        "UPDATE folder SET owners = array_remove(owners, $1) WHERE name = $2 AND workspace_id = $3 RETURNING name",
        owner,
        name,
        &w_id,
    )
    .fetch_optional(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "folder.remove_owner",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("owner", owner.as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Removed {} to folder {}", owner, name))
}
