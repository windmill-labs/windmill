/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub mod scopes;

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use axum::async_trait;
use axum::extract::FromRequestParts;
use http::request::Parts;

use windmill_audit::audit_oss::AuditAuthorable;
use windmill_common::{
    auth::{is_devops_email, is_super_admin_email},
    db::{Authable, Authed, AuthedRef},
    error::{self, Error, Result},
    DB,
};

use scopes::ScopeDefinition;

// ------------ FromRequestParts bridge via OnceLock ------------

type OptJobAuthedResolver = Box<
    dyn Fn(
            Parts,
        ) -> Pin<
            Box<
                dyn Future<Output = std::result::Result<(OptJobAuthed, Parts), (Error, Parts)>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

static OPT_JOB_AUTHED_RESOLVER: OnceLock<OptJobAuthedResolver> = OnceLock::new();

pub fn set_opt_job_authed_resolver<F, Fut>(f: F)
where
    F: Fn(Parts) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = std::result::Result<(OptJobAuthed, Parts), (Error, Parts)>>
        + Send
        + 'static,
{
    OPT_JOB_AUTHED_RESOLVER
        .set(Box::new(move |parts| Box::pin(f(parts))))
        .ok();
}

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
            "path cannot be empty for this operation".to_string(),
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

// ------------ FromRequestParts impls (delegates to OnceLock callback) ------------

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
        let resolver = OPT_JOB_AUTHED_RESOLVER
            .get()
            .expect("OPT_JOB_AUTHED_RESOLVER not initialized; call set_opt_job_authed_resolver during startup");

        // Swap out parts so we can pass ownership to the callback
        let owned_parts = std::mem::replace(parts, empty_parts());
        match resolver(owned_parts).await {
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
