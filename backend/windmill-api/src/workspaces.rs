/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    resources::{Resource, ResourceType},
    users::{Authed, WorkspaceInvite},
    utils::require_super_admin,
};
use axum::{
    body::StreamBody,
    extract::{Extension, Path, Query},
    headers,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{Error, JsonResult, Result},
    flows::Flow,
    scripts::{Schema, Script, ScriptLang},
    utils::{paginate, rd_string, require_admin, Pagination},
    variables::ExportableListableVariable,
};

use hyper::{header, StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tempfile::TempDir;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list_pending_invites", get(list_pending_invites))
        .route("/update", post(edit_workspace))
        .route("/delete", delete(delete_workspace))
        .route("/invite_user", post(invite_user))
        .route("/delete_invite", post(delete_invite))
        .route("/get_settings", get(get_settings))
        .route("/edit_slack_command", post(edit_slack_command))
        .route("/tarball", get(tarball_workspace))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/list_as_superadmin", get(list_workspaces_as_super_admin))
        .route("/list", get(list_workspaces))
        .route("/users", get(user_workspaces))
        .route("/create", post(create_workspace))
        .route("/exists", post(exists_workspace))
        .route("/exists_username", post(exists_username))
}

#[derive(FromRow, Serialize)]
struct Workspace {
    id: String,
    name: String,
    owner: String,
    domain: Option<String>,
    deleted: bool,
    premium: bool,
}

#[derive(FromRow, Serialize, Debug)]
pub struct WorkspaceSettings {
    pub workspace_id: String,
    pub slack_team_id: Option<String>,
    pub slack_name: Option<String>,
    pub slack_command_script: Option<String>,
    pub slack_email: String,
}

#[derive(FromRow, Serialize, Debug)]
pub struct Usage {
    pub workspace_id: String,
    pub slack_team_id: Option<String>,
    pub slack_name: Option<String>,
    pub slack_command_script: Option<String>,
    pub slack_email: String,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "WORKSPACE_KEY_KIND", rename_all = "lowercase")]
pub enum WorkspaceKeyKind {
    Cloud,
}

#[derive(Deserialize)]
struct EditCommandScript {
    slack_command_script: Option<String>,
}
#[derive(Deserialize)]
struct CreateWorkspace {
    id: String,
    name: String,
    username: String,
    domain: Option<String>,
}

#[derive(Deserialize)]
struct EditWorkspace {
    name: String,
    owner: String,
    domain: Option<String>,
}

#[derive(Serialize)]
struct WorkspaceList {
    pub email: String,
    pub workspaces: Vec<UserWorkspace>,
}

#[derive(Serialize)]
struct UserWorkspace {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[derive(Deserialize)]
struct WorkspaceId {
    pub id: String,
}

#[derive(Deserialize)]
struct ValidateUsername {
    pub id: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct NewWorkspaceInvite {
    pub email: String,
    pub is_admin: bool,
}

async fn list_pending_invites(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WorkspaceInvite>> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as!(
        WorkspaceInvite,
        "SELECT * from workspace_invite WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn exists_workspace(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Json(WorkspaceId { id }): Json<WorkspaceId>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace WHERE workspace.id = $1)",
        id
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(false);
    tx.commit().await?;
    Ok(Json(exists))
}

async fn list_workspaces(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Workspace>> {
    let mut tx = user_db.begin(&authed).await?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT workspace.* FROM workspace, usr WHERE usr.workspace_id = workspace.id AND \
         usr.email = $1 AND deleted = false",
        authed.email
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(workspaces))
}

async fn get_settings(
    authed: Authed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<WorkspaceSettings> {
    let mut tx = user_db.begin(&authed).await?;
    let settings = sqlx::query_as!(
        WorkspaceSettings,
        "SELECT * FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting settings: {e}")))?;

    tx.commit().await?;
    Ok(Json(settings))
}

async fn edit_slack_command(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { is_admin, username, .. }: Authed,
    Json(es): Json<EditCommandScript>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace_settings SET slack_command_script = $1 WHERE workspace_id = $2",
        es.slack_command_script,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "workspaces.edit_command_script",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [(
                "script",
                es.slack_command_script
                    .unwrap_or("NO_SCRIPT".to_string())
                    .as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit command script {}", &w_id))
}

async fn list_workspaces_as_super_admin(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Query(pagination): Query<Pagination>,
    Authed { email, .. }: Authed,
) -> JsonResult<Vec<Workspace>> {
    let mut tx = user_db.begin(&authed).await?;
    require_super_admin(&mut tx, &email).await?;
    let (per_page, offset) = paginate(pagination);

    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT * FROM workspace LIMIT $1 OFFSET $2",
        per_page as i32,
        offset as i32
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(workspaces))
}

async fn user_workspaces(
    Extension(db): Extension<DB>,
    Authed { email, .. }: Authed,
) -> JsonResult<WorkspaceList> {
    let mut tx = db.begin().await?;
    let workspaces = sqlx::query_as!(
        UserWorkspace,
        "SELECT workspace.id, workspace.name, usr.username
     FROM workspace, usr WHERE usr.workspace_id = workspace.id AND usr.email = $1 AND deleted = \
         false",
        email
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(WorkspaceList { email, workspaces }))
}

async fn create_workspace(
    authed: Authed,
    Extension(db): Extension<DB>,
    Json(nw): Json<CreateWorkspace>,
) -> Result<String> {
    if &nw.username == "bot" {
        return Err(Error::BadRequest("bot is a reserved username".to_string()));
    }
    let mut tx = db.begin().await?;
    sqlx::query!(
        "INSERT INTO workspace
            (id, name, owner, domain)
            VALUES ($1, $2, $3, $4)",
        nw.id,
        nw.name,
        authed.email,
        nw.domain
    )
    .execute(&mut tx)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_settings
            (workspace_id)
            VALUES ($1)",
        nw.id
    )
    .execute(&mut tx)
    .await?;
    let key = rd_string(64);
    sqlx::query!(
        "INSERT INTO workspace_key
            (workspace_id, kind, key)
            VALUES ($1, 'cloud', $2)",
        nw.id,
        &key
    )
    .execute(&mut tx)
    .await?;

    let mc = magic_crypt::new_magic_crypt!(key, 256);
    sqlx::query!(
        "INSERT INTO variable
            (workspace_id, path, value, is_secret, description)
            VALUES ($1, 'g/all/pretty_secret', $2, true, 'This item is secret'), 
                ($3, 'g/all/not_secret', $4, false, 'This item is not secret')",
        nw.id,
        crate::variables::encrypt(&mc, "pretty secret value"),
        nw.id,
        "finland does not actually exist",
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin)
            VALUES ($1, $2, $3, true)",
        nw.id,
        authed.email,
        nw.username,
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        "INSERT INTO group_
            VALUES ($1, 'all', 'The group that always contains all users of this workspace')",
        nw.id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        "INSERT INTO usr_to_group
            VALUES ($1, 'all', $2)",
        nw.id,
        nw.username
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "workspaces.create",
        ActionKind::Create,
        &nw.id,
        Some(nw.name.as_str()),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Created workspace {}", &nw.id))
}

async fn edit_workspace(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { is_admin, username, .. }: Authed,
    Json(ew): Json<EditWorkspace>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace SET name = $1, owner = $2, domain = $3 WHERE id = $4",
        ew.name,
        ew.owner,
        ew.domain,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "workspaces.update",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [(
                "domain",
                ew.domain.unwrap_or("NO_DOMAIN".to_string()).as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Updated workspace {}", &w_id))
}

async fn delete_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { is_admin, username, email, .. }: Authed,
) -> Result<String> {
    let w_id = match w_id.as_str() {
        "starter" => Err(Error::BadRequest(
            "starter workspace cannot be deleted".to_string(),
        )),
        "admins" => Err(Error::BadRequest(
            "admins workspace cannot be deleted".to_string(),
        )),
        _ => Ok(w_id),
    }?;
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = true WHERE id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    audit_log(
        &mut tx,
        &username,
        "workspaces.delete",
        ActionKind::Update,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Deleted workspace {}", &w_id))
}

async fn invite_user(
    Authed { username, is_admin, .. }: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "INSERT INTO workspace_invite
            (workspace_id, email, is_admin)
            VALUES ($1, $2, $3)",
        &w_id,
        nu.email,
        nu.is_admin
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("user with email {} invited", nu.email),
    ))
}

async fn delete_invite(
    Authed { username, is_admin, .. }: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE
        workspace_id = $1 AND email = $2 AND is_admin = $3",
        &w_id,
        nu.email,
        nu.is_admin
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("invite to email {} deleted", nu.email),
    ))
}

async fn exists_username(
    Extension(db): Extension<DB>,
    Json(vu): Json<ValidateUsername>,
) -> Result<String> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND workspace_id = $2)",
        vu.username,
        vu.id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);

    if exists {
        return Err(Error::BadRequest("username already taken".to_string()));
    }

    Ok("valid username".to_string())
}

#[derive(Serialize)]
struct ScriptMetadata {
    summary: String,
    description: String,
    schema: Option<Schema>,
    is_template: bool,
    lock: Vec<String>,
}

async fn tarball_workspace(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<([(headers::HeaderName, String); 2], impl IntoResponse)> {
    require_admin(authed.is_admin, &authed.username)?;

    let tmp_dir = TempDir::new_in(".")?;

    let name = format!("windmill-{w_id}.tar");
    let file_path = tmp_dir.path().join(&name);
    let file = File::create(&file_path).await?;
    let mut a = tokio_tar::Builder::new(file);

    {
        let scripts = sqlx::query_as::<_, Script>(
            "SELECT * FROM script as o WHERE workspace_id = $1 AND archived = false
            AND created_at = (select max(created_at) from script where path = o.path AND \
             workspace_id = $1)",
        )
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        for script in scripts {
            let ext = match script.language {
                ScriptLang::Python3 => "py",
                ScriptLang::Deno => "ts",
                ScriptLang::Go => "go",
                ScriptLang::Bash => "sh",
            };
            write_to_archive(
                script.content,
                format!("scripts/{}.{}", script.path, ext),
                &mut a,
            )
            .await?;

            let lock = script
                .lock
                .unwrap_or_else(|| "".to_string())
                .lines()
                .map(|x| x.to_string())
                .collect();
            let metadata = ScriptMetadata {
                summary: script.summary,
                description: script.description,
                schema: script.schema,
                is_template: script.is_template,
                lock,
            };
            let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
            write_to_archive(
                metadata_str,
                format!("scripts/{}.script.json", script.path),
                &mut a,
            )
            .await?;
        }
    }

    {
        let resources = sqlx::query_as!(
            Resource,
            "SELECT * FROM resource WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&db)
        .await?;

        for resource in resources {
            let resource_str = serde_json::to_string_pretty(&resource).unwrap();
            write_to_archive(
                resource_str,
                format!("resources/{}.resource.json", resource.path),
                &mut a,
            )
            .await?;
        }
    }

    {
        let resource_types = sqlx::query_as!(
            ResourceType,
            "SELECT * FROM resource_type WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&db)
        .await?;

        for resource_type in resource_types {
            let resource_str = serde_json::to_string_pretty(&resource_type).unwrap();
            write_to_archive(
                resource_str,
                format!("resource_types/{}.resource-type.json", resource_type.name),
                &mut a,
            )
            .await?;
        }
    }

    {
        let flows = sqlx::query_as::<_, Flow>(
            "SELECT * FROM flow WHERE workspace_id = $1 AND archived = false",
        )
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        for flow in flows {
            let flow_str = serde_json::to_string_pretty(&flow).unwrap();
            write_to_archive(flow_str, format!("flows/{}.flow.json", flow.path), &mut a).await?;
        }
    }

    {
        let variables = sqlx::query_as::<_, ExportableListableVariable>(
            "SELECT *, false as is_expired FROM variable WHERE workspace_id = $1 AND is_secret = false",
        )
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        for var in variables {
            let flow_str = serde_json::to_string_pretty(&var).unwrap();
            write_to_archive(
                flow_str,
                format!("variables/{}.variable.json", var.path),
                &mut a,
            )
            .await?;
        }
    }
    a.into_inner().await?;

    let file = tokio::fs::File::open(file_path).await?;

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let headers = [
        (header::CONTENT_TYPE, "application/x-tar".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        ),
    ];

    Ok((headers, body))
}

async fn write_to_archive(
    content: String,
    path: String,
    a: &mut tokio_tar::Builder<File>,
) -> Result<()> {
    let bytes = content.as_bytes();
    let mut header = tokio_tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mode(0o777);
    header.set_cksum();
    a.append_data(&mut header, path, bytes).await?;
    Ok(())
}
