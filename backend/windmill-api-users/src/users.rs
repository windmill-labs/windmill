/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#![allow(non_snake_case)]

use sqlx::{Postgres, Transaction};

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use windmill_api_auth::ApiAuthed;

pub use windmill_api_auth::Tokened;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{Extension, Path, Query},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::{header::LOCATION, StatusCode};
use windmill_api_auth::require_super_admin;
use windmill_common::usernames::{
    generate_instance_wide_unique_username, get_instance_username_or_create_pending,
};
use windmill_common::utils::{COOKIE_DOMAIN, IS_SECURE};
use windmill_common::webhook::WebhookShared;
use windmill_common::DB;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use tower_cookies::{Cookie, Cookies};
use tracing::Instrument;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::audit::AuditAuthor;
use windmill_common::auth::TOKEN_PREFIX_LEN;
use windmill_common::global_settings::AUTOMATE_USERNAME_CREATION_SETTING;
use windmill_common::oauth2::InstanceEvent;
use windmill_common::users::truncate_token;
use windmill_common::users::COOKIE_NAME;
use windmill_common::utils::paginate;
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::BASE_URL;
use windmill_common::{
    auth::{get_folders_for_user, get_groups_for_user},
    db::UserDB,
    error::{self, Error, JsonResult, Result},
    utils::{not_found_if_none, rd_string, require_admin, Pagination, StripPath},
};
use windmill_git_sync::handle_deployment_metadata;

const COOKIE_PATH: &str = "/";

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_users))
        .route("/list_usage", get(list_user_usage))
        .route("/list_usernames", get(list_usernames))
        .route("/exists", post(exists_username))
        .route("/get/:user", get(get_workspace_user))
        .route("/update/:user", post(update_workspace_user))
        .route("/delete/:user", delete(delete_workspace_user))
        .route("/convert_to_group/:user", post(convert_user_to_group))
        .route("/is_owner/*path", get(is_owner_of_path))
        .route("/whois/:username", get(whois))
        .route("/whoami", get(whoami))
        .route("/leave", post(leave_workspace))
        .route("/username_to_email/:username", get(username_to_email))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/exists/:email", get(exists_email))
        .route("/email", get(get_email))
        .route("/whoami", get(global_whoami))
        .route("/list_invites", get(list_invites))
        .route("/decline_invite", post(decline_invite))
        .route("/accept_invite", post(accept_invite))
        .route("/list_as_super_admin", get(list_users_as_super_admin))
        .route("/set_login_type/:user", post(set_login_type))
        .route("/update/:user", post(update_user))
        .route("/delete/:user", delete(delete_user))
        .route("/username_info/:user", get(get_instance_username_info))
        .route("/tokens/create", post(create_token))
        .route("/tokens/delete/:token_prefix", delete(delete_token))
        .route("/tokens/list", get(list_tokens))
        .route("/tokens/impersonate", post(impersonate))
        .route("/usage", get(get_usage))
        .route("/all_runnables", get(get_all_runnables))
        .route("/refresh_token", get(refresh_token))
        .route(
            "/tutorial_progress",
            post(update_tutorial_progress).get(get_tutorial_progress),
        )
        .route("/leave_instance", post(leave_instance))
        .route("/export", get(export_global_users))
        .route("/overwrite", post(overwrite_global_users))
}

pub fn make_unauthed_service() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout).get(logout))
        .route("/is_first_time_setup", get(is_first_time_setup))
        .route("/request_password_reset", post(request_password_reset))
        .route("/is_smtp_configured", get(is_smtp_configured))
}

pub use windmill_api_auth::{
    create_token_internal, fetch_api_authed, get_scope_tags, maybe_refresh_folders,
    require_is_writer, require_path_read_access_for_preview, NewToken, OptAuthed,
};

#[cfg(feature = "parquet")]
pub use windmill_api_auth::fetch_api_authed_from_permissioned_as;

#[derive(FromRow, Serialize)]
pub struct User {
    pub workspace_id: String,
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub operator: bool,
    pub disabled: bool,
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_via: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct UserWithUsage {
    pub email: String,
    pub executions: Option<i64>,
}

#[derive(FromRow, Serialize, Debug)]
pub struct GlobalUserInfo {
    email: String,
    login_type: Option<String>,
    super_admin: bool,
    devops: bool,
    verified: bool,
    name: Option<String>,
    company: Option<String>,
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_only: Option<bool>,
    first_time_user: bool,
}

#[derive(Serialize, Debug)]
pub struct UserInfo {
    pub workspace_id: String,
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_super_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub groups: Vec<String>,
    pub operator: bool,
    pub disabled: bool,
    pub role: Option<String>,
    pub folders_read: Vec<String>,
    pub folders: Vec<String>,
    pub folders_owners: Vec<String>,
    pub name: Option<String>,
}

#[derive(FromRow, Serialize)]
pub struct WorkspaceInvite {
    pub workspace_id: String,
    pub email: String,
    pub is_admin: bool,
    pub operator: bool,
    pub parent_workspace_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password: String,
    pub super_admin: bool,
    pub name: Option<String>,
    pub company: Option<String>,
    pub skip_email: Option<bool>,
}

#[derive(Deserialize)]
pub struct AcceptInvite {
    pub workspace_id: String,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct DeclineInvite {
    pub workspace_id: String,
}

#[derive(Deserialize)]
pub struct EditUser {
    pub is_super_admin: Option<bool>,
    pub is_devops: Option<bool>,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct EditWorkspaceUser {
    pub is_admin: Option<bool>,
    pub operator: Option<bool>,
    pub disabled: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct EditPassword {
    pub password: String,
}

#[derive(Deserialize)]
pub struct EditLoginType {
    pub login_type: String,
}

#[derive(FromRow, Serialize)]
pub struct TruncatedToken {
    pub label: Option<String>,
    pub token_prefix: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub scopes: Option<Vec<String>>,
}

// NewToken is re-exported from windmill-api-auth above

#[derive(Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

lazy_static::lazy_static! {
    static ref FIRST_TIME_SETUP: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

pub async fn is_first_time_setup(Extension(db): Extension<DB>) -> JsonResult<bool> {
    if !FIRST_TIME_SETUP.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(Json(false));
    }
    let single_user = sqlx::query_scalar!("SELECT 1 FROM password LIMIT 2")
        .fetch_all(&db)
        .await
        .ok()
        .unwrap_or_default()
        .len()
        == 1;
    if single_user {
        let user_is_admin_and_password_changeme = sqlx::query_scalar!(
            "SELECT 1 FROM password WHERE email = 'admin@windmill.dev' AND password_hash = '$argon2id$v=19$m=4096,t=3,p=1$oLJo/lPn/gezXCuFOEyaNw$i0T2tCkw3xUFsrBIKZwr8jVNHlIfoxQe+HfDnLtd12I'"
        ).fetch_all(&db)
        .await
        .ok()
        .unwrap_or_default()
        .len() == 1;
        if user_is_admin_and_password_changeme {
            let base_url_is_not_set =
                sqlx::query_scalar!("SELECT COUNT(*) FROM global_settings WHERE name = 'base_url'")
                    .fetch_optional(&db)
                    .await
                    .ok()
                    .flatten()
                    .flatten()
                    .unwrap_or(0)
                    == 0;
            if base_url_is_not_set {
                return Ok(Json(true));
            }
        }
    }
    FIRST_TIME_SETUP.store(false, std::sync::atomic::Ordering::Relaxed);
    Ok(Json(false))
}

#[derive(Deserialize)]
struct WorkspaceUsername {
    pub username: String,
}

async fn exists_username(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(WorkspaceUsername { username }): Json<WorkspaceUsername>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND username = $2)",
        &w_id,
        &username
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);
    tx.commit().await?;
    Ok(Json(exists))
}

async fn list_users(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<User>> {
    if *CLOUD_HOSTED && w_id == "demo" {
        require_admin(authed.is_admin, &authed.username)?;
    }
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as!(
        User,
        "
        SELECT *
          FROM usr
         WHERE workspace_id = $1
         ",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_user_usage(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<UserWithUsage>> {
    if *CLOUD_HOSTED && w_id == "demo" {
        require_admin(authed.is_admin, &authed.username)?;
    }
    let mut tx = user_db.begin(&authed).await?;
    let rows = tokio::time::timeout(
        Duration::from_secs(300),
        sqlx::query_as!(
            UserWithUsage,
            "
    SELECT usr.email, usage.executions
        FROM usr, LATERAL (
            SELECT COALESCE(SUM(c.duration_ms + 1000)/1000 , 0)::BIGINT executions
                FROM v2_job_completed c JOIN v2_job j USING (id)
                WHERE j.workspace_id = $1
                AND j.kind NOT IN ('flow', 'flowpreview', 'flownode')
                AND j.permissioned_as_email = usr.email
                AND now() - '1 week'::interval < j.created_at
            ) usage
        WHERE workspace_id = $1
        ",
            w_id
        )
        .fetch_all(&mut *tx),
    )
    .await
    .map_err(|e| Error::internal_err(format!("Timed out while fetching user usage: {e:#}")))??;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct ActiveUsersOnly {
    active_only: Option<bool>,
}

async fn list_users_as_super_admin(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(pagination): Query<Pagination>,
    Query(ActiveUsersOnly { active_only }): Query<ActiveUsersOnly>,
) -> JsonResult<Vec<GlobalUserInfo>> {
    require_super_admin(&db, &authed.email).await?;
    let per_page = pagination.per_page.unwrap_or(10000).max(1);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;

    let rows = if active_only.is_some_and(|x| x) {
        sqlx::query_as!(
            GlobalUserInfo,
            "WITH active_users AS (SELECT distinct username as email FROM audit WHERE timestamp > NOW() - INTERVAL '1 month' AND (operation = 'users.login' OR operation = 'oauth.login' OR operation = 'users.token.refresh')),
            authors as (SELECT distinct email FROM usr WHERE usr.operator IS false)
            SELECT email, email NOT IN (SELECT email FROM authors) as operator_only, login_type::text, verified, super_admin, devops, name, company, username, first_time_user
            FROM password
            WHERE email IN (SELECT email FROM active_users)
            ORDER BY super_admin DESC, devops DESC
            LIMIT $1 OFFSET $2",
            per_page as i32,
            offset as i32
        )
        .fetch_all(&db)
        .await?
    } else {
        sqlx::query_as!(
            GlobalUserInfo,
            "SELECT email, login_type::text, verified, super_admin, devops, name, company, username, NULL::bool as operator_only, first_time_user FROM password ORDER BY super_admin DESC, devops DESC, email LIMIT \
            $1 OFFSET $2",
            per_page as i32,
            offset as i32
        )
        .fetch_all(&db)
        .await?
    };

    Ok(Json(rows))
}

#[derive(Serialize, Deserialize)]
struct Progress {
    progress: u64,
    skipped_all: bool,
}
async fn get_tutorial_progress(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Progress> {
    let row = sqlx::query!(
        "SELECT progress::bigint as progress, skipped_all FROM tutorial_progress WHERE email = $1",
        authed.email
    )
    .fetch_optional(&db)
    .await?;

    if let Some(row) = row {
        Ok(Json(Progress {
            progress: row.progress.unwrap_or_default() as u64,
            skipped_all: row.skipped_all,
        }))
    } else {
        Ok(Json(Progress { progress: 0, skipped_all: false }))
    }
}

async fn update_tutorial_progress(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(progress): Json<Progress>,
) -> Result<String> {
    sqlx::query!(
        "INSERT INTO tutorial_progress (email, progress, skipped_all) VALUES ($2, $1::bigint::bit(64), $3) ON CONFLICT (email) DO UPDATE SET progress = EXCLUDED.progress, skipped_all = EXCLUDED.skipped_all",
        progress.progress as i64,
        authed.email,
        progress.skipped_all
    )
    .execute(&db)
    .await?;
    Ok("tutorial progress updated".to_string())
}

async fn list_usernames(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    if *CLOUD_HOSTED && w_id == "demo" {
        return Ok(Json(vec![
            authed.username,
            "other_usernames_redacted_in_demo_workspace".to_string(),
        ]));
    }
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_scalar!("SELECT username from usr WHERE workspace_id = $1", &w_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_invites(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<WorkspaceInvite>> {
    let mut tx = db.begin().await?;
    let rows = sqlx::query_as!(
        WorkspaceInvite,
        "SELECT
            workspace_invite.workspace_id,
            workspace_invite.email,
            workspace_invite.is_admin,
            workspace_invite.operator,
            workspace.parent_workspace_id
        FROM workspace_invite JOIN workspace ON workspace_invite.workspace_id = workspace.id WHERE email = $1",
        authed.email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

lazy_static::lazy_static! {
    static ref INVALIDATE_ALL_SESSIONS_ON_LOGOUT: bool = std::env::var("INVALIDATE_ALL_SESSIONS_ON_LOGOUT")
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
}

#[derive(Deserialize)]
struct LogoutQuery {
    rd: Option<String>,
}
async fn logout(
    Tokened { token }: Tokened,
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Query(LogoutQuery { rd }): Query<LogoutQuery>,
) -> Result<Response> {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_path(COOKIE_PATH);
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }
    cookies.remove(cookie);
    let mut tx = db.begin().await?;

    let email = if *INVALIDATE_ALL_SESSIONS_ON_LOGOUT {
        sqlx::query_scalar!(
            "WITH email_lookup AS (
                SELECT email FROM token WHERE token = $1
            )
            DELETE FROM token
            WHERE email = (SELECT email FROM email_lookup) AND label = 'session'
            RETURNING email",
            token
        )
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_scalar!("DELETE FROM token WHERE token = $1 RETURNING email", token)
            .fetch_optional(&mut *tx)
            .await?
    };

    if let Some(email) = email {
        let email = email.unwrap_or("noemail".to_string());
        let audit_message = if *INVALIDATE_ALL_SESSIONS_ON_LOGOUT {
            "users.logout_all"
        } else {
            "users.logout"
        };
        audit_log(
            &mut *tx,
            &AuditAuthor {
                email: email.clone(),
                username: email,
                username_override: None,
                token_prefix: Some(token[0..TOKEN_PREFIX_LEN].to_string()),
            },
            audit_message,
            ActionKind::Delete,
            "global",
            Some(&truncate_token(&token)),
            None,
        )
        .await?;
    }
    tx.commit().await?;
    if let Some(rd) = rd {
        Ok((StatusCode::TEMPORARY_REDIRECT, [(LOCATION, rd)]).into_response())
    } else {
        Ok((StatusCode::OK, "logged out successfully".to_string()).into_response())
    }
}

async fn whoami(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { username, email, is_admin, groups, folders, .. }: ApiAuthed,
) -> JsonResult<UserInfo> {
    let user = get_user(&w_id, &username, &db).await?;
    if let Some(user) = user {
        Ok(Json(user))
    } else {
        Ok(Json(UserInfo {
            workspace_id: w_id,
            email: email.clone(),
            username: email,
            name: None,
            is_admin,
            is_super_admin: is_admin,
            created_at: chrono::Utc::now(),
            groups: groups,
            operator: false,
            disabled: false,
            role: Some("superadmin".to_string()),
            folders_read: folders.clone().into_iter().map(|x| x.0).collect(),
            folders: folders
                .clone()
                .into_iter()
                .filter_map(|x| if x.1 { Some(x.0) } else { None })
                .collect(),
            folders_owners: folders
                .into_iter()
                .filter_map(|x| if x.2 { Some(x.0) } else { None })
                .collect(),
        }))
    }
}

async fn global_whoami(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
    Tokened { token }: Tokened,
) -> JsonResult<GlobalUserInfo> {
    let user = sqlx::query_as!(
        GlobalUserInfo,
        "SELECT email, login_type::TEXT, super_admin, devops, verified, name, company, username, NULL::bool as operator_only, first_time_user FROM password WHERE \
         email = $1",
        email
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::internal_err(format!("fetching global identity: {e:#}")));

    if let Ok(user) = user {
        Ok(Json(user))
    } else if std::env::var("SUPERADMIN_SECRET").ok() == Some(token) {
        Ok(Json(GlobalUserInfo {
            email: email.clone(),
            login_type: Some("superadmin_secret".to_string()),
            super_admin: true,
            devops: false,
            verified: true,
            name: None,
            company: None,
            username: None,
            operator_only: None,
            first_time_user: false,
        }))
    } else {
        Err(user.unwrap_err())
    }
}

async fn exists_email(Extension(db): Extension<DB>, Path(email): Path<String>) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM password WHERE email = $1)",
        email
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

async fn get_email(ApiAuthed { email, .. }: ApiAuthed) -> Result<String> {
    Ok(email)
}

async fn get_usage(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
) -> Result<String> {
    let usage = sqlx::query_scalar!(
        "
    SELECT usage.usage FROM usage
    WHERE is_workspace = false
    AND month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)
    AND id = $1",
        email
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(0);
    Ok(usage.to_string())
}

#[derive(FromRow, Serialize)]
pub struct User2 {
    pub workspace_id: String,
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub operator: bool,
    pub disabled: bool,
    pub role: Option<String>,
    pub super_admin: bool,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_via: Option<serde_json::Value>,
}

async fn get_user(w_id: &str, username: &str, db: &DB) -> Result<Option<UserInfo>> {
    let user = sqlx::query_as!(
        User2,
        "SELECT usr.*, password.super_admin, password.name FROM usr LEFT JOIN password ON usr.email = password.email Where usr.username = $1 AND workspace_id = $2
        ",
        username,
        w_id
    )
    .fetch_optional(db)
    .await?;
    let groups = get_groups_for_user(
        &w_id,
        username,
        &user
            .as_ref()
            .map(|x| x.email.to_string())
            .unwrap_or_else(|| "".to_string()),
        db,
    )
    .await?;
    let folders = get_folders_for_user(&w_id, username, &groups, db).await?;

    Ok(user.map(|usr| UserInfo {
        groups,
        workspace_id: usr.workspace_id,
        email: usr.email,
        username: usr.username,
        name: usr.name,
        is_admin: usr.is_admin,
        is_super_admin: usr.super_admin,
        created_at: usr.created_at,
        operator: usr.operator,
        disabled: usr.disabled,
        role: usr.role,
        folders_read: folders.clone().into_iter().map(|x| x.0).collect(),
        folders: folders
            .clone()
            .into_iter()
            .filter_map(|x| if x.1 { Some(x.0) } else { None })
            .collect(),
        folders_owners: folders
            .into_iter()
            .filter_map(|x| if x.2 { Some(x.0) } else { None })
            .collect(),
    }))
}

pub async fn is_owner_of_path(
    authed: ApiAuthed,
    Path((_w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    if authed.is_admin {
        Ok(Json(true))
    } else {
        Ok(Json(require_owner_of_path(&authed, path).is_ok()))
    }
}

pub use windmill_api_auth::require_owner_of_path;

// get_perm_in_extra_perms_for_authed and require_is_writer are re-exported from windmill-api-auth above
async fn whois(
    Extension(db): Extension<DB>,
    Path((w_id, username)): Path<(String, String)>,
) -> JsonResult<UserInfo> {
    let user_o = get_user(&w_id, &username, &db).await?;
    let user = not_found_if_none(user_o, "User", username)?;
    Ok(Json(user))
}

// async fn create_invite_code(
//     ApiAuthed { email, .. }: ApiAuthed,
//     Extension(db): Extension<DB>,
//     Json(nu): Json<NewInviteCode>,
// ) -> Result<(StatusCode, String)> {

//     let mut tx = db.begin().await?;
//     require_super_admin(&mut *tx, email).await?;

//     sqlx::query!(
//         "INSERT INTO invite_code
//             (code, seats_left)
//             VALUES ($1, $2)",
//         nu.code,
//         nu.seats
//     )
//     .execute(&mut *tx)
//     .await?;

//     tx.commit().await?;

//     Ok((
//         StatusCode::CREATED,
//         format!("new invite code {}", nu.code),
//     ))
// }

async fn decline_invite(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(nu): Json<DeclineInvite>,
) -> Result<(StatusCode, String)> {
    let mut tx = db.begin().await?;

    let is_admin = sqlx::query_scalar!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2 RETURNING is_admin",
        nu.workspace_id,
        authed.email,
    )
    .fetch_optional(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.decline_invite",
        ActionKind::Delete,
        &nu.workspace_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    if is_admin.is_some() {
        Ok((
            StatusCode::OK,
            format!(
                "user {} declined invite to workspace {}",
                &authed.email, nu.workspace_id
            ),
        ))
    } else {
        Err(Error::NotFound(format!(
            "invite for {} not found",
            authed.email
        )))
    }
}

pub use windmill_common::usernames::VALID_USERNAME;

async fn accept_invite(
    authed: ApiAuthed,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Json(nu): Json<AcceptInvite>,
) -> Result<(StatusCode, String)> {
    let mut tx = db.begin().await?;

    let r = sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2 RETURNING is_admin, operator",
        nu.workspace_id,
        authed.email,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(r) = r {
        let already_in_workspace = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
            &nu.workspace_id,
            &authed.email,
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);

        if already_in_workspace {
            tx.commit().await?;
            return Ok((
                StatusCode::CREATED,
                format!(
                    "user {} accepted invite to workspace {}",
                    &authed.email, nu.workspace_id
                ),
            ));
        }
        let username;
        (tx, username) = join_workspace(
            &nu.workspace_id,
            &authed,
            nu.username,
            r.is_admin,
            r.operator,
            tx,
        )
        .await?;

        audit_log(
            &mut *tx,
            &ApiAuthed { username: username.clone(), ..authed.clone() },
            "users.accept_invite",
            ActionKind::Create,
            &nu.workspace_id,
            Some(&authed.email),
            None,
        )
        .await?;
        tx.commit().await?;

        handle_deployment_metadata(
            &authed.email,
            &username,
            &db,
            &nu.workspace_id,
            windmill_git_sync::DeployedObject::User { email: authed.email.clone() },
            Some(format!("User '{}' accepted invite", &authed.email)),
            true,
            None,
        )
        .await?;
        webhook.send_instance_event(InstanceEvent::UserJoinedWorkspace {
            email: authed.email.clone(),
            workspace: nu.workspace_id.clone(),
            username: username,
        });
        Ok((
            StatusCode::CREATED,
            format!(
                "user {} accepted invite to workspace {}",
                &authed.email, nu.workspace_id
            ),
        ))
    } else {
        Err(Error::NotFound(format!(
            "invite for {} not found",
            authed.email
        )))
    }
}

async fn join_workspace<'c>(
    w_id: &str,
    authed: &ApiAuthed,
    username: Option<String>,
    is_admin: bool,
    operator: bool,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
) -> error::Result<(sqlx::Transaction<'c, sqlx::Postgres>, String)> {
    let automate_username_creation = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        AUTOMATE_USERNAME_CREATION_SETTING,
    )
    .fetch_optional(&mut *tx)
    .await?
    .map(|v| v.as_bool())
    .flatten()
    .unwrap_or(true);

    let username = if automate_username_creation {
        if username.is_some() && username.unwrap().len() > 0 {
            return Err(Error::BadRequest(
                "username is not allowed when username creation is automated".to_string(),
            ));
        }
        get_instance_username_or_create_pending(&mut tx, &authed.email).await?
    } else {
        let username = username.ok_or(Error::BadRequest("username is required".to_string()))?;
        let already_exists_username = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND username = $2)",
            &w_id,
            username,
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);

        if already_exists_username {
            return Err(Error::BadRequest(format!(
                "user with username {} already exists in workspace {}",
                username, w_id
            )));
        }

        if !VALID_USERNAME.is_match(&username) {
            return Err(windmill_common::error::Error::BadRequest(format!(
                "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
            )));
        }
        username.to_string()
    };

    let already_exists_email = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
        &w_id,
        authed.email,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if already_exists_email {
        return Err(Error::BadRequest(format!(
            "user with email {} already exists in workspace {}",
            authed.email, w_id
        )));
    }

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin, operator)
            VALUES ($1, $2, $3, $4, $5)",
        &w_id,
        authed.email,
        username,
        is_admin,
        operator
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        username,
        "all",
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &AuditAuthor { username: username.clone(), ..authed.into() },
        "users.add_to_workspace",
        ActionKind::Create,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    Ok((tx, username))
}

async fn leave_instance(Extension(db): Extension<DB>, authed: ApiAuthed) -> Result<String> {
    let mut tx = db.begin().await?;
    sqlx::query!("DELETE FROM password WHERE email = $1", &authed.email)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.leave",
        ActionKind::Delete,
        "global",
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Left instance",))
}

async fn get_workspace_user(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_update)): Path<(String, String)>,
) -> Result<Json<User>> {
    require_admin(is_admin, &username)?;

    let user = sqlx::query_as!(
        User,
        "SELECT * FROM usr WHERE username = $1 AND workspace_id = $2",
        &username_to_update,
        &w_id
    )
    .fetch_optional(&db)
    .await?;

    let user = not_found_if_none(user, "User", username_to_update)?;

    Ok(Json(user))
}

async fn update_workspace_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_update)): Path<(String, String)>,
    Json(eu): Json<EditWorkspaceUser>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    require_admin(authed.is_admin, &authed.username)?;

    update_workspace_user_internal(
        &w_id,
        &username_to_update,
        eu.is_admin,
        eu.operator,
        eu.disabled,
        &mut tx,
        Some(&authed),
    )
    .await?;

    let user_email = sqlx::query_scalar!(
        "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
        &username_to_update,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::User { email: user_email.clone() },
        Some(format!("Updated user '{}'", &user_email)),
        true,
        None,
    )
    .await?;

    Ok(format!("user {} updated", user_email))
}

async fn convert_user_to_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_convert)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = db.begin().await?;

    // Get user email and current status
    let user_info = sqlx::query!(
        "SELECT email, is_admin, operator, added_via FROM usr WHERE username = $1 AND workspace_id = $2",
        username_to_convert,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let user_info = not_found_if_none(user_info, "User", &username_to_convert)?;

    // Check if user is already a group user
    if let Some(added_via) = &user_info.added_via {
        if added_via.get("source").and_then(|v| v.as_str()) == Some("instance_group") {
            return Err(Error::BadRequest(
                "User is already a group user".to_string(),
            ));
        }
    }

    // Find which instance groups this user belongs to that are configured for auto-add in this workspace
    let eligible_groups = sqlx::query!(
        r#"
        SELECT
            eig.igroup as group_name,
            ws.auto_invite->'instance_groups_roles' as instance_groups_roles
        FROM email_to_igroup eig
        INNER JOIN workspace_settings ws ON ws.workspace_id = $1
        WHERE eig.email = $2
        AND ws.auto_invite->'instance_groups' ? eig.igroup
        "#,
        &w_id,
        &user_info.email
    )
    .fetch_all(&mut *tx)
    .await?;

    if eligible_groups.is_empty() {
        return Err(Error::BadRequest(
            "User is not a member of any instance groups configured for auto-add in this workspace"
                .to_string(),
        ));
    }

    // Determine the group with highest precedence (same logic as process_instance_group_auto_adds)
    let roles: std::collections::HashMap<String, String> =
        if let Some(roles_json) = &eligible_groups[0].instance_groups_roles {
            serde_json::from_value(roles_json.clone()).unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };

    let mut best_group = &eligible_groups[0].group_name;
    let mut best_precedence = 0u8;

    for group in &eligible_groups {
        let default_role = "developer".to_string();
        let role = roles.get(&group.group_name).unwrap_or(&default_role);

        let precedence = match role.as_str() {
            "admin" => 3,
            "developer" => 2,
            "operator" => 1,
            _ => 2,
        };

        if precedence > best_precedence {
            best_precedence = precedence;
            best_group = &group.group_name;
        }
    }

    let primary_group_name = best_group;

    // Determine role from group configuration using the selected primary group
    let default_role = "developer".to_string();
    let role = roles
        .get(primary_group_name)
        .unwrap_or(&default_role)
        .as_str();

    let (is_admin, is_operator) = match role {
        "admin" => (true, false),
        "operator" => (false, true),
        _ => (false, false),
    };

    // Update user with instance group information
    let instance_group_source = serde_json::json!({
        "source": "instance_group",
        "group": primary_group_name
    });

    sqlx::query!(
        "UPDATE usr SET added_via = $1, is_admin = $2, operator = $3 WHERE username = $4 AND workspace_id = $5",
        instance_group_source,
        is_admin,
        is_operator,
        username_to_convert,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.convert_to_group",
        ActionKind::Update,
        &w_id,
        Some(&username_to_convert),
        Some([("group", primary_group_name.as_str()), ("role", role)].into()),
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::User { email: user_info.email.clone() },
        Some(format!(
            "Converted user '{}' to group user (group: {}, role: {})",
            &user_info.email, primary_group_name, role
        )),
        true,
        None,
    )
    .await?;

    Ok(format!(
        "User {} converted to group user (group: {}, role: {})",
        username_to_convert, primary_group_name, role
    ))
}

async fn update_user(
    authed: ApiAuthed,
    Path(email_to_update): Path<String>,
    Extension(db): Extension<DB>,
    Json(eu): Json<EditUser>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    let mut new_super_admin: Option<bool> = None;
    if let Some(sa) = eu.is_super_admin {
        sqlx::query_scalar!(
            "UPDATE password SET super_admin = $1 WHERE email = $2",
            sa,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
        new_super_admin = Some(sa);
    }

    if let Some(dv) = eu.is_devops {
        sqlx::query_scalar!(
            "UPDATE password SET devops = $1 WHERE email = $2",
            dv,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
        // If super_admin wasn't explicitly set, we still need to refresh tokens
        if new_super_admin.is_none() {
            new_super_admin = sqlx::query_scalar!(
                "SELECT super_admin FROM password WHERE email = $1",
                &email_to_update
            )
            .fetch_optional(&mut *tx)
            .await?;
        }
    }

    if let Some(sa) = new_super_admin {
        // Delete session tokens to force re-login with new privileges
        sqlx::query!(
            "DELETE FROM token WHERE email = $1 AND label = 'session'",
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
        // Update super_admin flag on non-session tokens (webhooks, API tokens, etc.)
        sqlx::query!(
            "UPDATE token SET super_admin = $1 WHERE email = $2 AND label != 'session'",
            sa,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(n) = eu.name {
        sqlx::query_scalar!(
            "UPDATE password SET name = $1 WHERE email = $2",
            n,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.update",
        ActionKind::Update,
        "global",
        Some(&email_to_update),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("email {} updated", &email_to_update))
}

async fn delete_user(
    authed: ApiAuthed,
    Path(email_to_delete): Path<String>,
    Extension(db): Extension<DB>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    sqlx::query!("DELETE FROM password WHERE email = $1", &email_to_delete)
        .execute(&mut *tx)
        .await?;

    let usernames = sqlx::query_scalar!(
        "DELETE FROM usr WHERE email = $1 RETURNING username",
        &email_to_delete
    )
    .fetch_all(&mut *tx)
    .await?;

    for username in usernames {
        sqlx::query!("DELETE FROM password WHERE email = $1", &email_to_delete)
            .execute(&mut *tx)
            .await?;

        sqlx::query!("DELETE FROM usr_to_group WHERE usr = $1", &username)
            .execute(&mut *tx)
            .await?;

        sqlx::query!(
            "DELETE FROM workspace_invite WHERE email = $1",
            &email_to_delete
        )
        .execute(&mut *tx)
        .await?;
    }

    // Remove user from all instance groups email_to_igroup
    sqlx::query!(
        "DELETE FROM email_to_igroup WHERE email = $1",
        &email_to_delete
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.delete",
        ActionKind::Delete,
        "global",
        Some(&email_to_delete),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("email {} deleted", &email_to_delete))
}

lazy_static::lazy_static! {
    pub static ref NEW_USER_WEBHOOK: Option<String> = std::env::var("NEW_USER_WEBHOOK").ok();

}

/// Internal helper for updating workspace user permissions - used by both API and system operations
pub async fn update_workspace_user_internal(
    w_id: &str,
    username_to_update: &str,
    is_admin: Option<bool>,
    operator: Option<bool>,
    disabled: Option<bool>,
    tx: &mut Transaction<'_, Postgres>,
    authed: Option<&ApiAuthed>, // None for system operations
) -> Result<()> {
    if let Some(a) = is_admin {
        sqlx::query_scalar!(
            "UPDATE usr SET is_admin = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            username_to_update,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }
    if let Some(a) = operator {
        sqlx::query_scalar!(
            "UPDATE usr SET operator = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            username_to_update,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }
    if let Some(a) = disabled {
        sqlx::query_scalar!(
            "UPDATE usr SET disabled = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            username_to_update,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }

    // Only audit if we have an authenticated user (API calls)
    if let Some(auth) = authed {
        audit_log(
            &mut **tx,
            auth,
            "users.update",
            ActionKind::Update,
            w_id,
            Some(username_to_update),
            None,
        )
        .await?;
    }

    Ok(())
}

/// Internal helper for deleting workspace users - used by both API and system operations
pub async fn delete_workspace_user_internal(
    w_id: &str,
    username_to_delete: &str,
    email_to_delete: &str,
    tx: &mut Transaction<'_, Postgres>,
    authed: Option<&ApiAuthed>, // None for system operations
) -> Result<()> {
    sqlx::query_scalar!(
        "DELETE FROM usr WHERE email = $1 AND workspace_id = $2",
        email_to_delete,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM usr_to_group WHERE usr = $1 AND workspace_id = $2",
        username_to_delete,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // Only audit if we have an authenticated user (API calls)
    if let Some(auth) = authed {
        audit_log(
            &mut **tx,
            auth,
            "users.delete",
            ActionKind::Delete,
            w_id,
            Some(username_to_delete),
            None,
        )
        .await?;
    }

    Ok(())
}

async fn delete_workspace_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_delete)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    require_admin(authed.is_admin, &authed.username)?;

    let email_to_delete_o = sqlx::query_scalar!(
        "SELECT email FROM usr where username = $1 AND workspace_id = $2",
        username_to_delete,
        &w_id,
    )
    .fetch_optional(&db)
    .await?;

    let email_to_delete = not_found_if_none(email_to_delete_o, "User", &username_to_delete)?;

    delete_workspace_user_internal(
        &w_id,
        &username_to_delete,
        &email_to_delete,
        &mut tx,
        Some(&authed),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        windmill_git_sync::DeployedObject::User { email: email_to_delete.clone() },
        Some(format!(
            "Removed user '{}' from workspace",
            &email_to_delete
        )),
        true,
        None,
    )
    .await?;

    Ok(format!("username {} deleted", username_to_delete))
}

async fn set_login_type(
    Extension(db): Extension<DB>,
    Path(email): Path<String>,
    authed: ApiAuthed,
    Json(et): Json<EditLoginType>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE password SET login_type = $1 WHERE email = $2",
        et.login_type,
        email
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.set_login_type",
        ActionKind::Update,
        "global",
        Some(&email),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!(
        "login type of {} updated to {}",
        email, et.login_type
    ))
}

#[allow(unreachable_code, unused_variables)]
async fn login(
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(Login { email, password }): Json<Login>,
) -> Result<String> {
    #[cfg(feature = "no_auth")]
    {
        return Ok("no_auth".to_string());
    }

    let mut tx = db.begin().await?;
    let email = email.to_lowercase();
    let audit_author = AuditAuthor {
        email: email.clone(),
        username: email.clone(),
        username_override: None,
        token_prefix: None,
    };
    let email_w_h: Option<(String, String, bool)> = sqlx::query_as(
        "SELECT email, password_hash, super_admin FROM password WHERE email = $1 AND login_type = \
         'password'",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some((email, hash, super_admin)) = email_w_h {
        let parsed_hash =
            PasswordHash::new(&hash).map_err(|e| Error::internal_err(e.to_string()))?;
        if argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err()
        {
            audit_log(
                &mut *tx,
                &audit_author,
                "users.login_failure",
                ActionKind::Create,
                "global",
                None,
                None,
            )
            .await?;
            Err(Error::BadRequest("Invalid login".to_string()))
        } else {
            let token = create_session_token(&email, super_admin, &mut tx, cookies).await?;

            let audit_author = AuditAuthor {
                email: email.clone(),
                username: email.clone(),
                username_override: None,
                token_prefix: Some(token[0..TOKEN_PREFIX_LEN].to_string()),
            };

            audit_log(
                &mut *tx,
                &audit_author,
                "users.login",
                ActionKind::Create,
                "global",
                Some(&truncate_token(&token)),
                None,
            )
            .await?;

            tx.commit().await?;
            Ok(token)
        }
    } else {
        audit_log(
            &mut *tx,
            &audit_author,
            "users.login_failure",
            ActionKind::Create,
            "global",
            None,
            None,
        )
        .await?;
        Err(Error::BadRequest("Invalid login".to_string()))
    }
}

#[derive(Deserialize)]
struct RefreshTokenQuery {
    if_expiring_in_less_than_s: Option<i32>,
}
async fn refresh_token(
    Extension(db): Extension<DB>,
    Query(query): Query<RefreshTokenQuery>,
    Tokened { token }: Tokened,
    authed: ApiAuthed,
    cookies: Cookies,
) -> Result<String> {
    let mut tx = db.begin().await?;

    if let Some(thresh_s) = query.if_expiring_in_less_than_s {
        let not_expired = sqlx::query_scalar!("SELECT true FROM token WHERE token = $1 and expiration IS NOT NULL and expiration > now() + $2::int * '1 sec'::interval", &token, thresh_s)
            .fetch_optional(&db)
            .await?
            .flatten()
            .unwrap_or(false);
        if not_expired {
            return Ok("token expiry is far enough".to_string());
        }
    }

    let super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        &authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);

    let new_token = create_session_token(&authed.email, super_admin, &mut tx, cookies).await?;

    audit_log(
        &mut *tx,
        &AuditAuthor {
            email: authed.email.to_string(),
            username: authed.email.to_string(),
            username_override: None,
            token_prefix: authed.token_prefix,
        },
        "users.token.refresh",
        ActionKind::Create,
        &"global",
        Some(&truncate_token(&new_token)),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok("token refreshed".to_string())
}

lazy_static::lazy_static! {
    static ref MAX_SESSION_VALIDITY_SECONDS: i64 = std::env::var("MAX_SESSION_VALIDITY_SECONDS").ok().unwrap_or_else(|| String::new()).parse::<i64>().unwrap_or(3 * 24 * 60 * 60);
    static ref INVALIDATE_OLD_SESSIONS: bool = std::env::var("INVALIDATE_OLD_SESSIONS").ok().unwrap_or_else(|| String::new()).parse::<bool>().unwrap_or(false);
}

pub async fn create_session_token<'c>(
    email: &str,
    super_admin: bool,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    cookies: Cookies,
) -> Result<String> {
    let token = rd_string(32);

    if *INVALIDATE_OLD_SESSIONS {
        sqlx::query!(
            "DELETE FROM token WHERE email = $1 AND label = 'session'",
            email
        )
        .execute(&mut **tx)
        .await?;

        audit_log(
            &mut **tx,
            &AuditAuthor {
                email: email.to_string(),
                username: email.to_string(),
                username_override: None,
                token_prefix: Some(token[0..TOKEN_PREFIX_LEN].to_string()),
            },
            "users.token.invalidate_old_sessions",
            ActionKind::Delete,
            &"global",
            None,
            None,
        )
        .instrument(tracing::info_span!("token", email))
        .await?;
    }

    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin)
            VALUES ($1, $2, $3, now() + ($4 || ' seconds')::interval, $5)",
        token,
        email,
        "session",
        &MAX_SESSION_VALIDITY_SECONDS.to_string(),
        super_admin
    )
    .execute(&mut **tx)
    .await?;

    let mut cookie = Cookie::new(COOKIE_NAME, token.clone());
    cookie.set_secure(IS_SECURE.read().await.clone());
    cookie.set_same_site(Some(tower_cookies::cookie::SameSite::Lax));
    cookie.set_http_only(true);
    cookie.set_path(COOKIE_PATH);
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }

    let mut expire: OffsetDateTime = time::OffsetDateTime::now_utc();
    expire += time::Duration::seconds(*MAX_SESSION_VALIDITY_SECONDS);
    cookie.set_expires(expire);
    cookies.add(cookie);
    Ok(token)
}

// create_token_internal is re-exported from windmill-api-auth above

async fn create_token(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(token_config): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    let mut tx = db.begin().await?;

    let token = create_token_internal(&mut *tx, &db, &authed, token_config).await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, token))
}

async fn impersonate(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(new_token): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    let token = rd_string(32);
    require_super_admin(&db, &authed.email).await?;

    if new_token.impersonate_email.is_none() {
        return Err(Error::BadRequest(
            "impersonate_username is required".to_string(),
        ));
    }

    let impersonated = new_token.impersonate_email.unwrap();

    let is_super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        impersonated
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(false);
    let mut tx = db.begin().await?;

    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin)
            VALUES ($1, $2, $3, $4, $5)",
        token,
        impersonated,
        new_token.label,
        new_token.expiration,
        is_super_admin
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.impersonate",
        ActionKind::Delete,
        &"global",
        Some(&token[0..10]),
        Some([("impersonated", &format!("{impersonated}")[..])].into()),
    )
    .instrument(tracing::info_span!("token", email = &impersonated))
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, token))
}

#[derive(Deserialize)]
struct ListTokenQuery {
    exclude_ephemeral: Option<bool>,
}

async fn list_tokens(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
    Query(query): Query<ListTokenQuery>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<TruncatedToken>> {
    let (per_page, offset) = paginate(pagination);
    let rows = if query.exclude_ephemeral.unwrap_or(false) {
        sqlx::query_as!(
            TruncatedToken,
            "SELECT label, concat(substring(token for 10)) as token_prefix, expiration, created_at, \
             last_used_at, scopes FROM token WHERE email = $1 AND (label != 'ephemeral-script' OR label IS NULL)
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            email,
            per_page as i64,
            offset as i64,
        )
        .fetch_all(&db)
        .await?
    } else {
        sqlx::query_as!(
            TruncatedToken,
            "SELECT label, concat(substring(token for 10)) as token_prefix, expiration, created_at, \
            last_used_at, scopes FROM token WHERE email = $1
            ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            email,
            per_page as i64,
            offset as i64,
        )
        .fetch_all(&db)
        .await?
    };
    Ok(Json(rows))
}

async fn delete_token(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(token_prefix): Path<String>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    let tokens_deleted: Vec<String> = sqlx::query_scalar(
        "DELETE FROM token
               WHERE email = $1
                 AND token LIKE concat($2::text, '%')
           RETURNING concat(substring(token for 10), '*****')",
    )
    .bind(&authed.email)
    .bind(&token_prefix)
    .fetch_all(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.token.delete",
        ActionKind::Delete,
        &"global",
        Some(&token_prefix),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!(
        "deleted {} tokens {:?} with prefix {}",
        tokens_deleted.len(),
        tokens_deleted,
        token_prefix
    ))
}

async fn leave_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    let mut tx = db.begin().await?;
    sqlx::query!(
        "DELETE FROM usr WHERE workspace_id = $1 AND username = $2",
        &w_id,
        authed.username
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.leave_workspace",
        ActionKind::Delete,
        &w_id,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("left workspace {w_id}"))
}

#[derive(Serialize)]
struct Runnable {
    workspace: String,
    endpoint_async: String,
    endpoint_sync: String,
    summary: String,
    description: String,
    schema: Option<serde_json::Value>,
    kind: String,
    path: String,
}

async fn get_all_runnables(
    Extension(db): Extension<UserDB>,
    authed: ApiAuthed,
    Tokened { token }: Tokened,
    Extension(cache): Extension<Arc<windmill_api_auth::AuthCache>>,
) -> JsonResult<Vec<Runnable>> {
    let mut tx = db.clone().begin(&authed).await?;
    let mut runnables = Vec::new();
    let workspaces = sqlx::query_scalar!(
        "SELECT workspace.id as id FROM workspace, usr WHERE usr.workspace_id = workspace.id AND \
         usr.email = $1 AND deleted = false",
        authed.email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    for workspace in workspaces {
        let nauthed = cache
            .get_authed(Some(workspace.clone()), &token)
            .await
            .ok_or_else(|| {
                Error::BadRequest(format!("not authorized to access workspace: {workspace}"))
            })?;
        let mut tx = db.clone().begin(&nauthed).await?;
        let flows = sqlx::query!(
            "SELECT flow.workspace_id as workspace, flow.path, summary, description, flow_version.schema
            FROM flow
            LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
            WHERE flow.workspace_id = $1",
            workspace
        )
        .fetch_all(&mut *tx)
        .await?;
        runnables.extend(
            flows
                .into_iter()
                .map(|f| Runnable {
                    workspace: f.workspace.clone(),
                    endpoint_async: format!("/w/{}/jobs/run/f/{}", &f.workspace, &f.path),
                    endpoint_sync: format!(
                        "/w/{}/jobs/run_wait_result/f/{}",
                        &f.workspace, &f.path
                    ),
                    summary: f.summary,
                    description: f.description,
                    schema: f.schema,
                    kind: "flow".to_string(),
                    path: f.path,
                })
                .collect::<Vec<_>>(),
        );
        let scripts = sqlx::query!(
        "SELECT workspace_id as workspace, path, summary, description, schema FROM script as o
         WHERE created_at = (select max(created_at) from script where o.path = path and workspace_id = $1 AND archived = false)
         AND workspace_id = $1 and archived = false", workspace
    )
    .fetch_all(&mut *tx)
    .await?;
        runnables.extend(
            scripts
                .into_iter()
                .map(|s| Runnable {
                    workspace: s.workspace.clone(),
                    endpoint_async: format!("/w/{}/jobs/run/p/{}", &s.workspace, &s.path),
                    endpoint_sync: format!(
                        "/w/{}/jobs/run_wait_result/p/{}",
                        &s.workspace, &s.path
                    ),
                    summary: s.summary,
                    description: s.description,
                    schema: s.schema,
                    kind: "script".to_string(),
                    path: s.path,
                })
                .collect::<Vec<_>>(),
        );
        tx.commit().await?;
    }
    Ok(Json(runnables))
}

//used by oauth
#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct LoginUserInfo {
    pub email: Option<String>,
    pub name: Option<String>,
    pub company: Option<String>,
    pub preferred_username: Option<String>,
    pub displayName: Option<String>,
}

#[derive(Serialize)]
struct InstanceUsernameInfo {
    username: String,
    workspace_usernames: Vec<WorkspaceUsernameInfo>,
}

#[derive(Serialize)]
struct WorkspaceUsernameInfo {
    workspace_id: String,
    username: String,
}
async fn get_instance_username_info(
    ApiAuthed { email, .. }: ApiAuthed,
    Path(user_email): Path<String>,
    Extension(db): Extension<DB>,
) -> JsonResult<InstanceUsernameInfo> {
    require_super_admin(&db, &email).await?;
    let mut tx = db.begin().await?;
    let instance_username = match sqlx::query_scalar!(
        "SELECT username FROM password WHERE email = $1",
        &user_email
    )
    .fetch_one(&mut *tx)
    .await?
    {
        Some(username) => username,
        None => generate_instance_wide_unique_username(&mut tx, &user_email).await?,
    };

    let workspace_usernames = sqlx::query_as!(
        WorkspaceUsernameInfo,
        "SELECT workspace_id, username FROM usr WHERE email = $1",
        &user_email
    )
    .fetch_all(&mut *tx)
    .await?;

    Ok(Json(InstanceUsernameInfo {
        username: instance_username,
        workspace_usernames: workspace_usernames,
    }))
}

async fn username_to_email(
    Path((w_id, username)): Path<(String, String)>,
    Extension(db): Extension<DB>,
) -> Result<String> {
    let email = sqlx::query_scalar!(
        "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
        &username,
        &w_id
    )
    .fetch_optional(&db)
    .await?;

    let email = not_found_if_none(email, "user", username)?;

    Ok(email)
}

#[cfg(feature = "enterprise")]
#[derive(Serialize, Deserialize)]
struct ExportedGlobalUser {
    email: String,
    password_hash: Option<String>,
    login_type: String,
    super_admin: bool,
    verified: bool,
    name: Option<String>,
    company: Option<String>,
    first_time_user: bool,
    username: Option<String>,
}

#[cfg(feature = "enterprise")]
async fn export_global_users(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<Vec<ExportedGlobalUser>> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;
    let users = sqlx::query_as!(
        ExportedGlobalUser,
        "SELECT email, password_hash, login_type, super_admin, verified, name, company, first_time_user, username FROM password"
    )
    .fetch_all(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.export_export",
        ActionKind::Execute,
        "global",
        None,
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(users))
}

#[cfg(not(feature = "enterprise"))]
async fn export_global_users() -> JsonResult<String> {
    Err(Error::BadRequest(
        "This feature is only available in the enterprise version".to_string(),
    ))
}

#[cfg(feature = "enterprise")]
async fn overwrite_global_users(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(users): Json<Vec<ExportedGlobalUser>>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;
    sqlx::query!("DELETE FROM password")
        .execute(&mut *tx)
        .await?;
    for user in users {
        sqlx::query!(
            "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, company, first_time_user, username)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            user.email,
            user.password_hash,
            user.login_type,
            user.super_admin,
            user.verified,
            user.name,
            user.company,
            user.first_time_user,
            user.username
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed,
        "users.import_global",
        ActionKind::Create,
        "global",
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok("loaded global users".to_string())
}

#[cfg(not(feature = "enterprise"))]
async fn overwrite_global_users() -> JsonResult<String> {
    Err(Error::BadRequest(
        "This feature is only available in the enterprise version".to_string(),
    ))
}

// NOTE: rename_user, update_username_in_workpsace, and RenameUser
// are in windmill-api (depend on EE secret_backend_ext)

#[derive(Deserialize)]
pub struct RequestPasswordReset {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPassword {
    pub token: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct PasswordResetResponse {
    pub message: String,
}

// Password Reset Functions

/// Check if SMTP is configured
async fn is_smtp_configured(Extension(db): Extension<DB>) -> JsonResult<bool> {
    let smtp = windmill_common::server::load_smtp_config(&db).await?;
    Ok(Json(smtp.is_some()))
}

/// Request a password reset email
async fn request_password_reset(
    Extension(db): Extension<DB>,
    Json(req): Json<RequestPasswordReset>,
) -> Result<Json<PasswordResetResponse>> {
    let email = req.email.to_lowercase();

    // Check if SMTP is configured
    let smtp = windmill_common::server::load_smtp_config(&db).await?;
    let smtp = smtp.ok_or_else(|| {
        Error::BadRequest("SMTP is not configured. Password reset is not available.".to_string())
    })?;

    // Check if user exists with password login type
    let user_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM password WHERE email = $1 AND login_type = 'password')",
        &email
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    // Always return success to prevent email enumeration
    // But only send email if user exists
    if user_exists {
        // Generate a secure token
        let token = rd_string(32);

        // Delete any existing tokens for this email
        sqlx::query!("DELETE FROM magic_link WHERE email = $1", &email)
            .execute(&db)
            .await?;

        // Insert new token with 1 hour expiration
        sqlx::query!(
            "INSERT INTO magic_link (email, token, expiration) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
            &email,
            &token
        )
        .execute(&db)
        .await?;

        // Get the base URL for the reset link
        let base_url = BASE_URL.read().await.clone();
        let base_url = if base_url.is_empty() {
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost".to_string())
        } else {
            base_url
        };

        let reset_link = format!("{}/user/reset-password?token={}", base_url, token);

        // Send the email
        let subject = "Windmill Password Reset";
        let content = format!(
            "You have requested a password reset for your Windmill account.\n\n\
            Click the link below to reset your password:\n\
            {}\n\n\
            This link will expire in 1 hour.\n\n\
            If you did not request this password reset, you can safely ignore this email.",
            reset_link
        );

        // Send the email - don't fail the request if email fails
        if let Err(e) = windmill_common::email_oss::send_email_plain_text(
            subject,
            &content,
            vec![email.clone()],
            smtp,
            Some(Duration::from_secs(10)),
        )
        .await
        {
            tracing::error!("Failed to send password reset email to {}: {:?}", email, e);
        }
    }

    // Always return success to prevent email enumeration
    Ok(Json(PasswordResetResponse {
        message: "If an account with that email exists, a password reset link has been sent."
            .to_string(),
    }))
}

// NOTE: reset_password is in windmill-api (depends on users_oss::hash_password EE dispatch)
