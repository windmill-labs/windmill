#[cfg(feature = "enterprise")]
use crate::ee_oss::ExternalJwks;
use axum::{
    async_trait,
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
    auth::{get_folders_for_user, get_groups_for_user, JWTAuthClaims, TOKEN_PREFIX_LEN},
    error::{Error, JsonResult},
    jwt,
    users::{COOKIE_NAME, SUPERADMIN_SECRET_EMAIL},
};

lazy_static::lazy_static! {
    // Global auth cache accessible from main.rs for direct invalidation
    pub static ref AUTH_CACHE: Cache<(String, String), ExpiringAuthCache> = Cache::new(300);

}
// Global function to invalidate a specific token from cache
pub fn invalidate_token_from_cache(token: &str) {
    // Remove all cache entries for this token (across all workspaces)
    AUTH_CACHE.retain(|(_workspace_id, cached_token), _cached_value| cached_token != token);
    tracing::info!(
        "Invalidated token from auth cache: {}...",
        &token[..token.len().min(8)]
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
            _ if token.starts_with("jwt_") => {
                let jwt_token = token.trim_start_matches("jwt_");

                let jwt_result = jwt::decode_with_internal_secret::<JWTAuthClaims>(jwt_token).await;

                match jwt_result {
                    Ok(claims) => {
                        if w_id.is_some_and(|w_id| !claims.allowed_in_workspace(&w_id)) {
                            tracing::error!("JWT auth error: workspace_id mismatch");
                            return None;
                        }
                        let username_override = username_override_from_label(claims.label);

                        let authed = ApiAuthed {
                            email: claims.email,
                            username: claims.username,
                            is_admin: claims.is_admin,
                            is_operator: claims.is_operator,
                            groups: claims.groups,
                            folders: claims.folders,
                            scopes: None,
                            username_override,
                            token_prefix: claims.audit_span,
                        };
                        let job_id = claims.job_id.and_then(|j| uuid::Uuid::from_str(&j).ok());
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
                let user_o = sqlx::query!(
                    "UPDATE token SET last_used_at = now() WHERE
                        token = $1
                        AND (expiration > NOW() OR expiration IS NULL)
                        AND (workspace_id IS NULL OR workspace_id = $2)
                    RETURNING owner, email, super_admin, scopes, label",
                    token,
                    w_id.as_ref(),
                )
                .map(|x| (x.owner, x.email, x.super_admin, x.scopes, x.label))
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten();

                if let Some(user) = user_o {
                    let authed_o = {
                        match user {
                            (Some(owner), Some(email), super_admin, _, label) if w_id.is_some() => {
                                let username_override = username_override_from_label(label);
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
                                            username_override,
                                            token_prefix: Some(
                                                token[0..TOKEN_PREFIX_LEN].to_string(),
                                            ),
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
                                            username_override,
                                            token_prefix: Some(
                                                token[0..TOKEN_PREFIX_LEN].to_string(),
                                            ),
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
                                        username_override,
                                        token_prefix: Some(token[0..TOKEN_PREFIX_LEN].to_string()),
                                    })
                                }
                            }
                            (_, Some(email), super_admin, scopes, label) => {
                                let username_override = username_override_from_label(label);
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
                                                token_prefix: Some(
                                                    token[0..TOKEN_PREFIX_LEN].to_string(),
                                                ),
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
                                            username_override,
                                            token_prefix: Some(
                                                token[0..TOKEN_PREFIX_LEN].to_string(),
                                            ),
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
                                        username_override,
                                        token_prefix: Some(token[0..TOKEN_PREFIX_LEN].to_string()),
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
                        token_prefix: Some(token[0..TOKEN_PREFIX_LEN].to_string()),
                    };
                    Some(OptJobAuthed { authed, job_id: None })
                } else {
                    None
                }
            }
        }
    }
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
                BRUTE_FORCE_COUNTER.increment().await;
                Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
            }
        }
    }
}

#[async_trait]
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

/// Resolves OptJobAuthed from request parts.
/// Takes ownership of Parts and returns them back.
#[allow(unreachable_code, unused_mut)]
pub async fn resolve_opt_job_authed(
    mut parts: Parts,
) -> std::result::Result<(OptJobAuthed, Parts), (Error, Parts)> {
    if parts.method == http::Method::OPTIONS {
        return Ok((OptJobAuthed::default(), parts));
    };

    #[cfg(feature = "no_auth")]
    {
        let authed = ApiAuthed {
            email: "admin@windmill.dev".to_string(),
            username: "admin".to_string(),
            is_admin: true,
            is_operator: false,
            groups: Vec::new(),
            folders: Vec::new(),
            scopes: None,
            username_override: None,
            token_prefix: None,
        };
        return Ok((OptJobAuthed { authed, job_id: None }, parts));
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
            let workspace_id = maybe_get_workspace_id_from_path(&path_vec);

            if let Some(mut opt_job_authed) =
                cache.get_opt_job_authed(workspace_id.clone(), &token).await
            {
                let authed = &mut opt_job_authed.authed;
                if authed.scopes.is_some() {
                    transform_old_scope_to_new_scope(authed.scopes.as_mut());

                    let path = original_uri.path();
                    let method = parts.method.as_str();

                    if let Err(err) = crate::scopes::check_scopes_for_route(
                        authed.scopes.as_deref(),
                        path,
                        method,
                    ) {
                        BRUTE_FORCE_COUNTER.increment().await;
                        return Err((err, parts));
                    }
                }
                parts.extensions.insert(authed.clone());

                Span::current().record("username", &authed.username.as_str());
                Span::current().record("email", &authed.email);

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

fn username_override_from_label(label: Option<String>) -> Option<String> {
    match label {
        Some(label)
            if label.starts_with("webhook-")
                || label.starts_with("http-")
                || label.starts_with("email-")
                || label.starts_with("ws-") =>
        {
            Some(label)
        }
        Some(label) if label.starts_with("ephemeral-script-end-user-") => Some(
            label
                .trim_start_matches("ephemeral-script-end-user-")
                .to_string(),
        ),
        Some(label) if label == "Ephemeral lsp token" => Some("lsp".to_string()),
        Some(label) if label != "ephemeral-script" && label != "session" && !label.is_empty() => {
            Some(format!("label-{label}"))
        }
        _ => None,
    }
}

#[derive(FromRow, Serialize)]
pub struct TruncatedTokenWithEmail {
    pub label: Option<String>,
    pub token_prefix: Option<String>,
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
               concat(substring(token for 10)) AS token_prefix,
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
               concat(substring(token for 10)) AS token_prefix,
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
