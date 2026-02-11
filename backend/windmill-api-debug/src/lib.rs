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

use axum::{
    extract::Path,
    routing::{get, post},
    Extension, Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::types::Json as SqlxJson;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB, error::JsonResult, jobs::JobKind, scripts::ScriptLang,
    users::username_to_permissioned_as,
};

use windmill_api_auth::ApiAuthed;

/// TTL for debug tokens in seconds (60 seconds)
pub const DEBUG_TOKEN_TTL_SECS: i64 = 60;

lazy_static::lazy_static! {
    /// Ed25519 signing key for debug tokens.
    /// Generated at startup if not provided via environment variable.
    static ref DEBUG_SIGNING_KEY: Arc<RwLock<Option<SigningKey>>> = Arc::new(RwLock::new(None));
}

/// Initialize the debug signing key.
/// Call this at server startup.
pub async fn init_debug_signing_key() {
    let mut key_guard = DEBUG_SIGNING_KEY.write().await;

    // Check if key is provided via environment variable (base64-encoded seed)
    if let Ok(seed_b64) = std::env::var("DEBUG_SIGNING_KEY_SEED") {
        if let Ok(seed_bytes) = URL_SAFE_NO_PAD.decode(&seed_b64) {
            if seed_bytes.len() >= 32 {
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&seed_bytes[..32]);
                *key_guard = Some(SigningKey::from_bytes(&seed));
                tracing::info!("Debug signing key loaded from environment");
                return;
            }
        }
        tracing::warn!("Invalid DEBUG_SIGNING_KEY_SEED, generating new key");
    }

    // Generate a new random key using rand
    let mut seed = [0u8; 32];
    rand::Rng::fill(&mut rand::rng(), &mut seed);
    let signing_key = SigningKey::from_bytes(&seed);
    tracing::info!("Generated new debug signing key");
    *key_guard = Some(signing_key);
}

pub fn global_service() -> Router {
    Router::new().route("/jwks", get(get_jwks))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/sign", post(sign_debug_request))
        .route("/sign_expression", post(sign_expression))
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
    let resource = if request.expression.len() > 200 {
        format!("{}...", &request.expression[..200])
    } else {
        request.expression.clone()
    };

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
