use chrono::Utc;
use hyper::StatusCode;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};
use std::sync::LazyLock;

use crate::error::{Error, Result};
use crate::per_minute_counter::PerMinuteCounter;
use crate::worker::CLOUD_HOSTED;

const DEFAULT_PER_IP_LIMIT: u32 = 120;
const DEFAULT_PER_ACCOUNT_LIMIT: u32 = 30;
const DEFAULT_GLOBAL_LIMIT: i32 = 10000;

static IP_RATE_LIMIT: LazyLock<PerMinuteCounter<String>> = LazyLock::new(PerMinuteCounter::new);
static ACCOUNT_RATE_LIMIT: LazyLock<PerMinuteCounter<String>> =
    LazyLock::new(PerMinuteCounter::new);

static GLOBAL_COUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_MINUTE: AtomicI64 = AtomicI64::new(0);

static PER_IP_LIMIT: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_IP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PER_IP_LIMIT)
});

static PER_IP_LIMIT_EXPLICIT: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_IP")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .is_some()
});

static PER_ACCOUNT_LIMIT: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_ACCOUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PER_ACCOUNT_LIMIT)
});

static PER_ACCOUNT_LIMIT_EXPLICIT: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_ACCOUNT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .is_some()
});

static GLOBAL_LIMIT: LazyLock<i32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_GLOBAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_GLOBAL_LIMIT)
});

/// Extract client IP from proxy headers. Only meaningful when behind a trusted
/// reverse proxy (e.g. CLOUD_HOSTED). Returns `None` if no proxy header is present.
pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            let trimmed = ip.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(ips) = forwarded_for.to_str() {
            if let Some(first_ip) = ips.split(',').next() {
                let trimmed = first_ip.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    None
}

fn too_many_attempts() -> Error {
    Error::Generic(
        StatusCode::TOO_MANY_REQUESTS,
        "Too many login attempts. Please try again later.".to_string(),
    )
}

/// Called BEFORE authentication. Checks and increments global + per-IP counters.
/// The global counter counts all login attempts (not just failures), so it acts as
/// a general throttle on login traffic per server instance.
/// Per-IP is only active on CLOUD_HOSTED or when LOGIN_RATE_LIMIT_PER_IP is explicitly set.
pub fn check_and_increment_login_attempt(
    headers: &axum::http::HeaderMap,
    email: &str,
) -> Result<()> {
    // Global limit: always on, uses atomics (single key, no need for a map)
    check_and_increment_global()?;

    // Per-IP limit: CLOUD_HOSTED or explicit opt-in
    if *CLOUD_HOSTED || *PER_IP_LIMIT_EXPLICIT {
        if let Some(ip) = extract_client_ip(headers) {
            if !IP_RATE_LIMIT.try_increment(ip, *PER_IP_LIMIT) {
                return Err(too_many_attempts());
            }
        }
    }

    // Per-account check (read-only, does not increment — failures are recorded separately)
    if *CLOUD_HOSTED || *PER_ACCOUNT_LIMIT_EXPLICIT {
        if ACCOUNT_RATE_LIMIT.count(email) >= *PER_ACCOUNT_LIMIT {
            return Err(too_many_attempts());
        }
    }

    Ok(())
}

fn check_and_increment_global() -> Result<()> {
    let current_minute = Utc::now().timestamp() / 60;
    let stored_minute = GLOBAL_MINUTE.load(Ordering::Relaxed);
    if stored_minute != current_minute {
        // Minute rolled over — reset. Race here is benign: worst case two threads
        // both reset, and we lose a few counts at the boundary.
        GLOBAL_MINUTE.store(current_minute, Ordering::Relaxed);
        GLOBAL_COUNT.store(1, Ordering::Relaxed);
        return Ok(());
    }

    let count = GLOBAL_COUNT.fetch_add(1, Ordering::Relaxed);
    if count >= *GLOBAL_LIMIT {
        return Err(too_many_attempts());
    }

    Ok(())
}

/// Called AFTER authentication failure. Records per-account failure.
/// Per-account is only active on CLOUD_HOSTED or when LOGIN_RATE_LIMIT_PER_ACCOUNT is explicitly set.
pub fn record_login_failure(email: &str) {
    if *CLOUD_HOSTED || *PER_ACCOUNT_LIMIT_EXPLICIT {
        ACCOUNT_RATE_LIMIT.increment(email.to_string());
    }
}
