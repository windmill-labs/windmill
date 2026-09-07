use std::{
    hash::DefaultHasher,
    sync::atomic::{AtomicI64, Ordering},
};

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{Authed, AuthedRef},
    error::{Error, Result},
    jwt,
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL},
    utils::WarnAfterExt,
    DB,
};

/// Whether `label` denotes a user-created token rather than a system token
/// (`session`, `guest_session`, `ephemeral*`, `debugger-token`, `mcp-oauth-*`). System-token
/// labels are load-bearing — session cleanup, super_admin propagation, expiry
/// notifications and username overrides all key off them — so they must not be
/// user-editable. `None` (no label) is treated as a user token.
///
/// This is the canonical copy. When updating it, also update its mirrors:
/// - the `update_token_label` editability guard (SQL `WHERE`) in
///   windmill-api-users/src/users.rs
/// - `isUserToken` in frontend/src/lib/components/settings/TokensTable.svelte
pub fn is_user_token(label: Option<&str>) -> bool {
    match label {
        None => true,
        Some(l) => {
            // `ephemeral` is matched case-insensitively to agree exactly with the
            // frontend mirror (`label.toLowerCase().startsWith('ephemeral')`) and
            // the SQL `lower(label) NOT LIKE 'ephemeral%'` guard.
            l != "session"
                && l != GUEST_SESSION_LABEL
                && !l.to_lowercase().starts_with("ephemeral")
                && l != "debugger-token"
                && !l.starts_with("mcp-oauth-")
        }
    }
}

/// Whether `label` belongs to a namespace only the server mints, and which therefore must be
/// rejected by `create_token`. Narrower than [`is_user_token`], which also drives label
/// editability and expiry notifications and can afford to reserve more: `Ephemeral lsp token`
/// and `debugger-token` are minted by the editor and the debugger through that same handler,
/// so reserving them would break those features.
///
/// `username_override_from_label` trusts a label to name the entity acting only if it is in
/// here, so anything added must be unmintable by a member.
pub fn is_server_minted_label(label: &str) -> bool {
    label.starts_with("ephemeral-webhook-")
        || label.starts_with("ephemeral-script-end-user-")
        || label == "ephemeral-script"
        || label == "session"
        || label == GUEST_SESSION_LABEL
        || label.starts_with("mcp-oauth-")
}

/// Label on a guest session (the `guest` app execution mode). This is the *grant*:
/// `AuthCache` will resolve a token carrying it into an identity with no account behind
/// it, which nothing else can do. It must therefore stay unforgeable, which is what
/// listing it in [`is_server_minted_label`] buys — `/users/tokens/create` refuses it.
///
/// Do not move this test onto the token's scopes. Scopes on a user-minted token are
/// caller-supplied and only ever *narrow* (`app_embed`, `raw_app_sdk`), so a scope
/// that granted non-member access would be free for anyone to declare.
pub const GUEST_SESSION_LABEL: &str = "guest_session";

/// Whether `label` marks a guest session. See [`GUEST_SESSION_LABEL`].
///
/// Reserved in [`is_user_token`] as well as [`is_server_minted_label`]: the former
/// gates relabelling, and a user token that could be relabelled *into* this
/// namespace would become a guest session with no workspace pin — one that
/// authenticates everywhere.
pub fn is_guest_session_label(label: Option<&str>) -> bool {
    label == Some(GUEST_SESSION_LABEL)
}

/// Whether `path` can be spliced into a scope as one literal resource. The scope
/// grammar reserves three characters: `:` separates the parts, `,` separates
/// resources, `*` is a wildcard. App paths are otherwise free-form (spaces, `@`). A
/// leading `/` is refused too: routes strip it, so the scope would never match.
pub fn is_scope_literal_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.chars().any(|c| matches!(c, ':' | ',' | '*'))
}

/// Whether `label` is the one minted for a browser session at login. [`is_server_minted_label`]
/// stops a member minting it directly, but `/users/refresh_token` hands one to any authenticated
/// caller, so this attributes a request to the UI without proving it: never gate authority on it.
pub fn is_session_label(label: Option<&str>) -> bool {
    label == Some("session")
}

/// Hash a raw token using SHA-256 (hex-encoded, 64 chars).
/// Used to store and look up tokens without keeping plaintext in the DB.
pub fn hash_token(token: &str) -> String {
    crate::utils::calculate_hash(token)
}

#[derive(Debug)]
pub struct IdToken {
    token: String,
    expiration: DateTime<Utc>,
}

pub const TOKEN_PREFIX_LEN: usize = 10;

/// Safely extract the token prefix (first TOKEN_PREFIX_LEN chars).
/// Returns the full token if it's shorter than TOKEN_PREFIX_LEN, preventing panics.
pub fn safe_token_prefix(token: &str) -> String {
    token.get(..TOKEN_PREFIX_LEN).unwrap_or(token).to_string()
}

lazy_static::lazy_static! {
    // Cache for script hash permissions - (ApiAuthed hash, script_hash) -> permission result
    pub static ref HASH_PERMS_CACHE: PermsCache = PermsCache::new();
    pub static ref FLOW_PERMS_CACHE: PermsCache = PermsCache::new();
}

pub struct PermsCache(Cache<(u64, u64), ()>, AtomicI64);

use std::hash::Hash;
use std::hash::Hasher;

impl PermsCache {
    pub fn compute_hash(authed: &AuthedRef) -> u64 {
        let mut hasher = DefaultHasher::new();
        authed.username.hash(&mut hasher);
        authed.folders.hash(&mut hasher);
        authed.groups.hash(&mut hasher);
        authed.is_admin.hash(&mut hasher);
        hasher.finish()
    }
}

pub const PERMS_CACHE_EXPIRATION_SECONDS: i64 = 60 * 60;

impl PermsCache {
    pub fn new() -> Self {
        PermsCache(
            Cache::new(10000),
            AtomicI64::new(chrono::Utc::now().timestamp() as i64),
        )
    }

    pub fn check_perms_in_cache<'e, T: Into<u64>>(
        &self,
        authed: &'e AuthedRef<'e>,
        key: T,
    ) -> (bool, u64) {
        // Clear cache every hour
        if self.1.load(Ordering::Relaxed)
            < chrono::Utc::now().timestamp() - PERMS_CACHE_EXPIRATION_SECONDS
        {
            self.0.clear();
            self.1
                .store(chrono::Utc::now().timestamp() as i64, Ordering::Relaxed);
        }
        // Create hash of the ApiAuthed struct for caching
        let authed_hash = Self::compute_hash(authed);

        let key = key.into();
        tracing::debug!(
            "Checking cache for authed hash {authed_hash} and script hash {}",
            key
        );
        // Check cache first
        if let Some(_) = self.0.get(&(authed_hash, key)) {
            tracing::debug!("Cached result for authed hash {authed_hash}",);
            return (true, authed_hash);
        }

        return (false, authed_hash);
    }

    pub fn insert<'e, T: Into<u64>>(&self, authed_hash: u64, key: T) {
        let key = key.into();
        tracing::debug!("Inserting authed hash {authed_hash} and key {}", key);
        self.0.insert((authed_hash, key), ());
    }
}

/// Check a user's access level against an `extra_perms` JSONB object.
///
/// Returns `None` if the user has no matching entry (no access).
/// Returns `Some(true)` if the user (or any of their groups) has write access.
/// Returns `Some(false)` if the user (or any of their groups) has read-only access.
pub fn check_extra_perms(
    extra_perms: &serde_json::Map<String, serde_json::Value>,
    username: &str,
    groups: &[String],
) -> Option<bool> {
    // Check direct user permission
    use crate::users::{PERMISSIONED_AS_GROUP_PREFIX, PERMISSIONED_AS_USER_PREFIX};
    let user_key = if username.starts_with(PERMISSIONED_AS_USER_PREFIX) {
        username.to_string()
    } else {
        format!("{PERMISSIONED_AS_USER_PREFIX}{username}")
    };
    if let Some(v) = extra_perms.get(&user_key) {
        return Some(v.as_bool().unwrap_or(false));
    }

    // Check group permissions — return highest access level found
    let mut found = false;
    let mut write = false;
    for g in groups {
        let key = if g.starts_with(PERMISSIONED_AS_GROUP_PREFIX) {
            g.to_string()
        } else {
            format!("{PERMISSIONED_AS_GROUP_PREFIX}{g}")
        };
        if let Some(v) = extra_perms.get(&key) {
            found = true;
            if v.as_bool().unwrap_or(false) {
                write = true;
                break;
            }
        }
    }

    if found {
        Some(write)
    } else {
        None
    }
}

pub fn has_expired(expiration_time: DateTime<Utc>, take: Option<Duration>) -> bool {
    let now = Utc::now();

    let expiration = match take {
        Some(duration) => expiration_time - duration,
        None => expiration_time,
    };

    now > expiration
}

impl From<IdToken> for String {
    fn from(value: IdToken) -> Self {
        value.token
    }
}

impl ToString for IdToken {
    fn to_string(&self) -> String {
        self.token.clone()
    }
}

impl IdToken {
    pub fn new(token: String, expiration: DateTime<Utc>) -> Self {
        Self { token, expiration }
    }

    pub fn token(&self) -> &str {
        &self.token
    }
    pub fn expiration(&self) -> &DateTime<Utc> {
        &self.expiration
    }
}

#[derive(Deserialize, Serialize)]
pub struct JWTAuthClaims {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<(String, bool, bool)>,
    pub label: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_ids: Option<Vec<String>>,
    pub exp: usize,
    pub job_id: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub audit_span: Option<String>,
}

impl JWTAuthClaims {
    pub fn allowed_in_workspace(&self, w_id: &str) -> bool {
        self.workspace_id
            .as_ref()
            .is_some_and(|token_w_id| w_id == token_w_id)
            || self
                .workspace_ids
                .as_ref()
                .is_some_and(|token_w_ids| token_w_ids.iter().any(|token_w_id| w_id == token_w_id))
    }

    pub fn compute_ext_jwt_hash(&self) -> i64 {
        let mut hasher = DefaultHasher::new();
        self.email.hash(&mut hasher);
        self.username.hash(&mut hasher);
        self.is_admin.hash(&mut hasher);
        self.is_operator.hash(&mut hasher);
        self.groups.hash(&mut hasher);
        self.folders.hash(&mut hasher);
        self.workspace_id.hash(&mut hasher);
        self.workspace_ids.hash(&mut hasher);
        self.label.hash(&mut hasher);
        self.scopes.hash(&mut hasher);
        hasher.finish() as i64
    }
}

#[derive(Deserialize, Debug)]
pub struct JobPerms {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<serde_json::Value>,
    pub end_user_email: Option<String>,
}

impl From<JobPerms> for Authed {
    fn from(value: JobPerms) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value
                .folders
                .into_iter()
                .filter_map(|x| serde_json::from_value::<(String, bool, bool)>(x).ok())
                .collect(),
            scopes: None,
            token_prefix: None,
        }
    }
}

pub async fn is_super_admin_email<'c>(db: impl sqlx::PgExecutor<'c>, email: &str) -> Result<bool> {
    if email == SUPERADMIN_SECRET_EMAIL || email == SUPERADMIN_NOTIFICATION_EMAIL {
        return Ok(true);
    }

    let is_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_admin)
}

/// The three reserved internal identities that grant instance-superadmin at
/// execution: `superadmin_secret@` / `superadmin_notification@` (matched on the
/// email) and `superadmin_sync@` (matched on `permissioned_as`). They belong to
/// no real user, so a stored `on_behalf_of` (app policy, flow/script,
/// schedule, trigger) must never carry one as either field — it would be a
/// forged superadmin run identity. Mirror of the `is_super_admin` derivation in
/// [`fetch_authed_from_permissioned_as_inner`].
pub fn is_reserved_on_behalf_of_identity(
    permissioned_as: Option<&str>,
    on_behalf_of_email: Option<&str>,
) -> bool {
    const RESERVED: [&str; 3] = [
        SUPERADMIN_SECRET_EMAIL,
        SUPERADMIN_NOTIFICATION_EMAIL,
        SUPERADMIN_SYNC_EMAIL,
    ];
    [permissioned_as, on_behalf_of_email]
        .into_iter()
        .flatten()
        .any(|v| RESERVED.contains(&v))
}

/// Guard a caller-supplied `on_behalf_of` before it is persisted on a deployable
/// object (app policy, flow/script, schedule, trigger): reject the reserved
/// internal sentinels, which no legitimate deploy ever carries. The actual
/// escalation is closed at execution by the job-token cap in
/// [`require_super_admin`] — even a superadmin `on_behalf_of` yields a token
/// capped at workspace admin — so this is a cheap, non-breaking early guard, not
/// the primary defense. It deliberately does *not* restrict deploying on behalf
/// of a real user (including a real superadmin, e.g. git-sync of
/// superadmin-authored content), which is the intended `wm_deployers` capability.
pub fn validate_on_behalf_of(
    permissioned_as: Option<&str>,
    on_behalf_of_email: Option<&str>,
) -> Result<()> {
    if is_reserved_on_behalf_of_identity(permissioned_as, on_behalf_of_email) {
        return Err(Error::BadRequest(
            "on_behalf_of cannot be a reserved internal identity".to_string(),
        ));
    }
    Ok(())
}

pub async fn is_devops_email(db: &DB, email: &str) -> Result<bool> {
    if is_super_admin_email(db, email).await? {
        return Ok(true);
    }

    let is_devops = sqlx::query_scalar!("SELECT devops FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_devops)
}

pub fn permissioned_as_to_username(permissioned_as: &str) -> String {
    use crate::users::{PERMISSIONED_AS_USER_PREFIX, USERNAME_GROUP_PREFIX};
    if let Some(name) = permissioned_as.strip_prefix(PERMISSIONED_AS_USER_PREFIX) {
        name.to_string()
    } else if let Some(name) =
        permissioned_as.strip_prefix(crate::users::PERMISSIONED_AS_GROUP_PREFIX)
    {
        format!("{}{}", USERNAME_GROUP_PREFIX, name)
    } else {
        permissioned_as.to_string()
    }
}

pub fn fetch_authed_from_permissioned_as<'a, A>(
    permissioned_as: &'a str,
    email: &'a str,
    w_id: &'a str,
    db: A,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Authed>> + Send + 'a>>
where
    A: sqlx::Acquire<'a, Database = sqlx::Postgres> + Send + 'a,
{
    Box::pin(async move {
        let mut conn = db
            .acquire()
            .await
            .map_err(|e| Error::internal_err(format!("acquiring connection: {e:#}")))?;

        fetch_authed_from_permissioned_as_inner(permissioned_as, email, w_id, &mut *conn).await
    })
}

async fn fetch_authed_from_permissioned_as_inner(
    permissioned_as: &str,
    email: &str,
    w_id: &str,
    conn: &mut sqlx::PgConnection,
) -> Result<Authed> {
    let is_super_admin = permissioned_as == SUPERADMIN_SYNC_EMAIL
        || email == SUPERADMIN_SECRET_EMAIL
        || email == SUPERADMIN_NOTIFICATION_EMAIL
        || sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
            .unwrap_or(false);

    if let Some((prefix, name)) = permissioned_as.split_once('/') {
        if prefix == "u" {
            let (is_admin, is_operator) = if is_super_admin {
                (true, false)
            } else {
                let r = sqlx::query!(
                    "SELECT is_admin, operator FROM usr where username = $1 AND \
                                                 workspace_id = $2 AND disabled = false",
                    name,
                    &w_id
                )
                .fetch_optional(&mut *conn)
                .await?;
                if let Some(r) = r {
                    (r.is_admin, r.operator)
                } else {
                    return Err(Error::NotFound(format!(
                        "user {name} not found in workspace {w_id}"
                    )));
                }
            };

            let groups = get_groups_for_user(w_id, &name, email, &mut *conn).await?;

            let folders = get_folders_for_user(w_id, &name, &groups, &mut *conn).await?;

            Ok(Authed {
                email: email.to_string(),
                username: name.to_string(),
                is_admin,
                is_operator,
                groups,
                folders,
                scopes: None,
                token_prefix: None,
            })
        } else {
            let groups = vec![name.to_string()];
            let folders = get_folders_for_user(&w_id, "", &groups, &mut *conn).await?;
            Ok(Authed {
                email: email.to_string(),
                username: format!("{}{name}", crate::users::USERNAME_GROUP_PREFIX),
                is_admin: false,
                groups,
                is_operator: false,
                folders,
                scopes: None,
                token_prefix: None,
            })
        }
    } else {
        // Bare (no `u/`|`g/` prefix) permissioned_as is reached for superadmins
        // whose identifier is their email (they are not a workspace member). Use
        // the instance-derived username when available so no email leaks
        // downstream as the acting username.
        let username = if is_super_admin && permissioned_as == email {
            crate::usernames::get_instance_username_or_fallback_to_email(&mut *conn, email).await?
        } else {
            permissioned_as.to_string()
        };
        Ok(Authed {
            email: email.to_string(),
            username,
            is_admin: is_super_admin,
            is_operator: true,
            groups: vec![],
            folders: vec![],
            scopes: None,
            token_prefix: None,
        })
    }
}

pub async fn get_folders_for_user<'e, E: sqlx::PgExecutor<'e>>(
    w_id: &str,
    username: &str,
    groups: &[String],
    db: E,
) -> Result<Vec<(String, bool, bool)>> {
    let mut perms = groups
        .into_iter()
        .map(|x| format!("g/{}", x))
        .collect::<Vec<_>>();
    perms.insert(0, format!("u/{}", username));
    let folders = sqlx::query!(
        "SELECT name, (EXISTS (SELECT 1 FROM (SELECT key, value FROM jsonb_each_text(extra_perms) WHERE key = ANY($1)) t  WHERE value::boolean IS true)) as write, $1 && owners::text[] as owner  FROM folder
        WHERE extra_perms ?| $1  AND workspace_id = $2",
        &perms[..],
        w_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|x| (x.name, x.write.unwrap_or(false), x.owner.unwrap_or(false)))
    .collect();

    Ok(folders)
}

pub async fn get_groups_for_user<'e, E: sqlx::PgExecutor<'e>>(
    w_id: &str,
    username: &str,
    email: &str,
    db: E,
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

pub async fn get_job_perms<'a, E: sqlx::PgExecutor<'a>>(
    db: E,
    job_id: &Uuid,
    w_id: &str,
) -> sqlx::Result<Option<JobPerms>> {
    sqlx::query_as!(
        JobPerms,
        "SELECT email, username, is_admin, is_operator, groups, folders, end_user_email FROM job_perms WHERE job_id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .warn_after_seconds(3)
    .await
}

/// A job token is refreshed once its remaining lifetime drops below this. It must exceed the
/// 60s `jsonwebtoken` exp leeway, otherwise a token that still validates now could expire
/// mid-orchestration after being judged fresh.
pub const JOB_TOKEN_REFRESH_MARGIN_SECS: i64 = 120;

/// Seconds until an internal job JWT expires, or `None` when `token` is not a decodable job
/// JWT (e.g. the empty test token). The signature is intentionally not verified: the value only
/// gates whether to refresh the token, never whose identity to assume.
pub fn job_token_remaining_lifetime_secs(token: &str) -> Option<i64> {
    let raw = token.strip_prefix("jwt_")?;
    let claims: JWTAuthClaims = jwt::decode_without_verify(raw).ok()?;
    Some(claims.exp as i64 - Utc::now().timestamp())
}

/// Label for an ephemeral job token. For a job run on behalf of an end user (its
/// `permissioned_as` differs from its `created_by`) it encodes that end user so
/// `username_override_from_label` can recover them; otherwise it is the plain script label.
pub fn ephemeral_script_token_label(permissioned_as: &str, created_by: &str) -> String {
    if permissioned_as != format!("u/{created_by}") && permissioned_as != created_by {
        format!("ephemeral-script-end-user-{created_by}")
    } else {
        "ephemeral-script".to_string()
    }
}

/// Lifetime to mint an ephemeral job token with, resolved per workspace: a token narrower than
/// the run it serves 401s the job mid-flight, and one wider is a credential nothing revokes.
pub async fn job_token_expiry_secs(_db: &DB, _w_id: &str) -> u64 {
    if let Some(override_secs) = *crate::worker::SCRIPT_TOKEN_EXPIRY_OVERRIDE {
        return override_secs;
    }
    #[cfg(feature = "cloud")]
    let premium = *crate::worker::CLOUD_HOSTED
        && crate::workspaces::get_team_plan_status(_db, _w_id)
            .await
            .inspect_err(|err| {
                tracing::error!(
                    "Failed to get team plan status to size the job token for {_w_id}: {err:#}"
                )
            })
            .map(|s| s.premium)
            // Matches resolve_job_timeout: on a lookup failure assume the wider ceiling, so the
            // token cannot come out shorter than the timeout the job is actually held to.
            .unwrap_or(true);
    #[cfg(not(feature = "cloud"))]
    let premium = false;

    job_token_expiry_from_premium(premium)
}

/// Cap on the setup slack below. Where the ceiling is already days, slack buys nothing and only
/// widens the credential; an hour is what a dependency install realistically needs.
const JOB_TOKEN_SETUP_SLACK_CAP_SECS: u64 = 3600;

fn job_token_expiry_from_premium(cloud_premium_workspace: bool) -> u64 {
    // The token's clock starts at pull, but resolve_job_timeout runs per handle_child, so setup
    // (lock, install, bundle) spends a budget of its own before the run phase gets one. Slack
    // covers the realistic case; pull-to-finish is not bounded by the ceiling at all.
    let max_run = crate::worker::max_job_duration_secs(cloud_premium_workspace);
    max_run
        .saturating_add(max_run.min(JOB_TOKEN_SETUP_SLACK_CAP_SECS))
        // create_jwt_token casts this to i64; a saturated value there mints an expired token.
        .min(u32::MAX as u64)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner(
    db: &DB,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: u64,
    email: &str,
    job_id: &Uuid,
    perms: Option<JobPerms>,
    audit_span: Option<String>,
) -> crate::error::Result<String> {
    let job_perms = if perms.is_some() {
        Ok(perms)
    } else {
        get_job_perms(db, job_id, w_id).await
    };
    let job_authed = match job_perms {
        Ok(Some(jp)) => jp.into(),
        _ => {
            tracing::warn!("Could not get permissions for job {job_id} from job_perms table, getting permissions directly...");
            fetch_authed_from_permissioned_as(owner, email, w_id, db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Could not get permissions directly for job {job_id}: {e:#}"
                    ))
                })?
        }
    };

    create_jwt_token(
        job_authed,
        w_id,
        expires_in,
        Some(*job_id),
        Some(label.to_string()),
        audit_span,
        None,
    )
    .await
}

pub async fn create_jwt_token(
    authed: Authed,
    workspace_id: &str,
    expires_in_seconds: u64,
    job_id: Option<Uuid>,
    label: Option<String>,
    audit_span: Option<String>,
    scopes: Option<Vec<String>>,
) -> crate::error::Result<String> {
    let payload = JWTAuthClaims {
        email: authed.email.clone(),
        username: authed.username.clone(),
        is_admin: authed.is_admin,
        is_operator: authed.is_operator,
        groups: authed.groups.clone(),
        folders: authed.folders.clone(),
        label,
        workspace_id: Some(workspace_id.to_string()),
        workspace_ids: None,
        exp: (chrono::Utc::now() + chrono::Duration::seconds(expires_in_seconds as i64)).timestamp()
            as usize,
        job_id: job_id.map(|id| id.to_string()),
        scopes,
        audit_span,
    };

    let token = jwt::encode_with_internal_secret(&payload)
        .await
        .with_context(|| match job_id {
            Some(job_id) => format!("Could not encode JWT token for job {job_id}"),
            None => "Could not encode JWT token".to_string(),
        })?;

    Ok(format!("jwt_{}", token))
}

#[cfg(feature = "aws_auth")]
pub mod aws {

    use super::*;
    use crate::utils::empty_as_none;
    use aws_config::{BehaviorVersion, Region};
    use aws_sdk_sts::{
        config::Credentials as AwsCredentials,
        operation::{
            assume_role_with_saml::AssumeRoleWithSamlOutput,
            assume_role_with_web_identity::{
                builders::AssumeRoleWithWebIdentityFluentBuilder, AssumeRoleWithWebIdentityOutput,
            },
        },
        types::Credentials,
        Client,
    };

    pub const AWS_OIDC_AUDIENCE: &'static str = "sts.amazonaws.com";

    pub trait GetAuthenticationOutput {
        fn get_credentials(&self) -> Result<&Credentials>;
    }

    impl GetAuthenticationOutput for AssumeRoleWithSamlOutput {
        fn get_credentials(&self) -> Result<&Credentials> {
            let credentials = self.credentials.as_ref().ok_or(Error::BadGateway(
                "Error fetching credentials from AWS STS".to_string(),
            ))?;
            Ok(credentials)
        }
    }

    impl GetAuthenticationOutput for AssumeRoleWithWebIdentityOutput {
        fn get_credentials(&self) -> Result<&Credentials> {
            let credentials = self.credentials.as_ref().ok_or(Error::BadGateway(
                "Error fetching credentials from AWS STS".to_string(),
            ))?;
            Ok(credentials)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
    #[sqlx(type_name = "AWS_AUTH_RESOURCE_TYPE", rename_all = "lowercase")]
    #[serde(rename_all = "lowercase")]
    pub enum AwsAuthResourceType {
        Credentials,
        Oidc,
    }

    #[derive(Debug, Deserialize)]
    pub struct CredentialsAuth {
        #[serde(deserialize_with = "empty_as_none")]
        pub region: Option<String>,
        #[serde(rename = "awsAccessKeyId")]
        pub aws_access_key_id: String,
        #[serde(rename = "awsSecretAccessKey")]
        pub aws_secret_access_key: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct OidcAuth {
        #[serde(deserialize_with = "empty_as_none")]
        pub region: Option<String>,
        #[serde(rename = "roleArn")]
        pub role_arn: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    pub enum AWSAuthConfig {
        Credentials(CredentialsAuth),
        Oidc(OidcAuth),
    }

    pub async fn get_assume_role_with_web_identity_fluent_builder(
        oidc_auth: &OidcAuth,
        token: String,
        role_session_name: Option<impl ToString>,
    ) -> Result<AssumeRoleWithWebIdentityFluentBuilder> {
        let region = oidc_auth.region.as_deref().unwrap_or_else(|| "us-east-1");

        let credentials = AwsCredentials::new("", "", None, None, "UserInput");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(region.to_string()))
            .load()
            .await;

        let assume_role_with_web_identity_fluent_builder = Client::new(&config)
            .assume_role_with_web_identity()
            .set_role_arn(Some(oidc_auth.role_arn.to_owned()))
            .set_role_session_name(role_session_name.map(|str| str.to_string()))
            .set_web_identity_token(Some(token));

        Ok(assume_role_with_web_identity_fluent_builder)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_reserved_on_behalf_of_identity, is_user_token};
    use super::{job_token_expiry_from_premium, job_token_remaining_lifetime_secs};
    use super::{JWTAuthClaims, JOB_TOKEN_REFRESH_MARGIN_SECS};
    use crate::users::{
        SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL,
    };
    use crate::worker::max_job_duration_secs;

    fn job_jwt(exp_offset_secs: i64) -> String {
        let claims = JWTAuthClaims {
            email: String::new(),
            username: String::new(),
            is_admin: false,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            label: None,
            workspace_id: None,
            workspace_ids: None,
            exp: (chrono::Utc::now().timestamp() + exp_offset_secs) as usize,
            job_id: None,
            scopes: None,
            audit_span: None,
        };
        // Signature is irrelevant — the gate decodes without verifying — so any key works.
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(b"test"),
        )
        .unwrap();
        format!("jwt_{token}")
    }

    #[test]
    fn remaining_lifetime_reflects_exp_and_flags_near_expiry() {
        // A token minted for less than the margin reads as needing a refresh...
        let short = job_token_remaining_lifetime_secs(&job_jwt(30)).unwrap();
        assert!(short < JOB_TOKEN_REFRESH_MARGIN_SECS);
        // ...a long-lived one does not...
        let long = job_token_remaining_lifetime_secs(&job_jwt(10_000)).unwrap();
        assert!(long >= JOB_TOKEN_REFRESH_MARGIN_SECS);
        // ...and a non-JWT token (e.g. the empty test-run token) yields no lifetime.
        assert!(job_token_remaining_lifetime_secs("not-a-jwt").is_none());
        assert!(job_token_remaining_lifetime_secs("").is_none());
    }

    #[test]
    fn reserved_on_behalf_of_identity_matches_every_sentinel_in_either_field() {
        // Matched on the email (secret / notification) or on permissioned_as (sync).
        assert!(is_reserved_on_behalf_of_identity(
            None,
            Some(SUPERADMIN_SECRET_EMAIL)
        ));
        assert!(is_reserved_on_behalf_of_identity(
            None,
            Some(SUPERADMIN_NOTIFICATION_EMAIL)
        ));
        assert!(is_reserved_on_behalf_of_identity(
            Some(SUPERADMIN_SYNC_EMAIL),
            None
        ));
        // A sentinel smuggled as a raw-email permissioned_as (schedules/triggers
        // derive the email from it) is caught too.
        assert!(is_reserved_on_behalf_of_identity(
            Some(SUPERADMIN_SECRET_EMAIL),
            None
        ));
        // Ordinary identities pass.
        assert!(!is_reserved_on_behalf_of_identity(None, None));
        assert!(!is_reserved_on_behalf_of_identity(
            Some("u/alice"),
            Some("alice@example.com")
        ));
        assert!(!is_reserved_on_behalf_of_identity(Some("g/team"), None));
    }

    #[test]
    fn user_tokens_are_editable() {
        assert!(is_user_token(None)); // no label
        assert!(is_user_token(Some("")));
        assert!(is_user_token(Some("my-ci-token")));
        assert!(is_user_token(Some("webhook-foo"))); // username-override prefix, not a system kind here
    }

    #[test]
    fn system_tokens_are_not_editable() {
        assert!(!is_user_token(Some("session")));
        assert!(!is_user_token(Some("ephemeral-script")));
        assert!(!is_user_token(Some("ephemeral-webhook-x")));
        assert!(!is_user_token(Some("Ephemeral lsp token")));
        assert!(!is_user_token(Some("debugger-token")));
        assert!(!is_user_token(Some("mcp-oauth-client")));
    }

    #[test]
    fn ephemeral_match_is_case_insensitive() {
        // Must agree with the frontend mirror (`toLowerCase().startsWith('ephemeral')`)
        // so a token can't be relabeled to a casing the backend allows but the UI hides.
        assert!(!is_user_token(Some("Ephemeral-test")));
        assert!(!is_user_token(Some("ePhemeral-test")));
        assert!(!is_user_token(Some("EPHEMERAL-test")));
    }

    #[test]
    fn job_token_outlives_the_longest_job_it_may_serve() {
        for premium in [false, true] {
            assert!(
                job_token_expiry_from_premium(premium) > max_job_duration_secs(premium),
                "token expiry must exceed the job ceiling (premium: {premium})"
            );
        }
    }
}
