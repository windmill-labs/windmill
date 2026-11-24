/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Native TOTP (RFC 6238) second factor for password logins.

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use hmac::{Hmac, Mac};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use windmill_api_auth::{is_effectively_unscoped, require_super_admin, ApiAuthed};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::per_minute_counter::PerMinuteCounter;
use windmill_common::{
    error::{Error, JsonResult, Result},
    DB,
};

const STEP_SECONDS: u64 = 30;
const SECRET_BYTES: usize = 20;
const B32: base32::Alphabet = base32::Alphabet::Rfc4648 { padding: false };
// Counted per server process: N API replicas allow N times this many guesses a minute.
const MAX_FAILED_CODES_PER_MINUTE: u32 = 5;
static FAILED_CODES: LazyLock<PerMinuteCounter<String>> = LazyLock::new(PerMinuteCounter::new);

pub const TOTP_REQUIRED: &str = "Two-factor code required";
pub const TOTP_INVALID: &str = "Invalid two-factor code";

pub fn service() -> Router {
    Router::new()
        .route("/", get(status))
        .route("/setup", post(setup))
        .route("/enable", post(enable))
        .route("/disable", post(disable))
        .route("/reset/{user}", post(reset))
}

fn hotp(key: &[u8], counter: u64) -> u32 {
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(&counter.to_be_bytes());
    let h = mac.finalize().into_bytes();
    let o = (h[19] & 0x0f) as usize;
    u32::from_be_bytes([h[o] & 0x7f, h[o + 1], h[o + 2], h[o + 3]]) % 1_000_000
}

/// The time step `code` is valid for, allowing one step of clock drift either way.
fn matching_step(secret: &str, code: &str, now_s: u64) -> Option<i64> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let code: u32 = code.parse().ok()?;
    let key = base32::decode(B32, secret)?;
    let cur = now_s / STEP_SECONDS;
    // Every candidate is computed so the response time does not tell which one matched.
    [cur.saturating_sub(1), cur, cur + 1]
        .into_iter()
        .filter(|s| hotp(&key, *s) == code)
        .last()
        .map(|s| s as i64)
}

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Checks `code` against `secret` and consumes its time step, so a code that was already
/// accepted (by a login, enable or disable) cannot be replayed. The `UPDATE` row lock
/// serializes concurrent attempts with the same code.
///
/// Proves possession of `secret` only. The caller must already have authenticated `email`
/// (its password, or its own session) and read `secret` from that account's row.
pub(crate) async fn verify_and_consume<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    email: &str,
    secret: &str,
    code: &str,
) -> Result<bool> {
    // Always on, unlike the login limiter's per-account tier: a 6-digit code is the whole
    // second factor once the password is known.
    if FAILED_CODES.count(email) >= MAX_FAILED_CODES_PER_MINUTE {
        return Err(Error::Generic(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }
    let Some(step) = matching_step(secret, code, now_s()) else {
        FAILED_CODES.increment(email.to_string());
        return Ok(false);
    };
    let consumed = sqlx::query!(
        "UPDATE password SET totp_last_step = $2
         WHERE email = $1 AND (totp_last_step IS NULL OR totp_last_step < $2)",
        email,
        step
    )
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if consumed == 0 {
        FAILED_CODES.increment(email.to_string());
    }
    Ok(consumed == 1)
}

/// Enrolling or removing the second factor of the account is for its owner's own session:
/// a job or scoped token could otherwise lock the account or strip its second factor.
fn require_own_credentials(authed: &ApiAuthed) -> Result<()> {
    if authed.job_id.is_some()
        || authed.read_only
        || !is_effectively_unscoped(authed.scopes.as_deref())
    {
        return Err(Error::PermissionDenied(
            "Two-factor settings require your own session or an unscoped token".to_string(),
        ));
    }
    Ok(())
}

#[derive(Serialize)]
struct TotpStatus {
    enabled: bool,
}

async fn status(Extension(db): Extension<DB>, authed: ApiAuthed) -> JsonResult<TotpStatus> {
    let enabled = sqlx::query_scalar!(
        "SELECT totp_secret IS NOT NULL AS \"enabled!\" FROM password WHERE email = $1",
        &authed.email
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(TotpStatus { enabled }))
}

#[derive(Serialize)]
struct TotpSetup {
    secret: String,
    otpauth_url: String,
}

async fn setup(Extension(db): Extension<DB>, authed: ApiAuthed) -> JsonResult<TotpSetup> {
    require_own_credentials(&authed)?;
    let secret = base32::encode(B32, &rand::random::<[u8; SECRET_BYTES]>());
    // Refused once enrolled: replacing the secret must go through `disable`, which asks for
    // a code from the current authenticator.
    let updated = sqlx::query!(
        "UPDATE password SET totp_pending_secret = $2
         WHERE email = $1 AND login_type = 'password' AND totp_secret IS NULL",
        &authed.email,
        &secret
    )
    .execute(&db)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(Error::BadRequest(
            "Two-factor authentication is only available to password accounts that do not \
             have it enabled yet"
                .to_string(),
        ));
    }
    let label: String =
        url::form_urlencoded::byte_serialize(format!("Windmill:{}", authed.email).as_bytes())
            .collect();
    let otpauth_url = format!(
        "otpauth://totp/{label}?secret={secret}&issuer=Windmill&algorithm=SHA1&digits=6&period=30"
    );
    Ok(Json(TotpSetup { secret, otpauth_url }))
}

#[derive(Deserialize)]
struct TotpCode {
    code: String,
}

async fn enable(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(TotpCode { code }): Json<TotpCode>,
) -> Result<String> {
    require_own_credentials(&authed)?;
    let mut tx = db.begin().await?;
    let pending = sqlx::query_scalar!(
        "SELECT totp_pending_secret FROM password
         WHERE email = $1 AND login_type = 'password' AND totp_secret IS NULL FOR UPDATE",
        &authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten()
    .ok_or_else(|| Error::BadRequest("Start the two-factor setup first".to_string()))?;
    if !verify_and_consume(&mut tx, &authed.email, &pending, &code).await? {
        return Err(Error::BadRequest(TOTP_INVALID.to_string()));
    }
    sqlx::query!(
        "UPDATE password SET totp_secret = totp_pending_secret, totp_pending_secret = NULL
         WHERE email = $1",
        &authed.email
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "users.totp.enable",
        ActionKind::Update,
        "global",
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok("Two-factor authentication enabled".to_string())
}

async fn disable(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(TotpCode { code }): Json<TotpCode>,
) -> Result<String> {
    require_own_credentials(&authed)?;
    let mut tx = db.begin().await?;
    let secret = sqlx::query_scalar!(
        "SELECT totp_secret FROM password WHERE email = $1 FOR UPDATE",
        &authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten()
    .ok_or_else(|| Error::BadRequest("Two-factor authentication is not enabled".to_string()))?;
    if !verify_and_consume(&mut tx, &authed.email, &secret, &code).await? {
        return Err(Error::BadRequest(TOTP_INVALID.to_string()));
    }
    clear(&mut tx, &authed.email).await?;
    audit_log(
        &mut *tx,
        &authed,
        "users.totp.disable",
        ActionKind::Delete,
        "global",
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok("Two-factor authentication disabled".to_string())
}

/// Recovery for a user who lost their authenticator: a superadmin removes the second factor.
async fn reset(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(email): Path<String>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    let email = email.to_lowercase();
    let mut tx = db.begin().await?;
    if clear(&mut tx, &email).await? == 0 {
        return Err(Error::NotFound(format!("user {email} not found")));
    }
    audit_log(
        &mut *tx,
        &authed,
        "users.totp.reset",
        ActionKind::Delete,
        "global",
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Two-factor authentication of {email} removed"))
}

async fn clear<'c>(tx: &mut sqlx::Transaction<'c, sqlx::Postgres>, email: &str) -> Result<u64> {
    Ok(sqlx::query!(
        "UPDATE password SET totp_secret = NULL, totp_pending_secret = NULL, totp_last_step = NULL
         WHERE email = $1",
        email
    )
    .execute(&mut **tx)
    .await?
    .rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_sha1_vectors() {
        // RFC 6238 appendix B, truncated to 6 digits.
        let secret = base32::encode(B32, b"12345678901234567890");
        for (t, code) in [
            (59, "287082"),
            (1111111109, "081804"),
            (2000000000, "279037"),
        ] {
            assert_eq!(matching_step(&secret, code, t), Some((t / 30) as i64));
        }
        // one step of drift is accepted, two are not
        assert!(matching_step(&secret, "287082", 59 + 30).is_some());
        assert!(matching_step(&secret, "287082", 59 + 60).is_none());
        assert!(matching_step(&secret, "287 082", 59).is_some());
        assert!(matching_step(&secret, "28708", 59).is_none());
    }
}
