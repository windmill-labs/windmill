/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Debug session signing and audit logging.
//!
//! This module provides cryptographic signing of debug requests to ensure:
//! 1. All debug sessions are logged in the audit trail
//! 2. The debugger only executes code that has been authorized by the backend
//! 3. Replay attacks are prevented via timestamp validation
//!
//! Uses Ed25519 JWT signing. The debugger fetches the public key from /api/debug/jwks
//! and verifies tokens locally.
//!
//! Each debug session creates:
//! - A job entry in v2_job (kind=preview) for traceability
//! - A completed job entry in v2_job_completed
//! - An audit log entry identical to script preview runs
//!
//! The same signature is what authorizes the debugger's requests back to the API:
//! /api/debug/registry_config serves the instance's dependency-registry settings, which the
//! debugger cannot read for itself, to sessions whose token carries the `registry_config`
//! claim.

use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{get, post},
    Extension, Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::types::Json as SqlxJson;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
    global_settings::{
        BUNFIG_INSTALL_SCOPES_SETTING, EXTRA_PIP_INDEX_URL_SETTING, NPMRC_SETTING,
        NPM_CONFIG_REGISTRY_SETTING, PIP_INDEX_URL_SETTING, UV_INDEX_STRATEGY_SETTING,
        WORKSPACE_REGISTRIES_SETTING,
    },
    jobs::JobKind,
    jwt::JWT_SECRET,
    scripts::ScriptLang,
    users::username_to_permissioned_as,
    DB,
};

use windmill_api_auth::ApiAuthed;

/// TTL for debug tokens in seconds (60 seconds)
pub const DEBUG_TOKEN_TTL_SECS: i64 = 60;

lazy_static::lazy_static! {
    /// Ed25519 signing key for debug tokens.
    ///
    /// Derived deterministically from the instance `JWT_SECRET` so all API
    /// replicas agree on the same key without coordination. Refreshed via
    /// [`reload_debug_signing_key`] when `JWT_SECRET` is reloaded.
    static ref DEBUG_SIGNING_KEY: Arc<RwLock<Option<SigningKey>>> = Arc::new(RwLock::new(None));
}

/// Domain-separation tag so the debug Ed25519 seed cannot be confused with
/// any other HMAC/HS256 usage of `JWT_SECRET`.
const DEBUG_KEY_DERIVATION_TAG: &[u8] = b"windmill-debug-signing-key:v1:";

fn derive_signing_key_from_jwt_secret(jwt_secret: &str) -> SigningKey {
    let mut hasher = Sha256::new();
    hasher.update(DEBUG_KEY_DERIVATION_TAG);
    hasher.update(jwt_secret.as_bytes());
    let seed: [u8; 32] = hasher.finalize().into();
    SigningKey::from_bytes(&seed)
}

fn compute_debug_signing_key() -> Option<SigningKey> {
    // Env var override: base64url-encoded 32-byte seed. Useful for tests or
    // advanced deployments that want to pin the key independently.
    if let Ok(seed_b64) = std::env::var("DEBUG_SIGNING_KEY_SEED") {
        match URL_SAFE_NO_PAD.decode(&seed_b64) {
            Ok(seed_bytes) if seed_bytes.len() >= 32 => {
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&seed_bytes[..32]);
                tracing::info!("Debug signing key loaded from DEBUG_SIGNING_KEY_SEED");
                return Some(SigningKey::from_bytes(&seed));
            }
            _ => tracing::warn!(
                "Invalid DEBUG_SIGNING_KEY_SEED (expect base64url-encoded 32+ bytes); falling back to JWT_SECRET derivation"
            ),
        }
    }

    let jwt_secret = JWT_SECRET.load();
    if jwt_secret.is_empty() {
        return None;
    }
    Some(derive_signing_key_from_jwt_secret(&jwt_secret))
}

/// Initialize the debug signing key. Call once at server startup, after
/// `reload_jwt_secret_setting` so `JWT_SECRET` is populated.
pub async fn init_debug_signing_key() {
    reload_debug_signing_key().await;
}

/// Recompute and store the debug signing key. Call after `JWT_SECRET` is
/// (re)loaded so rotation propagates without a pod restart.
pub async fn reload_debug_signing_key() {
    let key = compute_debug_signing_key();
    if key.is_none() {
        tracing::warn!(
            "Debug signing key not initialized: JWT_SECRET is empty and DEBUG_SIGNING_KEY_SEED is not set. /api/debug/* endpoints will return an error."
        );
    } else {
        tracing::info!("Debug signing key initialized from JWT_SECRET");
    }
    *DEBUG_SIGNING_KEY.write().await = key;
}

pub fn global_service() -> Router {
    Router::new()
        .route("/jwks", get(get_jwks))
        .route("/registry_config", get(get_registry_config))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/sign", post(sign_debug_request))
        .route("/sign_expression", post(sign_expression))
        .route("/sign_multiplayer", post(sign_multiplayer))
}

/// JWKS response containing the public key for debug token verification
#[derive(Serialize)]
pub struct DebugJwks {
    pub keys: Vec<DebugJwk>,
}

/// JWK representation of an Ed25519 public key
#[derive(Serialize)]
pub struct DebugJwk {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub kid: String,
    #[serde(rename = "use")]
    pub use_: String,
    pub alg: String,
}

/// Get the JWKS containing the public key for debug token verification.
/// Debugger should fetch this at startup and cache it.
async fn get_jwks() -> JsonResult<DebugJwks> {
    let key_guard = DEBUG_SIGNING_KEY.read().await;
    let signing_key = key_guard.as_ref().ok_or_else(|| {
        windmill_common::error::Error::InternalErr("Debug signing key not initialized".to_string())
    })?;

    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes();

    // Compute key ID as hash of public key
    let mut hasher = Sha256::new();
    hasher.update(&public_key_bytes);
    let kid = hex::encode(&hasher.finalize()[..8]);

    Ok(Json(DebugJwks {
        keys: vec![DebugJwk {
            kty: "OKP".to_string(),
            crv: "Ed25519".to_string(),
            x: URL_SAFE_NO_PAD.encode(public_key_bytes),
            kid,
            use_: "sig".to_string(),
            alg: "EdDSA".to_string(),
        }],
    }))
}

/// The instance's dependency-registry configuration, as `windmill prepare-deps` consumes it.
/// Field names are the `global_settings` keys, so the debug service forwards this object to
/// the CLI as-is.
#[derive(Serialize, Default)]
pub struct DebugRegistryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_config_registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npmrc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bunfig_install_scopes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pip_index_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pip_extra_index_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uv_index_strategy: Option<String>,
    /// Why configured settings were withheld, for the debug service to show the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Placeholder an EE instance puts in an index URL for a token minted per install by
/// `EPHEMERAL_TOKEN_CMD` (`windmill-worker`'s `handle_ephemeral_token`).
const EPHEMERAL_TOKEN_MARKER: &str = "EPHEMERAL_TOKEN";

/// Every `global_settings` key [`get_registry_config`] reads, in one query. A setting
/// resolved there but missing here reads as unset, whatever the instance has stored.
const REGISTRY_SETTINGS: [&str; 7] = [
    NPM_CONFIG_REGISTRY_SETTING,
    NPMRC_SETTING,
    BUNFIG_INSTALL_SCOPES_SETTING,
    PIP_INDEX_URL_SETTING,
    EXTRA_PIP_INDEX_URL_SETTING,
    UV_INDEX_STRATEGY_SETTING,
    WORKSPACE_REGISTRIES_SETTING,
];

/// Which half of the settings a session installs with, from the language its token was signed
/// for: a session is served only what its own installer runs on, so a token minted for one
/// language cannot be replayed to read the other's credentials. Every language the debugger
/// accepts (`isDebuggableLanguage` in the frontend) has to appear here, or its sessions
/// silently install from the public registries.
fn registry_settings_for_language(language: &str) -> (bool, bool) {
    match language {
        "bun" | "typescript" | "deno" | "nativets" => (true, false),
        "python3" | "python" => (false, true),
        _ => (false, false),
    }
}

/// Resolve one setting the way a worker resolves it: a workspace override wins over the
/// instance value, which is `FORCE_<env>` > `global_settings` > `<env>` (the server's
/// `load_option_setting_value`). A blank value means unset from either source, so a
/// workspace can blank one out.
fn resolve_registry_setting(
    stored: &std::collections::HashMap<String, serde_json::Value>,
    workspace_overrides: Option<&serde_json::Value>,
    key: &str,
    env_var: &str,
) -> Option<String> {
    let as_str = |v: Option<&serde_json::Value>| v.and_then(|v| v.as_str()).map(|s| s.to_string());
    let instance_value = std::env::var(format!("FORCE_{env_var}"))
        .ok()
        .or_else(|| as_str(stored.get(key)))
        .or_else(|| std::env::var(env_var).ok());
    as_str(workspace_overrides.and_then(|w| w.get(key)))
        .or(instance_value)
        .filter(|v| !v.trim().is_empty())
}

/// Serve the dependency-registry settings to the debug service.
///
/// `windmill prepare-deps` installs a debug session's imports without a database
/// connection, so the service fetches the settings here and passes them down over the
/// CLI's stdin request. They stop there: a private index URL embeds credentials and the
/// debugged script can read its own process, so nothing served here reaches the session's
/// environment (see `debugger/README.md`).
///
/// Authorized by the launch token the service verified for that session, and only when the
/// token carries the `registry_config` claim (see [`sign_debug_request`] for what it means).
async fn get_registry_config(
    Extension(db): Extension<DB>,
    headers: HeaderMap,
) -> JsonResult<DebugRegistryConfig> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Error::NotAuthorized("Missing debug token".to_string()))?;

    let claims = verify_debug_token(token).await?;
    if !claims.registry_config {
        return Err(Error::NotAuthorized(
            "This debug session is not allowed to read the registry configuration".to_string(),
        ));
    }

    let names = REGISTRY_SETTINGS.map(String::from);
    let stored = sqlx::query!(
        "SELECT name, value FROM global_settings WHERE name = ANY($1)",
        &names[..]
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| (r.name, r.value))
    .collect::<std::collections::HashMap<_, _>>();

    let workspace_overrides = stored
        .get(WORKSPACE_REGISTRIES_SETTING)
        .and_then(|v| v.get(&claims.workspace_id));
    let (npm, python) = registry_settings_for_language(&claims.language);
    let resolve = |serve: bool, key: &str, env_var: &str| {
        serve
            .then(|| resolve_registry_setting(&stored, workspace_overrides, key, env_var))
            .flatten()
    };

    let mut config = DebugRegistryConfig {
        npm_config_registry: resolve(npm, NPM_CONFIG_REGISTRY_SETTING, "NPM_CONFIG_REGISTRY"),
        npmrc: resolve(npm, NPMRC_SETTING, "NPMRC"),
        bunfig_install_scopes: resolve(
            npm,
            BUNFIG_INSTALL_SCOPES_SETTING,
            "BUNFIG_INSTALL_SCOPES",
        ),
        pip_index_url: resolve(python, PIP_INDEX_URL_SETTING, "PIP_INDEX_URL"),
        pip_extra_index_url: resolve(python, EXTRA_PIP_INDEX_URL_SETTING, "PIP_EXTRA_INDEX_URL"),
        // Not a private-registry setting: a worker reads it on any edition, so it is
        // served below the Enterprise gate too.
        uv_index_strategy: resolve(python, UV_INDEX_STRATEGY_SETTING, "UV_INDEX_STRATEGY"),
        message: None,
    };

    if cfg!(feature = "enterprise") {
        // A worker substitutes this marker with the output of `EPHEMERAL_TOKEN_CMD`
        // (`handle_ephemeral_token`), a command the debug service has no way to run. Serving
        // the placeholder would install with a literal token, so the value is withheld and
        // the service falls back to the index URL in its own environment.
        let ephemeral = |url: &Option<String>| {
            url.as_ref()
                .is_some_and(|u| u.contains(EPHEMERAL_TOKEN_MARKER))
        };
        if ephemeral(&config.pip_index_url) || ephemeral(&config.pip_extra_index_url) {
            config.pip_index_url = None;
            config.pip_extra_index_url = None;
            config.message = Some(format!(
                "Python index configuration ignored: an {EPHEMERAL_TOKEN_MARKER} index URL can only be resolved on a worker"
            ));
        }
    } else {
        // A private registry is an Enterprise feature, and `read_ee_registry` drops these
        // same settings on a CE worker, so a CE debug session installs from the public registries
        // and says why, instead of gaining a capability jobs on that instance don't have.
        let configured = config.npm_config_registry.is_some()
            || config.npmrc.is_some()
            || config.bunfig_install_scopes.is_some()
            || config.pip_index_url.is_some()
            || config.pip_extra_index_url.is_some();
        config.npm_config_registry = None;
        config.npmrc = None;
        config.bunfig_install_scopes = None;
        config.pip_index_url = None;
        config.pip_extra_index_url = None;
        if configured {
            config.message = Some(
                "Private registry configuration ignored: this feature requires Windmill Enterprise Edition"
                    .to_string(),
            );
        }
    }

    Ok(Json(config))
}

/// Verify a token minted by [`sign_debug_request`] and return its claims.
///
/// The debug service verifies the same token itself against the JWKS public key; this is
/// the server-side half, for the requests the service makes back on a session's behalf.
async fn verify_debug_token(token: &str) -> Result<DebugTokenClaims, Error> {
    let key_guard = DEBUG_SIGNING_KEY.read().await;
    let signing_key = key_guard
        .as_ref()
        .ok_or_else(|| Error::InternalErr("Debug signing key not initialized".to_string()))?;

    let invalid = || Error::NotAuthorized("Invalid debug token".to_string());
    let mut parts = token.split('.');
    let (header_b64, claims_b64, signature_b64) =
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(header), Some(claims), Some(signature), None) => (header, claims, signature),
            _ => return Err(invalid()),
        };

    let signature = Signature::from_slice(
        &URL_SAFE_NO_PAD
            .decode(signature_b64)
            .map_err(|_| invalid())?,
    )
    .map_err(|_| invalid())?;
    signing_key
        .verifying_key()
        .verify_strict(format!("{header_b64}.{claims_b64}").as_bytes(), &signature)
        .map_err(|_| invalid())?;

    let claims: DebugTokenClaims =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims_b64).map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
    if Utc::now().timestamp() > claims.exp {
        return Err(Error::NotAuthorized("Debug token expired".to_string()));
    }
    Ok(claims)
}

#[derive(Deserialize)]
pub struct SignDebugRequest {
    /// The code to be debugged
    pub code: String,
    /// The programming language (python3, bun, typescript, etc.)
    pub language: String,
}

/// JWT claims for debug tokens
#[derive(Serialize, Deserialize)]
pub struct DebugTokenClaims {
    /// Code hash (SHA-256, first 16 bytes, hex encoded)
    pub code_hash: String,
    /// Programming language
    pub language: String,
    /// Workspace ID
    pub workspace_id: String,
    /// User email
    pub email: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Job ID for traceability
    pub job_id: String,
    /// Whether this session may be served the instance's dependency-registry settings
    /// (see [`get_registry_config`]). Defaults to `false` so a token that predates the
    /// claim is refused rather than silently trusted.
    #[serde(default)]
    pub registry_config: bool,
}

#[derive(Serialize)]
pub struct SignedDebugPayload {
    /// JWT token containing the signed claims
    pub token: String,
    /// The code (passed through for convenience)
    pub code: String,
    /// Job ID for the debug session (can be used to view job details)
    pub job_id: String,
}

/// Sign a debug request and create audit log + job entries for full traceability.
///
/// This endpoint must be called before starting a debug session.
/// Returns a JWT that the debugger will verify using the public key from /api/debug/jwks.
///
/// Creates:
/// - A job entry in v2_job (kind=preview) with the debug code
/// - A completed job entry in v2_job_completed (status=success)
/// - An audit log entry identical to "jobs.run.preview"
async fn sign_debug_request(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(request): Json<SignDebugRequest>,
) -> JsonResult<SignedDebugPayload> {
    let key_guard = DEBUG_SIGNING_KEY.read().await;
    let signing_key = key_guard.as_ref().ok_or_else(|| {
        windmill_common::error::Error::InternalErr("Debug signing key not initialized".to_string())
    })?;

    let now = Utc::now();
    let now_ts = now.timestamp();
    let exp = now_ts + DEBUG_TOKEN_TTL_SECS;

    // Parse the language
    let script_lang: ScriptLang = request.language.parse().unwrap_or(ScriptLang::Bun);
    // Taken from the parsed language, not the request's string: the telemetry key vocabulary has
    // to stay the closed set of languages rather than whatever a caller sent.
    let lang_key = script_lang.as_str();

    // Hash the code (we don't include full code in JWT to keep it small)
    let mut hasher = Sha256::new();
    hasher.update(request.code.as_bytes());
    let code_hash = hex::encode(&hasher.finalize()[..16]);

    // Generate job ID
    let job_id = Uuid::new_v4();

    let claims = DebugTokenClaims {
        code_hash,
        language: request.language.clone(),
        workspace_id: w_id.clone(),
        email: authed.email.clone(),
        iat: now_ts,
        exp,
        job_id: job_id.to_string(),
        // The registry settings embed credentials, and the token reaches the browser, so they
        // are only served for a session whose author can already install with them: someone
        // who can run a preview job. For npm that discloses nothing new, since a worker leaves
        // the same `.npmrc` / `bunfig.toml` in the directory the previewed script runs in; the
        // Python index URL only ever appears as uv's argv, so serving it here does widen what
        // a member of the workspace can read. Operators cannot run previews at all, so their
        // sessions install from the public registries.
        registry_config: !authed.is_operator,
    };

    // Create JWT manually with Ed25519 signature
    let header = serde_json::json!({
        "alg": "EdDSA",
        "typ": "JWT"
    });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&claims).unwrap());
    let message = format!("{}.{}", header_b64, claims_b64);

    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    let token = format!("{}.{}", message, signature_b64);

    // Create job entries and audit log in a transaction
    let mut tx = user_db.begin(&authed).await?;

    let tag = "debugger".to_string();
    let permissioned_as = username_to_permissioned_as(&authed.username);

    // Insert into v2_job (the job definition)
    sqlx::query!(
        "INSERT INTO v2_job (
            id,
            workspace_id,
            raw_code,
            tag,
            created_by,
            permissioned_as,
            permissioned_as_email,
            kind,
            script_lang,
            args
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::job_kind, $9::script_lang, $10)",
        job_id,
        w_id,
        request.code,
        tag,
        authed.display_username(),
        permissioned_as,
        authed.email,
        JobKind::Preview as JobKind,
        script_lang as ScriptLang,
        SqlxJson(serde_json::json!({})) as SqlxJson<serde_json::Value>,
    )
    .execute(&mut *tx)
    .await?;

    // Insert into v2_job_completed (mark as immediately completed)
    sqlx::query!(
        "INSERT INTO v2_job_completed (
            id,
            workspace_id,
            started_at,
            completed_at,
            duration_ms,
            result,
            status,
            worker
        ) VALUES ($1, $2, $3, $3, 0, $4, 'success'::job_status, 'debugger')",
        job_id,
        w_id,
        now,
        SqlxJson(serde_json::json!({"debug_session": true, "language": request.language}))
            as SqlxJson<serde_json::Value>,
    )
    .execute(&mut *tx)
    .await?;

    // Create audit log entry (identical to jobs.run.preview)
    audit_log(
        &mut *tx,
        &authed,
        "jobs.run.preview",
        ActionKind::Execute,
        &w_id,
        None,
        Some([("job_id", job_id.to_string().as_str())].into()),
    )
    .await?;

    tx.commit().await?;

    windmill_common::feature_usage::log_feature_usage("debugger", "session", lang_key);

    Ok(Json(SignedDebugPayload {
        token,
        code: request.code,
        job_id: job_id.to_string(),
    }))
}

#[derive(Deserialize)]
pub struct SignExpressionRequest {
    /// The expression to evaluate
    pub expression: String,
    /// The job ID of the parent debug session
    pub job_id: String,
}

/// JWT claims for expression evaluation tokens
#[derive(Serialize, Deserialize)]
pub struct ExpressionTokenClaims {
    /// Expression hash (SHA-256, first 16 bytes, hex encoded)
    pub expression_hash: String,
    /// Parent debug session job ID
    pub job_id: String,
    /// Workspace ID
    pub workspace_id: String,
    /// User email
    pub email: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
}

#[derive(Serialize)]
pub struct SignedExpressionPayload {
    /// JWT token containing the signed claims
    pub token: String,
}

/// Sign a console expression for evaluation and create audit log.
///
/// This endpoint must be called before evaluating an expression in the debug console.
/// Creates an audit log entry with the full expression for traceability.
async fn sign_expression(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(request): Json<SignExpressionRequest>,
) -> JsonResult<SignedExpressionPayload> {
    let key_guard = DEBUG_SIGNING_KEY.read().await;
    let signing_key = key_guard.as_ref().ok_or_else(|| {
        windmill_common::error::Error::InternalErr("Debug signing key not initialized".to_string())
    })?;

    let now = Utc::now();
    let now_ts = now.timestamp();
    let exp = now_ts + DEBUG_TOKEN_TTL_SECS;

    // Hash the expression
    let mut hasher = Sha256::new();
    hasher.update(request.expression.as_bytes());
    let expression_hash = hex::encode(&hasher.finalize()[..16]);

    let claims = ExpressionTokenClaims {
        expression_hash,
        job_id: request.job_id.clone(),
        workspace_id: w_id.clone(),
        email: authed.email.clone(),
        iat: now_ts,
        exp,
    };

    // Create JWT manually with Ed25519 signature
    let header = serde_json::json!({
        "alg": "EdDSA",
        "typ": "JWT"
    });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&claims).unwrap());
    let message = format!("{}.{}", header_b64, claims_b64);

    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    let token = format!("{}.{}", message, signature_b64);

    // Create audit log entry for the expression evaluation
    let mut tx = user_db.begin(&authed).await?;

    // Truncate expression for resource field if too long (max 255 chars)
    let resource = windmill_common::utils::truncate_with_ellipsis(&request.expression, 200);

    audit_log(
        &mut *tx,
        &authed,
        "debug.evaluate",
        ActionKind::Execute,
        &w_id,
        Some(&resource),
        Some(
            [
                ("job_id", request.job_id.as_str()),
                ("expression", request.expression.as_str()),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(SignedExpressionPayload { token }))
}

/// JWT claims for multiplayer session tokens
#[derive(Serialize, Deserialize)]
pub struct MultiplayerTokenClaims {
    /// Workspace ID
    pub workspace_id: String,
    /// User email
    pub email: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Token purpose (always "multiplayer")
    pub purpose: String,
}

#[derive(Serialize)]
pub struct SignedMultiplayerPayload {
    pub token: String,
}

/// Sign a multiplayer session request.
///
/// Returns a JWT that the multiplayer server will verify using the public key from /api/debug/jwks.
async fn sign_multiplayer(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
) -> JsonResult<SignedMultiplayerPayload> {
    let key_guard = DEBUG_SIGNING_KEY.read().await;
    let signing_key = key_guard.as_ref().ok_or_else(|| {
        windmill_common::error::Error::InternalErr("Debug signing key not initialized".to_string())
    })?;

    let now_ts = Utc::now().timestamp();
    let exp = now_ts + DEBUG_TOKEN_TTL_SECS;

    let claims = MultiplayerTokenClaims {
        workspace_id: w_id,
        email: authed.email,
        iat: now_ts,
        exp,
        purpose: "multiplayer".to_string(),
    };

    let header = serde_json::json!({
        "alg": "EdDSA",
        "typ": "JWT"
    });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&claims).unwrap());
    let message = format!("{}.{}", header_b64, claims_b64);

    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    let token = format!("{}.{}", message, signature_b64);

    Ok(Json(SignedMultiplayerPayload { token }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sign `claims` the way [`sign_debug_request`] does, with a key only this test knows.
    async fn signed(claims: &DebugTokenClaims) -> String {
        let key = derive_signing_key_from_jwt_secret("test-secret");
        *DEBUG_SIGNING_KEY.write().await = Some(key.clone());
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"EdDSA","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_string(claims).unwrap());
        let message = format!("{header}.{payload}");
        let signature = URL_SAFE_NO_PAD.encode(key.sign(message.as_bytes()).to_bytes());
        format!("{message}.{signature}")
    }

    fn claims(exp_in: i64) -> DebugTokenClaims {
        DebugTokenClaims {
            code_hash: "0".repeat(32),
            language: "bun".to_string(),
            workspace_id: "test".to_string(),
            email: "user@windmill.dev".to_string(),
            iat: Utc::now().timestamp(),
            exp: Utc::now().timestamp() + exp_in,
            job_id: Uuid::nil().to_string(),
            registry_config: true,
        }
    }

    /// The registry settings are credentials, and this signature is the only thing standing
    /// between them and any caller of `/api/debug/registry_config`. Each rejection here is a
    /// way in if it stops being checked: an edited claim, a session whose author may not read
    /// them, or a token replayed long after its session.
    #[tokio::test]
    async fn only_an_unexpired_token_with_the_claim_verifies() {
        let token = signed(&claims(60)).await;
        assert!(verify_debug_token(&token).await.is_ok());

        let no_claim = signed(&DebugTokenClaims { registry_config: false, ..claims(60) }).await;
        assert!(!verify_debug_token(&no_claim).await.unwrap().registry_config);

        let expired = signed(&claims(-1)).await;
        assert!(verify_debug_token(&expired).await.is_err());

        // Re-signing is the only way to change a claim: swapping the payload of a valid token
        // for one that grants itself the claim must not verify.
        let (header, rest) = token.split_once('.').unwrap();
        let (_, signature) = rest.split_once('.').unwrap();
        let forged = URL_SAFE_NO_PAD.encode(
            serde_json::to_string(&DebugTokenClaims { exp: i64::MAX, ..claims(60) }).unwrap(),
        );
        assert!(
            verify_debug_token(&format!("{header}.{forged}.{signature}"))
                .await
                .is_err()
        );
    }

    /// Every language the debugger can start a session for installs dependencies, so each one
    /// has to name the settings its installer reads. A language missing here is not a refusal:
    /// its sessions quietly install from the public registries instead.
    #[test]
    fn every_debuggable_language_is_served_its_own_settings() {
        // Mirrors `isDebuggableLanguage` in frontend/src/lib/components/debug/debugUtils.ts.
        for language in ["bun", "typescript", "deno", "nativets"] {
            assert_eq!(registry_settings_for_language(language), (true, false));
        }
        assert_eq!(registry_settings_for_language("python3"), (false, true));
        assert_eq!(registry_settings_for_language("go"), (false, false));
    }

    /// A debug session must resolve a registry setting to what a job in the same workspace
    /// resolves it to (`read_ee_registry_with_workspace_override`): the workspace override
    /// replaces the instance value, and a blank value from either source means unset, which
    /// is how a workspace opts out of an instance-wide registry.
    #[test]
    fn workspace_override_replaces_the_instance_value() {
        let stored = [(
            NPM_CONFIG_REGISTRY_SETTING.to_string(),
            serde_json::json!("https://instance.example/"),
        )]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        // Never set, so the environment fallback stays out of the comparison.
        let env_var = "WM_TEST_DEBUG_REGISTRY_UNSET";
        let resolve = |overrides: Option<&serde_json::Value>| {
            resolve_registry_setting(&stored, overrides, NPM_CONFIG_REGISTRY_SETTING, env_var)
        };

        assert_eq!(resolve(None).as_deref(), Some("https://instance.example/"));
        let workspace = serde_json::json!({ "npm_config_registry": "https://workspace.example/" });
        assert_eq!(
            resolve(Some(&workspace)).as_deref(),
            Some("https://workspace.example/")
        );
        let blanked = serde_json::json!({ "npm_config_registry": "  " });
        assert_eq!(resolve(Some(&blanked)), None);
        let unrelated = serde_json::json!({ "npmrc": "//other/:_authToken=x" });
        assert_eq!(
            resolve(Some(&unrelated)).as_deref(),
            Some("https://instance.example/")
        );
    }
}
