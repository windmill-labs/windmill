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

use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use http::request::Parts;

use windmill_audit::audit_oss::AuditAuthorable;
use windmill_common::{
    auth::{
        fetch_authed_from_permissioned_as, hash_token, is_devops_email, is_super_admin_email,
        TOKEN_PREFIX_LEN,
    },
    db::{Authable, Authed, AuthedRef},
    error::{self, Error, Result},
    users::username_to_permissioned_as,
    DB,
};

use scopes::ScopeDefinition;

// Re-export key auth types and functions
pub use auth::{
    get_end_user_email, invalidate_token_from_cache, is_no_auth, AuthCache, ExpiringAuthCache,
    OptTokened, Tokened, TruncatedTokenWithEmail, AUTH_CACHE,
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
    pub read_only: bool,
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
            read_only: false,
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

    fn read_only(&self) -> bool {
        self.read_only
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

/// Forbid sensitive global user/token management when authenticated as a
/// superadmin *via a job token* (`WM_TOKEN`).
///
/// A `WM_TOKEN`'s identity is derived from an app/flow `on_behalf_of`, which a
/// non-admin `wm_deployers` member can point at a superadmin. Trusting it for
/// these operations would let them establish *persistent* superadmin (promote a
/// user, reset a superadmin's password, mint a superadmin token, ...). `job_id`
/// is set only for `WM_TOKEN`s; regular session/API tokens have it `None`, so a
/// real superadmin who needs this from a script uses a dedicated superadmin API
/// token (which only a real superadmin can create) instead of `$WM_TOKEN`.
pub async fn forbid_superadmin_job_token(
    db: &DB,
    email: &str,
    job_id: Option<uuid::Uuid>,
) -> error::Result<()> {
    if job_id.is_some() && is_super_admin_email(db, email).await? {
        return Err(Error::NotAuthorized(
            "This operation cannot be performed with a job token ($WM_TOKEN) that runs as a \
             superadmin. If a script genuinely needs to do this, create a dedicated superadmin \
             token from the User settings drawer (the 'Tokens' section), store it as a secret, \
             and use that token explicitly instead of $WM_TOKEN."
                .to_owned(),
        ));
    }
    Ok(())
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
            return Err(Error::PermissionDenied(format!(
                "Required scope: {}",
                required_scope.as_string()
            )));
        }
    }
    Ok(())
}

/// Returns the caller's "real" scope restrictions: every scope other than
/// `if_jobs:filter_tags:` tag filters. `None` means the token is unscoped and
/// has the full privileges of its user; `Some` means it is restricted to the
/// returned scopes. An empty or filter-tags-only scope list is treated as
/// unscoped, mirroring `check_scopes`/`check_route_access`.
fn scope_restrictions(scopes: Option<&[String]>) -> Option<Vec<&String>> {
    let restrictions: Vec<&String> = scopes?
        .iter()
        .filter(|s| !s.starts_with("if_jobs:filter_tags:"))
        .collect();
    (!restrictions.is_empty()).then_some(restrictions)
}

/// Enforce monotonic privilege when a token lifecycle endpoint mints or rescopes
/// a credential on behalf of `authed`: the resulting credential must never be
/// more privileged than the caller's own token.
///
/// - An unscoped caller may grant any scopes (this is the existing UI/CLI flow).
/// - A scope-restricted caller may only grant scopes that are a subset of its
///   own, and may never produce an unscoped credential.
///
/// Without this, a `users:write` token could create or rescope a token to be
/// unscoped, and a `users:read` token could refresh into an unscoped session —
/// escaping its own restrictions.
pub fn ensure_scopes_within_caller(
    authed: &ApiAuthed,
    requested_scopes: Option<&[String]>,
) -> error::Result<()> {
    if let Some(caller_restrictions) = scope_restrictions(authed.scopes.as_deref()) {
        let Some(requested_restrictions) = scope_restrictions(requested_scopes) else {
            return Err(Error::PermissionDenied(
                "A scope-restricted token cannot create or update a token with broader (unscoped) \
                 privileges"
                    .to_string(),
            ));
        };

        // MCP scopes (`mcp:all`, `mcp:favorites`, `mcp:scripts:*`, etc.) use a
        // custom format that ScopeDefinition::from_scope_string parses
        // permissively but the MCP runtime interprets via its own parser
        // (parse_mcp_scopes). The two views disagree — e.g. the generic parser
        // accepts `mcp:scripts` as an unrestricted-resource scope, while the
        // MCP runtime ignores it as unrecognized but interprets `mcp:scripts:*`
        // as granting all scripts. So generic containment would silently allow
        // `mcp:scripts` → `mcp:scripts:*` (a widening). Legitimate MCP token
        // issuance goes through the OAuth gateway (mcp/oauth_server.rs), not
        // these user-token endpoints, so require byte-identical match for MCP
        // scopes here rather than trying to mirror MCP semantics in two places.
        // Unparseable non-MCP caller scopes are intentionally dropped
        // (fail-closed): a caller scope that fails to parse can only narrow
        // the set of requested scopes that get covered, never widen it.
        // Unparseable requested scopes surface as `BadRequest`, which is what
        // we want — the client is sending garbage.
        let parsed_caller: Vec<ScopeDefinition> = caller_restrictions
            .iter()
            .filter(|s| !s.starts_with("mcp:"))
            .filter_map(|s| ScopeDefinition::from_scope_string(s).ok())
            .collect();
        let caller_mcp: std::collections::HashSet<&str> = caller_restrictions
            .iter()
            .filter(|s| s.starts_with("mcp:"))
            .map(|s| s.as_str())
            .collect();

        for requested in requested_restrictions {
            if requested.starts_with("mcp:") {
                if !caller_mcp.contains(requested.as_str()) {
                    return Err(Error::PermissionDenied(format!(
                        "A scope-restricted token cannot grant MCP scope '{requested}' unless the \
                         caller holds the same scope verbatim"
                    )));
                }
                continue;
            }
            let requested_scope = ScopeDefinition::from_scope_string(requested)?;
            let covered = parsed_caller
                .iter()
                .any(|caller_scope| scope_contains(caller_scope, &requested_scope));
            if !covered {
                return Err(Error::PermissionDenied(format!(
                    "A scope-restricted token cannot grant scope '{requested}' which exceeds its \
                     own scopes"
                )));
            }
        }
    }

    // `if_jobs:filter_tags:` fences which job tags a token can run on (enforced
    // at job operations as `v2_job.tag = ANY(...)`), and is checked independently
    // of domain/action/resource subset. A caller restricted by filter_tags must
    // not be able to mint or rescope a credential that drops or widens the fence
    // — even if the caller has no other scope restrictions (filter_tags-only
    // tokens otherwise look "unscoped" to `scope_restrictions`).
    if let Some(caller_tags) = first_filter_tags(authed.scopes.as_deref()) {
        let Some(requested_tags) = first_filter_tags(requested_scopes) else {
            return Err(Error::PermissionDenied(
                "A token restricted by if_jobs:filter_tags cannot mint or rescope a token that \
                 drops the tag restriction"
                    .to_string(),
            ));
        };
        let caller_set: std::collections::HashSet<&str> = caller_tags.iter().copied().collect();
        for tag in &requested_tags {
            if !caller_set.contains(tag) {
                return Err(Error::PermissionDenied(format!(
                    "A token restricted by if_jobs:filter_tags cannot grant tag '{tag}' which is \
                     not within its own filter_tags"
                )));
            }
        }
    }

    Ok(())
}

/// Tags from the first `if_jobs:filter_tags:<a,b,...>` scope, matching the
/// semantics of [`get_scope_tags`] (which is what the job runtime consults).
/// Returns `None` if no such scope is present.
fn first_filter_tags(scopes: Option<&[String]>) -> Option<Vec<&str>> {
    scopes?.iter().find_map(|s| {
        s.strip_prefix("if_jobs:filter_tags:")
            .map(|tags| tags.split(',').collect())
    })
}

/// Whether `caller` grants at least everything `requested` grants (directional
/// containment).
///
/// This is intentionally NOT `ScopeDefinition::includes`: that method answers
/// "does this scope grant access to a required action" using OR semantics over
/// resources (any overlap counts, and a `*` on either side matches), which is
/// correct for access checks but unsafe for subset checks — it would let a
/// token scoped to `scripts:read:f/team/a` mint `scripts:read:*` or
/// `scripts:read:f/team/a,f/other/b`. Subset containment instead requires that
/// EVERY requested resource is covered by SOME caller resource.
fn scope_contains(caller: &ScopeDefinition, requested: &ScopeDefinition) -> bool {
    if caller.domain != requested.domain {
        return false;
    }

    // write subsumes read; otherwise the action must match exactly.
    match (caller.action.as_str(), requested.action.as_str()) {
        (c, r) if c == r || (c == "write" && r == "read") => {}
        _ => return false,
    }

    if caller.domain == "jobs" && caller.action == "run" {
        match (&caller.kind, &requested.kind) {
            (Some(caller_kind), Some(requested_kind)) if caller_kind != requested_kind => {
                return false
            }
            // Caller pinned to a kind, but the request covers any kind.
            (Some(_), None) => return false,
            _ => {}
        }
    }

    match (&caller.resource, &requested.resource) {
        // Caller is unrestricted on resources: covers everything.
        (None, _) => true,
        // Caller is resource-restricted but the request is not: broader.
        (Some(_), None) => false,
        (Some(caller_resources), Some(requested_resources)) => {
            resource_set_contains(caller_resources, requested_resources)
        }
    }
}

/// Every resource in `requested` must be covered by some resource in `caller`.
fn resource_set_contains(caller: &[String], requested: &[String]) -> bool {
    if caller.iter().any(|r| r == "*") {
        return true;
    }
    requested
        .iter()
        .all(|req| req != "*" && caller.iter().any(|c| resource_covers(c, req)))
}

/// Directional: does the single caller resource pattern cover `requested`?
/// `caller` may be an exact path or a `<prefix>/*` subtree wildcard; `requested`
/// may itself be a subtree wildcard, in which case the whole requested subtree
/// must fall within the caller's subtree.
fn resource_covers(caller: &str, requested: &str) -> bool {
    if caller == requested {
        return true;
    }
    let Some(prefix) = caller.strip_suffix("/*") else {
        // An exact caller resource only covers itself (handled above).
        return false;
    };
    let requested_base = requested.strip_suffix("/*").unwrap_or(requested);
    requested_base == prefix
        || (requested_base.starts_with(prefix)
            && requested_base.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Returns a predicate that checks whether `path` is within the token's
/// scope for `{domain}:{action}:{path}`. For tokens without scope
/// restrictions (no scopes at all, or only `if_jobs:filter_tags:*` scopes),
/// the predicate always returns `true`.
///
/// Pre-parses the token's scopes once so the returned closure can cheaply
/// filter large listings without re-parsing on each call.
pub fn build_scope_path_predicate(
    authed: &ApiAuthed,
    domain: &str,
    action: &str,
) -> impl Fn(&str) -> bool {
    // Mirror check_scopes semantics: a token is "scope-restricted" iff it has
    // at least one non-`if_jobs:filter_tags:` scope. Unparseable scopes still
    // count as restrictive — they just match nothing.
    let (is_scoped_token, parsed): (bool, Vec<ScopeDefinition>) = match authed.scopes.as_ref() {
        Some(scopes) => {
            let mut is_scoped = false;
            let parsed = scopes
                .iter()
                .filter(|s| !s.starts_with("if_jobs:filter_tags:"))
                .inspect(|_| is_scoped = true)
                .filter_map(|s| ScopeDefinition::from_scope_string(s).ok())
                .collect();
            (is_scoped, parsed)
        }
        None => (false, Vec::new()),
    };
    let domain = domain.to_string();
    let action = action.to_string();

    move |path: &str| -> bool {
        if !is_scoped_token {
            return true;
        }
        let required =
            match ScopeDefinition::from_scope_string(&format!("{}:{}:{}", domain, action, path)) {
                Ok(r) => r,
                Err(_) => return false,
            };
        parsed.iter().any(|s| s.includes(&required))
    }
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
        // A valid path is at least `<kind>/<name>` (e.g. `u/alice/...`,
        // `f/folder/...`). Guard the `splitted[1]` accesses below so a
        // malformed single-segment path returns a clear error instead of
        // panicking with an out-of-bounds index.
        if splitted.len() < 2 {
            return Err(Error::BadRequest(format!(
                "Invalid path '{}': a valid path starts with 'u/<user>/' or 'f/<folder>/'",
                path
            )));
        }
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

impl<S> OptionalFromRequestParts<S> for ApiAuthed
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Option<Self>, Self::Rejection> {
        Ok(
            <Self as FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

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

impl<S> FromRequestParts<S> for OptAuthed
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        <ApiAuthed as FromRequestParts<S>>::from_request_parts(parts, state)
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
                read_only: false,
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
    #[serde(default)]
    pub read_only: Option<bool>,
}

impl NewToken {
    pub fn new(
        label: Option<String>,
        expiration: Option<chrono::DateTime<chrono::Utc>>,
        impersonate_email: Option<String>,
        scopes: Option<Vec<String>>,
        workspace_id: Option<String>,
        read_only: Option<bool>,
    ) -> Self {
        Self { label, expiration, impersonate_email, scopes, workspace_id, read_only }
    }
}

/// Low-level token mint shared by trusted callers (the user-facing
/// `tokens/create` handler and internal mints such as native-trigger webhook
/// tokens). It does NOT enforce that `token_config.scopes` is within the
/// caller's own scopes — callers exposed to untrusted input must call
/// [`ensure_scopes_within_caller`] first (internal narrowing mints intentionally
/// skip it, since their scopes derive from the action being authorized, not the
/// caller's token).
pub async fn create_token_internal(
    tx: &mut sqlx::PgConnection,
    db: &DB,
    authed: &ApiAuthed,
    token_config: NewToken,
) -> Result<String> {
    use tracing::Instrument;
    use windmill_audit::{audit_oss::audit_log, ActionKind};
    use windmill_common::{
        min_version::MIN_VERSION_SUPPORTS_TOKEN_HASH, utils::rd_string, worker::CLOUD_HOSTED,
    };

    let token = rd_string(32);
    let t_hash = hash_token(&token);
    let t_prefix = token.get(..TOKEN_PREFIX_LEN).unwrap_or(&token);

    // Write plaintext token column until all workers support hash-based lookup
    let plaintext: Option<&str> = if MIN_VERSION_SUPPORTS_TOKEN_HASH.met().await {
        None
    } else {
        Some(&token)
    };

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
            (token_hash, token_prefix, token, email, label, expiration, super_admin, scopes, workspace_id, read_only)
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            WHERE $9::varchar IS NULL OR NOT EXISTS(
                SELECT 1 FROM workspace WHERE id = $9 AND deleted = true
            )",
        t_hash,
        t_prefix,
        plaintext as Option<&str>,
        authed.email,
        token_config.label,
        token_config.expiration,
        is_super_admin,
        token_config.scopes.as_ref().map(|x| x.as_slice()),
        token_config.workspace_id,
        token_config.read_only.unwrap_or(false),
    )
    .execute(&mut *tx)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "Cannot create a token for an archived workspace".to_string(),
        ));
    }

    register_token_expiry_notification(
        &mut *tx,
        &t_hash,
        token_config.label.as_deref(),
        token_config.expiration,
    )
    .await;

    audit_log(
        &mut *tx,
        authed,
        "users.token.create",
        ActionKind::Create,
        &"global",
        Some(t_prefix),
        None,
    )
    .instrument(tracing::info_span!("token", email = &authed.email))
    .await?;

    Ok(token)
}

/// Insert a pending expiry notification row for user tokens that have an expiration.
/// Stores the token_hash so the join in check_expiring_tokens works even when
/// the plaintext token column is NULL (after hash migration).
pub async fn register_token_expiry_notification(
    tx: &mut sqlx::PgConnection,
    token_hash: &str,
    label: Option<&str>,
    expiration: Option<chrono::DateTime<chrono::Utc>>,
) {
    let Some(expiration) = expiration else { return };
    // System tokens don't get expiry notifications.
    if !windmill_common::auth::is_user_token(label) {
        return;
    }
    if let Err(e) = sqlx::query!(
        "INSERT INTO token_expiry_notification (token_hash, expiration) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        token_hash,
        expiration,
    )
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Failed to register token expiry notification: {}", e);
    }
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

    // Reject path traversal before any privilege-based short-circuit. A Preview's
    // path is request-supplied and bypasses the DB `proper_id` CHECK that deployed
    // runnables get; it then flows to the worker where it builds on-disk module
    // directories. A `..` segment or an absolute path could let a write escape the
    // per-job dir.
    if path.starts_with('/') || path.split('/').any(|seg| seg == "..") || path.contains('\0') {
        return Err(Error::BadRequest(format!(
            "Invalid path for preview job: {}",
            path
        )));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn authed_with_scopes(scopes: Option<Vec<&str>>) -> ApiAuthed {
        ApiAuthed {
            scopes: scopes.map(|v| v.into_iter().map(String::from).collect()),
            ..Default::default()
        }
    }

    // Regression tests for the Preview path traversal: a Preview's path skips the
    // DB `proper_id` CHECK and reaches the worker, where it builds on-disk module
    // dirs. Traversal must be rejected even for admins, who otherwise bypass the
    // namespace/folder access check.
    #[test]
    fn preview_path_rejects_traversal() {
        let admin = ApiAuthed { is_admin: true, username: "admin".into(), ..Default::default() };
        for path in [
            "u/admin/../../../../../../tmp/evil/payload",
            "../../tmp/evil",
            "/tmp/evil",
            "u/admin/ok/../../../../etc/cron.d/x",
        ] {
            assert!(
                require_path_read_access_for_preview(&admin, &Some(path.to_string())).is_err(),
                "expected traversal path to be rejected: {path}"
            );
        }
    }

    #[test]
    fn preview_path_allows_legitimate_paths() {
        let alice = ApiAuthed { username: "alice".into(), ..Default::default() };
        assert!(require_path_read_access_for_preview(&alice, &None).is_ok());
        assert!(require_path_read_access_for_preview(&alice, &Some(String::new())).is_ok());
        assert!(
            require_path_read_access_for_preview(&alice, &Some("u/alice/my_script".into())).is_ok()
        );

        let admin = ApiAuthed { is_admin: true, username: "admin".into(), ..Default::default() };
        assert!(
            require_path_read_access_for_preview(&admin, &Some("hub/foo/bar/baz".into())).is_ok()
        );
        // `..` only as a substring of a segment is a valid name, not traversal.
        assert!(
            require_path_read_access_for_preview(&admin, &Some("f/team/my..script".into())).is_ok()
        );
    }

    // Regression for WIN-2157: a malformed single-segment path (e.g. a draft
    // saved at a bare `u`) must return a clear error, not panic on the
    // `splitted[1]` index. Non-admins reach this branch (admins short-circuit).
    #[test]
    fn require_owner_of_path_rejects_malformed_path_without_panicking() {
        let alice = ApiAuthed { username: "alice".into(), ..Default::default() };
        for path in ["u", "f", "g", "nonsense"] {
            let err =
                require_owner_of_path(&alice, path).expect_err("malformed path must be rejected");
            assert!(
                matches!(err, Error::BadRequest(_)),
                "expected BadRequest for '{path}', got {err:?}"
            );
        }
        // A well-formed foreign path returns the owner error, not a malformed one.
        assert!(require_owner_of_path(&alice, "u/bob/script").is_err());
        // The user's own namespace resolves.
        assert!(require_owner_of_path(&alice, "u/alice/script").is_ok());
    }

    #[test]
    fn predicate_no_scopes_allows_all() {
        let authed = authed_with_scopes(None);
        let allowed = build_scope_path_predicate(&authed, "resources", "read");
        assert!(allowed("u/alice/anything"));
        assert!(allowed("u/bob/other"));
    }

    #[test]
    fn predicate_tag_filter_only_allows_all() {
        let authed = authed_with_scopes(Some(vec!["if_jobs:filter_tags:default"]));
        let allowed = build_scope_path_predicate(&authed, "resources", "read");
        assert!(allowed("u/alice/foo"));
    }

    #[test]
    fn predicate_single_resource_scope_filters_others() {
        // Regression test for WIN-1981: a token scoped to one resource must
        // not match unrelated paths in listings (e.g. /resources/list_search).
        let authed = authed_with_scopes(Some(vec!["resources:read:u/alice/allowed_resource"]));
        let allowed = build_scope_path_predicate(&authed, "resources", "read");
        assert!(allowed("u/alice/allowed_resource"));
        assert!(!allowed("u/alice/other_resource"));
        assert!(!allowed("u/bob/foo"));
    }

    #[test]
    fn predicate_wildcard_scope_matches_subtree() {
        let authed = authed_with_scopes(Some(vec!["resources:read:f/team/*"]));
        let allowed = build_scope_path_predicate(&authed, "resources", "read");
        assert!(allowed("f/team/db"));
        assert!(allowed("f/team/sub/nested"));
        assert!(!allowed("f/other/db"));
    }

    #[test]
    fn predicate_wrong_domain_is_rejected() {
        let authed = authed_with_scopes(Some(vec!["variables:read:u/alice/secret"]));
        let allowed = build_scope_path_predicate(&authed, "resources", "read");
        assert!(!allowed("u/alice/secret"));
    }

    #[test]
    fn predicate_write_implies_read() {
        let authed = authed_with_scopes(Some(vec!["resources:write:u/alice/foo"]));
        let allowed = build_scope_path_predicate(&authed, "resources", "read");
        assert!(allowed("u/alice/foo"));
        assert!(!allowed("u/alice/bar"));
    }

    fn opt_scopes(scopes: Option<Vec<&str>>) -> Option<Vec<String>> {
        scopes.map(|v| v.into_iter().map(String::from).collect())
    }

    // Regression tests for WIN-1999: scoped user tokens must not be able to
    // mint or rescope credentials with broader privileges than themselves.

    #[test]
    fn unscoped_caller_can_grant_anything() {
        let authed = authed_with_scopes(None);
        assert!(ensure_scopes_within_caller(&authed, None).is_ok());
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["jobs:run:scripts"])).as_deref()
        )
        .is_ok());
    }

    #[test]
    fn filter_tags_only_caller_is_unrestricted_on_domain_action_dimension() {
        // The domain/action/resource subset check treats filter-tags-only as
        // unrestricted, mirroring check_scopes/check_route_access. The tag
        // dimension is checked separately (see filter_tags_dimension_is_monotonic).
        let authed = authed_with_scopes(Some(vec!["if_jobs:filter_tags:default"]));
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["users:write", "if_jobs:filter_tags:default"])).as_deref()
        )
        .is_ok());
    }

    #[test]
    fn filter_tags_dimension_is_monotonic() {
        // Caller restricted to tag fence "a" cannot drop the fence …
        let single = authed_with_scopes(Some(vec!["if_jobs:filter_tags:a"]));
        assert!(ensure_scopes_within_caller(&single, None).is_err());
        assert!(
            ensure_scopes_within_caller(&single, opt_scopes(Some(vec!["users:read"])).as_deref())
                .is_err(),
            "minting a token without filter_tags must be rejected"
        );
        // … cannot widen to a tag it lacks …
        assert!(ensure_scopes_within_caller(
            &single,
            opt_scopes(Some(vec!["if_jobs:filter_tags:a,b"])).as_deref()
        )
        .is_err());
        // … and cannot mint a token fenced on a disjoint tag.
        assert!(ensure_scopes_within_caller(
            &single,
            opt_scopes(Some(vec!["if_jobs:filter_tags:b"])).as_deref()
        )
        .is_err());
        // Narrowing or matching the tag fence is allowed.
        let multi = authed_with_scopes(Some(vec!["if_jobs:filter_tags:a,b"]));
        assert!(ensure_scopes_within_caller(
            &multi,
            opt_scopes(Some(vec!["if_jobs:filter_tags:a"])).as_deref()
        )
        .is_ok());
        assert!(ensure_scopes_within_caller(
            &multi,
            opt_scopes(Some(vec!["if_jobs:filter_tags:a,b"])).as_deref()
        )
        .is_ok());
        // A caller with a real scope plus a tag fence cannot drop just the fence.
        let mixed = authed_with_scopes(Some(vec!["jobs:run:scripts", "if_jobs:filter_tags:a"]));
        assert!(ensure_scopes_within_caller(
            &mixed,
            opt_scopes(Some(vec!["jobs:run:scripts"])).as_deref()
        )
        .is_err());
        assert!(ensure_scopes_within_caller(
            &mixed,
            opt_scopes(Some(vec!["jobs:run:scripts", "if_jobs:filter_tags:a"])).as_deref()
        )
        .is_ok());
        // An unrestricted caller may grant filter_tags freely.
        let unscoped = authed_with_scopes(None);
        assert!(ensure_scopes_within_caller(
            &unscoped,
            opt_scopes(Some(vec!["if_jobs:filter_tags:x"])).as_deref()
        )
        .is_ok());
    }

    #[test]
    fn scoped_caller_cannot_mint_unscoped_token() {
        // Primitive 2 in the report: a users:write token minting an unscoped token.
        let authed = authed_with_scopes(Some(vec!["users:write"]));
        assert!(ensure_scopes_within_caller(&authed, None).is_err());
        // Empty scope list is effectively unscoped and must also be rejected.
        assert!(ensure_scopes_within_caller(&authed, Some(&[])).is_err());
        // A scope list of only tag filters is effectively unscoped too.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["if_jobs:filter_tags:default"])).as_deref()
        )
        .is_err());
    }

    #[test]
    fn scoped_caller_cannot_remove_its_own_scopes() {
        // Primitive 3 in the report: a users:write token setting its scopes to null.
        let authed = authed_with_scopes(Some(vec!["users:write"]));
        assert!(ensure_scopes_within_caller(&authed, None).is_err());
    }

    #[test]
    fn scoped_caller_cannot_grant_scope_it_lacks() {
        let authed = authed_with_scopes(Some(vec!["users:write"]));
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["jobs:run:scripts"])).as_deref()
        )
        .is_err());
    }

    #[test]
    fn scoped_caller_can_grant_subset_of_own_scopes() {
        let authed = authed_with_scopes(Some(vec!["users:write", "jobs:run:scripts"]));
        // Equal scope is allowed.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["jobs:run:scripts"])).as_deref()
        )
        .is_ok());
        // write implies read, so a narrower read scope is allowed.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["users:read"])).as_deref()
        )
        .is_ok());
        // Tag filters narrow further and are always permitted.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["users:read", "if_jobs:filter_tags:default"])).as_deref()
        )
        .is_ok());
    }

    #[test]
    fn scoped_caller_cannot_broaden_resource_scope() {
        let authed = authed_with_scopes(Some(vec!["scripts:read:f/team/*"]));
        // Narrower resource within the subtree is allowed.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["scripts:read:f/team/sub"])).as_deref()
        )
        .is_ok());
        // A nested subtree within the caller's subtree is allowed.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["scripts:read:f/team/sub/*"])).as_deref()
        )
        .is_ok());
        // The subtree root itself is allowed.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["scripts:read:f/team"])).as_deref()
        )
        .is_ok());
        // A path outside the subtree is rejected.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["scripts:read:f/other/x"])).as_deref()
        )
        .is_err());
        // read caller cannot grant write.
        assert!(ensure_scopes_within_caller(
            &authed,
            opt_scopes(Some(vec!["scripts:write:f/team/db"])).as_deref()
        )
        .is_err());
    }

    #[test]
    fn mcp_scopes_require_byte_identical_match() {
        // Regression for the access-grant-OR vs runtime-MCP-parser confusion:
        // ScopeDefinition treats `mcp:scripts` as an unrestricted-resource scope
        // and `mcp:scripts:*` as a strictly narrower one, so generic containment
        // would silently allow widening. The MCP runtime however ignores
        // `mcp:scripts` (unrecognized) while `mcp:scripts:*` grants all scripts.
        // Legitimate MCP token issuance is the OAuth gateway, not these
        // user-token endpoints, so MCP scopes must match the caller verbatim.

        // The bypass the reviewer flagged: malformed `mcp:scripts` would widen
        // into the real `mcp:scripts:*` under generic containment.
        let bypass = authed_with_scopes(Some(vec!["users:write", "mcp:scripts"]));
        assert!(ensure_scopes_within_caller(
            &bypass,
            opt_scopes(Some(vec!["users:write", "mcp:scripts:*"])).as_deref()
        )
        .is_err());

        // A caller without any MCP scope cannot grant one (widening on the MCP
        // dimension), even if the rest of the requested scopes are within reach.
        let no_mcp = authed_with_scopes(Some(vec!["users:write"]));
        assert!(ensure_scopes_within_caller(
            &no_mcp,
            opt_scopes(Some(vec!["users:write", "mcp:scripts:*"])).as_deref()
        )
        .is_err());

        // Byte-identical MCP scope passes; an additional non-matching MCP scope
        // alongside it does not.
        let mcp_caller = authed_with_scopes(Some(vec!["mcp:scripts:*"]));
        assert!(ensure_scopes_within_caller(
            &mcp_caller,
            opt_scopes(Some(vec!["mcp:scripts:*"])).as_deref()
        )
        .is_ok());
        assert!(ensure_scopes_within_caller(
            &mcp_caller,
            opt_scopes(Some(vec!["mcp:scripts:*", "mcp:flows:*"])).as_deref()
        )
        .is_err());

        // Even a narrowing within MCP semantics (`mcp:all` → `mcp:scripts:*`)
        // is rejected by the byte-identical rule. This is intentional — these
        // endpoints are not the legitimate path for narrowing MCP tokens.
        let mcp_all = authed_with_scopes(Some(vec!["mcp:all"]));
        assert!(ensure_scopes_within_caller(
            &mcp_all,
            opt_scopes(Some(vec!["mcp:scripts:*"])).as_deref()
        )
        .is_err());
    }

    #[test]
    fn scoped_caller_cannot_escalate_to_wildcard_or_superset() {
        // Regression for the access-grant-OR vs subset-containment confusion:
        // ScopeDefinition::includes would (incorrectly) allow all of these.
        let star = authed_with_scopes(Some(vec!["scripts:read:f/team/a"]));
        // Minting `*` from a single-path scope must be rejected.
        assert!(ensure_scopes_within_caller(
            &star,
            opt_scopes(Some(vec!["scripts:read:*"])).as_deref()
        )
        .is_err());
        // Minting a broader subtree must be rejected.
        assert!(ensure_scopes_within_caller(
            &star,
            opt_scopes(Some(vec!["scripts:read:f/team/*"])).as_deref()
        )
        .is_err());

        // A comma-separated list that adds an uncovered resource must be rejected,
        // even though one element overlaps the caller's scope.
        let list = authed_with_scopes(Some(vec!["scripts:read:f/team/a"]));
        assert!(ensure_scopes_within_caller(
            &list,
            opt_scopes(Some(vec!["scripts:read:f/team/a,f/other/b"])).as_deref()
        )
        .is_err());

        // A subset of a multi-resource caller scope is allowed.
        let multi = authed_with_scopes(Some(vec!["scripts:read:f/team/a,f/team/b"]));
        assert!(ensure_scopes_within_caller(
            &multi,
            opt_scopes(Some(vec!["scripts:read:f/team/a"])).as_deref()
        )
        .is_ok());

        // A wildcard caller covers any subset, but not `*`-less escalation rules apply
        // only when the caller itself lacks `*`.
        let wildcard = authed_with_scopes(Some(vec!["scripts:read:*"]));
        assert!(ensure_scopes_within_caller(
            &wildcard,
            opt_scopes(Some(vec!["scripts:read:f/team/a"])).as_deref()
        )
        .is_ok());
    }
}
