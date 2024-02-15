/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use crate::db::ApiAuthed;

use crate::{
    db::DB,
    users::{AuthCache, Tokened},
    webhook_util::{WebhookMessage, WebhookShared},
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use lazy_static::lazy_static;
use regex::Regex;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, JsonResult, Result},
    users::username_to_permissioned_as,
    utils::{not_found_if_none, paginate, Pagination},
};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_folders))
        .route("/listnames", get(list_foldernames))
        .route("/create", post(create_folder))
        .route("/get/:name", get(get_folder))
        .route("/update/:name", post(update_folder))
        .route("/getusage/:name", get(get_folder_usage))
        .route("/delete/:name", delete(delete_folder))
        .route("/addowner/:name", post(add_owner))
        .route("/removeowner/:name", post(remove_owner))
        .route("/is_owner/*path", get(is_owner_api))
}

#[derive(FromRow, Serialize, Deserialize, Clone)]
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
pub struct UpdateFolder {
    pub display_name: Option<String>,
    pub owners: Option<Vec<String>>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct Owner {
    pub owner: String,
}

async fn list_folders(
    authed: ApiAuthed,
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
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(rows))
}
async fn list_foldernames(
    authed: ApiAuthed,
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
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn check_name_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    name: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM folder WHERE name = $1 AND workspace_id = $2)",
        name,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Folder {} already exists",
            name
        )));
    }
    return Ok(());
}

lazy_static! {
    static ref VALID_FOLDER_NAME: Regex = Regex::new(r#"^[a-zA-Z_0-9]+$"#).unwrap();
}

async fn create_folder(
    authed: ApiAuthed,
    Tokened { token }: Tokened,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(cache): Extension<Arc<AuthCache>>,
    Path(w_id): Path<String>,
    Json(ng): Json<NewFolder>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    if !VALID_FOLDER_NAME.is_match(&ng.name) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Folder name can only contain alphanumeric characters, underscores"
        )));
    }
    check_name_conflict(&mut tx, &w_id, &ng.name).await?;
    cache.invalidate(&w_id, token).await;
    let owner = username_to_permissioned_as(&authed.username);
    let owners = ng.owners.unwrap_or_else(|| vec![owner.clone()]);
    let owners = if owners.contains(&owner) {
        owners.clone()
    } else {
        owners
            .iter()
            .cloned()
            .chain(std::iter::once(owner))
            .collect()
    };

    let mut extra_perms = ng
        .extra_perms
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    if extra_perms.is_object() {
        let extra_mut = extra_perms.as_object_mut().unwrap();
        for o in &owners {
            extra_mut.insert(o.clone(), serde_json::json!(true));
        }
    } else {
        return Err(error::Error::BadRequest(
            "extra_perms must be an object".to_string(),
        ));
    }

    sqlx::query_as!(
        Folder,
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES ($1, $2, $3, $4, $5)",
        w_id,
        ng.name,
        ng.display_name.unwrap_or(ng.name.clone()),
        &owners,
        extra_perms,
    )
    .execute(&mut *tx)
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Folder { path: format!("f/{}", ng.name) },
        Some(format!("Folder '{}' created", ng.name)),
        rsmq,
        true,
    )
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "folder.create",
        ActionKind::Create,
        &w_id,
        Some(&ng.name.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateFolder { workspace: w_id, name: ng.name.clone() },
    );

    Ok(format!("Created folder {}", ng.name))
}

pub async fn is_owner_api(
    authed: ApiAuthed,
    Path((_w_id, name)): Path<(String, String)>,
) -> JsonResult<bool> {
    Ok(Json(is_owner(&authed, &name)))
}

pub fn is_owner(ApiAuthed { is_admin, folders, .. }: &ApiAuthed, name: &str) -> bool {
    if *is_admin {
        true
    } else {
        folders.into_iter().any(|x| x.0 == name && x.2)
    }
}

pub fn require_is_owner(authed: &ApiAuthed, name: &str) -> Result<()> {
    if is_owner(authed, name) {
        Ok(())
    } else {
        Err(windmill_common::error::Error::NotAuthorized(format!(
            "You are not owner of the folder {}",
            name
        )))
    }
}

async fn update_folder(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
    Json(ng): Json<UpdateFolder>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let mut sqlb = SqlBuilder::update_table("folder");
    sqlb.and_where_eq("name", "?".bind(&name));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(display_name) = ng.display_name {
        sqlb.set("display_name", "?".bind(&display_name));
    }
    if let Some(owners) = ng.owners {
        sqlb.set(
            "owners",
            "?".bind(&format!(
                "{{{}}}",
                owners
                    .iter()
                    .map(|x| format!("\"{x}\""))
                    .collect::<Vec<_>>()
                    .join(","),
            )),
        );
    }
    if let Some(extra_perms) = ng.extra_perms {
        sqlb.set(
            "extra_perms",
            "?".bind(&serde_json::to_string(&extra_perms).map_err(to_anyhow)?),
        );
    }

    sqlb.returning("*");

    let mut tx = user_db.begin(&authed).await?;

    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::InternalErr(e.to_string()))?;
    let nfolder = sqlx::query_as::<_, Folder>(&sql)
        .fetch_one(&mut *tx)
        .await?;

    if let Some(extra_perms) = nfolder.extra_perms.as_object() {
        for o in nfolder.owners {
            if !extra_perms
                .get(&o)
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
            {
                return Err(windmill_common::error::Error::BadRequest(format!(
                    "Owner {} would not have permission to write to folder and that is an invalid state",
                    o
                )));
            }
        }
    }

    audit_log(
        &mut *tx,
        &authed.username,
        "folder.update",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    webhook.send_message(
        w_id.clone().clone(),
        WebhookMessage::UpdateFolder { workspace: w_id, name: name.to_owned() },
    );

    Ok(format!("Updated folder {}", name))
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
    .fetch_optional(&mut **db)
    .await?;
    Ok(folderopt)
}

async fn get_folder(
    authed: ApiAuthed,
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
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<FolderUsage> {
    let mut tx = user_db.begin(&authed).await?;

    let scripts = sqlx::query_scalar!(
        "SELECT count(path) FROM script WHERE path LIKE 'f/' || $1 || '%' AND archived IS false AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    let schedules = sqlx::query_scalar!(
        "SELECT count(path) FROM schedule WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    let flows = sqlx::query_scalar!(
        "SELECT count(path) FROM flow WHERE path LIKE 'f/' || $1 || '%' AND archived IS false AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    let apps = sqlx::query_scalar!(
        "SELECT count(path) FROM app WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    let raw_apps = sqlx::query_scalar!(
        "SELECT count(path) FROM raw_app WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    let resources = sqlx::query_scalar!(
        "SELECT count(path) FROM resource WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    let variables = sqlx::query_scalar!(
        "SELECT count(path) FROM variable WHERE path LIKE 'f/' || $1 || '%'  AND workspace_id = $2",
        name,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    tx.commit().await?;
    Ok(Json(FolderUsage {
        scripts,
        flows,
        schedules,
        apps: apps + raw_apps,
        resources,
        variables,
    }))
}

async fn delete_folder(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    sqlx::query!(
        "DELETE FROM folder WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed.username,
        "folder.delete",
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
        DeployedObject::Folder { path: format!("f/{}", name) },
        Some(format!("Folder '{}' deleted", name)),
        rsmq,
        true,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteFolder { workspace: w_id, name: name.clone() },
    );

    Ok(format!("delete folder at name {}", name))
}

async fn add_owner(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Owner { owner }): Json<Owner>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;
    require_is_owner(&authed, &name)?;

    sqlx::query!(
        "UPDATE folder SET owners = array_append(owners::text[], $1) WHERE name = $2 AND workspace_id = $3 AND NOT $1 = ANY(owners) RETURNING name",
        owner,
        name,
        &w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "folder.add_owner",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("owner", owner.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateFolder { workspace: w_id, name: name.clone() },
    );

    Ok(format!("Added {} to folder {}", owner, name))
}

pub async fn get_folders_for_user(
    w_id: &str,
    username: &str,
    groups: &[String],
    db: &DB,
) -> Result<Vec<(String, bool, bool)>> {
    let mut perms = groups
        .into_iter()
        .map(|x| format!("g/{}", x))
        .collect::<Vec<_>>();
    perms.insert(0, format!("u/{}", username));
    let folders = sqlx::query!(
        "SELECT name, (EXISTS (SELECT 1 FROM (SELECT key, value FROM jsonb_each_text(extra_perms) WHERE key = ANY($1)) t  WHERE value::boolean IS true)) as write, $1 && owners::text[] as owner  FROM folder
        WHERE extra_perms ?| $1  AND workspace_id = $2",
        &perms[..],
        w_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|x| (x.name, x.write.unwrap_or(false), x.owner.unwrap_or(false)))
    .collect();

    Ok(folders)
}

async fn remove_owner(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Owner { owner }): Json<Owner>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;
    require_is_owner(&authed, &name)?;

    sqlx::query!(
        "UPDATE folder SET owners = array_remove(owners, $1::varchar) WHERE name = $2 AND workspace_id = $3 RETURNING name",
        owner,
        name,
        &w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "folder.remove_owner",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("owner", owner.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateFolder { workspace: w_id, name: name.clone() },
    );

    Ok(format!("Removed {} to folder {}", owner, name))
}
