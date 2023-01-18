/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use crate::{
    db::{UserDB, DB},
    folders::Folder,
    resources::{Resource, ResourceType},
    users::{Authed, WorkspaceInvite},
    utils::require_super_admin,
    BaseUrl,
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
use sqlx::{FromRow, Postgres, Transaction};
use tempfile::TempDir;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list_pending_invites", get(list_pending_invites))
        .route("/update", post(edit_workspace))
        .route("/archive", post(archive_workspace))
        .route("/invite_user", post(invite_user))
        .route("/add_user", post(add_user))
        .route("/delete_invite", post(delete_invite))
        .route("/get_settings", get(get_settings))
        .route("/edit_slack_command", post(edit_slack_command))
        .route("/edit_auto_invite", post(edit_auto_invite))
        .route("/tarball", get(tarball_workspace))
        .route("/premium_info", get(premium_info))
        .route("/checkout", get(stripe_checkout))
}
pub fn global_service() -> Router {
    Router::new()
        .route("/list_as_superadmin", get(list_workspaces_as_super_admin))
        .route("/list", get(list_workspaces))
        .route("/users", get(user_workspaces))
        .route("/create", post(create_workspace))
        .route("/exists", post(exists_workspace))
        .route("/exists_username", post(exists_username))
        .route("/allowed_domain_auto_invite", get(is_allowed_auto_domain))
        .route("/unarchive/:workspace", post(unarchive_workspace))
        .route("/delete/:workspace", delete(delete_workspace))
}

#[derive(FromRow, Serialize)]
struct Workspace {
    id: String,
    name: String,
    owner: String,
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
    pub auto_invite_domain: Option<String>,
    pub auto_invite_operator: Option<bool>,
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
struct EditAutoInvite {
    operator: Option<bool>,
}

#[derive(Deserialize)]
struct CreateWorkspace {
    id: String,
    name: String,
    username: String,
}

#[derive(Deserialize)]
struct EditWorkspace {
    name: String,
    owner: String,
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
    pub operator: bool,
}

#[derive(Deserialize)]
pub struct NewWorkspaceUser {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub operator: bool,
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

#[derive(Serialize, FromRow)]
pub struct PremiumWorkspaceInfo {
    pub premium: bool,
    pub usage: Option<i32>,
}
async fn premium_info(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<PremiumWorkspaceInfo> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query_as::<_, PremiumWorkspaceInfo>(
        "SELECT premium, usage.usage FROM workspace LEFT JOIN usage ON workspace.id = usage.id AND usage.is_workspace IS true WHERE workspace.id = $1",
    )
    .bind(w_id)
    .fetch_one(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(row))
}

async fn stripe_checkout(authed: Authed, Extension(base_url): Extension<Arc<BaseUrl>>) {
    #[cfg(feature = "enterprise")]
    {
        let client = stripe::Client::new(std::env::var("STRIPE_KEY").expect("STRIPE_KEY"));
        let success_rd = format!(
            "{}/workspace_settings?session={{CHECKOUT_SESSION_ID}}",
            base_url.0
        );
        let failure_rd = format!("{}/workspace_settings", base_url.0);
        let checkout_session = {
            let mut params = stripe::CreateCheckoutSession::new(&failure_rd, &success_rd);
            params.mode = Some(stripe::CheckoutSessionMode::Subscription);
            params.line_items = Some(vec![
                stripe::CreateCheckoutSessionLineItems {
                    quantity: None,
                    price: Some("price_1MQzMHGU3NdFi9eLWFC7IXEv".to_string()),
                    ..Default::default()
                },
                stripe::CreateCheckoutSessionLineItems {
                    quantity: None,
                    price: Some("price_1MR2BZGU3NdFi9eLNRuibxPx".to_string()),
                    ..Default::default()
                },
            ]);
            params.customer_email = Some(&authed.email);
            params.client_reference_id = Some("foo");
            stripe::CheckoutSession::create(&client, params)
                .await
                .unwrap()
        };

        println!(
            "created a {}  at {}",
            checkout_session.payment_status,
            checkout_session.url.unwrap()
        );
    }
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

const BANNED_DOMAINS: &str = include_str!("../banned_domains.txt");

async fn is_allowed_auto_domain(Authed { email, .. }: Authed) -> JsonResult<bool> {
    let domain = email.split('@').last().unwrap();
    return Ok(Json(!BANNED_DOMAINS.contains(domain)));
}

async fn edit_auto_invite(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { is_admin, email, username, .. }: Authed,
    Json(ea): Json<EditAutoInvite>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let domain = email.split('@').last().unwrap();

    let mut tx = db.begin().await?;

    if let Some(operator) = ea.operator {
        if BANNED_DOMAINS.contains(domain) {
            return Err(Error::BadRequest(format!(
                "Domain {} is not allowed",
                domain
            )));
        }

        sqlx::query!(
            "UPDATE workspace_settings SET auto_invite_domain = $1, auto_invite_operator = $2 WHERE workspace_id = $3",
            domain,
            operator,
            &w_id
        )
        .execute(&mut tx)
        .await?;

        sqlx::query!(
            "INSERT INTO workspace_invite
        (workspace_id, email, is_admin, operator)
        SELECT $1::text, email, false, $3 FROM password WHERE email LIKE CONCAT('%', $2::text) AND NOT EXISTS (
            SELECT 1 FROM usr WHERE workspace_id = $1::text AND email = password.email
        )
        ON CONFLICT DO NOTHING",
            &w_id,
            &domain,
            operator
        )
        .execute(&mut tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET auto_invite_domain = NULL, auto_invite_operator = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut tx)
        .await?;
    }
    audit_log(
        &mut tx,
        &authed.username,
        "workspaces.edit_auto_invite_domain",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("operator", &format!("{:?}", ea.operator)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!(
        "Edit auto-invite for workspace {} to {}",
        &w_id, domain
    ))
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

async fn check_name_conflict<'c>(tx: &mut Transaction<'c, Postgres>, w_id: &str) -> Result<()> {
    let exists = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = $1)", w_id)
        .fetch_one(tx)
        .await?
        .unwrap_or(false);
    if exists {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Workspace {} already exists",
            w_id
        )));
    }
    return Ok(());
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
    check_name_conflict(&mut tx, &nw.id).await?;
    sqlx::query!(
        "INSERT INTO workspace
            (id, name, owner)
            VALUES ($1, $2, $3)",
        nw.id,
        nw.name,
        authed.email,
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
        "UPDATE workspace SET name = $1, owner = $2 WHERE id = $3",
        ew.name,
        ew.owner,
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
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Updated workspace {}", &w_id))
}

async fn archive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { is_admin, username, email, .. }: Authed,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = true WHERE id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    audit_log(
        &mut tx,
        &username,
        "workspaces.archive",
        ActionKind::Update,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Archived workspace {}", &w_id))
}

async fn unarchive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { is_admin, username, email, .. }: Authed,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = false WHERE id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    audit_log(
        &mut tx,
        &username,
        "workspaces.unarchive",
        ActionKind::Update,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Unarchived workspace {}", &w_id))
}

async fn delete_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Authed { username, email, .. }: Authed,
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
    let mut tx = db.begin().await?;
    require_super_admin(&mut tx, &email).await?;

    sqlx::query!("DELETE FROM script WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;
    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;
    sqlx::query!("DELETE FROM app WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;
    sqlx::query!("DELETE FROM variable WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;
    sqlx::query!("DELETE FROM resource WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!("DELETE FROM schedule WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!("DELETE FROM completed_job WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!("DELETE FROM usr WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!("DELETE FROM usr_to_group WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!("DELETE FROM folder WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!("DELETE FROM workspace_key WHERE workspace_id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!("DELETE FROM workspace WHERE id = $1", &w_id)
        .execute(&mut tx)
        .await?;

    audit_log(
        &mut tx,
        &username,
        "workspaces.delete",
        ActionKind::Delete,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Deleted workspace {}", &w_id))
}

pub async fn invite_user_to_all_auto_invite_worspaces(db: &DB, email: &str) -> Result<()> {
    let mut tx = db.begin().await?;
    let domain = email.split('@').last().unwrap();
    let workspaces = sqlx::query!(
        "SELECT workspace_id, auto_invite_operator FROM workspace_settings WHERE auto_invite_domain = $1",
        domain
    )
    .fetch_all(&mut tx)
    .await?;
    for r in workspaces {
        sqlx::query!(
            "INSERT INTO workspace_invite
                (workspace_id, email, is_admin, operator)
                VALUES ($1, $2, false, $3)
                ON CONFLICT DO NOTHING",
            r.workspace_id,
            email,
            r.auto_invite_operator
        )
        .execute(&mut tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
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
            (workspace_id, email, is_admin, operator)
            VALUES ($1, $2, $3, $4)",
        &w_id,
        nu.email,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("user with email {} invited", nu.email),
    ))
}

async fn add_user(
    Authed { username, is_admin, .. }: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nu): Json<NewWorkspaceUser>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin, operator)
            VALUES ($1, $2, $3, $4, $5)",
        &w_id,
        nu.email,
        nu.username,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("user with email {} added", nu.email),
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
        workspace_id = $1 AND email = $2 AND is_admin = $3 AND operator = $4",
        &w_id,
        nu.email,
        nu.is_admin,
        nu.operator
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
        let folders = sqlx::query_as::<_, Folder>("SELECT * FROM folder WHERE workspace_id = $1")
            .bind(&w_id)
            .fetch_all(&db)
            .await?;

        for folder in folders {
            write_to_archive(
                serde_json::to_string_pretty(&folder).unwrap(),
                format!("f/{}/folder.meta.json", folder.name),
                &mut a,
            )
            .await?;
        }
    }

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
            write_to_archive(script.content, format!("{}.{}", script.path, ext), &mut a).await?;

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
            write_to_archive(metadata_str, format!("{}.script.json", script.path), &mut a).await?;
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
                format!("{}.resource.json", resource.path),
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
                format!("{}.resource-type.json", resource_type.name),
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
            write_to_archive(flow_str, format!("{}.flow.json", flow.path), &mut a).await?;
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
            write_to_archive(flow_str, format!("{}.variable.json", var.path), &mut a).await?;
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
