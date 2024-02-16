/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#![allow(non_snake_case)]

use std::sync::Arc;

use crate::db::ApiAuthed;

use crate::oauth2_ee::{check_nb_of_user, InstanceEvent};
use crate::{
    db::DB, folders::get_folders_for_user, utils::require_super_admin, webhook_util::WebhookShared,
    workspaces::invite_user_to_all_auto_invite_worspaces, BASE_URL, COOKIE_DOMAIN, IS_SECURE,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    async_trait,
    extract::{Extension, FromRequestParts, OriginalUri, Path, Query},
    http::{self, request::Parts},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::{header::LOCATION, StatusCode};
use lazy_static::lazy_static;
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use quick_cache::sync::Cache;
use rand::rngs::OsRng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use tower_cookies::{Cookie, Cookies};
use tracing::{Instrument, Span};
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::users::truncate_token;
use windmill_common::worker::{CLOUD_HOSTED, SERVER_CONFIG};
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, Error, JsonResult, Result},
    users::SUPERADMIN_SECRET_EMAIL,
    utils::{not_found_if_none, rd_string, require_admin, Pagination, StripPath},
};

pub const TTL_TOKEN_DB_H: u32 = 72;

const COOKIE_NAME: &str = "token";
const COOKIE_PATH: &str = "/";

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_users))
        .route("/list_usernames", get(list_usernames))
        .route("/exists", post(exists_username))
        .route("/update/:user", post(update_workspace_user))
        .route("/delete/:user", delete(delete_workspace_user))
        .route("/is_owner/*path", get(is_owner_of_path))
        .route("/whois/:username", get(whois))
        .route("/whoami", get(whoami))
        .route("/leave", post(leave_workspace))
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
        .route("/create", post(create_user))
        .route("/update/:user", post(update_user))
        .route("/delete/:user", delete(delete_user))
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
    // .route("/list_invite_codes", get(list_invite_codes))
    // .route("/create_invite_code", post(create_invite_code))
    // .route("/signup", post(signup))
    // .route("/lost_password", post(lost_password))
    // .route("/use_magic_link", get(use_magic_link))
}

pub fn make_unauthed_service() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/logout", get(logout))
}

#[derive(Clone)]
pub struct ExpiringAuthCache {
    pub authed: ApiAuthed,
    pub expiry: Option<chrono::DateTime<chrono::Utc>>,
}
pub struct AuthCache {
    cache: Cache<(String, String), ExpiringAuthCache>,
    db: DB,
    superadmin_secret: Option<String>,
}

impl AuthCache {
    pub fn new(db: DB, superadmin_secret: Option<String>) -> Self {
        AuthCache { cache: Cache::new(300), db, superadmin_secret }
    }

    pub async fn invalidate(&self, w_id: &str, token: String) {
        self.cache.remove(&(w_id.to_string(), token));
    }

    pub async fn get_authed(&self, w_id: Option<String>, token: &str) -> Option<ApiAuthed> {
        let key = (
            w_id.as_ref().unwrap_or(&"".to_string()).to_string(),
            token.to_string(),
        );
        let s = self.cache.get(&key).map(|c| c.to_owned());
        match s {
            Some(ExpiringAuthCache { authed, expiry })
                if expiry.is_none() || expiry.unwrap() > chrono::Utc::now() =>
            {
                Some(authed)
            }
            _ => {
                let user_o = sqlx::query_as::<_, (Option<String>, Option<String>, bool, Option<Vec<String>>, Option<chrono::DateTime<chrono::Utc>>)>(
                    "UPDATE token SET last_used_at = now() WHERE token = $1 AND (expiration > NOW() \
                     OR expiration IS NULL) RETURNING owner, email, super_admin, scopes, expiration",
                )
                .bind(token)
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten();

                if let Some(user) = user_o {
                    let authed_o = {
                        match user {
                            (Some(owner), Some(email), super_admin, _, _) if w_id.is_some() => {
                                if let Some((prefix, name)) = owner.split_once('/') {
                                    if prefix == "u" {
                                        let (is_admin, is_operator) = if super_admin {
                                            (true, false)
                                        } else {
                                            let r = sqlx::query!(
                                                "SELECT is_admin, operator FROM usr where username = $1 AND \
                                                 workspace_id = $2 AND disabled = false",
                                                name,
                                                &w_id.as_ref().unwrap()
                                            )
                                            .fetch_one(&self.db)
                                            .await
                                            .ok();
                                            if let Some(r) = r {
                                                (r.is_admin, r.operator)
                                            } else {
                                                (false, true)
                                            }
                                        };

                                        let w_id = &w_id.unwrap();
                                        let groups =
                                            get_groups_for_user(w_id, &name, &email, &self.db)
                                                .await
                                                .ok()
                                                .unwrap_or_default();

                                        let folders =
                                            get_folders_for_user(w_id, &name, &groups, &self.db)
                                                .await
                                                .ok()
                                                .unwrap_or_default();

                                        Some(ApiAuthed {
                                            email: email,
                                            username: name.to_string(),
                                            is_admin,
                                            is_operator,
                                            groups,
                                            folders,
                                            scopes: None,
                                        })
                                    } else {
                                        let groups = vec![name.to_string()];
                                        let folders = get_folders_for_user(
                                            &w_id.unwrap(),
                                            "",
                                            &groups,
                                            &self.db,
                                        )
                                        .await
                                        .ok()
                                        .unwrap_or_default();
                                        Some(ApiAuthed {
                                            email: email,
                                            username: format!("group-{name}"),
                                            is_admin: false,
                                            groups,
                                            is_operator: false,
                                            folders,
                                            scopes: None,
                                        })
                                    }
                                } else {
                                    let groups = vec![];
                                    let folders = vec![];
                                    Some(ApiAuthed {
                                        email: email,
                                        username: owner,
                                        is_admin: super_admin,
                                        is_operator: true,
                                        groups,
                                        folders,
                                        scopes: None,
                                    })
                                }
                            }
                            (_, Some(email), super_admin, scopes, _) => {
                                if w_id.is_some() {
                                    let row_o = sqlx::query_as::<_, (String, bool, bool)>(
                                        "SELECT username, is_admin, operator FROM usr where email = $1 AND \
                                         workspace_id = $2 AND disabled = false",
                                    )
                                    .bind(&email)
                                    .bind(&w_id.as_ref().unwrap())
                                    .fetch_optional(&self.db)
                                    .await
                                    .unwrap_or(Some(("error".to_string(), false, false)));

                                    match row_o {
                                        Some((username, is_admin, is_operator)) => {
                                            let groups = get_groups_for_user(
                                                &w_id.as_ref().unwrap(),
                                                &username,
                                                &email,
                                                &self.db,
                                            )
                                            .await
                                            .ok()
                                            .unwrap_or_default();

                                            let folders = get_folders_for_user(
                                                &w_id.unwrap(),
                                                &username,
                                                &groups,
                                                &self.db,
                                            )
                                            .await
                                            .ok()
                                            .unwrap_or_default();
                                            Some(ApiAuthed {
                                                email,
                                                username,
                                                is_admin: is_admin || super_admin,
                                                is_operator,
                                                groups,
                                                folders,
                                                scopes,
                                            })
                                        }
                                        None if super_admin => Some(ApiAuthed {
                                            email: email.clone(),
                                            username: email,
                                            is_admin: super_admin,
                                            is_operator: false,
                                            groups: vec![],
                                            folders: vec![],
                                            scopes,
                                        }),
                                        None => None,
                                    }
                                } else {
                                    Some(ApiAuthed {
                                        email: email.to_string(),
                                        username: email,
                                        is_admin: super_admin,
                                        is_operator: true,
                                        groups: Vec::new(),
                                        folders: Vec::new(),
                                        scopes,
                                    })
                                }
                            }
                            _ => None,
                        }
                    };
                    if let Some(authed) = authed_o.as_ref() {
                        self.cache.insert(
                            key,
                            ExpiringAuthCache { authed: authed.clone(), expiry: user.4 },
                        );
                    }
                    authed_o
                } else if self
                    .superadmin_secret
                    .as_ref()
                    .map(|x| x == token)
                    .unwrap_or(false)
                {
                    Some(ApiAuthed {
                        email: SUPERADMIN_SECRET_EMAIL.to_string(),
                        username: "superadmin_secret".to_string(),
                        is_admin: true,
                        is_operator: false,
                        groups: Vec::new(),
                        folders: Vec::new(),
                        scopes: None,
                    })
                } else {
                    None
                }
            }
        }
    }
}

async fn extract_token<S: Send + Sync>(parts: &mut Parts, state: &S) -> Option<String> {
    let auth_header = parts
        .headers
        .get(http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let from_cookie = match auth_header {
        Some(x) => Some(x.to_owned()),
        None => Extension::<Cookies>::from_request_parts(parts, state)
            .await
            .ok()
            .and_then(|cookies| cookies.get(COOKIE_NAME).map(|c| c.value().to_owned())),
    };

    #[derive(Deserialize)]
    struct Token {
        token: Option<String>,
    }
    match from_cookie {
        Some(token) => Some(token),
        None => Query::<Token>::from_request_parts(parts, state)
            .await
            .ok()
            .and_then(|token| token.token.clone()),
    }
}

#[derive(Clone, Debug)]
pub struct Tokened {
    pub token: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for Tokened
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        if parts.method == http::Method::OPTIONS {
            return Ok(Tokened { token: "".to_string() });
        };
        let already_tokened = parts.extensions.get::<Tokened>();
        if let Some(tokened) = already_tokened {
            Ok(tokened.clone())
        } else {
            let token_o = extract_token(parts, state).await;
            if let Some(token) = token_o {
                let tokened = Self { token };
                parts.extensions.insert(tokened.clone());
                Ok(tokened)
            } else {
                Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
            }
        }
    }
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

#[async_trait]
impl<S> FromRequestParts<S> for ApiAuthed
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        if parts.method == http::Method::OPTIONS {
            return Ok(ApiAuthed {
                email: "".to_owned(),
                username: "".to_owned(),
                is_admin: false,
                is_operator: false,
                groups: Vec::new(),
                folders: Vec::new(),
                scopes: None,
            });
        };
        let already_authed = parts.extensions.get::<ApiAuthed>();
        if let Some(authed) = already_authed {
            Ok(authed.clone())
        } else {
            let already_tokened = parts.extensions.get::<Tokened>();
            let token_o = if let Some(token) = already_tokened {
                Some(token.token.clone())
            } else {
                extract_token(parts, state).await
            };
            let original_uri = OriginalUri::from_request_parts(parts, state)
                .await
                .ok()
                .map(|x| x.0)
                .unwrap_or_default();
            let path_vec: Vec<&str> = original_uri.path().split("/").collect();

            let workspace_id = if path_vec.len() >= 4 && path_vec[0] == "" && path_vec[2] == "w" {
                Some(path_vec[3].to_owned())
            } else {
                None
            };
            if let Some(token) = token_o {
                if let Ok(Extension(cache)) =
                    Extension::<Arc<AuthCache>>::from_request_parts(parts, state).await
                {
                    if let Some(authed) = cache.get_authed(workspace_id.clone(), &token).await {
                        parts.extensions.insert(authed.clone());
                        if authed.scopes.is_some() && (path_vec.len() < 3 || path_vec[4] != "jobs")
                        {
                            return Err((
                                StatusCode::UNAUTHORIZED,
                                format!("Unauthorized scoped token: {:?}", authed.scopes),
                            ));
                        }
                        Span::current().record("username", &authed.username.as_str());
                        Span::current().record("email", &authed.email);

                        if let Some(workspace_id) = workspace_id {
                            Span::current().record("workspace_id", &workspace_id);
                        }
                        return Ok(authed);
                    }
                }
            }
            Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
        }
    }
}

pub fn check_scopes<F>(authed: &ApiAuthed, required: F) -> error::Result<()>
where
    F: FnOnce() -> String,
{
    if let Some(scopes) = &authed.scopes {
        let req = &required();
        if !scopes.contains(req) {
            return Err(Error::BadRequest(format!("missing required scope: {req}")));
        }
    }
    Ok(())
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

#[derive(FromRow, Serialize)]
pub struct Usage {
    pub executions: i64,
}

#[derive(Serialize)]
pub struct UserWithUsage {
    #[serde(flatten)]
    pub user: User,
    pub usage: Usage,
}

#[derive(FromRow, Serialize, Debug)]
pub struct GlobalUserInfo {
    email: String,
    login_type: Option<String>,
    super_admin: bool,
    verified: bool,
    name: Option<String>,
    company: Option<String>,
}

#[derive(Serialize)]
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
    pub folders: Vec<String>,
    pub folders_owners: Vec<String>,
}

#[derive(FromRow, Serialize)]
pub struct WorkspaceInvite {
    pub workspace_id: String,
    pub email: String,
    pub is_admin: bool,
    pub operator: bool,
}

#[derive(FromRow, Serialize)]
pub struct InviteCode {
    pub code: String,
    pub seats_left: i32,
    pub seats_given: i32,
}

#[derive(Deserialize)]
pub struct NewInviteCode {
    pub code: String,
    pub seats: i32,
}

#[derive(FromRow)]
pub struct MagicLink {
    pub email: String,
    pub token: String,
    pub expiration: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct UseMagicLink {
    pub email: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password: String,
    pub super_admin: bool,
    pub name: Option<String>,
    pub company: Option<String>,
}

#[derive(Deserialize)]
pub struct AcceptInvite {
    pub workspace_id: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct DeclineInvite {
    pub workspace_id: String,
}

#[derive(Deserialize)]
pub struct EditUser {
    pub is_super_admin: Option<bool>,
}

#[derive(Deserialize)]
pub struct EditWorkspaceUser {
    pub is_admin: Option<bool>,
    pub operator: Option<bool>,
    pub disabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct EditPassword {
    pub password: String,
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
}

#[derive(Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct Signup {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub company: Option<String>,
}

#[derive(Deserialize)]
pub struct LostPassword {
    pub email: String,
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
) -> JsonResult<Vec<UserWithUsage>> {
    if *CLOUD_HOSTED && w_id == "demo" {
        require_admin(authed.is_admin, &authed.username)?;
    }
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query(
        "
        SELECT usr.*, usage.*
          FROM usr
             , LATERAL (
                SELECT COALESCE(SUM(duration_ms + 1000)/1000 , 0)::BIGINT executions
                  FROM completed_job
                 WHERE workspace_id = $1
                   AND job_kind NOT IN ('flow', 'flowpreview')
                   AND email = usr.email
                   AND now() - '1 week'::interval < created_at 
               ) usage
         WHERE workspace_id = $1
         ",
    )
    .bind(&w_id)
    .try_map(|row| {
        // flatten not released yet https://github.com/launchbadge/sqlx/pull/1959
        Ok(UserWithUsage { user: FromRow::from_row(&row)?, usage: FromRow::from_row(&row)? })
    })
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_users_as_super_admin(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<GlobalUserInfo>> {
    require_super_admin(&db, &authed.email).await?;
    let per_page = pagination.per_page.unwrap_or(10000).max(1);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;

    let rows = sqlx::query_as!(
        GlobalUserInfo,
        "SELECT email, login_type::text, verified, super_admin, name, company from password ORDER BY super_admin DESC, email LIMIT \
         $1 OFFSET $2",
        per_page as i32,
        offset as i32
    )
    .fetch_all(&db)
    .await?;
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

// async fn list_invite_codes(
//     authed: ApiAuthed,
//     Extension(db): Extension<DB>,
//     Query(pagination): Query<Pagination>
// ) -> JsonResult<Vec<InviteCode>> {
//     let mut tx = db.begin().await?;
//     require_super_admin(&mut tx, authed.email).await?;
//     let (per_page, offset) = crate::utils::paginate(pagination);

//     let rows = sqlx::query_as!(InviteCode, "SELECT * from invite_code LIMIT $1 OFFSET $2", per_page as i32, offset as i32)
//         .fetch_all(&mut tx)
//         .await?;
//     tx.commit().await?;
//     Ok(Json(rows))
// }

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
    let email = sqlx::query_scalar!("DELETE FROM token WHERE token = $1 RETURNING email", token)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(email) = email {
        audit_log(
            &mut *tx,
            &email.unwrap_or("noemail".to_string()),
            "users.logout",
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
            is_admin,
            is_super_admin: is_admin,
            created_at: chrono::Utc::now(),
            groups: groups,
            operator: false,
            disabled: false,
            role: Some("superadmin".to_string()),
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
        "SELECT email, login_type::TEXT, super_admin, verified, name, company FROM password WHERE \
         email = $1",
        email
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching global identity: {e}")));

    if let Ok(user) = user {
        Ok(Json(user))
    } else if std::env::var("SUPERADMIN_SECRET").ok() == Some(token) {
        Ok(Json(GlobalUserInfo {
            email: email.clone(),
            login_type: Some("superadmin_secret".to_string()),
            super_admin: true,
            verified: true,
            name: None,
            company: None,
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

async fn get_user(w_id: &str, username: &str, db: &DB) -> Result<Option<UserInfo>> {
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM usr where username = $1 AND workspace_id = $2",
        username,
        w_id
    )
    .fetch_optional(db)
    .await?;
    let is_super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        user.as_ref().map(|x| &x.email)
    )
    .fetch_optional(db)
    .await?
    .unwrap_or(false);
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
        is_admin: usr.is_admin,
        is_super_admin,
        created_at: usr.created_at,
        operator: usr.operator,
        disabled: usr.disabled,
        role: usr.role,
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

pub async fn get_groups_for_user(
    w_id: &str,
    username: &str,
    email: &str,
    db: &DB,
) -> Result<Vec<String>> {
    let groups = sqlx::query_scalar!(
        "SELECT group_ FROM usr_to_group where usr = $1 AND workspace_id = $2 UNION ALL SELECT igroup FROM email_to_igroup WHERE email = $3",
        username,
        w_id,
        email
    )
    .fetch_all(db)
    .await?
    .into_iter().filter_map(|x| x)
    .collect();
    Ok(groups)
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
//     require_super_admin(&mut tx, email).await?;

//     sqlx::query!(
//         "INSERT INTO invite_code
//             (code, seats_left)
//             VALUES ($1, $2)",
//         nu.code,
//         nu.seats
//     )
//     .execute(&mut tx)
//     .await?;

//     tx.commit().await?;

//     Ok((
//         StatusCode::CREATED,
//         format!("new invite code {}", nu.code),
//     ))
// }

async fn decline_invite(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(nu): Json<DeclineInvite>,
) -> Result<(StatusCode, String)> {
    let mut tx = db.begin().await?;

    let is_admin = sqlx::query_scalar!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2 RETURNING is_admin",
        nu.workspace_id,
        email,
    )
    .fetch_optional(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &email,
        "users.decline_invite",
        ActionKind::Delete,
        &nu.workspace_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    if is_admin.is_some() {
        Ok((
            StatusCode::OK,
            format!(
                "user {} declined invite to workspace {}",
                &email, nu.workspace_id
            ),
        ))
    } else {
        Err(Error::NotFound(format!("invite for {email} not found")))
    }
}

lazy_static! {
    pub static ref VALID_USERNAME: Regex = Regex::new(r#"^[a-zA-Z][a-zA-Z_0-9]*$"#).unwrap();
}

async fn accept_invite(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Json(nu): Json<AcceptInvite>,
) -> Result<(StatusCode, String)> {
    if !VALID_USERNAME.is_match(&nu.username) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
        )));
    }

    let mut tx = db.begin().await?;

    let r = sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2 RETURNING is_admin, operator",
        nu.workspace_id,
        email,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let is_some = r.is_some();
    if let Some(r) = r {
        tx = add_user_to_workspace(
            &nu.workspace_id,
            &email,
            &nu.username,
            r.is_admin,
            r.operator,
            tx,
        )
        .await?;

        audit_log(
            &mut *tx,
            &nu.username,
            "users.accept_invite",
            ActionKind::Create,
            &nu.workspace_id,
            Some(&email),
            None,
        )
        .await?;
        tx.commit().await?;
    }

    if is_some {
        webhook.send_instance_event(InstanceEvent::UserJoinedWorkspace {
            email: email.clone(),
            workspace: nu.workspace_id.clone(),
            username: nu.username.clone(),
        });
        Ok((
            StatusCode::CREATED,
            format!(
                "user {} accepted invite to workspace {}",
                &email, nu.workspace_id
            ),
        ))
    } else {
        Err(Error::NotFound(format!("invite for {email} not found")))
    }
}

async fn add_user_to_workspace<'c>(
    w_id: &str,
    email: &str,
    username: &str,
    is_admin: bool,
    operator: bool,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
) -> error::Result<sqlx::Transaction<'c, sqlx::Postgres>> {
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

    if !VALID_USERNAME.is_match(username) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
        )));
    }

    let already_exists_email = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
        &w_id,
        username,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if already_exists_email {
        return Err(Error::BadRequest(format!(
            "user with email {} already exists in workspace {}",
            email, w_id
        )));
    }

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin, operator)
            VALUES ($1, $2, $3, $4, $5)",
        &w_id,
        email,
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
        username,
        "users.add_to_workspace",
        ActionKind::Create,
        &w_id,
        Some(email),
        None,
    )
    .await?;
    Ok(tx)
}

async fn leave_instance(
    Extension(db): Extension<DB>,
    ApiAuthed { email, username, .. }: ApiAuthed,
) -> Result<String> {
    let mut tx = db.begin().await?;
    sqlx::query!("DELETE FROM password WHERE email = $1", &email)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &username,
        "workspaces.leave",
        ActionKind::Delete,
        "global",
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Left instance",))
}
async fn update_workspace_user(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_update)): Path<(String, String)>,
    Json(eu): Json<EditWorkspaceUser>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    require_admin(is_admin, &username)?;

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
        &username,
        "users.update",
        ActionKind::Update,
        &w_id,
        Some(&username_to_update),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("user {username} updated"))
}

async fn update_user(
    ApiAuthed { email, .. }: ApiAuthed,
    Path(email_to_update): Path<String>,
    Extension(db): Extension<DB>,
    Json(eu): Json<EditUser>,
) -> Result<String> {
    require_super_admin(&db, &email).await?;
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

    audit_log(
        &mut *tx,
        &email,
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
    ApiAuthed { email, .. }: ApiAuthed,
    Path(email_to_delete): Path<String>,
    Extension(db): Extension<DB>,
) -> Result<String> {
    require_super_admin(&db, &email).await?;
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
        &email,
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
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(mut nu): Json<NewUser>,
) -> Result<(StatusCode, String)> {
    require_super_admin(&db, &email).await?;
    let mut tx = db.begin().await?;

    nu.email = nu.email.to_lowercase();

    if nu.email == SUPERADMIN_SECRET_EMAIL {
        return Err(Error::BadRequest(
            "The superadmin email is a reserved email".into(),
        ));
    }

    check_nb_of_user(&db).await?;

    sqlx::query!(
        "INSERT INTO password(email, verified, password_hash, login_type, super_admin, name, \
         company)
    VALUES ($1, $2, $3, 'password', $4, $5, $6)",
        &nu.email,
        true,
        &hash_password(argon2, nu.password.clone())?,
        &nu.super_admin,
        nu.name,
        nu.company
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &email,
        "users.add_global",
        ActionKind::Create,
        "global",
        Some(&nu.email),
        None,
    )
    .await?;
    tx = add_to_demo_if_exists(tx, &email).await?;

    tx.commit().await?;

    invite_user_to_all_auto_invite_worspaces(&db, &nu.email).await?;
    send_email_if_possible(
        "Invited to Windmill",
        &format!(
            "You have been granted access to Windmill by {email}.

Log in and change your password: {}/user/login?email={}&password={}&rd=%2F%23user-settings

You can then join or create a workspace. Happy building!",
            BASE_URL.read().await.clone(),
            &nu.email,
            &nu.password
        ),
        &nu.email,
    );
    webhook.send_instance_event(InstanceEvent::UserAdded { email: nu.email.clone() });
    Ok((StatusCode::CREATED, format!("email {} created", nu.email)))
}

pub fn send_email_if_possible(subject: &str, content: &str, to: &str) {
    let subject = subject.to_string();
    let content = content.to_string();
    let to = to.to_string();
    tokio::spawn(async move {
        if let Err(e) = send_email_if_possible_intern(&subject, &content, &to).await {
            tracing::error!("Failed to send email to {}: {}", &to, e);
        }
    });
}

pub async fn send_email_if_possible_intern(subject: &str, content: &str, to: &str) -> Result<()> {
    if let Some(smtp) = SERVER_CONFIG.read().await.smtp.clone() {
        let client = SmtpClientBuilder::new(smtp.host, smtp.port)
            .implicit_tls(smtp.tls_implicit.unwrap_or(false));
        let client = if let (Some(username), Some(password)) = (smtp.username, smtp.password) {
            if !username.is_empty() {
                client.credentials((username, password))
            } else {
                client
            }
        } else {
            client
        };
        let message = MessageBuilder::new()
            .from(("Windmill", smtp.from.as_str()))
            .to(to)
            .subject(subject)
            .text_body(content);
        client
            .connect()
            .await
            .map_err(to_anyhow)?
            .send(message)
            .await
            .map_err(to_anyhow)?;
        tracing::info!("Sent email to {to}: {subject}");
    }
    return Ok(());
}

async fn delete_workspace_user(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username_to_delete)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    require_admin(is_admin, &username)?;

    let email_to_delete_o = sqlx::query_scalar!(
        "SELECT email FROM usr where username = $1 AND workspace_id = $2",
        username_to_delete,
        &w_id,
    )
    .fetch_optional(&db)
    .await?;

    let email_to_delete = not_found_if_none(email_to_delete_o, "User", &username_to_delete)?;

    let username = sqlx::query_scalar!(
        "DELETE FROM usr WHERE email = $1 AND workspace_id = $2 RETURNING username",
        email_to_delete,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM usr_to_group WHERE usr = $1 AND workspace_id = $2",
        &username,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &username,
        "users.delete",
        ActionKind::Delete,
        &w_id,
        Some(&username_to_delete),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("username {} deleted", username_to_delete))
}

async fn set_password(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    ApiAuthed { username, email, .. }: ApiAuthed,
    Json(EditPassword { password }): Json<EditPassword>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    let custom_type = sqlx::query_scalar!(
        "SELECT login_type::TEXT FROM password WHERE email = $1",
        &email
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("setting password: {e}")))?
    .unwrap_or("".to_string());

    if custom_type != "password".to_string() {
        return Err(Error::BadRequest(format!(
            "login type for {email} is of type {custom_type}. Cannot set password."
        )));
    }

    sqlx::query!(
        "UPDATE password SET password_hash = $1 WHERE email = $2",
        &hash_password(argon2, password)?,
        &email,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &username,
        "users.setpassword",
        ActionKind::Update,
        "global",
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("password of {} updated", email))
}

pub fn hash_password(argon2: Arc<Argon2>, password: String) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::InternalErr(e.to_string()))?
        .to_string();
    Ok(password_hash)
}

// async fn lost_password(
//     Extension(db): Extension<DB>,
//     Extension(es): Extension<Arc<EmailSender>>,
//     TypedHeader(host): TypedHeader<headers::Host>,
//     Json(LostPassword {
//         email
//     }): Json<LostPassword>,
// ) -> Result<String> {
//     let mut tx = db.begin().await?;

//     let exists = sqlx::query_scalar!(
//             "SELECT EXISTS(SELECT 1 FROM password WHERE email = $1)",
//         &email)
//         .fetch_one(&mut tx)
//         .await?
//         .unwrap_or(false);

//     if !exists {
//         return Err(Error::NotFound(format!("no user found at email {email}")))
//     }

//     let already = sqlx::query_scalar!(
//             "SELECT EXISTS(SELECT 1 FROM magic_link WHERE email = $1)",
//         &email)
//         .fetch_one(&mut tx)
//         .await?
//         .unwrap_or(false);

//     if already {
//         return Err(Error::BadRequest(format!("a magic link was already sent at {email}")))
//     }

//     let tx = create_magic_link(&host.hostname(), &email, &es, tx).await?;
//     tx.commit().await?;

//     Ok(format!("Magic link sent to {email}"))
// }

// async fn use_magic_link(
//     cookies: Cookies,
//     Extension(db): Extension<DB>,
//     Query(UseMagicLink {
//         email,
//         token
//     }): Query<UseMagicLink>,
// ) -> Result<String> {
//     let mut tx = db.begin().await?;

//     let email_o = sqlx::query_scalar!(
//         "DELETE FROM magic_link WHERE email = $1 AND token = $2
//         RETURNING email", email, token
//     )
//     .fetch_optional(&mut tx)
//     .await?;

//     if let Some(email) = email_o {
//         let is_super_admin = sqlx::query_scalar!("UPDATE password SET verified = true WHERE email = $1 RETURNING super_admin", email)
//             .fetch_optional(&mut tx)
//             .await?
//             .unwrap_or(false);

//         let token = create_session_token(&email, is_super_admin, &mut tx, cookies).await?;
//         tx.commit().await?;
//         Ok(token)
//     } else {
//         Err(Error::NotFound(format!("magic link for {email} not found")))
//     }
// }

// async fn signup(
//     TypedHeader(host): TypedHeader<headers::Host>,
//     Extension(db): Extension<DB>,
//     Extension(argon2): Extension<Arc<Argon2<'_>>>,
//     Extension(es): Extension<Arc<EmailSender>>,
//     Json(Signup {
//         email,
//         password,
//         name,
//         company
//     }): Json<Signup>,
// ) -> Result<(StatusCode, String)> {
//     let mut tx = db.begin().await?;

//     let email = sqlx::query_scalar!(
//             "INSERT INTO password (email, password_hash, name, company) VALUES ($1, $2, $3, $4) RETURNING email",
//         &email, &hash_password(argon2, password)?, name, company)
//         .fetch_optional(&mut tx)
//         .await?;

//     if let Some(email) = email {
//         let tx = create_magic_link(&host.hostname(), &email, &es, tx).await?;
//         tx.commit().await?;

//         Ok((
//             StatusCode::CREATED,
//             format!("user with email {} created", email),
//         ))
//     } else {
//         Err(Error::BadRequest("Invalid login".to_string()))
//     }
// }

// async fn create_magic_link<'c>(host: &str, email: &str, es: &EmailSender, mut tx: sqlx::Transaction<'c, sqlx::Postgres>) -> error::Result<sqlx::Transaction<'c, sqlx::Postgres>> {
//     let token = gen_token();

//     sqlx::query!(
//         "INSERT INTO magic_link
//             (email, token)
//             VALUES ($1, $2)",
//         email,
//         &token
//     )
//     .execute(&mut tx)
//     .await?;

//     let encoded_token = urlencoding::encode(&token);
//     let encoded_email = urlencoding::encode(email);
//     es.send_email(Message::builder()
//         .to(email.parse().unwrap())
//         .subject("New magic link")
//         .body(format!("Magic link: https://{host}/magic_link?token={encoded_token}&email={encoded_email}"))
//         .unwrap()).await?;

//     audit_log(
//          &mut tx,
//         email,
//         "users.magic_link",
//         ActionKind::Create,
//         "global",
//         Some(email),
//         None,
//     )
//     .await?;
//     Ok(tx)
// }

async fn login(
    cookies: Cookies,
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(Login { email, password }): Json<Login>,
) -> Result<String> {
    let mut tx = db.begin().await?;
    let email = email.to_lowercase();
    let email_w_h: Option<(String, String, bool, bool)> = sqlx::query_as(
        "SELECT email, password_hash, super_admin, first_time_user FROM password WHERE email = $1 AND login_type = \
         'password'",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some((email, hash, super_admin, first_time_user)) = email_w_h {
        let parsed_hash =
            PasswordHash::new(&hash).map_err(|e| Error::InternalErr(e.to_string()))?;
        if argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err()
        {
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

            tx.commit().await?;
            Ok(token)
        }
    } else {
        Err(Error::BadRequest("Invalid login".to_string()))
    }
}

async fn refresh_token(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    cookies: Cookies,
) -> Result<String> {
    let mut tx = db.begin().await?;

    let super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        &authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);

    let _ = create_session_token(&authed.email, super_admin, &mut tx, cookies).await?;

    tx.commit().await?;
    Ok("token refreshed".to_string())
}

pub async fn create_session_token<'c>(
    email: &str,
    super_admin: bool,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    cookies: Cookies,
) -> Result<String> {
    let token = rd_string(30);
    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin)
            VALUES ($1, $2, $3, now() + ($4 || ' hours')::interval, $5)",
        token,
        email,
        "session",
        TTL_TOKEN_DB_H.to_string(),
        super_admin
    )
    .execute(&mut **tx)
    .await?;
    let mut cookie = Cookie::new(COOKIE_NAME, token.clone());
    cookie.set_secure(IS_SECURE.read().await.clone());
    cookie.set_same_site(Some(cookie::SameSite::Lax));
    cookie.set_http_only(true);
    cookie.set_path(COOKIE_PATH);
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }
    let mut expire: OffsetDateTime = time::OffsetDateTime::now_utc();
    expire += time::Duration::days(3);
    cookie.set_expires(expire);
    cookies.add(cookie);
    Ok(token)
}

async fn create_token(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
    Json(new_token): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    let token = rd_string(30);
    let mut tx = db.begin().await?;

    let is_super_admin =
        sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(false);
    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin, scopes)
            VALUES ($1, $2, $3, $4, $5, $6)",
        token,
        email,
        new_token.label,
        new_token.expiration,
        is_super_admin,
        new_token.scopes.as_ref().map(|x| x.as_slice())
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &email,
        "users.token.create",
        ActionKind::Create,
        &"global",
        Some(&token[0..10]),
        None,
    )
    .instrument(tracing::info_span!("token", email = &email))
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, token))
}

async fn impersonate(
    Extension(db): Extension<DB>,
    ApiAuthed { email, username, .. }: ApiAuthed,
    Json(new_token): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    let token = rd_string(30);
    require_super_admin(&db, &email).await?;

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
        &username,
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
) -> JsonResult<Vec<TruncatedToken>> {
    let rows = if query.exclude_ephemeral.unwrap_or(false) {
        sqlx::query_as!(
            TruncatedToken,
            "SELECT label, concat(substring(token for 10)) as token_prefix, expiration, created_at, \
             last_used_at, scopes FROM token WHERE email = $1 AND label != 'ephemeral-script'
             ORDER BY created_at DESC",
            email,
        )
        .fetch_all(&db)
        .await?
    } else {
        sqlx::query_as!(
            TruncatedToken,
            "SELECT label, concat(substring(token for 10)) as token_prefix, expiration, created_at, \
            last_used_at, scopes FROM token WHERE email = $1
            ORDER BY created_at DESC",
            email,
        )
        .fetch_all(&db)
        .await?
    };
    Ok(Json(rows))
}

async fn delete_token(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
    Path(token_prefix): Path<String>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    let tokens_deleted: Vec<String> = sqlx::query_scalar(
        "DELETE FROM token
               WHERE email = $1
                 AND token LIKE concat($2::text, '%')
           RETURNING concat(substring(token for 10), '*****')",
    )
    .bind(&email)
    .bind(&token_prefix)
    .fetch_all(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &email,
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
    ApiAuthed { username, .. }: ApiAuthed,
) -> Result<String> {
    let mut tx = db.begin().await?;
    sqlx::query!(
        "DELETE FROM usr WHERE workspace_id = $1 AND username = $2",
        &w_id,
        username
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &username,
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
    Extension(cache): Extension<Arc<AuthCache>>,
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
            "SELECT workspace_id as workspace, path, summary, description, schema FROM flow WHERE workspace_id = $1", workspace
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

#[derive(Deserialize, Debug, Clone)]
pub struct LoginUserInfo {
    pub email: Option<String>,
    pub name: Option<String>,
    pub company: Option<String>,

    pub displayName: Option<String>,
}

pub async fn add_to_demo_if_exists<'c>(
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    email: &String,
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    let demo_exists =
        sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = 'demo')")
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);
    if demo_exists {
        if let Err(e) = sqlx::query!(
            "INSERT INTO workspace_invite
                (workspace_id, email, is_admin)
                VALUES ('demo', $1, false)
                ON CONFLICT DO NOTHING",
            &email
        )
        .execute(&mut *tx)
        .await
        {
            tracing::error!("error inserting invite: {:#?}", e);
        }
    }
    return Ok(tx);
}
