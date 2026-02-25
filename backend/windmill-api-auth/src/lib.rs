/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub mod auth;
#[cfg(feature = "private")]
pub mod ee;
pub mod ee_oss;
pub mod scopes;

use axum::async_trait;
use axum::extract::FromRequestParts;
use http::request::Parts;

use windmill_audit::audit_oss::AuditAuthorable;
use windmill_common::{
    auth::{fetch_authed_from_permissioned_as, is_devops_email, is_super_admin_email},
    db::{Authable, Authed, AuthedRef},
    error::{self, Error, Result},
    users::username_to_permissioned_as,
    DB,
};

use scopes::ScopeDefinition;

// Re-export key auth types and functions
pub use auth::{
    invalidate_token_from_cache, AuthCache, ExpiringAuthCache, OptTokened, Tokened,
    TruncatedTokenWithEmail, AUTH_CACHE,
};

// ------------ ApiAuthed & OptJobAuthed types ------------

#[derive(Default, Clone, Debug)]
pub struct OptJobAuthed {
    pub job_id: Option<uuid::Uuid>,
    pub authed: ApiAuthed,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq)]
pub struct ApiAuthed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    // (folder name, can write, is owner)
    pub folders: Vec<(String, bool, bool)>,
    pub scopes: Option<Vec<String>>,
    pub username_override: Option<String>,
    pub token_prefix: Option<String>,
}

impl ApiAuthed {
    pub fn to_authed_ref<'e>(&'e self) -> AuthedRef<'e> {
        AuthedRef {
            email: &self.email,
            username: &self.username,
            is_admin: &self.is_admin,
            is_operator: &self.is_operator,
            groups: &self.groups,
            folders: &self.folders,
            scopes: &self.scopes,
            token_prefix: &self.token_prefix,
        }
    }

    pub fn display_username(&self) -> &str {
        self.username_override.as_ref().unwrap_or(&self.username)
    }
}

impl From<ApiAuthed> for Authed {
    fn from(value: ApiAuthed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
            token_prefix: value.token_prefix,
        }
    }
}

impl From<Authed> for ApiAuthed {
    fn from(value: Authed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
            username_override: None,
            token_prefix: value.token_prefix,
        }
    }
}

impl AuditAuthorable for ApiAuthed {
    fn username(&self) -> &str {
        self.username.as_str()
    }
    fn email(&self) -> &str {
        self.email.as_str()
    }
    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }
    fn token_prefix(&self) -> Option<&str> {
        self.token_prefix.as_deref()
    }
}

impl Authable for ApiAuthed {
    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[std::string::String]> {
        self.scopes.as_ref().map(|x| x.as_slice())
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }
}

// ------------ McpAuth impl (feature-gated) ------------

#[cfg(feature = "mcp")]
impl windmill_mcp::server::McpAuth for ApiAuthed {
    fn username(&self) -> &str {
        &self.username
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[String]> {
        self.scopes.as_deref()
    }
}

// ------------ Utility functions ------------

pub async fn require_super_admin(db: &DB, email: &str) -> error::Result<()> {
    let is_admin = is_super_admin_email(db, email).await?;

    if !is_admin {
        Err(Error::NotAuthorized(
            "This endpoint requires the caller to be a super admin".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub fn check_scopes<F>(authed: &ApiAuthed, required: F) -> error::Result<()>
where
    F: FnOnce() -> String,
{
    if let Some(scopes) = authed.scopes.as_ref() {
        let mut is_scoped_token = false;
        let required_scope = ScopeDefinition::from_scope_string(&required())?;
        for scope in scopes {
            if !scope.starts_with("if_jobs:filter_tags:") {
                if !is_scoped_token {
                    is_scoped_token = true;
                }

                match ScopeDefinition::from_scope_string(scope) {
                    Ok(scope) if scope.includes(&required_scope) => return Ok(()),
                    _ => {}
                }
            }
        }

        if is_scoped_token {
            return Err(Error::NotAuthorized(format!(
                "Required scope: {}",
                required_scope.as_string()
            )));
        }
    }
    Ok(())
}

pub async fn require_devops_role(db: &DB, email: &str) -> error::Result<()> {
    let is_devops = is_devops_email(db, email).await?;

    if is_devops {
        Ok(())
    } else {
        Err(Error::NotAuthorized(
            "This endpoint requires the caller to have the `devops` role".to_string(),
        ))
    }
}

// ------------ Folder ownership checks ------------

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
        Err(Error::NotAuthorized(format!(
            "You are not owner of the folder {}",
            name
        )))
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
            require_is_owner(authed, splitted[1])
        } else {
            Err(Error::BadRequest(format!(
                "Not recognized path kind: {}",
                path
            )))
        }
    } else {
        Err(Error::BadRequest(
            "Cannot be owner of an empty path".to_string(),
        ))
    }
}

// ------------ Scope tag helpers ------------

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

// ------------ Maybe refresh folders ------------

pub async fn maybe_refresh_folders(
    path: &str,
    w_id: &str,
    authed: ApiAuthed,
    db: &DB,
) -> ApiAuthed {
    use windmill_common::auth::{get_folders_for_user, get_groups_for_user};

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

// ------------ FromRequestParts impls (direct call to auth module) ------------

#[async_trait]
impl<S> FromRequestParts<S> for ApiAuthed
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let opt_job_authed = OptJobAuthed::from_request_parts(parts, _state).await?;
        Ok(opt_job_authed.authed)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OptJobAuthed
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        // Swap out parts so we can pass ownership to resolve_opt_job_authed
        let owned_parts = std::mem::replace(parts, empty_parts());
        match auth::resolve_opt_job_authed(owned_parts).await {
            Ok((result, returned_parts)) => {
                *parts = returned_parts;
                Ok(result)
            }
            Err((err, returned_parts)) => {
                *parts = returned_parts;
                Err(err)
            }
        }
    }
}

fn empty_parts() -> Parts {
    let (parts, _body) = http::Request::new(()).into_parts();
    parts
}

// ------------ OptAuthed (optional auth extractor) ------------

#[derive(Clone, Debug)]
pub struct OptAuthed(pub Option<ApiAuthed>);

#[async_trait]
impl<S> FromRequestParts<S> for OptAuthed
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, String);

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

// ------------ fetch_api_authed helpers ------------

lazy_static::lazy_static! {
    static ref API_AUTHED_CACHE: quick_cache::sync::Cache<(String,String,String), ExpiringAuthCache> = quick_cache::sync::Cache::new(300);
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
                fetch_authed_from_permissioned_as(&permissioned_as, &email, w_id, db).await?;

            let api_authed = ApiAuthed {
                username: authed.username,
                email,
                is_admin: authed.is_admin,
                is_operator: authed.is_operator,
                groups: authed.groups,
                folders: authed.folders,
                scopes: authed.scopes,
                username_override: None,
                token_prefix: authed.token_prefix,
            };

            API_AUTHED_CACHE.insert(
                key,
                ExpiringAuthCache {
                    authed: api_authed.clone(),
                    expiry: chrono::Utc::now() + chrono::Duration::try_seconds(120).unwrap(),
                    job_id: None,
                },
            );

            api_authed
        }
    };

    api_authed.username_override = username_override;
    Ok(api_authed)
}

// ------------ Token creation ------------

#[derive(serde::Deserialize)]
pub struct NewToken {
    pub label: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub impersonate_email: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub workspace_id: Option<String>,
}

impl NewToken {
    pub fn new(
        label: Option<String>,
        expiration: Option<chrono::DateTime<chrono::Utc>>,
        impersonate_email: Option<String>,
        scopes: Option<Vec<String>>,
        workspace_id: Option<String>,
    ) -> Self {
        Self { label, expiration, impersonate_email, scopes, workspace_id }
    }
}

pub async fn create_token_internal(
    tx: &mut sqlx::PgConnection,
    db: &DB,
    authed: &ApiAuthed,
    token_config: NewToken,
) -> Result<String> {
    use tracing::Instrument;
    use windmill_audit::{audit_oss::audit_log, ActionKind};
    use windmill_common::{utils::rd_string, worker::CLOUD_HOSTED};

    let token = rd_string(32);

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
                .fetch_one(db)
                .await?;
        if nb_tokens.unwrap_or(0) >= 10000 {
            return Err(Error::BadRequest(
                "You have reached the maximum number of tokens (10000) on cloud. Contact support@windmill.dev to increase the limit"
                    .to_string(),
            ));
        }
    }
    let rows = sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin, scopes, workspace_id)
            SELECT $1, $2, $3, $4, $5, $6, $7
            WHERE $7::varchar IS NULL OR NOT EXISTS(
                SELECT 1 FROM workspace WHERE id = $7 AND deleted = true
            )",
        token,
        authed.email,
        token_config.label,
        token_config.expiration,
        is_super_admin,
        token_config.scopes.as_ref().map(|x| x.as_slice()),
        token_config.workspace_id,
    )
    .execute(&mut *tx)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "Cannot create a token for an archived workspace".to_string(),
        ));
    }

    audit_log(
        &mut *tx,
        authed,
        "users.token.create",
        ActionKind::Create,
        &"global",
        Some(&token[0..10]),
        None,
    )
    .instrument(tracing::info_span!("token", email = &authed.email))
    .await?;

    Ok(token)
}

// ------------ Permission helpers ------------

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

// ------------ Preview access check ------------

pub fn require_path_read_access_for_preview(
    authed: &ApiAuthed,
    path: &Option<String>,
) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };

    if authed.is_admin {
        return Ok(());
    }

    if path.is_empty() {
        return Ok(());
    }

    let splitted: Vec<&str> = path.split('/').collect();
    if splitted.len() < 2 {
        return Err(Error::BadRequest(format!(
            "Invalid path format for preview job: {}",
            path
        )));
    }

    match splitted[0] {
        "u" => {
            if splitted[1] == authed.username {
                Ok(())
            } else {
                Err(Error::BadRequest(format!(
                    "You can only run preview jobs in your own namespace (u/{}) or in folders you have read access to",
                    authed.username
                )))
            }
        }
        "f" => {
            let folder = splitted[1];
            if authed.folders.iter().any(|(f, _, _)| f == folder) {
                Ok(())
            } else {
                Err(Error::BadRequest(format!(
                    "You do not have read access to folder '{}'. Preview jobs require at least read access to the target folder.",
                    folder
                )))
            }
        }
        "hub" => Ok(()),
        _ => Err(Error::BadRequest(format!(
            "Invalid path format for preview job: {}. Path must start with 'u/' or 'f/'",
            path
        ))),
    }
}
