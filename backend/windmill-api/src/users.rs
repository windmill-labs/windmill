/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#![allow(non_snake_case)]

use quick_cache::sync::Cache;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::db::ApiAuthed;

pub use crate::auth::Tokened;

use crate::scopes::ScopeDefinition;
use crate::utils::{
    generate_instance_wide_unique_username, get_instance_username_or_create_pending,
};
use crate::{
    auth::ExpiringAuthCache, db::DB, utils::require_super_admin, webhook_util::WebhookShared,
    COOKIE_DOMAIN, IS_SECURE,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    async_trait,
    extract::{Extension, FromRequestParts, Path, Query},
    http::request::Parts,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::{header::LOCATION, StatusCode};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use tower_cookies::{Cookie, Cookies};
use tracing::Instrument;
use windmill_audit::audit_oss::{audit_log, AuditAuthor};
use windmill_audit::ActionKind;
use windmill_common::auth::fetch_authed_from_permissioned_as;
use windmill_common::global_settings::AUTOMATE_USERNAME_CREATION_SETTING;
use windmill_common::oauth2::InstanceEvent;
use windmill_common::users::COOKIE_NAME;
use windmill_common::users::{truncate_token, username_to_permissioned_as};
use windmill_common::utils::paginate;
use windmill_common::worker::CLOUD_HOSTED;
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
        .route("/setpassword", post(set_password))
        .route("/set_password_of/:user", post(set_password_of_user))
        .route("/set_login_type/:user", post(set_login_type))
        .route("/create", post(create_user))
        .route("/update/:user", post(update_user))
        .route("/delete/:user", delete(delete_user))
        .route("/username_info/:user", get(get_instance_username_info))
        .route("/rename/:user", post(rename_user))
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

    // .route("/list_invite_codes", get(list_invite_codes))
    // .route("/create_invite_code", post(create_invite_code))
    // .route("/signup", post(signup))
    // .route("/lost_password", post(lost_password))
    // .route("/use_magic_link", get(use_magic_link))
}

pub fn make_unauthed_service() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout).get(logout))
        .route("/is_first_time_setup", get(is_first_time_setup))
}

pub async fn maybe_refresh_folders(
    path: &str,
    w_id: &str,
    authed: ApiAuthed,
    db: &DB,
) -> ApiAuthed {
    if authed.is_admin {
        return authed;
    }
    let splitted = path.split('/').collect::<Vec<_>>();
    if splitted.len() >= 2
        && splitted[0] == "f"
        && !authed.folders.iter().any(|(f, _, _)| f == splitted[1])
    {
        let name = &authed.username;
        let groups = get_groups_for_user(w_id, name, &authed.email, db)
            .await
            .ok()
            .unwrap_or_default();

        let folders = get_folders_for_user(w_id, name, &groups, db)
            .await
            .ok()
            .unwrap_or_default();
        ApiAuthed { folders, ..authed }
    } else {
        authed
    }
}

pub fn check_scopes<F>(authed: &ApiAuthed, required: F) -> error::Result<()>
where
    F: FnOnce() -> String,
{
    if let Some(scopes) = authed.scopes.as_ref() {
        let mut has_other_scope = false;
        let required_scope = ScopeDefinition::from_scope_string(&required())?;
        for scope in scopes {
            if !scope.starts_with("if_jobs:filter_tags:") {
                if !has_other_scope {
                    has_other_scope = true;
                }

                match ScopeDefinition::from_scope_string(scope) {
                    Ok(scope) if scope.includes(&required_scope) => return Ok(()),
                    _ => {}
                }
            }
        }

        if has_other_scope {
            return Err(Error::BadRequest(format!(
                "Required scope: {}",
                required_scope.as_string()
            )));
        }
    }
    Ok(())
}

pub fn get_scope_tags(authed: &ApiAuthed) -> Option<Vec<&str>> {
    authed.scopes.as_ref()?.iter().find_map(|s| {
        if s.starts_with("if_jobs:filter_tags:") {
            Some(
                s.trim_start_matches("if_jobs:filter_tags:")
                    .split(",")
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        }
    })
}

#[derive(Clone, Debug)]
pub struct OptAuthed(pub Option<ApiAuthed>);

#[async_trait]
impl<S> FromRequestParts<S> for OptAuthed
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        ApiAuthed::from_request_parts(parts, state)
            .await
            .map(|authed| Self(Some(authed)))
            .or_else(|_| Ok(Self(None)))
    }
}

#[allow(unused)]
pub async fn fetch_api_authed(
    username: String,
    email: String,
    w_id: &str,
    db: &DB,
    username_override: Option<String>,
) -> error::Result<ApiAuthed> {
    let permissioned_as = username_to_permissioned_as(username.as_str());
    fetch_api_authed_from_permissioned_as(permissioned_as, email, w_id, db, username_override).await
}

lazy_static::lazy_static! {
    static ref API_AUTHED_CACHE: Cache<(String,String,String), ExpiringAuthCache> = Cache::new(300);
}

#[allow(unused)]
pub async fn fetch_api_authed_from_permissioned_as(
    permissioned_as: String,
    email: String,
    w_id: &str,
    db: &DB,
    username_override: Option<String>,
) -> error::Result<ApiAuthed> {
    let key = (w_id.to_string(), permissioned_as.clone(), email.clone());

    let mut api_authed = match API_AUTHED_CACHE.get(&key) {
        Some(expiring_authed) if expiring_authed.expiry > chrono::Utc::now() => {
            tracing::debug!("API authed cache hit for user {}", email);
            expiring_authed.authed
        }
        _ => {
            tracing::debug!("API authed cache miss for user {}", email);
            let authed =
                fetch_authed_from_permissioned_as(permissioned_as, email.clone(), w_id, db).await?;

            let api_authed = ApiAuthed {
                username: authed.username,
                email: email,
                is_admin: authed.is_admin,
                is_operator: authed.is_operator,
                groups: authed.groups,
                folders: authed.folders,
                scopes: authed.scopes,
                username_override: None,
            };

            API_AUTHED_CACHE.insert(
                key,
                ExpiringAuthCache {
                    authed: api_authed.clone(),
                    expiry: chrono::Utc::now() + chrono::Duration::try_seconds(120).unwrap(),
                },
            );

            api_authed
        }
    };

    api_authed.username_override = username_override;
    Ok(api_authed)
}

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

#[derive(Deserialize)]
pub struct NewToken {
    pub label: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub impersonate_email: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub workspace_id: Option<String>,
}

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
        FROM usr
            , LATERAL (
            SELECT COALESCE(SUM(duration_ms + 1000)/1000 , 0)::BIGINT executions
                FROM v2_as_completed_job
                WHERE workspace_id = $1
                AND job_kind NOT IN ('flow', 'flowpreview', 'flownode')
                AND email = usr.email
                AND now() - '1 week'::interval < created_at 
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
            SELECT email, email NOT IN (SELECT email FROM authors) as operator_only, login_type::text, verified, super_admin, devops, name, company, username
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
            "SELECT email, login_type::text, verified, super_admin, devops, name, company, username, NULL::bool as operator_only FROM password ORDER BY super_admin DESC, devops DESC, email LIMIT \
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
}
async fn get_tutorial_progress(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Progress> {
    let res = sqlx::query_scalar!(
        "SELECT progress::bigint FROM tutorial_progress WHERE email = $1",
        authed.email
    )
    .fetch_optional(&db)
    .await?
    .flatten()
    .unwrap_or_default() as u64;
    Ok(Json(Progress { progress: res }))
}

async fn update_tutorial_progress(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(progress): Json<Progress>,
) -> Result<String> {
    sqlx::query_scalar!(
        "INSERT INTO tutorial_progress VALUES ($2, $1::bigint::bit(64)) ON CONFLICT (email) DO UPDATE SET progress = $1::bigint::bit(64)",
        progress.progress as i64,
        authed.email
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
        "SELECT * from workspace_invite WHERE email = $1",
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
            &AuditAuthor { email: email.clone(), username: email, username_override: None },
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
        "SELECT email, login_type::TEXT, super_admin, devops, verified, name, company, username, NULL::bool as operator_only FROM password WHERE \
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

pub fn require_owner_of_path(authed: &ApiAuthed, path: &str) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    if !path.is_empty() {
        let splitted = path.split("/").collect::<Vec<&str>>();
        if splitted[0] == "u" {
            if splitted[1] == authed.username {
                Ok(())
            } else {
                Err(Error::BadRequest(format!(
                    "only the owner {} is authorized to perform this operation",
                    splitted[1]
                )))
            }
        } else if splitted[0] == "f" {
            crate::folders::require_is_owner(authed, splitted[1])
        } else {
            Err(Error::BadRequest(format!(
                "Not recognized path kind: {}",
                path
            )))
        }
    } else {
        Err(Error::BadRequest(format!(
            "Cannot be owner of an empty path"
        )))
    }
}

pub fn get_perm_in_extra_perms_for_authed(
    v: serde_json::Value,
    authed: &ApiAuthed,
) -> Option<bool> {
    match v {
        serde_json::Value::Object(obj) => {
            let mut keys = vec![format!("u/{}", authed.username)];
            for g in authed.groups.iter() {
                keys.push(format!("g/{}", g));
            }
            let mut res = None;
            for k in keys {
                if let Some(v) = obj.get(&k) {
                    if let Some(v) = v.as_bool() {
                        if v {
                            return Some(true);
                        }
                        res = Some(v);
                    }
                }
            }
            res
        }
        _ => None,
    }
}

pub async fn require_is_writer(
    authed: &ApiAuthed,
    path: &str,
    w_id: &str,
    db: DB,
    query: &str,
    kind: &str,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    if !path.is_empty() {
        if require_owner_of_path(authed, path).is_ok() {
            return Ok(());
        }
        if path.starts_with("f/") && path.split('/').count() >= 2 {
            let folder = path.split('/').nth(1).unwrap();
            let extra_perms = sqlx::query_scalar!(
                "SELECT extra_perms FROM folder WHERE name = $1 AND workspace_id = $2",
                folder,
                w_id
            )
            .fetch_optional(&db)
            .await?;
            if let Some(perms) = extra_perms {
                let is_folder_writer =
                    get_perm_in_extra_perms_for_authed(perms, authed).unwrap_or(false);
                if is_folder_writer {
                    return Ok(());
                }
            }
        }
        let extra_perms = sqlx::query_scalar(query)
            .bind(path)
            .bind(w_id)
            .fetch_optional(&db)
            .await?;
        if let Some(perms) = extra_perms {
            let perm = get_perm_in_extra_perms_for_authed(perms, authed);
            match perm {
                Some(true) => Ok(()),
                Some(false) => Err(Error::BadRequest(format!(
                    "User {} is not a writer of {kind} path {path}",
                    authed.username
                ))),
                None => Err(Error::BadRequest(format!(
                    "User {} has neither read or write permission on {kind} {path}",
                    authed.username
                ))),
            }
        } else {
            Err(Error::BadRequest(format!(
                "{path} does not exist yet and user {} is not an owner of the parent folder",
                authed.username
            )))
        }
    } else {
        Err(Error::BadRequest(format!(
            "Cannot be writer of an empty path"
        )))
    }
}
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

lazy_static! {
    pub static ref VALID_USERNAME: Regex = Regex::new(r#"^[a-zA-Z][a-zA-Z_0-9]*$"#).unwrap();
}

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
    .unwrap_or(false);

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

    if let Some(a) = eu.is_admin {
        sqlx::query_scalar!(
            "UPDATE usr SET is_admin = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            &username_to_update,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(a) = eu.operator {
        sqlx::query_scalar!(
            "UPDATE usr SET operator = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            &username_to_update,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(a) = eu.disabled {
        sqlx::query_scalar!(
            "UPDATE usr SET disabled = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            &username_to_update,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.update",
        ActionKind::Update,
        &w_id,
        Some(&username_to_update),
        None,
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
    )
    .await?;

    Ok(format!("user {} updated", user_email))
}

async fn update_user(
    authed: ApiAuthed,
    Path(email_to_update): Path<String>,
    Extension(db): Extension<DB>,
    Json(eu): Json<EditUser>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let mut tx = db.begin().await?;

    if let Some(sa) = eu.is_super_admin {
        sqlx::query_scalar!(
            "UPDATE password SET super_admin = $1 WHERE email = $2",
            sa,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(dv) = eu.is_devops {
        sqlx::query_scalar!(
            "UPDATE password SET devops = $1 WHERE email = $2",
            dv,
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

async fn create_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(nu): Json<NewUser>,
) -> Result<(StatusCode, String)> {
    crate::users_oss::create_user(authed, db, webhook, argon2, nu).await
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

    sqlx::query_scalar!(
        "DELETE FROM usr WHERE email = $1 AND workspace_id = $2",
        email_to_delete,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM usr_to_group WHERE usr = $1 AND workspace_id = $2",
        &username_to_delete,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.delete",
        ActionKind::Delete,
        &w_id,
        Some(&username_to_delete),
        None,
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
    )
    .await?;

    Ok(format!("username {} deleted", username_to_delete))
}

async fn set_password(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    authed: ApiAuthed,
    Json(ep): Json<EditPassword>,
) -> Result<String> {
    let email = authed.email.clone();
    crate::users_oss::set_password(db, argon2, authed, &email, ep).await
}

async fn set_password_of_user(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Path(email): Path<String>,
    authed: ApiAuthed,
    Json(ep): Json<EditPassword>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    crate::users_oss::set_password(db, argon2, authed, &email, ep).await
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

async fn login(
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(Login { email, password }): Json<Login>,
) -> Result<String> {
    let mut tx = db.begin().await?;
    let email = email.to_lowercase();
    let audit_author =
        AuditAuthor { email: email.clone(), username: email.clone(), username_override: None };
    let email_w_h: Option<(String, String, bool, bool)> = sqlx::query_as(
        "SELECT email, password_hash, super_admin, first_time_user FROM password WHERE email = $1 AND login_type = \
         'password'",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some((email, hash, super_admin, first_time_user)) = email_w_h {
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
            if first_time_user {
                sqlx::query_scalar!(
                    "UPDATE password SET first_time_user = false WHERE email = $1",
                    &email
                )
                .execute(&mut *tx)
                .await?;
                let mut c = Cookie::new("first_time", "1");
                if let Some(domain) = COOKIE_DOMAIN.as_ref() {
                    c.set_domain(domain);
                }
                c.set_secure(false);
                c.set_expires(time::OffsetDateTime::now_utc() + time::Duration::minutes(15));
                c.set_http_only(false);
                c.set_path("/");

                cookies.add(c);
            }

            let token = create_session_token(&email, super_admin, &mut tx, cookies).await?;

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

async fn create_token(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(new_token): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    let token = rd_string(32);
    let mut tx = db.begin().await?;

    let is_super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);
    if *CLOUD_HOSTED {
        let nb_tokens =
            sqlx::query_scalar!("SELECT COUNT(*) FROM token WHERE email = $1", &authed.email)
                .fetch_one(&db)
                .await?;
        if nb_tokens.unwrap_or(0) >= 10000 {
            return Err(Error::BadRequest(
                "You have reached the maximum number of tokens (10000) on cloud. Contact support@windmill.dev to increase the limit"
                    .to_string(),
            ));
        }
    }
    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin, scopes, workspace_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        token,
        authed.email,
        new_token.label,
        new_token.expiration,
        is_super_admin,
        new_token.scopes.as_ref().map(|x| x.as_slice()),
        new_token.workspace_id,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.token.create",
        ActionKind::Create,
        &"global",
        Some(&token[0..10]),
        None,
    )
    .instrument(tracing::info_span!("token", email = &authed.email))
    .await?;
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
    endpoint_openai_sync: String,
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
    Extension(cache): Extension<Arc<crate::auth::AuthCache>>,
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
                    endpoint_openai_sync: format!(
                        "/w/{}/jobs/openai_sync/f/{}",
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
        "SELECT workspace_id as workspace, path, summary, description, schema FROM script as o WHERE created_at = (select max(created_at) from script where o.path = path and workspace_id = $1) and workspace_id = $1", workspace
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
                    endpoint_openai_sync: format!(
                        "/w/{}/jobs/openai_sync/p/{}",
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

#[derive(Deserialize)]
struct RenameUser {
    new_username: String,
}

async fn rename_user(
    authed: ApiAuthed,
    Path(user_email): Path<String>,
    Extension(db): Extension<DB>,
    Json(ru): Json<RenameUser>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    let mut tx = db.begin().await?;

    let username_conflict = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 and email != $2 UNION SELECT 1 FROM password WHERE username = $1 UNION SELECT 1 FROM pending_user WHERE username = $1)",
        &ru.new_username,
        &user_email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if username_conflict {
        return Err(Error::BadRequest(format!(
            "username {} already used by another user",
            &ru.new_username
        )));
    }

    if !VALID_USERNAME.is_match(&ru.new_username) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
        )));
    }

    sqlx::query!(
        "UPDATE password SET username = $1 WHERE email = $2",
        ru.new_username,
        user_email
    )
    .execute(&mut *tx)
    .await?;

    let workspace_usernames = sqlx::query!(
        "SELECT workspace_id, username FROM usr WHERE email = $1",
        &user_email
    )
    .fetch_all(&mut *tx)
    .await?;

    for w_u in workspace_usernames {
        if ru.new_username == w_u.username {
            continue;
        }
        update_username_in_workpsace(
            &mut tx,
            &user_email,
            &w_u.username,
            &ru.new_username,
            &w_u.workspace_id,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.rename",
        ActionKind::Update,
        "global",
        Some(&user_email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!(
        "updated username of user {} to {}",
        &user_email, &ru.new_username
    ))
}

async fn update_username_in_workpsace<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    email: &str,
    old_username: &str,
    new_username: &str,
    w_id: &str,
) -> error::Result<()> {
    // ---- instance and workspace users ----
    sqlx::query!(
        "UPDATE usr SET username = $1 WHERE email = $2",
        new_username,
        email
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE usr_to_group SET usr = $1 WHERE usr = $2",
        new_username,
        old_username
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job ----
    sqlx::query!(
        r#"UPDATE v2_job SET runnable_path = REGEXP_REPLACE(runnable_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE v2_job SET trigger = REGEXP_REPLACE(trigger,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE trigger LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET permissioned_as = ('u/' || $1) WHERE permissioned_as = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job_queue ----
    sqlx::query!(
        "UPDATE v2_job_queue SET canceled_by = $1 WHERE canceled_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job_completed ----
    sqlx::query!(
        "UPDATE v2_job_completed SET canceled_by = $1 WHERE canceled_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- resources----
    sqlx::query!(
        r#"UPDATE resource SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE resource_type SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE resource SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE resource SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- variables ----

    sqlx::query!(
        r#"UPDATE variable SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE variable SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- scripts ----
    sqlx::query!(
        r#"UPDATE script SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE script SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE script SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- flows ----
    sqlx::query!(
        r#"INSERT INTO flow
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at) 
        SELECT workspace_id, REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1'), summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
            FROM flow 
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE flow_version SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET flow_path = REGEXP_REPLACE(flow_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE flow_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET app_path = REGEXP_REPLACE(app_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE app_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET runnable_path = REGEXP_REPLACE(runnable_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE flow_node SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM flow WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $2",
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE flow SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- draft ----
    sqlx::query!(
        r#"UPDATE draft SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['path'], to_jsonb(REGEXP_REPLACE(value->>'path','u/' || $2 || '/(.*)','u/' || $1 || '/\1')))) WHERE value->>'path' LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    // ---- app ----
    sqlx::query!(
        r#"UPDATE app SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'], to_jsonb('u/' || $1)) WHERE policy->>'on_behalf_of' = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- app_version ----

    sqlx::query!(
        "UPDATE app_version SET created_by = $1 WHERE created_by = $2 AND EXISTS (SELECT 1 FROM app WHERE workspace_id = $3 AND app.id = app_version.app_id)",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- schedules ----

    sqlx::query!(
        r#"UPDATE schedule SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE schedule SET script_path = REGEXP_REPLACE(script_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE script_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE schedule SET edited_by = $1 WHERE edited_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE schedule SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- group_ ----

    sqlx::query!(
        "UPDATE group_ SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- folders ----

    sqlx::query!(
        "UPDATE folder SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE folder SET owners = ARRAY_REPLACE(owners, 'u/' || $2, 'u/' || $1) WHERE  ('u/' || $2) = ANY(owners) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE folder SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- input ----

    sqlx::query!(
        "UPDATE input SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- favorite ----

    sqlx::query!(
        "UPDATE favorite SET usr = $1 WHERE usr = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- capture ----

    sqlx::query!(
        "UPDATE capture SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- token ----

    sqlx::query!(
        "UPDATE token SET owner = ('u/' || $1) WHERE owner = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        r#"UPDATE token SET scopes = array(select regexp_replace(unnest(scopes), 'run:([^/]+)/u/' || $2 || '/(.+)', 'run:\1/u/' || $1 || '/\2')) WHERE EXISTS (SELECT 1 FROM UNNEST(scopes) scope WHERE scope LIKE ('run:%/u/' || $2 || '/%')) AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- raw_app ----

    sqlx::query!(
        "UPDATE raw_app SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
