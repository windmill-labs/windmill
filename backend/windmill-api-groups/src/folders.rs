/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use lazy_static::lazy_static;
use regex::Regex;
use windmill_api_auth::{check_scopes, ApiAuthed, AuthCache, Tokened};
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::{error::Error, webhook::{WebhookMessage, WebhookShared}, workspaces::{check_user_against_rule, ProtectionRuleKind, RuleCheckResult}};
use windmill_common::DB;
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
        .route("/exists/:name", get(exists_folder))
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
    pub summary: Option<String>,
    pub created_by: Option<String>,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct NewFolder {
    pub name: String,
    pub summary: Option<String>,
    pub display_name: Option<String>,
    pub owners: Option<Vec<String>>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct UpdateFolder {
    pub summary: Option<String>,
    pub display_name: Option<String>,
    pub owners: Option<Vec<String>>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct Owner {
    pub owner: String,
    pub write: Option<bool>,
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
        "SELECT workspace_id, name, display_name, owners, extra_perms, summary, created_by, edited_at FROM folder WHERE workspace_id = $1 ORDER BY name asc LIMIT $2 OFFSET $3",
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
        "SELECT name FROM folder WHERE workspace_id = $1 ORDER BY name asc LIMIT $2 OFFSET $3",
        w_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

fn validate_owner(owner: &str) -> Result<()> {
    if !owner
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '/' || c == '-')
    {
        return Err(error::Error::BadRequest(
            "Invalid owner: must contain only alphanumeric characters, underscores, hyphens, or slashes".to_string(),
        ));
    }
    Ok(())
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
    Extension(cache): Extension<Arc<AuthCache>>,
    Path(w_id): Path<String>,
    Json(ng): Json<NewFolder>,
) -> Result<String> {
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = user_db.clone().begin(&authed).await?;

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

    if let Err(e) =
    sqlx::query_as!(
        Folder,
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, created_by, edited_at) VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
        w_id,
        ng.name,
        ng.display_name.unwrap_or(ng.name.clone()),
        &owners,
        extra_perms,
        ng.summary,
        authed.username
    )
    .execute(&mut *tx)
    .await {
        drop(tx);
        let mut tx = user_db.begin(&authed).await?;

        let exists_for_user = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM folder WHERE name = $1 AND workspace_id = $2 AND $3 = ANY(owners))",
            ng.name,
            w_id,
            authed.username
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM folder WHERE name = $1 AND workspace_id = $2)",
            ng.name,
            w_id
        )
        .fetch_one(&db)
        .await?
        .unwrap_or(false);
        if !exists_for_user && exists {
            return Err(windmill_common::error::Error::BadRequest(format!(
                "Folder '{}' already exists in workspace '{}' but you do not have permission to read to it", ng.name, w_id
            )));
        }  else if exists {
            return Err(windmill_common::error::Error::BadRequest(format!(
                "Folder '{}' already exists in workspace '{}'", ng.name, w_id
            )));
        } else {
            return Err(windmill_common::error::Error::InternalErr(format!(
                "Failed to create folder: {}", e
            )));
        }
    }

    audit_log(
        &mut *tx,
        &authed,
        "folder.create",
        ActionKind::Create,
        &w_id,
        Some(&ng.name.to_string()),
        None,
    )
    .await?;

    log_folder_permission_change(&mut *tx, &w_id, &ng.name, &authed.username, "create", None)
        .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Folder { path: format!("f/{}", ng.name) },
        Some(format!("Folder '{}' created", ng.name)),
        true,
        None,
    )
    .await?;

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

use windmill_api_auth::is_owner;
pub use windmill_api_auth::require_is_owner;

async fn update_folder(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
    Json(mut ng): Json<UpdateFolder>,
) -> Result<String> {
    use sql_builder::prelude::*;

    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut sqlb = SqlBuilder::update_table("folder");
    sqlb.and_where_eq("name", "?".bind(&name));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(display_name) = ng.display_name {
        sqlb.set("display_name", "?".bind(&display_name));
    }

    if let Some(summary) = ng.summary {
        sqlb.set("summary", "?".bind(&summary));
    }

    sqlb.set("edited_at", "now()");

    // Track whether permission-related fields are being updated
    let owners_changed = ng.owners.is_some();
    let extra_perms_changed = ng.extra_perms.is_some();

    if !authed.is_admin {
        let prefixed_username = format!("u/{}", authed.username);
        if ng.owners.as_ref().is_some_and(|x| {
            !x.contains(&prefixed_username)
                && !authed.groups.iter().any(|g| x.contains(&format!("g/{g}")))
        }) {
            ng.owners.as_mut().unwrap().push(prefixed_username.clone());
            if ng.extra_perms.is_none() {
                ng.extra_perms = Some(serde_json::Value::Object(serde_json::Map::new()));
            }
            ng.extra_perms
                .as_mut()
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(prefixed_username, serde_json::json!(true));
        }
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
        if !extra_perms.is_object() {
            return Err(windmill_common::error::Error::BadRequest(format!(
                "extra_perms must be an object, received {}",
                extra_perms.to_string()
            )));
        }
        sqlb.set(
            "extra_perms",
            "?".bind(&serde_json::to_string(&extra_perms).map_err(to_anyhow)?),
        );
    }

    sqlb.returning("*");

    let mut tx = user_db.begin(&authed).await?;

    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::internal_err(e.to_string()))?;
    let nfolder = sqlx::query_as::<_, Folder>(&sql)
        .fetch_optional(&mut *tx)
        .await?;

    let nfolder = nfolder.ok_or_else(|| {
        windmill_common::error::Error::NotAuthorized(format!(
            "You are not an owner of {} and hence cannot modify it",
            name
        ))
    })?;

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
        &authed,
        "folder.update",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        None,
    )
    .await?;

    // Log permission changes if owners or extra_perms were updated
    if owners_changed {
        log_folder_permission_change(
            &mut *tx,
            &w_id,
            &name,
            &authed.username,
            "update_owners",
            None,
        )
        .await?;
    }
    if extra_perms_changed {
        log_folder_permission_change(&mut *tx, &w_id, &name, &authed.username, "update_acl", None)
            .await?;
    }

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Folder { path: format!("f/{}", name) },
        Some(format!("Folder '{}' updated", name)),
        true,
        None,
    )
    .await?;

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
        "SELECT workspace_id, name, display_name, owners, extra_perms, summary, created_by, edited_at FROM folder WHERE name = $1 AND workspace_id = $2",
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
    check_scopes(&authed, || format!("folders:read:f/{}", name))?;
    let mut tx = user_db.begin(&authed).await?;

    let folder = not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    tx.commit().await?;
    Ok(Json(folder))
}

async fn exists_folder(
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM folder WHERE name = $1 AND workspace_id = $2)",
        name,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
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
    check_scopes(&authed, || format!("folders:read:f/{}", name))?;
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
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;

    let del = sqlx::query_scalar!(
        "DELETE FROM folder WHERE name = $1 AND workspace_id = $2 RETURNING 1",
        name,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    if del.is_none() {
        return Err(windmill_common::error::Error::NotAuthorized(format!(
            "Not authorized to delete folder {}",
            name
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
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
        true,
        None,
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
    Json(Owner { owner, .. }): Json<Owner>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;
    require_is_owner(&authed, &name)?;

    sqlx::query!(
        "UPDATE folder SET owners = array_append(owners::text[], $1) WHERE name = $2 AND workspace_id = $3 AND NOT $1 = ANY(owners) RETURNING name",
        owner,
        &name,
        &w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    validate_owner(&owner)?;
    sqlx::query(&format!(
        "UPDATE folder SET extra_perms = jsonb_set(extra_perms, '{{\"{owner}\"}}', to_jsonb($1), \
         true) WHERE name = $2 AND workspace_id = $3 RETURNING extra_perms"
    ))
    .bind(true)
    .bind(&name)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "folder.add_owner",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("owner", owner.as_str())].into()),
    )
    .await?;

    log_folder_permission_change(
        &mut *tx,
        &w_id,
        &name,
        &authed.username,
        "grant_admin",
        Some(&owner),
    )
    .await?;

    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateFolder { workspace: w_id, name: name.clone() },
    );

    Ok(format!("Added {} to folder {}", owner, name))
}

async fn remove_owner(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, name)): Path<(String, String)>,
    Json(Owner { owner, write }): Json<Owner>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    not_found_if_none(get_folderopt(&mut tx, &w_id, &name).await?, "Folder", &name)?;
    require_is_owner(&authed, &name)?;
    validate_owner(&owner)?;

    let folder = sqlx::query!(
        "UPDATE folder SET owners = array_remove(owners, $1::varchar) WHERE name = $2 AND workspace_id = $3 AND $1 = ANY(owners) RETURNING name",
        owner,
        &name,
        &w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if folder.is_none() && write.is_none() {
        return Ok(format!(
            "Owner {} is already not a member of folder {}",
            owner, name
        ));
    }

    if let Some(write) = write {
        let old_write = sqlx::query_scalar::<_, Option<bool>>(&format!(
            "UPDATE folder SET extra_perms = jsonb_set(extra_perms, '{{\"{owner}\"}}', to_jsonb($1), \
             true) FROM (SELECT (extra_perms->>'{owner}')::boolean as old_val FROM folder WHERE name = $2 AND workspace_id = $3) old \
             WHERE name = $2 AND workspace_id = $3 RETURNING old.old_val"
        ))
        .bind(write)
        .bind(&name)
        .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        if folder.is_none() && old_write.is_none_or(|ow| ow == write) {
            return Ok(format!(
                "Owner {} is already not a member of folder {} and write permission was already {}",
                owner, name, write
            ));
        }
    }

    audit_log(
        &mut *tx,
        &authed,
        "folder.remove_owner",
        ActionKind::Update,
        &w_id,
        Some(&name.to_string()),
        Some([("owner", owner.as_str())].into()),
    )
    .await?;

    let change_type = match write {
        Some(true) => "grant_writer_only",
        Some(false) => "grant_viewer_only",
        None => "revoke_all",
    };
    log_folder_permission_change(
        &mut *tx,
        &w_id,
        &name,
        &authed.username,
        change_type,
        Some(&owner),
    )
    .await?;

    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateFolder { workspace: w_id, name: name.clone() },
    );

    Ok(format!("Removed {} to folder {}", owner, name))
}

pub async fn log_folder_permission_change<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    folder_name: &str,
    changed_by: &str,
    change_type: &str,
    affected: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO folder_permission_history
         (workspace_id, folder_name, changed_by, change_type, affected)
         VALUES ($1, $2, $3, $4, $5)",
        workspace_id,
        folder_name,
        changed_by,
        change_type,
        affected
    )
    .execute(db)
    .await?;
    Ok(())
}
