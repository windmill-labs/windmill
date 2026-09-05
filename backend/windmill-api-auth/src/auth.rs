#[cfg(feature = "enterprise")]
use crate::ee_oss::ExternalJwks;
use axum::{
    extract::{FromRequestParts, OriginalUri, Query},
    Extension, Json,
};
use chrono::TimeZone;
use http::{request::Parts, StatusCode};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tower_cookies::Cookies;
use tracing::Span;

use crate::{ApiAuthed, OptJobAuthed};
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Arc,
    },
};
#[cfg(feature = "enterprise")]
use tokio::sync::RwLock;
use windmill_common::DB;

use windmill_common::{
    auth::{
        get_folders_for_user, get_groups_for_user, hash_token, is_session_label, safe_token_prefix,
        JWTAuthClaims,
    },
    error::{Error, JsonResult},
    jwt,
    usernames::get_instance_username_or_fallback_to_email,
    users::{COOKIE_NAME, SUPERADMIN_SECRET_EMAIL},
};

lazy_static::lazy_static! {
    // Global auth cache accessible from main.rs for direct invalidation
    pub static ref AUTH_CACHE: Cache<(String, String), ExpiringAuthCache> = Cache::new(300);
    // Cache for token -> email lookups (for non-workspace-member authenticated users)
    static ref TOKEN_EMAIL_CACHE: Cache<String, (Option<String>, std::time::Instant)> = Cache::new(500);
}

/// A token keeps its identity when a superadmin moves the account to another address, so entries
/// here must expire on their own; nothing invalidates them by token hash.
const TOKEN_EMAIL_CACHE_TTL_SECS: u64 = 60;

/// Get email from a valid token, with caching.
/// Used for WM_END_USER_EMAIL when user is authenticated but not a workspace member.
async fn get_email_from_token(db: &DB, token: &str) -> Option<String> {
    let t_hash = hash_token(token);
    if let Some((cached, cached_at)) = TOKEN_EMAIL_CACHE.get(&t_hash) {
        if cached_at.elapsed().as_secs() < TOKEN_EMAIL_CACHE_TTL_SECS {
            return cached;
        }
    }

    let email = sqlx::query_scalar!(
        "SELECT email FROM token WHERE token_hash = $1 AND (expiration > NOW() OR expiration IS NULL)",
        t_hash
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten(); // email column is nullable, so we get Option<Option<String>>

    TOKEN_EMAIL_CACHE.insert(t_hash, (email.clone(), std::time::Instant::now()));
    email
}

/// Get end user email from authenticated user or token.
/// Returns email if user is authenticated (workspace member) or has valid instance token.
pub async fn get_end_user_email(
    db: &DB,
    opt_authed: Option<&ApiAuthed>,
    token: Option<&str>,
) -> Option<String> {
    if let Some(authed) = opt_authed {
        return Some(authed.email.clone());
    }
    if let Some(token) = token {
        return get_email_from_token(db, token).await;
    }
    None
}
// Global function to invalidate tokens from cache by prefix
pub fn invalidate_token_from_cache(token_prefix: &str) {
    // Remove all cache entries whose raw token starts with this prefix (across all workspaces)
    AUTH_CACHE.retain(|(_workspace_id, cached_token), _cached_value| {
        !cached_token.starts_with(token_prefix)
    });
    tracing::info!(
        "Invalidated token(s) from auth cache with prefix: {}...",
        &token_prefix[..token_prefix.len().min(8)]
    );
}

#[derive(Clone)]
pub struct ExpiringAuthCache {
    pub authed: ApiAuthed,
    pub expiry: chrono::DateTime<chrono::Utc>,
    pub job_id: Option<uuid::Uuid>,
}

pub struct AuthCache {
    db: DB,
    superadmin_secret: Option<String>,
    #[cfg(feature = "enterprise")]
    ext_jwks: Option<Arc<RwLock<ExternalJwks>>>,
}

impl AuthCache {
    pub fn new(
        db: DB,
        superadmin_secret: Option<String>,
        #[cfg(feature = "enterprise")] ext_jwks: Option<Arc<RwLock<ExternalJwks>>>,
    ) -> Self {
        AuthCache {
            db,
            superadmin_secret,
            #[cfg(feature = "enterprise")]
            ext_jwks,
        }
    }

    pub async fn invalidate(&self, w_id: &str, token: String) {
        AUTH_CACHE.remove(&(w_id.to_string(), token));
    }

    pub async fn get_authed(&self, w_id: Option<String>, token: &str) -> Option<ApiAuthed> {
        Some(self.get_opt_job_authed(w_id, token).await?.authed)
    }

    pub async fn get_opt_job_authed(
        &self,
        w_id: Option<String>,
        token: &str,
    ) -> Option<OptJobAuthed> {
        let mut opt_job_authed = self.get_opt_job_authed_inner(w_id.clone(), token).await?;
        // Single source of truth: mirror the resolved job_id onto the authed so
        // every consumer (require_super_admin, ...) sees that this identity came
        // from a job's WM_TOKEN, even on an AUTH_CACHE hit whose cached authed
        // predates this field.
        opt_job_authed.authed.job_id = opt_job_authed.job_id;
        // The workspace's guest switch is enforced here, once, for every guest request
        // — not per handler, where each guest-reachable route would have to remember
        // it. Uncached, so turning guests off takes effect on the next request of every
        // guest session and every token derived from one.
        if crate::scopes::has_guest_sentinel(opt_job_authed.authed.scopes.as_deref()) {
            let Some(w_id) = w_id else { return None };
            let email = &opt_job_authed.authed.email;
            match windmill_common::workspaces::guest_session_stands(&self.db, &w_id, email).await {
                Ok(true) => {}
                Ok(false) => return None,
                Err(e) => {
                    tracing::error!("guest session check failed for {w_id}: {e:#}");
                    return None;
                }
            }
        }
        Some(opt_job_authed)
    }

    async fn get_opt_job_authed_inner(
        &self,
        w_id: Option<String>,
        token: &str,
    ) -> Option<OptJobAuthed> {
        // In no-auth mode there are no real tokens: resolve directly as the
        // admin superadmin so direct cache callers (e.g. get_all_runnables,
        // which re-validates the request token per workspace) don't reject the
        // fabricated token.
        if is_no_auth() {
            return Some(OptJobAuthed { authed: no_auth_admin_authed(), job_id: None });
        }
        // Reject an oversized guest bearer before the cache key is built from it: the key
        // copies and hashes the whole token, so the cap should bound that work too. Log it
        // like the other guest refusals, since get_opt_job_authed turns None into a bare 401.
        if token.starts_with(windmill_common::guest_jwt::BEARER_PREFIX)
            && token.len() > windmill_common::guest_jwt::MAX_GUEST_JWT_LEN
        {
            tracing::error!(
                "guest JWT refused: bearer is longer than {} bytes",
                windmill_common::guest_jwt::MAX_GUEST_JWT_LEN
            );
            return None;
        }
        let key = (
            w_id.as_ref().unwrap_or(&"".to_string()).to_string(),
            token.to_string(),
        );
        let s = AUTH_CACHE.get(&key).map(|c| c.to_owned());
        match s {
            Some(ExpiringAuthCache { authed, expiry, job_id }) if expiry > chrono::Utc::now() => {
                Some(OptJobAuthed { authed, job_id })
            }
            #[cfg(feature = "enterprise")]
            _ if token.starts_with("jwt_ext_") => {
                let authed_and_exp = match crate::ee_oss::jwt_ext_auth(
                    w_id.as_ref(),
                    token.trim_start_matches("jwt_ext_"),
                    self.ext_jwks.clone(),
                    &self.db,
                )
                .await
                {
                    Ok(r) => Some(r),
                    Err(e) => {
                        tracing::error!("JWT_EXT auth error: {:?}", e);
                        None
                    }
                };

                if let Some((authed, exp, job_id)) = authed_and_exp.clone() {
                    AUTH_CACHE.insert(
                        key,
                        ExpiringAuthCache {
                            authed: authed.clone(),
                            expiry: chrono::Utc.timestamp_nanos(exp as i64 * 1_000_000_000),
                            job_id,
                        },
                    );

                    Some(OptJobAuthed { authed, job_id })
                } else {
                    None
                }
            }
            _ if token.starts_with(windmill_common::guest_jwt::BEARER_PREFIX) => {
                // A workspace-less route never accepts a guest JWT: the identity is
                // pinned to the workspace its claim names, like a DB guest session.
                let Some(w_id) = w_id.as_deref() else {
                    return None;
                };
                // Strip exactly one prefix: `trim_start_matches` would strip repeated prefixes,
                // so `jwt_guest_jwt_guest_<jwt>` would reduce to a valid token that verifies and
                // is then cached under the full, non-canonical bearer key.
                let jwt = token
                    .strip_prefix(windmill_common::guest_jwt::BEARER_PREFIX)
                    .unwrap_or(token);
                let claims =
                    match windmill_common::guest_jwt::verify_for_workspace(&self.db, w_id, jwt)
                        .await
                    {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::error!("guest JWT auth error for {w_id}: {e:#}");
                            return None;
                        }
                    };
                // The workspace switch, the instance switch and the app being in guest
                // mode, in one answer (guest_app_admits). The door re-reads the switches
                // and the no-account rule per request through the sentinel below
                // (guest_session_stands), so turning any of them off stops a cached JWT
                // session on its next call.
                match windmill_common::workspaces::guest_app_admits(
                    &self.db,
                    w_id,
                    &claims.app_path,
                )
                .await
                {
                    Ok(true) => {}
                    Ok(false) => return None,
                    Err(e) => {
                        tracing::error!("guest JWT admit check failed for {w_id}: {e:#}");
                        return None;
                    }
                }
                // Resolve on the lowercased email: accounts are stored lowercased, so a
                // mixed-case claim would otherwise slip past the no-account gate and
                // resolve an account holder to a guest, and split the activity rows the
                // seat count reads.
                let email = claims.email.to_lowercase();
                // A guest is someone with no account at all; an account holder is refused,
                // never downgraded (the same rule as the signed-in guest mint).
                match windmill_common::users::has_any_account(&self.db, &email).await {
                    Ok(false) => {}
                    Ok(true) => return None,
                    Err(e) => {
                        tracing::error!("guest JWT account check failed: {e:#}");
                        return None;
                    }
                }
                // The instance allowance, checked and recorded transactionally. A stranger
                // past the cap on a capped instance is refused here; a returning guest
                // always passes. Recording an account holder is avoided by the check above.
                if !admit_and_record_guest_jwt(&self.db, w_id, &email, &claims.app_path).await {
                    return None;
                }
                // guest_session_scopes already carries the sentinel, and it is the whole
                // grant; a JWT has no label, so the sentinel is what governs it. It also
                // re-checks the path holds no scope metacharacter (verify already did).
                let scopes = match crate::scopes::guest_session_scopes(&claims.app_path) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        tracing::error!("guest JWT app_path cannot be scoped for {w_id}: {e:#}");
                        return None;
                    }
                };
                // The JWT's own expiry caps a token minted from this session. The auth
                // cache entry itself is capped far shorter (GUEST_JWT_CACHE_TTL) so a
                // rotated or cleared key stops the session on re-verification, within
                // minutes, rather than only at exp (up to 24h away).
                let credential_expiry =
                    chrono::Utc.timestamp_nanos(claims.exp as i64 * 1_000_000_000);
                let cache_expiry = credential_expiry.min(chrono::Utc::now() + GUEST_JWT_CACHE_TTL);
                let authed = ApiAuthed {
                    username: email.clone(),
                    email,
                    is_admin: false,
                    is_operator: true,
                    groups: vec![],
                    folders: vec![],
                    scopes,
                    username_override: None,
                    username_override_is_token_label: false,
                    is_session_token: false,
                    token_prefix: Some(safe_token_prefix(token)),
                    read_only: false,
                    job_id: None,
                    credential_expiry: Some(credential_expiry),
                };
                AUTH_CACHE.insert(
                    key,
                    ExpiringAuthCache {
                        authed: authed.clone(),
                        expiry: cache_expiry,
                        job_id: None,
                    },
                );
                Some(OptJobAuthed { authed, job_id: None })
            }
            _ if token.starts_with("jwt_") => {
                let jwt_token = token.trim_start_matches("jwt_");

                let jwt_result = jwt::decode_with_internal_secret::<JWTAuthClaims>(jwt_token).await;

                match jwt_result {
                    Ok(claims) => {
                        if w_id.is_some_and(|w_id| !claims.allowed_in_workspace(&w_id)) {
                            tracing::error!("JWT auth error: workspace_id mismatch");
                            return None;
                        }
                        let is_session_token = is_session_label(claims.label.as_deref());
                        let (username_override, username_override_is_token_label) =
                            username_override_from_label(claims.label);

                        let authed = ApiAuthed {
                            email: claims.email,
                            username: claims.username,
                            is_admin: claims.is_admin,
                            is_operator: claims.is_operator,
                            groups: claims.groups,
                            folders: claims.folders,
                            // Honor the scopes embedded in the JWT (mirrors the EE
                            // jwt_ext_ branch). The route middleware only enforces
                            // scopes when Some, so a None-scoped JWT (e.g. the job
                            // WM_TOKEN) keeps full user privileges as before.
                            scopes: claims.scopes,
                            username_override,
                            username_override_is_token_label,
                            is_session_token,
                            token_prefix: claims.audit_span,
                            read_only: false,
                            job_id: None,
                            credential_expiry: None,
                        };
                        // Fail closed: a `job_id` claim that does not parse must reject
                        // the token rather than resolve to `None`, which would clear the
                        // job provenance and uncap the token (GHSA-hfh4-cx4h-3fcr).
                        let job_id = match claims.job_id {
                            Some(j) => match uuid::Uuid::from_str(&j) {
                                Ok(job_id) => Some(job_id),
                                Err(_) => {
                                    tracing::error!("JWT auth error: job_id claim is not a uuid");
                                    return None;
                                }
                            },
                            None => None,
                        };
                        AUTH_CACHE.insert(
                            key,
                            ExpiringAuthCache {
                                authed: authed.clone(),
                                expiry: chrono::Utc
                                    .timestamp_nanos(claims.exp as i64 * 1_000_000_000),
                                job_id,
                            },
                        );

                        Some(OptJobAuthed { authed, job_id })
                    }
                    Err(err) => {
                        tracing::error!("JWT auth error: {:?}", err);
                        None
                    }
                }
            }
            _ => {
                let t_hash = hash_token(token);
                let user_o = sqlx::query!(
                    "UPDATE token SET last_used_at = now() WHERE
                        token_hash = $1
                        AND (expiration > NOW() OR expiration IS NULL)
                        AND (workspace_id IS NULL OR workspace_id = $2)
                    RETURNING owner, email, super_admin, scopes, label, read_only",
                    t_hash,
                    w_id.as_ref(),
                )
                .map(|x| {
                    (
                        x.owner,
                        x.email,
                        x.super_admin,
                        x.scopes,
                        x.label,
                        x.read_only,
                    )
                })
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten();

                if let Some(user) = user_o {
                    let authed_o = {
                        match user {
                            (Some(owner), Some(email), super_admin, _, label, read_only)
                                if w_id.is_some() =>
                            {
                                let is_session_token = is_session_label(label.as_deref());
                                let (username_override, username_override_is_token_label) =
                                    username_override_from_label(label);
                                if let Some((prefix, name)) = owner.split_once('/') {
                                    if prefix == "u" {
                                        let lookup = if super_admin {
                                            Some((true, false))
                                        } else {
                                            sqlx::query!(
                                                "SELECT is_admin, operator FROM usr where username = $1 AND \
                                                 workspace_id = $2 AND disabled = false",
                                                name,
                                                &w_id.as_ref().unwrap()
                                            )
                                            .fetch_optional(&self.db)
                                            .await
                                            .ok()
                                            .flatten()
                                            .map(|r| (r.is_admin, r.operator))
                                        };

                                        if let Some((is_admin, is_operator)) = lookup {
                                            let w_id = &w_id.unwrap();
                                            let groups =
                                                get_groups_for_user(w_id, &name, &email, &self.db)
                                                    .await
                                                    .ok()
                                                    .unwrap_or_default();

                                            let folders = get_folders_for_user(
                                                w_id, &name, &groups, &self.db,
                                            )
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
                                                username_override,
                                                username_override_is_token_label,
                                                is_session_token,
                                                token_prefix: Some(safe_token_prefix(token)),
                                                read_only,
                                                job_id: None,
                                                credential_expiry: None,
                                            })
                                        } else {
                                            tracing::warn!(
                                                "Token owner u/{} is not a member of workspace {}; rejecting auth",
                                                name,
                                                w_id.as_deref().unwrap_or("")
                                            );
                                            None
                                        }
                                    } else if prefix == "g" {
                                        let group_exists = if super_admin {
                                            true
                                        } else {
                                            sqlx::query_scalar!(
                                                "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)",
                                                &w_id.as_ref().unwrap(),
                                                name,
                                            )
                                            .fetch_one(&self.db)
                                            .await
                                            .ok()
                                            .flatten()
                                            .unwrap_or(false)
                                        };

                                        if group_exists {
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
                                                username: format!(
                                                    "{}{name}",
                                                    windmill_common::users::USERNAME_GROUP_PREFIX
                                                ),
                                                is_admin: false,
                                                groups,
                                                is_operator: false,
                                                folders,
                                                scopes: None,
                                                username_override,
                                                username_override_is_token_label,
                                                is_session_token,
                                                token_prefix: Some(safe_token_prefix(token)),
                                                read_only,
                                                job_id: None,
                                                credential_expiry: None,
                                            })
                                        } else {
                                            tracing::warn!(
                                                "Token owner g/{} is not a group in workspace {}; rejecting auth",
                                                name,
                                                w_id.as_deref().unwrap_or("")
                                            );
                                            None
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Token owner '{}' has unrecognised prefix '{}'; rejecting auth",
                                            owner,
                                            prefix
                                        );
                                        None
                                    }
                                } else {
                                    tracing::warn!(
                                        "Token owner '{}' is missing a prefix (expected u/ or g/); rejecting auth",
                                        owner
                                    );
                                    None
                                }
                            }
                            (_, Some(email), super_admin, scopes, label, read_only) => {
                                let is_session_token = is_session_label(label.as_deref());
                                let is_guest_session =
                                    windmill_common::auth::is_guest_session_label(label.as_deref());
                                let (username_override, username_override_is_token_label) =
                                    username_override_from_label(label);
                                if w_id.is_some() {
                                    let row_o = sqlx::query!(
                                        "SELECT username, is_admin, operator FROM usr WHERE
                                            email = $1 AND workspace_id = $2 AND disabled = false",
                                        &email,
                                        w_id.as_ref().unwrap()
                                    )
                                    .map(|x| (x.username, x.is_admin, x.operator))
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
                                                username_override,
                                                username_override_is_token_label,
                                                is_session_token,
                                                token_prefix: Some(safe_token_prefix(token)),
                                                read_only,
                                                job_id: None,
                                                credential_expiry: None,
                                            })
                                        }
                                        None if super_admin => {
                                            // Fail closed on a DB error rather than
                                            // letting the email leak in as the username.
                                            match get_instance_username_or_fallback_to_email(
                                                &self.db, &email,
                                            )
                                            .await
                                            {
                                                Ok(username) => Some(ApiAuthed {
                                                    email,
                                                    username,
                                                    is_admin: super_admin,
                                                    is_operator: false,
                                                    groups: vec![],
                                                    folders: vec![],
                                                    scopes,
                                                    username_override,
                                                    username_override_is_token_label,
                                                    is_session_token,
                                                    token_prefix: Some(safe_token_prefix(token)),
                                                    read_only,
                                                    job_id: None,
                                                    credential_expiry: None,
                                                }),
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to resolve instance username for superadmin {email}: {e:#}"
                                                    );
                                                    None
                                                }
                                            }
                                        }
                                        // A guest session: IdP-authenticated, member of
                                        // nothing. No `usr` lookup, groups or folders, so
                                        // every ACL denies it and the token's scopes are
                                        // its whole grant. After the superadmin arm, so
                                        // that token is never demoted into this one.
                                        None if is_guest_session => {
                                            // The server-minted label is the grant, never
                                            // the `guest` scope (a user-minted token's
                                            // scopes are whatever the caller typed); the
                                            // sentinel is pinned on here so every guest
                                            // control downstream sees a guest regardless.
                                            let scopes = Some(crate::scopes::with_guest_sentinel(
                                                scopes.unwrap_or_default(),
                                            ));
                                            Some(ApiAuthed {
                                                username: email.clone(),
                                                email,
                                                is_admin: false,
                                                is_operator: true,
                                                groups: vec![],
                                                folders: vec![],
                                                scopes,
                                                username_override,
                                                username_override_is_token_label,
                                                is_session_token,
                                                token_prefix: Some(safe_token_prefix(token)),
                                                read_only,
                                                job_id: None,
                                                credential_expiry: None,
                                            })
                                        }
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
                                        username_override,
                                        username_override_is_token_label,
                                        is_session_token,
                                        token_prefix: Some(safe_token_prefix(token)),
                                        read_only,
                                        job_id: None,
                                        credential_expiry: None,
                                    })
                                }
                            }
                            _ => None,
                        }
                    };
                    if let Some(authed) = authed_o.as_ref() {
                        AUTH_CACHE.insert(
                            key,
                            ExpiringAuthCache {
                                authed: authed.clone(),
                                expiry: chrono::Utc::now()
                                    + chrono::Duration::try_seconds(120).unwrap(),
                                job_id: None,
                            },
                        );
                    }
                    authed_o.map(|authed| OptJobAuthed { authed, job_id: None })
                } else if self
                    .superadmin_secret
                    .as_ref()
                    .map(|x| x == token)
                    .unwrap_or(false)
                {
                    let authed = ApiAuthed {
                        email: SUPERADMIN_SECRET_EMAIL.to_string(),
                        username: "superadmin_secret".to_string(),
                        is_admin: true,
                        is_operator: false,
                        groups: Vec::new(),
                        folders: Vec::new(),
                        scopes: None,
                        username_override: None,
                        username_override_is_token_label: false,
                        is_session_token: false,
                        token_prefix: Some(safe_token_prefix(token)),
                        read_only: false,
                        job_id: None,
                        credential_expiry: None,
                    };
                    Some(OptJobAuthed { authed, job_id: None })
                } else {
                    None
                }
            }
        }
    }
}

/// How long a guest JWT resolves from the auth cache before the arm re-runs (and
/// re-reads the key). A guest JWT is not revocable except by the workspace switch or
/// by rotating the key, so the entry must be short enough that a rotated key bites
/// soon, unlike a normal token whose row can be deleted. Also what makes the
/// day-keyed activity dedupe below reachable across a midnight.
const GUEST_JWT_CACHE_TTL: chrono::Duration = chrono::Duration::minutes(5);

/// A refused JWT (a stranger past the allowance) is remembered this long so a replayed
/// bearer does not take the instance-wide allowance advisory lock on every request.
/// Short, so a stranger admitted once the window frees is re-checked soon.
const GUEST_JWT_REFUSED_TTL: std::time::Duration = std::time::Duration::from_secs(30);

lazy_static::lazy_static! {
    // One `guest_activity` upsert and one `users.login_guest` audit per email,
    // workspace and day: the arm re-runs every GUEST_JWT_CACHE_TTL, and neither the
    // seat scan nor the audit trail wants a write each time. LRU-bounded; the day is in
    // the key, so a new day writes again.
    static ref GUEST_JWT_ACTIVITY_CACHE: Cache<String, ()> = Cache::new(2000);
    static ref GUEST_JWT_REFUSED_CACHE: Cache<String, std::time::Instant> = Cache::new(2000);
}

/// Admit a JWT guest against the instance allowance and record today's activity, in one
/// transaction so the advisory lock in `guest_admission` spans the count check and the
/// row that changes it. Returns false when the allowance refuses the email or on a DB
/// error, both of which deny the guest. Cached per email, workspace and day: a bearer
/// replayed every request runs this at most once a day, and a refused one is remembered
/// briefly so it does not re-take the allowance lock. `email` is already lowercased.
async fn admit_and_record_guest_jwt(db: &DB, w_id: &str, email: &str, app_path: &str) -> bool {
    let cache_key = format!("{email}|{w_id}|{}", chrono::Utc::now().date_naive());
    if GUEST_JWT_ACTIVITY_CACHE.get(&cache_key).is_some() {
        return true;
    }
    if GUEST_JWT_REFUSED_CACHE
        .get(&cache_key)
        .is_some_and(|at| at.elapsed() < GUEST_JWT_REFUSED_TTL)
    {
        return false;
    }
    let mut tx = match db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("guest JWT tx begin failed for {w_id}: {e:#}");
            return false;
        }
    };
    // The allowance and the row that changes it, in one transaction: guest_admission
    // takes a transaction-scoped advisory lock, so the count check and the insert cannot
    // race two strangers past the cap. Only a real allowance refusal is negative-cached;
    // a transient DB error denies this request but must not lock the email out for 30s.
    match windmill_common::workspaces::guest_admission(&mut *tx, email).await {
        Ok(()) => {}
        Err(e @ windmill_common::error::Error::PermissionDenied(_)) => {
            // The guest hits a bare 401 (the reason must not leak to an unauthenticated caller);
            // warn so an admin sees the cap in logs, since it is the actionable signal here.
            tracing::warn!("guest JWT refused (guest allowance) for {w_id}: {e:#}");
            GUEST_JWT_REFUSED_CACHE.insert(cache_key, std::time::Instant::now());
            return false;
        }
        Err(e) => {
            tracing::error!("guest JWT allowance check failed for {w_id}: {e:#}");
            return false;
        }
    }
    // The conditional `WHERE NOT jwt_entry` flips the flag only on its false-to-true
    // transition, so the upsert returns a row exactly once per email per day: on the
    // fresh insert, or on the first JWT after an identity-provider sign-in created
    // today's row with `jwt_entry = false`. The audit is gated on that, decided
    // atomically by the conflicting tuple, so concurrent first requests (a metered
    // instance takes no advisory lock) audit at most once.
    let first_jwt = sqlx::query_scalar!(
        r#"INSERT INTO guest_activity (email, workspace_id, day, jwt_entry)
         VALUES ($1, $2, CURRENT_DATE, true)
         ON CONFLICT (email, workspace_id, day)
         DO UPDATE SET jwt_entry = true, last_seen_at = now()
         WHERE NOT guest_activity.jwt_entry
         RETURNING 1 AS "audited!""#,
        email,
        w_id,
    )
    .fetch_optional(&mut *tx)
    .await;
    let first_jwt = match first_jwt {
        Ok(v) => v.is_some(),
        Err(e) => {
            tracing::error!("recording guest JWT activity for {w_id}: {e:#}");
            return false;
        }
    };
    if let Err(e) = tx.commit().await {
        tracing::error!("guest JWT tx commit failed for {w_id}: {e:#}");
        return false;
    }
    GUEST_JWT_ACTIVITY_CACHE.insert(cache_key, ());
    // Audit last, best-effort, on its own connection: the EE writer swallows an
    // `audit_partitioned` failure but that failing statement still aborts the
    // transaction it runs in, so auditing before the commit would let the whole
    // activity row roll back while this returned success, admitting an uncounted guest.
    if first_jwt {
        let author = windmill_common::audit::AuditAuthor {
            email: email.to_string(),
            username: email.to_string(),
            username_override: None,
            token_prefix: None,
        };
        if let Err(e) = windmill_audit::audit_oss::audit_log(
            db,
            &author,
            "users.login_guest",
            windmill_audit::ActionKind::Create,
            w_id,
            Some(app_path),
            Some([("entry", "jwt")].into()),
        )
        .await
        {
            tracing::error!("auditing guest JWT login for {w_id}: {e:#}");
        }
    }
    true
}

pub(crate) async fn extract_token<S: Send + Sync>(parts: &mut Parts, state: &S) -> Option<String> {
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
            .and_then(|cookies| {
                cookies
                    .get(COOKIE_NAME)
                    .map(|c| c.value_trimmed().to_owned())
            }),
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

#[derive(Clone, Debug)]
pub struct OptTokened {
    #[allow(dead_code)]
    pub token: Option<String>,
}

struct BruteForceCounter {
    counter: AtomicU64,
    last_reset: AtomicI64,
}

lazy_static::lazy_static! {
    static ref BRUTE_FORCE_COUNTER: BruteForceCounter =
        BruteForceCounter { last_reset: AtomicI64::new(0), counter: AtomicU64::new(0) };
}

impl BruteForceCounter {
    async fn increment(&self) {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if self.counter.fetch_add(1, Ordering::Relaxed) > 10000 {
            tracing::error!(
                "Brute force attack to find valid token detected, sleeping unauthorized response for 2 seconds"
            );
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        if now - self.last_reset.load(Ordering::Relaxed) > 60 {
            self.counter.store(0, Ordering::Relaxed);
            self.last_reset.store(now, Ordering::Relaxed);
        }
    }
}

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
            } else if is_no_auth() {
                // In `--no-auth` mode requests carry no token, but handlers that
                // also require Tokened (e.g. global_whoami) must still resolve.
                let tokened = Self { token: "no_auth".to_string() };
                parts.extensions.insert(tokened.clone());
                Ok(tokened)
            } else {
                BRUTE_FORCE_COUNTER.increment().await;
                Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
            }
        }
    }
}

impl<S> FromRequestParts<S> for OptTokened
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        if parts.method == http::Method::OPTIONS {
            return Ok(OptTokened { token: None });
        };
        let already_tokened = parts.extensions.get::<Tokened>();
        if let Some(tokened) = already_tokened {
            Ok(OptTokened { token: Some(tokened.token.clone()) })
        } else {
            let token_o = extract_token(parts, state).await;
            Ok(OptTokened { token: token_o })
        }
    }
}

pub fn transform_old_scope_to_new_scope(scopes: Option<&mut Vec<String>>) {
    if let Some(scopes) = scopes {
        for scope in scopes.iter_mut() {
            if scope.starts_with("run:") {
                let (_, part_scope) = scope.split_once(":").unwrap();

                if let Some((kind, path)) = part_scope.split_once("/") {
                    //appending a 's' as runnable kind is singular while new scope format expect it to be plural
                    *scope = format!("jobs:run:{}s:{}", kind, path);
                }
            } else if scope.starts_with("jobs:") {
                // Map old jobs scopes to new format
                let new_scope = match scope.as_str() {
                    "jobs:listjobs" => "jobs:read",
                    "jobs:runscript" => "jobs:run:scripts",
                    "jobs:runflow" => "jobs:run:flows",
                    "jobs:resumeflow" => "jobs:run:flows",
                    "jobs:deletejob" => "jobs:write",
                    _ => continue,
                };

                *scope = new_scope.to_string();
            }
        }
    }
}

fn maybe_get_workspace_id_from_path(path_vec: &[&str]) -> Option<String> {
    let workspace_id = if path_vec.len() >= 4 && path_vec[0] == "" && path_vec[2] == "w" {
        Some(path_vec[3].to_owned())
    } else if path_vec.len() >= 5
        && path_vec[0] == ""
        && path_vec[1] == "api"
        && path_vec[2] == "mcp"
        && path_vec[3] == "w"
    {
        Some(path_vec[4].to_owned())
    } else {
        if path_vec.len() >= 5 && path_vec[0] == "" && path_vec[2] == "srch" && path_vec[3] == "w" {
            Some(path_vec[4].to_owned())
        } else {
            None
        }
    };

    workspace_id
}

/// `--no-auth` mode: compiled-in `oss` builds, or the `NO_AUTH` runtime flag on
/// any build (the runtime flag is force-disabled on CLOUD_HOSTED). When on,
/// every request resolves as the admin superadmin so a fronting gateway can
/// handle authentication instead.
pub fn is_no_auth() -> bool {
    cfg!(feature = "no_auth") || *windmill_common::worker::NO_AUTH
}

/// The synthetic superadmin identity returned for every request in no-auth mode.
fn no_auth_admin_authed() -> ApiAuthed {
    ApiAuthed {
        email: "admin@windmill.dev".to_string(),
        username: "admin".to_string(),
        is_admin: true,
        is_operator: false,
        groups: Vec::new(),
        folders: Vec::new(),
        scopes: None,
        username_override: None,
        username_override_is_token_label: false,
        is_session_token: false,
        token_prefix: None,
        read_only: false,
        job_id: None,
        credential_expiry: None,
    }
}

/// Resolves OptJobAuthed from request parts.
/// Takes ownership of Parts and returns them back.
#[allow(unreachable_code, unused_mut)]
pub async fn resolve_opt_job_authed(
    mut parts: Parts,
) -> std::result::Result<(OptJobAuthed, Parts), (Error, Parts)> {
    if parts.method == http::Method::OPTIONS {
        return Ok((OptJobAuthed::default(), parts));
    };

    if is_no_auth() {
        return Ok((
            OptJobAuthed { authed: no_auth_admin_authed(), job_id: None },
            parts,
        ));
    }

    let already_authed = parts.extensions.get::<OptJobAuthed>().cloned();

    if let Some(authed) = already_authed {
        return Ok((authed, parts));
    }

    let already_tokened = parts.extensions.get::<Tokened>().cloned();
    let token_o = if let Some(token) = already_tokened {
        Some(token.token.clone())
    } else {
        extract_token(&mut parts, &()).await
    };
    if let Some(token) = token_o {
        if let Ok(Extension(cache)) =
            Extension::<Arc<AuthCache>>::from_request_parts(&mut parts, &()).await
        {
            let original_uri = OriginalUri::from_request_parts(&mut parts, &())
                .await
                .ok()
                .map(|x| x.0)
                .unwrap_or_default();
            let path_vec: Vec<&str> = original_uri.path().split("/").collect();
            let workspace_id = maybe_get_workspace_id_from_path(&path_vec).or_else(|| {
                parts
                    .extensions
                    .get::<windmill_common::db::GatewayWorkspaceId>()
                    .map(|g| g.0.clone())
            });

            if let Some(mut opt_job_authed) =
                cache.get_opt_job_authed(workspace_id.clone(), &token).await
            {
                let path = original_uri.path();
                let method = parts.method.as_str();
                if workspace_id.is_none() && opt_job_authed.job_id.is_some() {
                    if let Err(err) = crate::scopes::check_job_token_for_global_route(path, method)
                    {
                        return Err((err, parts));
                    }
                }
                let authed = &mut opt_job_authed.authed;
                if authed.scopes.is_some() {
                    transform_old_scope_to_new_scope(authed.scopes.as_mut());

                    if let Err(err) = crate::scopes::check_scopes_for_route(
                        authed.scopes.as_deref(),
                        path,
                        method,
                    ) {
                        return Err((err, parts));
                    }
                }
                if authed.read_only {
                    // MCP transport runs over POST (streamable HTTP / SSE handshake),
                    // so the middleware can't safely reject mutating methods on it —
                    // the MCP runner itself filters out write tools and rejects
                    // mutating tool calls for read-only tokens. Narrow to the actual
                    // transport endpoints: anything else under `/api/mcp/*` (OAuth
                    // approve, token exchange, client registration) must still go
                    // through the read-only check, otherwise a read-only token
                    // could approve an OAuth flow that mints a new non-read-only
                    // token.
                    let is_mcp_transport = path == "/api/mcp/gateway"
                        || (path.starts_with("/api/mcp/w/")
                            && (path.ends_with("/mcp")
                                || path.ends_with("/sse")
                                || path.ends_with("/list_tools")));
                    if !is_mcp_transport {
                        if let Err(err) = crate::scopes::check_read_only_for_route(path, method) {
                            return Err((err, parts));
                        }
                    }
                }
                parts.extensions.insert(authed.clone());

                Span::current().record("username", &authed.username.as_str());
                Span::current().record("email", &authed.email);

                // Mirror into the per-request LogContext so exported OTEL
                // LogRecords carry the same identifiers (the log bridge
                // doesn't walk span fields — see windmill_common::log_context).
                let username_copy = authed.username.clone();
                let email_copy = authed.email.clone();
                let workspace_copy = workspace_id.clone();
                windmill_common::log_context::update_log_context(move |c| {
                    windmill_common::log_context::LogContext {
                        username: Some(username_copy),
                        email: Some(email_copy),
                        workspace_id: workspace_copy.or_else(|| c.workspace_id.clone()),
                        ..c.clone()
                    }
                });

                if let Some(workspace_id) = workspace_id {
                    Span::current().record("workspace_id", &workspace_id);
                }
                return Ok((opt_job_authed, parts));
            }
        }
    }
    BRUTE_FORCE_COUNTER.increment().await;
    Err((Error::NotAuthorized("Unauthorized".to_string()), parts))
}

/// Returns the override and whether it names the token's *label* rather than the entity that
/// fired the request. Callers must not re-derive the second element from the first: the
/// `ephemeral-script-end-user-` arm forwards a `created_by` verbatim, and `created_by` is
/// unconstrained, so it may itself look like any of these shapes.
///
/// Only namespaces `create_token` rejects (`is_server_minted_label`) are trusted to name the
/// entity acting, so the label can only have come from a server-side mint. Tokens minted
/// before that guard existed are the remaining hole; closing it needs the token row to record
/// who minted it rather than inferring it from the label.
///
/// Note that a trigger whose identity is set server-side — the SMTP one builds an `email-*`
/// override directly — does not rely on this at all, so its prefix must not be trusted here.
pub(crate) fn username_override_from_label(label: Option<String>) -> (Option<String>, bool) {
    match label {
        Some(label) if label.starts_with("ephemeral-webhook-") => (Some(label), false),
        Some(label) if label.starts_with("ephemeral-script-end-user-") => (
            Some(
                label
                    .trim_start_matches("ephemeral-script-end-user-")
                    .to_string(),
            ),
            false,
        ),
        // User-mintable, so they name nobody in particular — the trigger panels merely
        // pre-fill `webhook-`/`http-`, and the editor mints the lsp one. The override keeps
        // its value because `require_job_read_access` matches it against the `created_by` of
        // jobs launched under it, which these shapes produced while they were trusted.
        Some(label) if label == "Ephemeral lsp token" => (Some("lsp".to_string()), true),
        Some(label)
            if label.starts_with("webhook-")
                || label.starts_with("http-")
                || label.starts_with("email-")
                || label.starts_with("ws-") =>
        {
            (Some(label), true)
        }
        Some(label)
            if label != "ephemeral-script"
                && label != "session"
                && label != windmill_common::auth::GUEST_SESSION_LABEL
                && !label.is_empty() =>
        {
            (
                Some(format!("{}{label}", crate::GENERIC_TOKEN_LABEL_PREFIX)),
                true,
            )
        }
        _ => (None, false),
    }
}

#[derive(FromRow, Serialize)]
pub struct TruncatedTokenWithEmail {
    pub label: Option<String>,
    pub token_prefix: String,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub scopes: Option<Vec<String>>,
    pub email: Option<String>,
}

pub async fn list_tokens_internal(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
) -> JsonResult<Vec<TruncatedTokenWithEmail>> {
    let tokens = if is_flow {
        sqlx::query_as!(
            TruncatedTokenWithEmail,
            r#"
        SELECT label,
               token_prefix,
               expiration,
               created_at,
               last_used_at,
               scopes,
               email
        FROM token
        WHERE workspace_id = $1
          AND (
               scopes @> ARRAY['jobs:run:flows:' || $2]::text[]
               OR scopes @> ARRAY['run:flow/' || $2]::text[]
              )
        "#,
            w_id,
            path
        )
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as!(
            TruncatedTokenWithEmail,
            r#"
        SELECT label,
               token_prefix,
               expiration,
               created_at,
               last_used_at,
               scopes,
               email
        FROM token
        WHERE workspace_id = $1
          AND (
               scopes @> ARRAY['jobs:run:scripts:' || $2]::text[]
               OR scopes @> ARRAY['run:script/' || $2]::text[]
              )
        "#,
            w_id,
            path
        )
        .fetch_all(db)
        .await?
    };

    Ok(Json(tokens))
}
