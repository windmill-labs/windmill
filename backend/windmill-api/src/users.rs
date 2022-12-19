/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{sync::Arc, time::Duration};

use crate::{
    db::{UserDB, DB},
    folders::get_folders_for_user,
    utils::require_super_admin,
    workspaces::invite_user_to_all_auto_invite_worspaces,
    CookieDomain, IsSecure,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, Path, Query, RequestParts},
    http,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::{header::LOCATION, StatusCode};
use rand::rngs::OsRng;
use retainer::Cache;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use tower_cookies::{Cookie, Cookies};
use tracing::{Instrument, Span};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, Error, JsonResult, Result},
    utils::{not_found_if_none, rd_string, require_admin, Pagination},
};
use windmill_queue::CLOUD_HOSTED;

const TTL_TOKEN_CACHE_S: u64 = 60 * 5; // 5 minutes
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
        .route("/whois/:email", get(whois))
        .route("/whoami", get(whoami))
        .route("/leave", post(leave_workspace))
}

pub fn global_service() -> Router {
    Router::new()
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
        .route("/usage", get(get_usage))
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

pub struct AuthCache {
    cache: Cache<(String, String), Authed>,
    db: DB,
}

impl AuthCache {
    pub fn new(db: DB) -> Self {
        AuthCache { cache: Cache::new(), db }
    }

    pub async fn get_authed(&self, w_id: Option<String>, token: &str) -> Option<Authed> {
        let key = (
            w_id.as_ref().unwrap_or(&"".to_string()).to_string(),
            token.to_string(),
        );
        let s = self.cache.get(&key).await.map(|c| c.to_owned());
        match s {
            a @ Some(_) => a,
            None => {
                let user_o = sqlx::query_as::<_, (Option<String>, Option<String>, bool)>(
                    "UPDATE token SET last_used_at = now() WHERE token = $1 AND (expiration > NOW() \
                     OR expiration IS NULL) RETURNING owner, email, super_admin",
                )
                .bind(token)
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten();

                if let Some(user) = user_o {
                    let authed_o = {
                        match user {
                            (Some(owner), email, super_admin) if w_id.is_some() => {
                                if let Some((prefix, name)) = owner.split_once('/') {
                                    if prefix == "u" {
                                        let is_admin = super_admin
                                            || sqlx::query_scalar!(
                                                "SELECT is_admin FROM usr where username = $1 AND \
                                                 workspace_id = $2 AND disabled = false",
                                                name,
                                                &w_id.as_ref().unwrap()
                                            )
                                            .fetch_one(&self.db)
                                            .await
                                            .ok()
                                            .unwrap_or(false);

                                        let w_id = &w_id.unwrap();
                                        let groups = get_groups_for_user(w_id, &name, &self.db)
                                            .await
                                            .ok()
                                            .unwrap_or_default();

                                        let folders =
                                            get_folders_for_user(w_id, &name, &groups, &self.db)
                                                .await
                                                .ok()
                                                .unwrap_or_default();

                                        Some(Authed {
                                            email: email
                                                .unwrap_or_else(|| "missing@email.xyz".to_string()),
                                            username: name.to_string(),
                                            is_admin,
                                            groups,
                                            folders,
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
                                        Some(Authed {
                                            email: email
                                                .unwrap_or_else(|| "missing@email.xyz".to_string()),
                                            username: format!("group-{name}"),
                                            is_admin: false,
                                            groups,
                                            folders,
                                        })
                                    }
                                } else {
                                    None
                                }
                            }
                            (_, Some(email), super_admin) => {
                                if w_id.is_some() {
                                    let row_o = sqlx::query_as::<_, (String, bool)>(
                                        "SELECT username, is_admin FROM usr where email = $1 AND \
                                         workspace_id = $2 AND disabled = false",
                                    )
                                    .bind(&email)
                                    .bind(&w_id.as_ref().unwrap())
                                    .fetch_optional(&self.db)
                                    .await
                                    .unwrap_or(Some(("error".to_string(), false)));

                                    match row_o {
                                        Some((username, is_admin)) => {
                                            let groups = get_groups_for_user(
                                                &w_id.as_ref().unwrap(),
                                                &username,
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
                                            Some(Authed {
                                                email,
                                                username,
                                                is_admin: is_admin || super_admin,
                                                groups,
                                                folders,
                                            })
                                        }
                                        None if super_admin || w_id.unwrap() == "starter" => {
                                            Some(Authed {
                                                email: email.clone(),
                                                username: email,
                                                is_admin: super_admin,
                                                groups: vec![],
                                                folders: vec![],
                                            })
                                        }
                                        None => None,
                                    }
                                } else {
                                    Some(Authed {
                                        email: email.to_string(),
                                        username: email,
                                        is_admin: super_admin,
                                        groups: Vec::new(),
                                        folders: Vec::new(),
                                    })
                                }
                            }
                            _ => None,
                        }
                    };
                    if let Some(authed) = authed_o.as_ref() {
                        self.cache
                            .insert(key, authed.clone(), Duration::from_secs(TTL_TOKEN_CACHE_S))
                            .await;
                    }
                    authed_o
                } else {
                    None
                }
            }
        }
    }

    pub async fn monitor(&self) {
        self.cache.monitor(20, 0.25, Duration::from_secs(10)).await;
    }
}

async fn extract_token<B: Send>(req: &mut RequestParts<B>) -> Option<String> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let from_cookie = match auth_header {
        Some(x) => Some(x.to_owned()),
        None => Extension::<Cookies>::from_request(req)
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
        None => Query::<Token>::from_request(req)
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
impl<B> FromRequest<B> for Tokened
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
        let already_tokened = req.extensions().get::<Tokened>();
        if let Some(tokened) = already_tokened {
            Ok(tokened.clone())
        } else {
            let token_o = extract_token(req).await;
            if let Some(token) = token_o {
                let tokened = Self { token };
                req.extensions_mut().insert(tokened.clone());
                Ok(tokened)
            } else {
                Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Authed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub groups: Vec<String>,
    pub folders: Vec<(String, bool)>,
}

#[async_trait]
impl<B> FromRequest<B> for Authed
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
        let already_authed = req.extensions().get::<Authed>();
        if let Some(authed) = already_authed {
            Ok(authed.clone())
        } else {
            let already_tokened = req.extensions().get::<Tokened>();
            let token_o = if let Some(token) = already_tokened {
                Some(token.token.clone())
            } else {
                extract_token(req).await
            };
            let path_vec: Vec<&str> = req.uri().path().split("/").collect();
            let workspace_id = if path_vec[0] == "" && path_vec[1] == "w" {
                Some(path_vec[2].to_owned())
            } else {
                None
            };
            if let Some(token) = token_o {
                if let Ok(Extension(cache)) = Extension::<Arc<AuthCache>>::from_request(req).await {
                    if let Some(authed) = cache.get_authed(workspace_id.clone(), &token).await {
                        req.extensions_mut().insert(authed.clone());
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

#[derive(Clone, Debug)]
pub struct OptAuthed(pub Option<Authed>);

#[async_trait]
impl<B> FromRequest<B> for OptAuthed
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
        Authed::from_request(req)
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

#[derive(FromRow, Serialize)]
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
}

#[derive(Deserialize)]
pub struct NewToken {
    pub label: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
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
    authed: Authed,
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
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(false);
    tx.commit().await?;
    Ok(Json(exists))
}

async fn list_users(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<UserWithUsage>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query(
        "
        SELECT usr.*, usage.*
          FROM usr
             , LATERAL (
                SELECT COALESCE(SUM(duration_ms + 1000)/1000 , 0) executions
                  FROM completed_job
                 WHERE workspace_id = usr.workspace_id
                   AND job_kind NOT IN ('flow', 'flowpreview')
                   AND email = usr.email
                   AND now() - '5 week'::interval < created_at 
               ) usage
         WHERE workspace_id = $1
         ",
    )
    .bind(&w_id)
    .try_map(|row| {
        // flatten not released yet https://github.com/launchbadge/sqlx/pull/1959
        Ok(UserWithUsage { user: FromRow::from_row(&row)?, usage: FromRow::from_row(&row)? })
    })
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_users_as_super_admin(
    authed: Authed,
    Extension(db): Extension<DB>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<GlobalUserInfo>> {
    let mut tx = db.begin().await?;
    require_super_admin(&mut tx, &authed.email).await?;
    let per_page = pagination.per_page.unwrap_or(10000).max(1);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;

    let rows = sqlx::query_as!(
        GlobalUserInfo,
        "SELECT email, login_type::text, verified, super_admin, name, company from password LIMIT \
         $1 OFFSET $2",
        per_page as i32,
        offset as i32
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

// async fn list_invite_codes(
//     authed: Authed,
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
    authed: Authed,
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
        .fetch_all(&mut tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_invites(
    authed: Authed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<WorkspaceInvite>> {
    let mut tx = db.begin().await?;
    let rows = sqlx::query_as!(
        WorkspaceInvite,
        "SELECT * from workspace_invite WHERE email = $1",
        authed.email
    )
    .fetch_all(&mut tx)
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
    Extension(cookie_domain): Extension<Arc<CookieDomain>>,
    Query(LogoutQuery { rd }): Query<LogoutQuery>,
) -> Result<Response> {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_path(COOKIE_PATH);
    let domain = cookie_domain.0.clone();
    if domain.is_some() {
        cookie.set_domain(domain.clone().unwrap());
    }
    cookies.remove(cookie);
    let mut tx = db.begin().await?;
    let email = sqlx::query_scalar!("DELETE FROM token WHERE token = $1 RETURNING email", token)
        .fetch_optional(&mut tx)
        .await?;
    if let Some(email) = email {
        audit_log(
            &mut tx,
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
    Authed { username, email, is_admin, groups, folders }: Authed,
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
                .into_iter()
                .filter_map(|x| if x.1 { Some(x.0) } else { None })
                .collect(),
        }))
    }
}

async fn global_whoami(
    Extension(db): Extension<DB>,
    Authed { email, .. }: Authed,
) -> JsonResult<GlobalUserInfo> {
    let user: GlobalUserInfo = sqlx::query_as!(
        GlobalUserInfo,
        "SELECT email, login_type::TEXT, super_admin, verified, name, company FROM password WHERE \
         email = $1",
        email
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching global identity: {e}")))?;

    Ok(Json(user))
}

async fn get_email(Authed { email, .. }: Authed) -> Result<String> {
    Ok(email)
}

async fn get_usage(Extension(db): Extension<DB>, Authed { email, .. }: Authed) -> Result<String> {
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
    let groups = get_groups_for_user(&w_id, username, db).await?;
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
            .into_iter()
            .filter_map(|x| if x.1 { Some(x.0) } else { None })
            .collect(),
    }))
}

pub async fn get_groups_for_user(w_id: &str, username: &str, db: &DB) -> Result<Vec<String>> {
    let groups = sqlx::query_scalar!(
        "SELECT group_ FROM usr_to_group where usr = $1 AND workspace_id = $2",
        username,
        w_id
    )
    .fetch_all(db)
    .await?;
    Ok(groups)
}

pub async fn require_owner_of_path(w_id: &str, username: &str, path: &str, db: &DB) -> Result<()> {
    let splitted = path.split("/").collect::<Vec<&str>>();
    if splitted[0] == "u" {
        if splitted[1] == username {
            return Ok(());
        } else {
            return Err(Error::BadRequest(format!(
                "only the owner {} is authorized to perform this operation",
                splitted[1]
            )));
        }
    } else if splitted[0] == "g" {
        let groups = get_groups_for_user(w_id, username, db).await?;
        if groups.contains(&username.to_string()) {
            return Ok(());
        } else {
            return Err(Error::BadRequest(format!(
                "{} is not a member of {} and hence is not authorized to perform this operation",
                username, splitted[1]
            )));
        }
    }
    Err(Error::BadRequest(format!("not recognized owner kind")))
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
//     Authed { email, .. }: Authed,
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
    Authed { email, .. }: Authed,
    Extension(db): Extension<DB>,
    Json(nu): Json<DeclineInvite>,
) -> Result<(StatusCode, String)> {
    let mut tx = db.begin().await?;

    let is_admin = sqlx::query_scalar!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2 RETURNING is_admin",
        nu.workspace_id,
        email,
    )
    .fetch_optional(&mut tx)
    .await?;

    audit_log(
        &mut tx,
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

async fn accept_invite(
    Authed { email, .. }: Authed,
    Extension(db): Extension<DB>,
    Json(nu): Json<AcceptInvite>,
) -> Result<(StatusCode, String)> {
    if &nu.username == "bot" {
        return Err(Error::BadRequest("bot is a reserved username".to_string()));
    }
    let mut tx = db.begin().await?;

    let r = sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1 AND email = $2 RETURNING is_admin, operator",
        nu.workspace_id,
        email,
    )
    .fetch_optional(&mut tx)
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
            &mut tx,
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
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(false);

    if already_exists_username {
        return Err(Error::BadRequest(format!(
            "user with username {} already exists in workspace {}",
            username, w_id
        )));
    }

    let already_exists_email = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
        &w_id,
        username,
    )
    .fetch_one(&mut tx)
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
    .execute(&mut tx)
    .await?;
    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        username,
        "all",
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
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

async fn update_workspace_user(
    Authed { username, is_admin, .. }: Authed,
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
        .execute(&mut tx)
        .await?;
    }

    if let Some(a) = eu.operator {
        sqlx::query_scalar!(
            "UPDATE usr SET operator = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            &username_to_update,
            &w_id
        )
        .execute(&mut tx)
        .await?;
    }

    if let Some(a) = eu.disabled {
        sqlx::query_scalar!(
            "UPDATE usr SET disabled = $1 WHERE username = $2 AND workspace_id = $3",
            a,
            &username_to_update,
            &w_id
        )
        .execute(&mut tx)
        .await?;
    }

    audit_log(
        &mut tx,
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
    Authed { email, .. }: Authed,
    Path(email_to_update): Path<String>,
    Extension(db): Extension<DB>,
    Json(eu): Json<EditUser>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    require_super_admin(&mut tx, &email).await?;

    if let Some(sa) = eu.is_super_admin {
        sqlx::query_scalar!(
            "UPDATE password SET super_admin = $1 WHERE email = $2",
            sa,
            &email_to_update
        )
        .execute(&mut tx)
        .await?;
    }

    audit_log(
        &mut tx,
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
    Authed { email, .. }: Authed,
    Path(email_to_delete): Path<String>,
    Extension(db): Extension<DB>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    require_super_admin(&mut tx, &email).await?;

    sqlx::query!("DELETE FROM password WHERE email = $1", &email_to_delete)
        .execute(&mut tx)
        .await?;

    let usernames = sqlx::query_scalar!(
        "DELETE FROM usr WHERE email = $1 RETURNING username",
        &email_to_delete
    )
    .fetch_all(&mut tx)
    .await?;

    for username in usernames {
        sqlx::query!("DELETE FROM password WHERE email = $1", &email_to_delete)
            .execute(&mut tx)
            .await?;

        sqlx::query!("DELETE FROM usr_to_group WHERE usr = $1", &username)
            .execute(&mut tx)
            .await?;

        sqlx::query!(
            "DELETE FROM workspace_invite WHERE email = $1",
            &email_to_delete
        )
        .execute(&mut tx)
        .await?;
    }
    audit_log(
        &mut tx,
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

async fn create_user(
    Authed { email, .. }: Authed,
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(nu): Json<NewUser>,
) -> Result<(StatusCode, String)> {
    let mut tx = db.begin().await?;

    require_super_admin(&mut tx, &email).await?;

    sqlx::query!(
        "INSERT INTO password(email, verified, password_hash, login_type, super_admin, name, \
         company)
    VALUES ($1, $2, $3, 'password', $4, $5, $6)",
        &nu.email,
        true,
        &hash_password(argon2, nu.password)?,
        &nu.super_admin,
        nu.name,
        nu.company
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &email,
        "users.add_global",
        ActionKind::Create,
        "global",
        Some(&nu.email),
        None,
    )
    .await?;
    tx.commit().await?;
    invite_user_to_all_auto_invite_worspaces(&db, &nu.email).await?;

    Ok((StatusCode::CREATED, format!("email {} created", nu.email)))
}

async fn delete_workspace_user(
    Authed { username, is_admin, .. }: Authed,
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
        "DELETE FROM usr WHERE email = $1 RETURNING username",
        email_to_delete
    )
    .fetch_one(&mut tx)
    .await?;

    sqlx::query!("DELETE FROM usr_to_group WHERE usr = $1", &username)
        .execute(&mut tx)
        .await?;

    audit_log(
        &mut tx,
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
    Authed { username, email, .. }: Authed,
    Json(EditPassword { password }): Json<EditPassword>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    let custom_type = sqlx::query_scalar!(
        "SELECT login_type::TEXT FROM password WHERE email = $1",
        &email
    )
    .fetch_one(&mut tx)
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
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
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
    Extension(is_secure): Extension<Arc<IsSecure>>,
    Extension(cookie_domain): Extension<Arc<CookieDomain>>,
    Json(Login { email, password }): Json<Login>,
) -> Result<String> {
    let mut tx = db.begin().await?;

    let email_w_h: Option<(String, String, bool)> = sqlx::query_as(
        "SELECT email, password_hash, super_admin FROM password WHERE email = $1 AND login_type = \
         'password'",
    )
    .bind(&email)
    .fetch_optional(&mut tx)
    .await?;

    if let Some((email, hash, super_admin)) = email_w_h {
        let parsed_hash =
            PasswordHash::new(&hash).map_err(|e| Error::InternalErr(e.to_string()))?;
        if argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err()
        {
            Err(Error::BadRequest("Invalid login".to_string()))
        } else {
            let token = create_session_token(
                &email,
                super_admin,
                &mut tx,
                cookies,
                is_secure.0,
                &cookie_domain.as_ref().0,
            )
            .await?;
            tx.commit().await?;
            Ok(token)
        }
    } else {
        Err(Error::BadRequest("Invalid login".to_string()))
    }
}

pub async fn create_session_token<'c>(
    email: &str,
    super_admin: bool,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    cookies: Cookies,
    is_secure: bool,
    domain: &Option<String>,
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
    .execute(tx)
    .await?;
    let mut cookie = Cookie::new(COOKIE_NAME, token.clone());
    cookie.set_secure(is_secure);
    cookie.set_same_site(cookie::SameSite::Lax);
    cookie.set_http_only(true);
    cookie.set_path(COOKIE_PATH);
    if domain.is_some() {
        cookie.set_domain(domain.clone().unwrap());
    }
    let mut expire: OffsetDateTime = time::OffsetDateTime::now_utc();
    expire += time::Duration::days(3);
    cookie.set_expires(expire);
    cookies.add(cookie);
    Ok(token)
}

async fn create_token(
    Extension(db): Extension<DB>,
    Authed { email, .. }: Authed,
    Json(new_token): Json<NewToken>,
) -> Result<(StatusCode, String)> {
    let token = rd_string(30);
    let mut tx = db.begin().await?;

    let is_super_admin =
        sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(&mut tx)
            .await?
            .unwrap_or(false);
    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin)
            VALUES ($1, $2, $3, $4, $5)",
        token,
        email,
        new_token.label,
        new_token.expiration,
        is_super_admin
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &email,
        "users.token.create",
        ActionKind::Delete,
        &"global",
        Some(&token[0..10]),
        None,
    )
    .instrument(tracing::info_span!("token", email = &email))
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, token))
}

async fn list_tokens(
    Extension(db): Extension<DB>,
    Authed { email, .. }: Authed,
) -> JsonResult<Vec<TruncatedToken>> {
    let rows = sqlx::query_as!(
        TruncatedToken,
        "SELECT label, concat(substring(token for 10)) as token_prefix, expiration, created_at, \
         last_used_at FROM token WHERE email = $1
         ORDER BY created_at DESC",
        email,
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(rows))
}

async fn delete_token(
    Extension(db): Extension<DB>,
    Authed { email, .. }: Authed,
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
    .fetch_all(&mut tx)
    .await?;

    audit_log(
        &mut tx,
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
    Authed { username, .. }: Authed,
) -> Result<String> {
    let mut tx = db.begin().await?;
    sqlx::query!(
        "DELETE FROM usr WHERE workspace_id = $1 AND username = $2",
        &w_id,
        username
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
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

pub async fn delete_expired_items_perdiodically(
    db: &DB,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    loop {
        let tokens_deleted_r: std::result::Result<Vec<String>, _> = sqlx::query_scalar(
            "DELETE FROM token WHERE expiration <= now()
        RETURNING concat(substring(token for 10), '*****')",
        )
        .fetch_all(db)
        .await;

        match tokens_deleted_r {
            Ok(tokens) => tracing::debug!("deleted {} tokens: {:?}", tokens.len(), tokens),
            Err(e) => tracing::error!("Error deleting token: {}", e.to_string()),
        }

        let magic_links_deleted_r: std::result::Result<Vec<String>, _> = sqlx::query_scalar(
            "DELETE FROM magic_link WHERE expiration <= now()
        RETURNING concat(substring(token for 10), '*****')",
        )
        .fetch_all(db)
        .await;

        match magic_links_deleted_r {
            Ok(tokens) => tracing::debug!("deleted {} tokens: {:?}", tokens.len(), tokens),
            Err(e) => tracing::error!("Error deleting token: {}", e.to_string()),
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(600))     => (),
            _ = rx. recv() => {
                 println!("received killpill for delete expired tokens");
                 break;
            }
        }
    }
}

pub fn truncate_token(token: &str) -> String {
    let mut s = token[..10].to_owned();
    s.push_str("*****");
    s
}
