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
use std::sync::LazyLock;
use std::time::Duration;

use windmill_api_auth::ApiAuthed;

pub use windmill_api_auth::Tokened;

use argon2::{Argon2, PasswordVerifier};
use axum::{
    extract::{Extension, Path, Query},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::{header::LOCATION, StatusCode};
use windmill_api_auth::{
    forbid_elevated_job_token, forbid_job_token_account_destruction, forbid_superadmin_job_token,
    require_super_admin, OptJobAuthed,
};
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
use windmill_common::auth::{safe_token_prefix, TOKEN_PREFIX_LEN};
use windmill_common::global_settings::AUTOMATE_USERNAME_CREATION_SETTING;
use windmill_common::oauth2::InstanceEvent;
use windmill_common::per_minute_counter::PerMinuteCounter;
use windmill_common::users::truncate_token;
use windmill_common::users::COOKIE_NAME;
use windmill_common::users::{
    username_to_permissioned_as, PERMISSIONED_AS_MAX_LEN, SUPERADMIN_NOTIFICATION_EMAIL,
    SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL, VALID_EMAIL,
};
use windmill_common::utils::paginate;
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::{
    auth::{get_folders_for_user, get_groups_for_user},
    db::UserDB,
    error::{self, Error, JsonResult, Result},
    utils::{
        escape_ilike_pattern, not_found_if_none, rd_string, require_admin, Pagination, StripPath,
    },
};
use windmill_common::{BASE_URL, HUB_BASE_URL};
use windmill_git_sync::handle_deployment_metadata;

pub const COOKIE_PATH: &str = "/";

const TOKEN_CREATE_LIMIT_PER_MINUTE: u32 = 10;

static TOKEN_CREATE_RATE_LIMIT: LazyLock<PerMinuteCounter<String>> =
    LazyLock::new(PerMinuteCounter::new);

fn check_token_create_rate_limit(username: &str) -> Result<()> {
    if !*CLOUD_HOSTED {
        return Ok(());
    }

    if TOKEN_CREATE_RATE_LIMIT.try_increment(username.to_string(), TOKEN_CREATE_LIMIT_PER_MINUTE) {
        return Ok(());
    }

    Err(Error::Generic(
        StatusCode::TOO_MANY_REQUESTS,
        "Too many token creation requests. Please try again later.".to_string(),
    ))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_users))
        .route("/list_addable", get(list_addable_instance_users))
        .route("/list_usage", get(list_user_usage))
        .route("/list_usernames", get(list_usernames))
        .route("/exists", post(exists_username))
        .route("/get/{user}", get(get_workspace_user))
        .route("/update/{user}", post(update_workspace_user))
        .route("/delete/{user}", delete(delete_workspace_user))
        .route("/convert_to_group/{user}", post(convert_user_to_group))
        .route("/is_owner/{*path}", get(is_owner_of_path))
        .route("/whois/{username}", get(whois))
        .route("/whoami", get(whoami))
        .route("/leave", post(leave_workspace))
        .route("/username_to_email/{username}", get(username_to_email))
        .route(
            "/impersonate_service_account",
            post(impersonate_service_account),
        )
        .route("/exit_impersonation", post(exit_impersonation))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/exists/{email}", get(exists_email))
        .route("/email", get(get_email))
        .route("/whoami", get(global_whoami))
        .route("/list_invites", get(list_invites))
        .route("/decline_invite", post(decline_invite))
        .route("/accept_invite", post(accept_invite))
        .route("/list_as_super_admin", get(list_users_as_super_admin))
        .route("/set_login_type/{user}", post(set_login_type))
        .route("/update/{user}", post(update_user))
        .route("/delete/{user}", delete(delete_user))
        .route("/username_info/{user}", get(get_instance_username_info))
        .route("/change_email/{user}", post(change_user_email))
        .route("/tokens/create", post(create_token))
        .route("/tokens/delete/{token_prefix}", delete(delete_token))
        .route(
            "/tokens/update_scopes/{token_prefix}",
            post(update_token_scopes),
        )
        .route(
            "/tokens/update_label/{token_prefix}",
            post(update_token_label),
        )
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
        .route(
            "/is_password_login_disabled",
            get(is_password_login_disabled),
        )
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
    pub is_service_account: bool,
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
    /// Populated only for service-account rows (which are workspace-scoped).
    /// `None` for password users since their admin status varies per workspace
    /// and is not surfaced by this aggregation.
    #[serde(skip_serializing_if = "Option::is_none")]
    is_workspace_admin: Option<bool>,
    first_time_user: bool,
    role_source: String,
    disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    pub is_service_account: bool,
    // True when this row is a superadmin viewing a workspace they are not a
    // member of (so `is_admin`/`role` reflect the superadmin fallback, not an
    // actual membership). Always false for real member rows.
    #[serde(default)]
    pub non_member: bool,
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
    pub disabled: Option<bool>,
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
    pub token_prefix: String,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub scopes: Option<Vec<String>>,
    pub workspace_id: Option<String>,
    pub read_only: bool,
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
        SELECT workspace_id, username, email, is_admin, created_at, operator, disabled, role, added_via, is_service_account
          FROM usr
         WHERE workspace_id = $1
         ORDER BY email
         ",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize)]
struct AddableInstanceUser {
    email: String,
    username: Option<String>,
}

#[derive(Deserialize)]
struct AddableInstanceUsersQuery {
    search: Option<String>,
    per_page: Option<i64>,
}

/// Instance accounts that can still be added to `w_id`, for the member picker. Service accounts
/// live in `usr` only, so selecting from `password` leaves them out.
async fn list_addable_instance_users(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(AddableInstanceUsersQuery { search, per_page }): Query<AddableInstanceUsersQuery>,
) -> JsonResult<Vec<AddableInstanceUser>> {
    require_super_admin(&db, &authed).await?;
    let per_page = per_page.unwrap_or(10).clamp(1, 100);
    // An absent search yields '%%', which matches every row.
    let search = format!(
        "%{}%",
        escape_ilike_pattern(search.as_deref().unwrap_or_default())
    );

    // Every exclusion is part of the query so that the limit counts addable accounts only.
    let rows = sqlx::query_as!(
        AddableInstanceUser,
        "SELECT email, username FROM password
         WHERE disabled IS false
           AND (email ILIKE $2 OR username ILIKE $2)
           AND NOT EXISTS (SELECT 1 FROM usr WHERE usr.workspace_id = $1 AND usr.email = password.email)
         ORDER BY email
         LIMIT $3",
        w_id,
        search,
        per_page
    )
    .fetch_all(&db)
    .await?;

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
                AND j.kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlestepflow')
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
    require_super_admin(&db, &authed).await?;
    let per_page = pagination.per_page.unwrap_or(10000).max(1);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;

    let rows = if active_only.is_some_and(|x| x) {
        sqlx::query_as!(
            GlobalUserInfo,
            r#"WITH active_users AS (SELECT distinct username as email FROM (SELECT username, timestamp, operation FROM audit_partitioned UNION ALL SELECT username, timestamp, operation FROM audit) AS a WHERE timestamp > NOW() - INTERVAL '1 month' AND (operation = 'users.login' OR operation = 'oauth.login' OR operation = 'users.token.refresh')),
            authors as (SELECT distinct email FROM usr WHERE usr.operator IS false)
            SELECT email as "email!", (email NOT IN (SELECT email FROM authors)) as operator_only, NULL::bool as is_workspace_admin, login_type::text, verified as "verified!", super_admin as "super_admin!", devops as "devops!", name, company, username, first_time_user as "first_time_user!", role_source as "role_source!", disabled as "disabled!", NULL::text as workspace_id
            FROM password
            WHERE email IN (SELECT email FROM active_users)
            UNION ALL
            SELECT email as "email!", operator as operator_only, is_admin as is_workspace_admin, 'service_account'::text as login_type, true as "verified!", false as "super_admin!", false as "devops!", NULL::text as name, NULL::text as company, username, false as "first_time_user!", 'service_account'::text as "role_source!", disabled as "disabled!", workspace_id
            FROM usr
            WHERE is_service_account IS true
            ORDER BY "super_admin!" DESC, "devops!" DESC
            LIMIT $1 OFFSET $2"#,
            per_page as i32,
            offset as i32
        )
        .fetch_all(&db)
        .await?
    } else {
        sqlx::query_as!(
            GlobalUserInfo,
            r#"SELECT email as "email!", login_type::text, verified as "verified!", super_admin as "super_admin!", devops as "devops!", name, company, username, NULL::bool as operator_only, NULL::bool as is_workspace_admin, first_time_user as "first_time_user!", role_source as "role_source!", disabled as "disabled!", NULL::text as workspace_id FROM password
            UNION ALL
            SELECT email as "email!", 'service_account'::text as login_type, true as "verified!", false as "super_admin!", false as "devops!", NULL::text as name, NULL::text as company, username, operator as operator_only, is_admin as is_workspace_admin, false as "first_time_user!", 'service_account'::text as "role_source!", disabled as "disabled!", workspace_id
            FROM usr
            WHERE is_service_account IS true
            ORDER BY "super_admin!" DESC, "devops!" DESC, "email!"
            LIMIT $1 OFFSET $2"#,
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
    let t_hash = windmill_common::auth::hash_token(&token);
    let t_prefix = token.get(..TOKEN_PREFIX_LEN).unwrap_or(&token);

    let email = if *INVALIDATE_ALL_SESSIONS_ON_LOGOUT {
        // A guest's browser session is a session too: this is its one user-driven revocation.
        sqlx::query_scalar::<_, Option<String>>(
            "WITH email_lookup AS (
                SELECT email FROM token WHERE token_hash = $1
            )
            DELETE FROM token
            WHERE email = (SELECT email FROM email_lookup)
                AND label IN ('session', 'guest_session')
            RETURNING email",
        )
        .bind(&t_hash)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_scalar!(
            "DELETE FROM token WHERE token_hash = $1 RETURNING email",
            t_hash
        )
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
                token_prefix: Some(t_prefix.to_string()),
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
        if is_valid_logout_redirect(&rd).await {
            Ok((StatusCode::TEMPORARY_REDIRECT, [(LOCATION, rd)]).into_response())
        } else {
            tracing::warn!("Blocked logout redirect to non-whitelisted URL: {}", rd);
            Ok((StatusCode::OK, "logged out successfully".to_string()).into_response())
        }
    } else {
        Ok((StatusCode::OK, "logged out successfully".to_string()).into_response())
    }
}

async fn is_valid_logout_redirect(rd: &str) -> bool {
    // Allow relative paths (same-origin redirects)
    if rd.starts_with('/') && !rd.starts_with("//") {
        return true;
    }
    let parsed = match url::Url::parse(rd) {
        Ok(u) => u,
        Err(_) => return false,
    };
    let host: &str = match parsed.host_str() {
        Some(h) => h,
        None => return false,
    };
    if host == "windmill.dev" || host.ends_with(".windmill.dev") {
        return true;
    }
    let hub_url = (**HUB_BASE_URL.load()).clone();
    if let Ok(hub_parsed) = url::Url::parse(&hub_url) {
        if let Some(hub_host) = hub_parsed.host_str() {
            if host == hub_host {
                return true;
            }
        }
    }
    false
}

async fn whoami(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> JsonResult<UserInfo> {
    let is_guest = windmill_api_auth::scopes::has_guest_sentinel(authed.scopes.as_deref());
    let ApiAuthed { username, email, is_admin, groups, folders, .. } = authed;
    // A guest would otherwise fall through to the non-member branch below and be
    // handed a `superadmin` role. Answer it here, as the operator-shaped identity it is.
    if is_guest {
        return Ok(Json(UserInfo {
            workspace_id: w_id,
            email,
            username,
            name: None,
            is_admin: false,
            is_super_admin: false,
            created_at: chrono::Utc::now(),
            groups: vec![],
            operator: true,
            disabled: false,
            role: Some("guest".to_string()),
            folders_read: vec![],
            folders: vec![],
            folders_owners: vec![],
            is_service_account: false,
            non_member: true,
        }));
    }
    let user = get_user(&w_id, &username, &db).await?;
    // Only treat the row as "this user is a member" when its email matches; the
    // derived username is instance-unique so a match on a different email should
    // never happen, but guard against it so a non-member superadmin is never
    // shown another member's identity/role.
    if let Some(user) = user.filter(|u| u.email == email) {
        Ok(Json(user))
    } else {
        Ok(Json(UserInfo {
            workspace_id: w_id,
            email,
            username,
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
            is_service_account: false,
            non_member: true,
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
        "SELECT email, login_type::TEXT, super_admin, devops, verified, name, company, username, NULL::bool as operator_only, NULL::bool as is_workspace_admin, first_time_user, role_source, disabled, NULL::text as workspace_id FROM password WHERE \
         email = $1",
        email
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| Error::internal_err(format!("fetching global identity: {e:#}")))?;

    if let Some(user) = user {
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
            is_workspace_admin: None,
            first_time_user: false,
            role_source: "manual".to_string(),
            disabled: false,
            workspace_id: None,
        }))
    } else {
        // Service accounts don't have a password row. The SA email is unique
        // per (workspace, username) and pinpoints a single usr row, so we can
        // surface its real role rather than pinning to operator.
        let sa_role = sqlx::query!(
            "SELECT operator, is_admin FROM usr WHERE email = $1 AND is_service_account IS true LIMIT 1",
            email
        )
        .fetch_optional(&db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching service-account role: {e:#}")))?;

        Ok(Json(GlobalUserInfo {
            email: email.clone(),
            login_type: Some("service_account".to_string()),
            super_admin: false,
            devops: false,
            verified: true,
            name: None,
            company: None,
            username: None,
            operator_only: sa_role.as_ref().map(|r| r.operator).or(Some(true)),
            is_workspace_admin: sa_role.as_ref().map(|r| r.is_admin),
            first_time_user: false,
            role_source: "service_account".to_string(),
            disabled: false,
            workspace_id: None,
        }))
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
    pub is_service_account: bool,
}

async fn get_user(w_id: &str, username: &str, db: &DB) -> Result<Option<UserInfo>> {
    let user = sqlx::query_as!(
        User2,
        "SELECT usr.*, COALESCE(password.super_admin, false) as \"super_admin!\", password.name FROM usr LEFT JOIN password ON usr.email = password.email Where usr.username = $1 AND workspace_id = $2
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
        is_service_account: usr.is_service_account,
        non_member: false,
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
    forbid_job_token_account_destruction(&authed)?;
    let mut tx = db.begin().await?;
    sqlx::query!("DELETE FROM password WHERE email = $1", &authed.email)
        .execute(&mut *tx)
        .await?;
    windmill_common::user_drafts::delete_drafts_of_email(&mut *tx, &authed.email).await?;

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
        "SELECT workspace_id, username, email, is_admin, created_at, operator, disabled, role, added_via, is_service_account FROM usr WHERE username = $1 AND workspace_id = $2",
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

    // Determine the group with highest precedence (same logic as reconcile_workspace_instance_groups)
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
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Path(email_to_update): Path<String>,
    Extension(db): Extension<DB>,
    Json(eu): Json<EditUser>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;
    let mut tx = db.begin().await?;

    let mut new_super_admin: Option<bool> = None;
    if let Some(sa) = eu.is_super_admin {
        sqlx::query_scalar!(
            "UPDATE password SET super_admin = $1, role_source = 'manual' WHERE email = $2",
            sa,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
        new_super_admin = Some(sa);
    }

    if let Some(dv) = eu.is_devops {
        sqlx::query_scalar!(
            "UPDATE password SET devops = $1, role_source = 'manual' WHERE email = $2",
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

    // If the result is "user" (no elevation), recompute from instance groups.
    // Setting to "user" means "clear manual override, fall back to group role".
    // Manual elevated roles (devops/superadmin) are never overridden by groups.
    if eu.is_super_admin.is_some() || eu.is_devops.is_some() {
        let current = sqlx::query!(
            "SELECT super_admin, devops FROM password WHERE email = $1",
            &email_to_update
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(c) = current {
            if !c.super_admin && !c.devops {
                // Compute effective role from all instance groups
                let roles = sqlx::query_scalar!(
                    "SELECT ig.instance_role FROM email_to_igroup eig
                     JOIN instance_group ig ON ig.name = eig.igroup
                     WHERE eig.email = $1 AND ig.instance_role IS NOT NULL",
                    &email_to_update
                )
                .fetch_all(&mut *tx)
                .await?;

                let mut effective: Option<&str> = None;
                for role in roles.iter().flatten() {
                    match role.as_str() {
                        "superadmin" => {
                            effective = Some("superadmin");
                            break;
                        }
                        "devops" if effective.is_none() => {
                            effective = Some("devops");
                        }
                        _ => {}
                    }
                }

                if let Some(role) = effective {
                    let (sa, dv) = match role {
                        "superadmin" => (true, false),
                        _ => (false, true),
                    };
                    sqlx::query!(
                        "UPDATE password SET super_admin = $1, devops = $2, role_source = 'instance_group' WHERE email = $3",
                        sa, dv, &email_to_update
                    )
                    .execute(&mut *tx)
                    .await?;

                    // Re-invalidate tokens with the group role
                    sqlx::query!(
                        "DELETE FROM token WHERE email = $1 AND label = 'session'",
                        &email_to_update
                    )
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query!(
                        "UPDATE token SET super_admin = $1 WHERE email = $2 AND label != 'session'",
                        sa,
                        &email_to_update
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
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

    if let Some(d) = eu.disabled {
        #[cfg(feature = "enterprise")]
        if !d {
            if let Some(msg) =
                windmill_common::ee_oss::check_seat_cap_for_reactivation(&db, &email_to_update)
                    .await?
            {
                return Err(Error::BadRequest(msg));
            }
        }
        sqlx::query_scalar!(
            "UPDATE password SET disabled = $1 WHERE email = $2",
            d,
            &email_to_update
        )
        .execute(&mut *tx)
        .await?;
        if d {
            // Delete all tokens for immediate session revocation
            sqlx::query!("DELETE FROM token WHERE email = $1", &email_to_update)
                .execute(&mut *tx)
                .await?;
        }
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
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Path(email_to_delete): Path<String>,
    Extension(db): Extension<DB>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;
    let mut tx = db.begin().await?;

    sqlx::query!("DELETE FROM token WHERE email = $1", &email_to_delete)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM password WHERE email = $1", &email_to_delete)
        .execute(&mut *tx)
        .await?;
    windmill_common::user_drafts::delete_drafts_of_email(&mut *tx, &email_to_delete).await?;

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

#[derive(Deserialize)]
struct ChangeUserEmail {
    new_email: String,
}

/// `workspace.owner`, `workspace_settings.slack_email` and `usage.id` hold an email in a
/// `varchar(50)`, and `v2_job.permissioned_as` in a `varchar(55)`; every other email column is
/// `varchar(255)`. The strictest of the two bounds is used for all of them.
const SHORT_EMAIL_COLUMN_MAX_LEN: usize = 50;
const EMAIL_COLUMN_MAX_LEN: usize = 255;

/// Move an account to a new email address, in place: the `password` row (and with it the
/// instance-wide username, the role and the login type) is kept and every email-keyed row is
/// repointed at the new address.
///
/// `audit` is deliberately left alone: it records who did what at the time, so rewriting it
/// would falsify history.
async fn change_user_email(
    authed: ApiAuthed,
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Path(old_email): Path<String>,
    Extension(db): Extension<DB>,
    Json(ce): Json<ChangeUserEmail>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;

    // The target is matched verbatim (accounts predating email normalization can hold uppercase),
    // while the new address is normalized the same way account creation and login do.
    let old_email = old_email.trim().to_string();
    let new_email = ce.new_email.trim().to_lowercase();

    if !VALID_EMAIL.is_match(&new_email) || new_email.len() > EMAIL_COLUMN_MAX_LEN {
        return Err(Error::BadRequest(format!(
            "{new_email} is not a valid email address of at most {EMAIL_COLUMN_MAX_LEN} characters"
        )));
    }

    if new_email == old_email {
        return Err(Error::BadRequest(
            "The new email is identical to the current one".to_string(),
        ));
    }

    // Every API server caches the caller's identity behind their token and only drops it when the
    // invalidation event is polled, so moving your own account would leave you authenticating as an
    // address that no longer exists for a few seconds.
    if old_email.eq_ignore_ascii_case(&authed.email) {
        return Err(Error::BadRequest(
            "You cannot change your own email, ask another superadmin to do it".to_string(),
        ));
    }

    for reserved in [
        SUPERADMIN_SECRET_EMAIL,
        SUPERADMIN_NOTIFICATION_EMAIL,
        SUPERADMIN_SYNC_EMAIL,
    ] {
        if old_email == reserved || new_email == reserved {
            return Err(Error::BadRequest(format!(
                "{reserved} is a reserved email address"
            )));
        }
    }

    let mut tx = db.begin().await?;

    // FOR UPDATE serializes concurrent moves of *this* account. Two moves of different accounts
    // onto the same destination are stopped by the `password` primary key instead, which is why the
    // unique violation below is mapped back onto the same 400 as the conflict check.
    let username = sqlx::query_scalar!(
        "SELECT username FROM password WHERE email = $1 FOR UPDATE",
        &old_email
    )
    .fetch_optional(&mut *tx)
    .await?;
    let username = not_found_if_none(username, "user", &old_email)?;

    // Compared case-insensitively: login lowercases what it is given, so an account stored with
    // uppercase would be shadowed by a lowercase twin rather than collide with it. The moved
    // account is excluded so that normalizing its own address to lowercase stays allowed.
    let taken = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM password WHERE lower(email) = $1 AND email <> $2
            UNION ALL SELECT 1 FROM usr WHERE lower(email) = $1 AND email <> $2)",
        &new_email,
        &old_email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if taken {
        return Err(Error::BadRequest(format!(
            "{new_email} is already used by another account"
        )));
    }

    if new_email.len() > SHORT_EMAIL_COLUMN_MAX_LEN {
        let referenced_by_short_column = sqlx::query_scalar!(
            "SELECT EXISTS(
                SELECT 1 FROM workspace WHERE owner = $1
                UNION ALL SELECT 1 FROM workspace_settings WHERE slack_email = $1
                UNION ALL SELECT 1 FROM usage WHERE id = $1 AND NOT is_workspace
                UNION ALL SELECT 1 FROM v2_job WHERE permissioned_as = $1 AND id IN (SELECT id FROM v2_job_queue))",
            &old_email
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);
        if referenced_by_short_column {
            return Err(Error::BadRequest(format!(
                "{new_email} is longer than {SHORT_EMAIL_COLUMN_MAX_LEN} characters and this user owns a workspace, a Slack connection, usage counters or a queued job, whose columns cannot hold it"
            )));
        }
    }

    // An account named by its address carries that address into every principal column, and
    // `v2_job.permissioned_as` is narrower than all of them: the move would leave runnables that
    // look configured but cannot enqueue. Same limit the deploy path applies.
    let old_principal_probe = username_to_permissioned_as(&old_email);
    if username_to_permissioned_as(&new_email).chars().count() > PERMISSIONED_AS_MAX_LEN {
        let names_a_runnable = sqlx::query_scalar!(
            "SELECT EXISTS(
                SELECT 1 FROM script WHERE on_behalf_of = $1
                UNION ALL SELECT 1 FROM flow WHERE on_behalf_of = $1
                UNION ALL SELECT 1 FROM app WHERE policy->>'on_behalf_of' = $1
                UNION ALL SELECT 1 FROM schedule WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM http_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM websocket_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM postgres_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM mqtt_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM kafka_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM nats_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM sqs_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM gcp_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM email_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM amqp_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM azure_trigger WHERE permissioned_as = $1
                UNION ALL SELECT 1 FROM folder
                    WHERE default_permissioned_as @> jsonb_build_array(
                        jsonb_build_object('permissioned_as', $1::text)))",
            &old_principal_probe
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);
        if names_a_runnable {
            return Err(Error::BadRequest(format!(
                "{new_email} is longer than the {PERMISSIONED_AS_MAX_LEN} characters a job can \
                 carry, and runnables or triggers run on behalf of this account by its address"
            )));
        }
    }

    // A pending_user row only reserves a username for an address that has no account yet, which
    // stops being true here. The moved account keeps its own username.
    sqlx::query!("DELETE FROM pending_user WHERE email = $1", &new_email)
        .execute(&mut *tx)
        .await?;

    // ---- account ----
    sqlx::query!(
        "UPDATE password SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            Error::BadRequest(format!("{new_email} is already used by another account"))
        }
        _ => e.into(),
    })?;
    windmill_common::user_drafts::rename_drafts_of_email(&mut *tx, &old_email, &new_email).await?;

    sqlx::query!(
        "UPDATE usr SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace SET owner = $1 WHERE owner = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // Slack commands run as this address when the command maps to no workspace user.
    sqlx::query!(
        "UPDATE workspace_settings SET slack_email = $1 WHERE slack_email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // Per-user monthly execution counters, keyed by the email.
    sqlx::query!(
        "DELETE FROM usage WHERE id = $1 AND NOT is_workspace",
        &new_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE usage SET id = $1 WHERE id = $2 AND NOT is_workspace",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // ---- instance groups and invites ---- (both keyed on the email, so drop the rows that would
    // collide with what the new address was already granted before merging the old ones in)
    sqlx::query!(
        "DELETE FROM email_to_igroup o WHERE o.email = $1 AND EXISTS (SELECT 1 FROM email_to_igroup n WHERE n.email = $2 AND n.igroup = o.igroup)",
        &old_email,
        &new_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE email_to_igroup SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM workspace_invite o WHERE o.email = $1 AND EXISTS (SELECT 1 FROM workspace_invite n WHERE n.email = $2 AND n.workspace_id = o.workspace_id)",
        &old_email,
        &new_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_invite SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM tutorial_progress WHERE email = $1", &new_email)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "UPDATE tutorial_progress SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // ---- credentials ---- (a password reset link was mailed to the old address)
    sqlx::query!("DELETE FROM magic_link WHERE email = $1", &old_email)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "UPDATE token SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // Tokens stay valid, but every API server caches the authed user behind the raw token, so ask
    // them all to drop those entries rather than serve the previous address until they expire.
    sqlx::query!(
        "INSERT INTO notify_event (channel, payload) SELECT 'notify_token_invalidation', token_prefix FROM token WHERE email = $1",
        &new_email
    )
    .execute(&mut *tx)
    .await?;

    // An external JWT still asserts the old address, so its cached mapping is stale.
    sqlx::query!(
        "DELETE FROM unique_ext_jwt_token WHERE email = $1",
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE mcp_oauth_refresh_token SET user_email = $1 WHERE user_email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE mcp_oauth_server_code SET user_email = $1 WHERE user_email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // ---- runnables run on behalf of the user ----
    sqlx::query!(
        "UPDATE schedule SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE capture_config SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // Apps store an address next to their principal, and the synthetic
    // `group-{name}@windmill.dev` may be a real user's, so a group-owned app has to keep its
    // address when a colliding user moves: an app running in Anonymous or Publisher mode takes
    // its permissions from that pair rather than from the caller, and a half-rewritten pair
    // names two accounts. Drafts below carry the same pair and need the same guard.
    sqlx::query!(
        "UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of_email'], to_jsonb($1::text)) WHERE policy->>'on_behalf_of_email' = $2 AND (policy->>'on_behalf_of' IS NULL OR policy->>'on_behalf_of' NOT LIKE 'g/%')",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // ---- permissioned_as naming the account by its address ----
    // `usr.username` is constrained to `[\w-]+`, so a workspace member is always named
    // `u/{username}` and their principals survive an address change untouched. The address form
    // belongs to an account acting without a `usr` row — a superadmin outside their workspaces,
    // named by `password.username` or, failing that, by the address itself. Those are the rows
    // that go stale here, and `username_to_permissioned_as` is what encodes both ends of the
    // move (an address containing a `/` is prefixed, since readers split on the first one).
    let old_principal = username_to_permissioned_as(&old_email);
    let new_principal = username_to_permissioned_as(&new_email);

    sqlx::query!(
        "UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'], to_jsonb($1::text)) WHERE policy->>'on_behalf_of' = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE script SET on_behalf_of = $1 WHERE on_behalf_of = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    // The address these two keep beside the principal is what a worker predating
    // MIN_VERSION_SUPPORTS_ON_BEHALF_OF_PRINCIPAL reads, so it follows the account for as long
    // as one may be live. Group-owned rows are held back for the reason given above the app
    // sweep: their address is the group's, which a colliding user does not take with them.
    sqlx::query!(
        "UPDATE script SET on_behalf_of_email = $1 WHERE on_behalf_of_email = $2 AND (on_behalf_of IS NULL OR on_behalf_of NOT LIKE 'g/%')",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE flow SET on_behalf_of = $1 WHERE on_behalf_of = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE flow SET on_behalf_of_email = $1 WHERE on_behalf_of_email = $2 AND (on_behalf_of IS NULL OR on_behalf_of NOT LIKE 'g/%')",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // Script/flow rows store only the principal, but drafts carry an address beside it and
    // `deployDraft` sends both — left stale it contradicts the principal, which resolves to the
    // new address, and the deploy is rejected. Group-owned drafts are held back for the reason
    // given above the app sweep.
    sqlx::query!(
        r#"UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['on_behalf_of_email'], to_jsonb($1::text))) WHERE typ IN ('script', 'flow') AND value->>'on_behalf_of_email' = $2 AND (value->>'on_behalf_of' IS NULL OR value->>'on_behalf_of' NOT LIKE 'g/%')"#,
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['on_behalf_of'], to_jsonb($1::text))) WHERE typ IN ('script', 'flow') AND value->>'on_behalf_of' = $2"#,
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    // A folder's default rules are an ordered array, first match wins, so the rewrite has to
    // preserve their order. A rule left on the old address makes `ensure_permissioned_as_exists`
    // reject the creation of every runnable the rule matches.
    sqlx::query!(
        r#"UPDATE folder SET default_permissioned_as = (
            SELECT jsonb_agg(
                CASE WHEN rule->>'permissioned_as' = $2
                     THEN jsonb_set(rule, ARRAY['permissioned_as'], to_jsonb($1::text))
                     ELSE rule END
                ORDER BY ord)
            FROM jsonb_array_elements(default_permissioned_as) WITH ORDINALITY AS t(rule, ord))
        WHERE default_permissioned_as @> jsonb_build_array(jsonb_build_object('permissioned_as', $2::text))"#,
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE schedule SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE http_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE websocket_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE postgres_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE mqtt_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE kafka_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE nats_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE sqs_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE gcp_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE email_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE amqp_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE azure_trigger SET permissioned_as = $1 WHERE permissioned_as = $2",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    // ---- jobs ---- Restricted to what is still queued: those rows drive the permissions of a run
    // that has not finished yet, whereas completed jobs are history and neither column is indexed
    // (rewriting every past row of a busy user would hold this transaction's locks for minutes).
    sqlx::query!(
        "UPDATE v2_job SET permissioned_as_email = $1 WHERE permissioned_as_email = $2 AND id IN (SELECT id FROM v2_job_queue)",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET permissioned_as = $1 WHERE permissioned_as = $2 AND id IN (SELECT id FROM v2_job_queue)",
        &new_principal,
        &old_principal
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE job_perms SET email = $1 WHERE email = $2 AND job_id IN (SELECT id FROM v2_job_queue)",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    // ---- fork deployment requests ----
    sqlx::query!(
        "UPDATE workspace_fork_deployment_request SET requested_by_email = $1 WHERE requested_by_email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_fork_deployment_request_assignee SET email = $1 WHERE email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_fork_deployment_request_comment SET author_email = $1 WHERE author_email = $2",
        &new_email,
        &old_email
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "users.change_email",
        ActionKind::Update,
        "global",
        Some(&old_email),
        Some([("new_email", new_email.as_str())].into()),
    )
    .await?;

    // Read back inside the transaction: the address is derived at dispatch through a cache
    // that nothing else evicts, so without this a job pushed in the next 60s would resolve
    // the old address and with it the wrong superadmin flag and instance groups.
    let memberships =
        sqlx::query_scalar!("SELECT workspace_id FROM usr WHERE email = $1", &new_email)
            .fetch_all(&mut *tx)
            .await?;

    tx.commit().await?;

    if let Some(username) = username.as_deref() {
        for w_id in &memberships {
            windmill_common::users::invalidate_email_cache(w_id, username);
        }
    }

    Ok(format!(
        "changed email of user {old_email} to {new_email}{}",
        username
            .map(|u| format!(", keeping the instance username {u}"))
            .unwrap_or_default()
    ))
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
    // ---- Clean up extra_perms referencing this user ----
    let extra_perms_tables = [
        "script",
        "flow",
        "app",
        "resource",
        "eval_dataset",
        "variable",
        "schedule",
        "group_",
        "folder",
        "raw_app",
        "http_trigger",
        "websocket_trigger",
        "kafka_trigger",
        "postgres_trigger",
        "mqtt_trigger",
        "amqp_trigger",
        "nats_trigger",
        "sqs_trigger",
        "gcp_trigger",
        "azure_trigger",
        "email_trigger",
    ];
    // SAFETY: `table` comes from a hardcoded allowlist `extra_perms_tables`, not user input.
    for table in &extra_perms_tables {
        sqlx::query(&format!(
            "UPDATE {table} SET extra_perms = extra_perms - ('u/' || $1) \
             WHERE extra_perms ? ('u/' || $1) AND workspace_id = $2"
        ))
        .bind(username_to_delete)
        .bind(w_id)
        .execute(&mut **tx)
        .await?;
    }

    // ---- Clean up folder owners ----
    sqlx::query!(
        "UPDATE folder SET owners = array_remove(owners, 'u/' || $1) WHERE ('u/' || $1) = ANY(owners) AND workspace_id = $2",
        username_to_delete, w_id
    ).execute(&mut **tx).await?;

    // ---- Delete personal data ----
    sqlx::query!(
        "DELETE FROM draft WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $2",
        username_to_delete,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM favorite WHERE usr = $1 AND workspace_id = $2",
        username_to_delete,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM input WHERE created_by = $1 AND workspace_id = $2",
        username_to_delete,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM capture WHERE created_by = $1 AND workspace_id = $2",
        username_to_delete,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- Delete user records ----
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

    sqlx::query!(
        "DELETE FROM token WHERE email = $1 AND workspace_id = $2",
        email_to_delete,
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

/// Non-admin path for `delete_workspace_user`: the creator of a fork may remove non-admin members
/// from the fork they created, so that adding the wrong collaborator is theirs to undo rather than
/// an admin's. Never on a root workspace, and never against an admin of the fork — the counterpart
/// of the add grant, whose bounds are spelled out on `add_user` in `windmill-api-workspaces`.
///
/// `target_is_admin` must come from a row locked by the caller's deletion transaction: the grant
/// turns on the target not being an admin, so a promotion committing between the check and the
/// delete would remove an admin after all. `None` (no such member) is left to the caller's 404,
/// which is raised only after this returns so that a non-creator cannot probe who exists.
async fn authorize_fork_owner_delete_user(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    authed: &ApiAuthed,
    username_to_delete: &str,
    target_is_admin: Option<bool>,
) -> Result<()> {
    if windmill_common::workspaces::fork_owned_by(&mut **tx, w_id, &authed.email)
        .await?
        .is_none()
    {
        return Err(Error::RequireAdmin(authed.username.clone()));
    }

    if target_is_admin == Some(true) {
        return Err(Error::PermissionDenied(format!(
            "as the creator of fork {w_id} you cannot remove {username_to_delete}, who is an admin \
             of it"
        )));
    }

    Ok(())
}

async fn delete_workspace_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_delete)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    // Locked so that the authorization below and the delete it guards see the same row.
    let target = sqlx::query!(
        "SELECT email, is_admin FROM usr where username = $1 AND workspace_id = $2 FOR UPDATE",
        username_to_delete,
        &w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if !authed.is_admin {
        authorize_fork_owner_delete_user(
            &mut tx,
            &w_id,
            &authed,
            &username_to_delete,
            target.as_ref().map(|t| t.is_admin),
        )
        .await?;
    }

    let email_to_delete = not_found_if_none(target, "User", &username_to_delete)?.email;

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
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Json(et): Json<EditLoginType>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;
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
    headers: axum::http::HeaderMap,
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(Login { email, password }): Json<Login>,
) -> Result<String> {
    // In `--no-auth` mode there is no real login; the frontend never needs a
    // session cookie because every request already resolves as the admin
    // superadmin (see resolve_opt_job_authed).
    if windmill_api_auth::is_no_auth() {
        return Ok("no_auth".to_string());
    }

    let email = email.to_lowercase();

    if windmill_common::global_settings::DISABLE_PASSWORD_LOGIN
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(Error::BadRequest(
            "Password login is disabled on this instance".to_string(),
        ));
    }

    windmill_common::login_rate_limit::check_and_increment_login_attempt(&headers, &email)?;

    let mut tx = db.begin().await?;
    let audit_author = AuditAuthor {
        email: email.clone(),
        username: email.clone(),
        username_override: None,
        token_prefix: None,
    };
    let email_w_h: Option<(String, String, bool)> = sqlx::query_as(
        "SELECT email, password_hash, super_admin FROM password WHERE email = $1 AND login_type = \
         'password' AND disabled = false",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some((email, hash, super_admin)) = email_w_h {
        if argon2
            .verify_password(password.as_bytes(), hash.as_str())
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
            windmill_common::login_rate_limit::record_login_failure(&email);
            Err(Error::BadRequest("Invalid login".to_string()))
        } else {
            let token =
                create_session_token(&email, super_admin, None, false, &mut tx, cookies).await?;

            let audit_author = AuditAuthor {
                email: email.clone(),
                username: email.clone(),
                username_override: None,
                token_prefix: Some(safe_token_prefix(&token)),
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
        windmill_common::login_rate_limit::record_login_failure(&email);
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
    // The session token minted below is database-backed and carries no job provenance,
    // so a job token that exchanged itself for one would shed the `job_id` every
    // `$WM_TOKEN` cap keys off (GHSA-hfh4-cx4h-3fcr). Only a browser session refreshes.
    if authed.job_id.is_some() {
        return Err(Error::NotAuthorized(
            "This endpoint cannot be called with a job token ($WM_TOKEN). If a script \
             genuinely needs a token of its own, create a dedicated token from the User \
             settings drawer (the 'Tokens' section), store it as a secret, and use that \
             token explicitly instead of $WM_TOKEN."
                .to_string(),
        ));
    }
    if let Some(thresh_s) = query.if_expiring_in_less_than_s {
        let t_hash = windmill_common::auth::hash_token(&token);
        let not_expired = sqlx::query_scalar!("SELECT true FROM token WHERE token_hash = $1 and expiration IS NOT NULL and expiration > now() + $2::int * '1 sec'::interval", &t_hash, thresh_s)
            .fetch_optional(&db)
            .await?
            .flatten()
            .unwrap_or(false);
        if not_expired {
            return Ok("token expiry is far enough".to_string());
        }
    }

    let mut tx = db.begin().await?;

    let super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1 AND disabled = false",
        &authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);

    let new_token = create_session_token(
        &authed.email,
        super_admin,
        authed.scopes.as_deref(),
        authed.read_only,
        &mut tx,
        cookies,
    )
    .await?;

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
    scopes: Option<&[String]>,
    read_only: bool,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    cookies: Cookies,
) -> Result<String> {
    use windmill_common::min_version::MIN_VERSION_SUPPORTS_TOKEN_HASH;

    let token = rd_string(32);
    let t_hash = windmill_common::auth::hash_token(&token);
    let t_prefix = token.get(..TOKEN_PREFIX_LEN).unwrap_or(&token);
    let plaintext: Option<&str> = if MIN_VERSION_SUPPORTS_TOKEN_HASH.met().await {
        None
    } else {
        Some(&token)
    };

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
                token_prefix: Some(t_prefix.to_string()),
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
            (token_hash, token_prefix, token, email, label, expiration, super_admin, scopes, read_only)
            VALUES ($1, $2, $3, $4, $5, now() + ($6 || ' seconds')::interval, $7, $8, $9)",
        t_hash,
        t_prefix,
        plaintext as Option<&str>,
        email,
        "session",
        &MAX_SESSION_VALIDITY_SECONDS.to_string(),
        super_admin,
        scopes,
        read_only,
    )
    .execute(&mut **tx)
    .await?;

    set_session_cookie(&cookies, &token, *MAX_SESSION_VALIDITY_SECONDS);
    Ok(token)
}

fn set_session_cookie(cookies: &Cookies, token: &str, validity_seconds: i64) {
    let mut cookie = Cookie::new(COOKIE_NAME, token.to_string());
    cookie.set_secure(IS_SECURE.load(std::sync::atomic::Ordering::Relaxed));
    cookie.set_same_site(Some(tower_cookies::cookie::SameSite::Lax));
    cookie.set_http_only(true);
    cookie.set_path(COOKIE_PATH);
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }

    let mut expire: OffsetDateTime = time::OffsetDateTime::now_utc();
    expire += time::Duration::seconds(validity_seconds);
    cookie.set_expires(expire);
    cookies.add(cookie);
}

lazy_static::lazy_static! {
    /// A guest session is the only credential held by someone with no account, so
    /// there is nothing to disable when the workspace revokes guest access or the
    /// identity provider removes them — the expiry is the revocation. Much shorter
    /// than a member session for that reason.
    static ref GUEST_SESSION_VALIDITY_SECONDS: i64 = std::env::var("GUEST_SESSION_VALIDITY_SECONDS")
        .ok()
        .and_then(|x| x.parse::<i64>().ok())
        .unwrap_or(8 * 60 * 60);
}

/// Mint a browser session for someone the identity provider authenticated who is a
/// member of no workspace, so they can open one guest-mode app. Writes no `password`
/// and no `usr` row: that absence is what keeps a guest off every seat counter, so
/// nothing here may be "helpfully" upgraded into provisioning.
///
/// Pinned to `w_id` (`AuthCache` matches on `token.workspace_id`): without the pin an
/// `apps:run:<path>` scope would unlock a same-path app elsewhere. So a guest cannot
/// authenticate on any workspace-less route (`/api/users/*`, `/api/settings/*`); a
/// page that needs one for a guest must become workspace-scoped, not loosen the pin.
///
/// Refuses unless every gate says yes (`guest_app_admits`, then the allowance in
/// `guest_admission`), so no caller can mint where a guest is not wanted, whatever it
/// believed when it decided to call. All that is left to the caller is the
/// authentication of `email`.
pub async fn create_guest_session_token<'c>(
    email: &str,
    w_id: &str,
    app_path: &str,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    cookies: Cookies,
) -> Result<String> {
    use windmill_common::min_version::MIN_VERSION_SUPPORTS_TOKEN_HASH;

    let token = rd_string(32);
    let t_hash = windmill_common::auth::hash_token(&token);
    let t_prefix = token.get(..TOKEN_PREFIX_LEN).unwrap_or(&token);
    let plaintext: Option<&str> = if MIN_VERSION_SUPPORTS_TOKEN_HASH.met().await {
        None
    } else {
        Some(&token)
    };
    let scopes = windmill_api_auth::scopes::guest_session_scopes(app_path)?;

    // No account at all (see `has_any_account`): an account holder is refused a guest
    // session, never handed a second, cheaper identity. The same helper the JWT arm uses.
    if windmill_common::users::has_any_account(&mut **tx, email).await? {
        return Err(Error::NotAuthorized(
            "an existing account cannot hold a guest session".to_string(),
        ));
    }
    if !windmill_common::workspaces::guest_app_admits(&mut **tx, w_id, app_path).await? {
        return Err(Error::NotAuthorized(format!(
            "app {app_path} is not open to guests"
        )));
    }
    windmill_common::workspaces::guest_admission(&mut **tx, email).await?;

    sqlx::query!(
        "INSERT INTO token
            (token_hash, token_prefix, token, email, label, expiration, super_admin, scopes, workspace_id)
            VALUES ($1, $2, $3, $4, $5, now() + ($6 || ' seconds')::interval, false, $7, $8)",
        t_hash,
        t_prefix,
        plaintext as Option<&str>,
        email,
        windmill_common::auth::GUEST_SESSION_LABEL,
        &GUEST_SESSION_VALIDITY_SECONDS.to_string(),
        &scopes,
        w_id,
    )
    .execute(&mut **tx)
    .await?;

    // The only durable record that a guest was here, and the set the allowance is
    // counted on; not the audit log, see the migration. Idempotent per email,
    // workspace and day.
    sqlx::query!(
        "INSERT INTO guest_activity (email, workspace_id, day)
         VALUES ($1, $2, CURRENT_DATE)
         ON CONFLICT (email, workspace_id, day)
         DO UPDATE SET last_seen_at = now()",
        email,
        w_id,
    )
    .execute(&mut **tx)
    .await?;

    audit_log(
        &mut **tx,
        &AuditAuthor {
            email: email.to_string(),
            username: email.to_string(),
            username_override: None,
            token_prefix: Some(t_prefix.to_string()),
        },
        "users.login_guest",
        ActionKind::Create,
        w_id,
        Some(app_path),
        Some([("entry", "idp")].into()),
    )
    .await?;

    set_session_cookie(&cookies, &token, *GUEST_SESSION_VALIDITY_SECONDS);
    Ok(token)
}

// create_token_internal is re-exported from windmill-api-auth above

async fn create_token(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Json(token_config): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    forbid_elevated_job_token(&db, &authed.email, job_id).await?;
    check_token_create_rate_limit(&authed.username)?;

    // `username_override_from_label` trusts a server-minted label to name the entity acting,
    // so a forged one would put an arbitrary name in `created_by` and the audit trail.
    // Deliberately narrower than the `is_user_token` guard on relabelling: the editor and the
    // debugger mint their own tokens through this handler. Server-side mints bypass it by
    // calling `create_token_internal` / `create_token_for_owner` directly.
    if token_config
        .label
        .as_deref()
        .is_some_and(windmill_common::auth::is_server_minted_label)
    {
        return Err(Error::BadRequest(
            "label collides with a reserved system-token namespace".to_string(),
        ));
    }

    windmill_api_auth::ensure_scopes_within_caller(&authed, token_config.scopes.as_deref())?;

    let mut tx = db.begin().await?;

    let token = create_token_internal(&mut *tx, &db, &authed, token_config).await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, token))
}

async fn impersonate(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Json(new_token): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    use windmill_common::min_version::MIN_VERSION_SUPPORTS_TOKEN_HASH;

    let token = rd_string(32);
    let t_hash = windmill_common::auth::hash_token(&token);
    let t_prefix = token.get(..TOKEN_PREFIX_LEN).unwrap_or(&token);
    let plaintext: Option<&str> = if MIN_VERSION_SUPPORTS_TOKEN_HASH.met().await {
        None
    } else {
        Some(&token)
    };
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;

    if new_token.impersonate_email.is_none() {
        return Err(Error::BadRequest(
            "impersonate_username is required".to_string(),
        ));
    }
    // This route writes its own row rather than going through the `create_token`
    // handler, so it repeats that handler's guard: impersonation names its
    // subject in `impersonate_email`, and a server-minted label — which
    // `username_override_from_label` trusts to name the entity acting — would
    // attribute this token's jobs to a third identity.
    if new_token
        .label
        .as_deref()
        .is_some_and(windmill_common::auth::is_server_minted_label)
    {
        return Err(Error::BadRequest(
            "label collides with a reserved system-token namespace".to_string(),
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
            (token_hash, token_prefix, token, email, label, expiration, super_admin)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        t_hash,
        t_prefix,
        plaintext as Option<&str>,
        impersonated,
        new_token.label,
        new_token.expiration,
        is_super_admin
    )
    .execute(&mut *tx)
    .await?;

    windmill_api_auth::register_token_expiry_notification(
        &mut *tx,
        &t_hash,
        new_token.label.as_deref(),
        new_token.expiration,
    )
    .await;

    audit_log(
        &mut *tx,
        &authed,
        "users.impersonate",
        ActionKind::Delete,
        &"global",
        Some(t_prefix),
        Some([("impersonated", &format!("{impersonated}")[..])].into()),
    )
    .instrument(tracing::info_span!("token", email = &impersonated))
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, token))
}

#[derive(Deserialize)]
pub struct ImpersonateServiceAccountRequest {
    pub username: String,
}

async fn impersonate_service_account(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    cookies: Cookies,
    Tokened { token: current_token }: Tokened,
    Path(w_id): Path<String>,
    Json(req): Json<ImpersonateServiceAccountRequest>,
) -> Result<(StatusCode, String)> {
    crate::users_oss::impersonate_service_account(db, authed, cookies, current_token, w_id, req)
        .await
}

#[derive(Deserialize)]
struct ExitImpersonationRequest {
    token: String,
}

async fn exit_impersonation(
    cookies: Cookies,
    Json(req): Json<ExitImpersonationRequest>,
) -> Result<String> {
    let mut cookie = tower_cookies::Cookie::new(COOKIE_NAME, req.token);
    cookie.set_secure(IS_SECURE.load(std::sync::atomic::Ordering::Relaxed));
    cookie.set_same_site(Some(tower_cookies::cookie::SameSite::Lax));
    cookie.set_http_only(true);
    cookie.set_path(COOKIE_PATH);
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }
    cookies.add(cookie);
    Ok("exited impersonation".to_string())
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
            "SELECT label, token_prefix, expiration, created_at, \
             last_used_at, scopes, workspace_id, read_only FROM token WHERE email = $1 AND (label != 'ephemeral-script' OR label IS NULL)
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
            "SELECT label, token_prefix, expiration, created_at, \
            last_used_at, scopes, workspace_id, read_only FROM token WHERE email = $1
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
    forbid_job_token_account_destruction(&authed)?;
    let mut tx = db.begin().await?;

    let tokens_deleted: Vec<String> = sqlx::query_scalar(
        "DELETE FROM token
               WHERE email = $1
                 AND token_prefix = $2
           RETURNING concat(token_prefix, '*****')",
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

#[derive(Deserialize)]
struct UpdateTokenScopesRequest {
    scopes: Option<Vec<String>>,
}

async fn update_token_scopes(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(token_prefix): Path<String>,
    Json(req): Json<UpdateTokenScopesRequest>,
) -> Result<String> {
    // Widening is what makes a narrowly-scoped mint (app embed, raw-app SDK, MCP
    // OAuth) recoverable as a general credential: a job token is unscoped, so the
    // caller check below would let it clear the scopes of any token sharing its
    // email (GHSA-hfh4-cx4h-3fcr).
    forbid_elevated_job_token(&db, &authed.email, authed.job_id).await?;
    windmill_api_auth::ensure_scopes_within_caller(&authed, req.scopes.as_deref())?;

    let mut tx = db.begin().await?;

    // A guest-labelled token is never rescoped: its scopes are its whole confinement,
    // and after promotion the same email owns an account that could otherwise strip
    // them from the still-valid guest credential. Same shape as the relabel guard.
    let updated: Option<String> = sqlx::query_scalar!(
        "UPDATE token SET scopes = $1
           WHERE email = $2 AND token_prefix = $3
             AND (label IS NULL OR label <> 'guest_session')
           RETURNING token_prefix",
        req.scopes.as_deref(),
        &authed.email,
        &token_prefix,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let prefix = updated.ok_or_else(|| {
        Error::NotFound(format!(
            "token {token_prefix} not found, not owned by user, or not rescopable"
        ))
    })?;

    let scopes_json = serde_json::to_string(&req.scopes).unwrap_or_default();
    audit_log(
        &mut *tx,
        &authed,
        "users.token.update_scopes",
        ActionKind::Update,
        &"global",
        Some(&prefix),
        Some([("scopes", scopes_json.as_str())].into()),
    )
    .await?;

    tx.commit().await?;

    windmill_api_auth::invalidate_token_from_cache(&prefix);

    Ok(format!("updated scopes for token {prefix}"))
}

#[derive(Deserialize)]
struct UpdateTokenLabelRequest {
    label: Option<String>,
}

async fn update_token_label(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(token_prefix): Path<String>,
    Json(req): Json<UpdateTokenLabelRequest>,
) -> Result<String> {
    // The new label must not collide with a system-token namespace (`session`,
    // `ephemeral*`, `debugger-token`, `mcp-oauth-*`): those labels are
    // load-bearing, and a user-set collision would orphan the token — hidden
    // from the UI (`isUserToken`) and rejected by the editability guard below —
    // while it still authenticates. (`is_user_token(None)` is true, so clearing
    // the label is allowed.)
    if !windmill_common::auth::is_user_token(req.label.as_deref()) {
        return Err(Error::BadRequest(
            "label collides with a reserved system-token namespace".to_string(),
        ));
    }

    // Matches the `token.label VARCHAR(1000)` column — reject overlong labels with
    // a 400 rather than letting Postgres raise a 500.
    const MAX_TOKEN_LABEL_LEN: usize = 1000;
    if req
        .label
        .as_deref()
        .is_some_and(|l| l.chars().count() > MAX_TOKEN_LABEL_LEN)
    {
        return Err(Error::BadRequest(format!(
            "label must be at most {MAX_TOKEN_LABEL_LEN} characters"
        )));
    }

    let mut tx = db.begin().await?;

    // Only user-created tokens may be relabeled — system tokens carry the
    // load-bearing labels described above. This SQL mirrors the canonical
    // `windmill_common::auth::is_user_token`; keep the two in sync (note the
    // case-insensitive `ephemeral` match).
    let updated: Option<String> = sqlx::query_scalar!(
        "UPDATE token SET label = $1
           WHERE email = $2 AND token_prefix = $3
             AND (label IS NULL OR (
                 label <> 'session'
                 AND label <> 'guest_session'
                 AND lower(label) NOT LIKE 'ephemeral%'
                 AND label <> 'debugger-token'
                 AND label NOT LIKE 'mcp-oauth-%'
             ))
           RETURNING token_prefix",
        req.label.as_deref(),
        &authed.email,
        &token_prefix,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let prefix = updated.ok_or_else(|| {
        Error::NotFound(format!(
            "token {token_prefix} not found, not owned by user, or not editable"
        ))
    })?;

    audit_log(
        &mut *tx,
        &authed,
        "users.token.update_label",
        ActionKind::Update,
        &"global",
        Some(&prefix),
        Some([("label", req.label.as_deref().unwrap_or(""))].into()),
    )
    .await?;

    tx.commit().await?;

    windmill_api_auth::invalidate_token_from_cache(&prefix);

    Ok(format!("updated label for token {prefix}"))
}

async fn leave_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    forbid_job_token_account_destruction(&authed)?;
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
    authed: ApiAuthed,
    Path(user_email): Path<String>,
    Extension(db): Extension<DB>,
) -> JsonResult<InstanceUsernameInfo> {
    require_super_admin(&db, &authed).await?;
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
    // Members only: this workspace-scoped endpoint has no superadmin/target gate,
    // so it must NOT use the `password` superadmin fallback — otherwise any
    // workspace-authenticated caller could turn a guessed derived username into a
    // non-member superadmin's email. Internal callers that legitimately need the
    // fallback (schedule/trigger/draft resolution) use `resolve_username_to_email`
    // directly and never return the email to an arbitrary caller.
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
    OptJobAuthed { job_id, .. }: OptJobAuthed,
) -> JsonResult<Vec<ExportedGlobalUser>> {
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;
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
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Json(users): Json<Vec<ExportedGlobalUser>>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    forbid_superadmin_job_token(&db, &authed.email, job_id).await?;
    let mut tx = db.begin().await?;
    // Replaces the account table, so — unlike the paths that remove one account — it deliberately
    // does not call `delete_drafts_of_email`: the addresses are about to be reinstated, and
    // dropping every draft on the instance to restore accounts would be pure collateral.
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

/// Check if password login is disabled (instance-wide)
async fn is_password_login_disabled() -> JsonResult<bool> {
    Ok(Json(
        windmill_common::global_settings::DISABLE_PASSWORD_LOGIN
            .load(std::sync::atomic::Ordering::Relaxed),
    ))
}

/// Request a password reset email
async fn request_password_reset(
    Extension(db): Extension<DB>,
    Json(req): Json<RequestPasswordReset>,
) -> Result<Json<PasswordResetResponse>> {
    if windmill_common::global_settings::DISABLE_PASSWORD_LOGIN
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(Error::BadRequest(
            "Password login is disabled on this instance".to_string(),
        ));
    }

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
        let base_url = (**BASE_URL.load()).clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Stored hashes outlive the hashing crate: every instance still holds hashes minted by
    /// older argon2 releases, and an upgrade that stopped reading them locks their users out.
    #[test]
    fn verifies_a_hash_minted_by_an_older_argon2() {
        // The seeded admin hash from migration 20220508150023, m=4096,t=3,p=1.
        let seeded = "$argon2id$v=19$m=4096,t=3,p=1$oLJo/lPn/gezXCuFOEyaNw$i0T2tCkw3xUFsrBIKZwr8jVNHlIfoxQe+HfDnLtd12I";

        assert!(Argon2::default()
            .verify_password(b"changeme", seeded)
            .is_ok());
        assert!(Argon2::default()
            .verify_password(b"not-the-password", seeded)
            .is_err());
    }
}
