#[cfg(feature = "enterprise")]
use crate::ee::ExternalJwks;
use axum::{
    async_trait,
    extract::{FromRequestParts, OriginalUri, Query},
    Extension,
};
use chrono::TimeZone;
use http::{request::Parts, StatusCode};
use quick_cache::sync::Cache;
use serde::Deserialize;
use tower_cookies::Cookies;
use tracing::Span;

use crate::db::{ApiAuthed, DB};
use std::sync::{
    atomic::{AtomicI64, AtomicU64, Ordering},
    Arc,
};
#[cfg(feature = "enterprise")]
use tokio::sync::RwLock;

use windmill_common::{
    auth::{get_folders_for_user, get_groups_for_user, JWTAuthClaims, JWT_SECRET},
    users::{COOKIE_NAME, SUPERADMIN_SECRET_EMAIL},
};

#[derive(Clone)]
pub struct ExpiringAuthCache {
    pub authed: ApiAuthed,
    pub expiry: chrono::DateTime<chrono::Utc>,
}

pub struct AuthCache {
    cache: Cache<(String, String), ExpiringAuthCache>,
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
            cache: Cache::new(300),
            db,
            superadmin_secret,
            #[cfg(feature = "enterprise")]
            ext_jwks,
        }
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
            Some(ExpiringAuthCache { authed, expiry }) if expiry > chrono::Utc::now() => {
                Some(authed)
            }
            #[cfg(feature = "enterprise")]
            _ if token.starts_with("jwt_ext_") => {
                let authed_and_exp = match crate::ee::jwt_ext_auth(
                    w_id.as_ref(),
                    token.trim_start_matches("jwt_ext_"),
                    self.ext_jwks.clone(),
                )
                .await
                {
                    Ok(r) => Some(r),
                    Err(e) => {
                        tracing::error!("JWT_EXT auth error: {:?}", e);
                        None
                    }
                };

                if let Some((authed, exp)) = authed_and_exp.clone() {
                    self.cache.insert(
                        key,
                        ExpiringAuthCache {
                            authed: authed.clone(),
                            expiry: chrono::Utc.timestamp_nanos(exp as i64 * 1_000_000_000),
                        },
                    );

                    Some(authed)
                } else {
                    None
                }
            }
            _ if token.starts_with("jwt_") => {
                let jwt_secret = JWT_SECRET.read().await;
                if !jwt_secret.is_empty() {
                    let jwt_token = token.trim_start_matches("jwt_");

                    let jwt_result = jsonwebtoken::decode::<JWTAuthClaims>(
                        jwt_token,
                        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
                        &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
                    );

                    match jwt_result {
                        Ok(payload) => {
                            if w_id.is_some_and(|w_id| w_id != payload.claims.workspace_id) {
                                tracing::error!("JWT auth error: workspace_id mismatch");
                                return None;
                            }

                            let username_override =
                                username_override_from_label(payload.claims.label);
                            let authed = crate::db::ApiAuthed {
                                email: payload.claims.email,
                                username: payload.claims.username,
                                is_admin: payload.claims.is_admin,
                                is_operator: payload.claims.is_operator,
                                groups: payload.claims.groups,
                                folders: payload.claims.folders,
                                scopes: None,
                                username_override,
                            };

                            self.cache.insert(
                                key,
                                ExpiringAuthCache {
                                    authed: authed.clone(),
                                    expiry: chrono::Utc
                                        .timestamp_nanos(payload.claims.exp as i64 * 1_000_000_000),
                                },
                            );

                            Some(authed)
                        }
                        Err(err) => {
                            tracing::error!("JWT auth error: {:?}", err);
                            None
                        }
                    }
                } else {
                    tracing::error!("JWT auth error: no jwt secret set");
                    None
                }
            }
            _ => {
                let user_o = sqlx::query_as::<_, (Option<String>, Option<String>, bool, Option<Vec<String>>, Option<String>)>(
                    "UPDATE token SET last_used_at = now() WHERE token = $1 AND (expiration > NOW() \
                     OR expiration IS NULL) AND (workspace_id IS NULL OR workspace_id = $2) RETURNING owner, email, super_admin, scopes, label",
                )
                .bind(token)
                .bind(w_id.as_ref())
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
                                    })
                                }
                            }
                            (_, Some(email), super_admin, scopes, label) => {
                                let username_override = username_override_from_label(label);
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
                                                username_override,
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
                                    })
                                }
                            }
                            _ => None,
                        }
                    };
                    if let Some(authed) = authed_o.as_ref() {
                        self.cache.insert(
                            key,
                            ExpiringAuthCache {
                                authed: authed.clone(),
                                expiry: chrono::Utc::now()
                                    + chrono::Duration::try_seconds(120).unwrap(),
                            },
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
                        username_override: None,
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
                username_override: None,
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
                if path_vec.len() >= 5
                    && path_vec[0] == ""
                    && path_vec[2] == "srch"
                    && path_vec[3] == "w"
                {
                    Some(path_vec[4].to_string())
                } else {
                    None
                }
            };
            if let Some(token) = token_o {
                if let Ok(Extension(cache)) =
                    Extension::<Arc<AuthCache>>::from_request_parts(parts, state).await
                {
                    if let Some(authed) = cache.get_authed(workspace_id.clone(), &token).await {
                        parts.extensions.insert(authed.clone());
                        if authed.scopes.as_ref().is_some_and(|scopes| {
                            scopes
                                .iter()
                                .any(|s| s.starts_with("jobs:") || s.starts_with("run:"))
                        }) && (path_vec.len() < 3
                            || (path_vec[4] != "jobs" && path_vec[4] != "jobs_u"))
                        {
                            BRUTE_FORCE_COUNTER.increment().await;
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
            BRUTE_FORCE_COUNTER.increment().await;
            Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
        }
    }
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
