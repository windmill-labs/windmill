use chrono::Utc;
use dashmap::DashMap;
use hyper::StatusCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

use crate::error::{Error, Result};
use crate::worker::CLOUD_HOSTED;

const DEFAULT_PER_IP_LIMIT: i32 = 30;
const DEFAULT_PER_ACCOUNT_LIMIT: i32 = 10;
const DEFAULT_GLOBAL_LIMIT: i32 = 120;
const EVICTION_INTERVAL: u64 = 256;

struct RateLimitEntry {
    count: i32,
    minute_bucket: i64,
}

static IP_RATE_LIMIT: LazyLock<DashMap<String, RateLimitEntry>> = LazyLock::new(DashMap::new);
static ACCOUNT_RATE_LIMIT: LazyLock<DashMap<String, RateLimitEntry>> = LazyLock::new(DashMap::new);
static GLOBAL_RATE_LIMIT: LazyLock<DashMap<String, RateLimitEntry>> = LazyLock::new(DashMap::new);

static EVICTION_COUNTER: AtomicU64 = AtomicU64::new(0);

static PER_IP_LIMIT: LazyLock<i32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_IP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PER_IP_LIMIT)
});

static PER_IP_LIMIT_EXPLICIT: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_IP")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .is_some()
});

static PER_ACCOUNT_LIMIT: LazyLock<i32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_ACCOUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PER_ACCOUNT_LIMIT)
});

static PER_ACCOUNT_LIMIT_EXPLICIT: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_ACCOUNT")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
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

fn maybe_evict(maps: &[&DashMap<String, RateLimitEntry>], current_minute: i64) {
    let count = EVICTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    if count % EVICTION_INTERVAL == 0 {
        for map in maps {
            map.retain(|_, v| v.minute_bucket >= current_minute - 1);
        }
    }
}

/// Atomically check the rate limit and increment the counter. Follows the
/// `public_app_rate_limit.rs` pattern — the DashMap entry lock is held across
/// both the check and the increment, preventing TOCTOU races.
fn check_and_increment(
    map: &DashMap<String, RateLimitEntry>,
    key: &str,
    limit: i32,
    current_minute: i64,
) -> Result<()> {
    let mut entry = map
        .entry(key.to_string())
        .or_insert(RateLimitEntry { count: 0, minute_bucket: current_minute });

    if entry.minute_bucket != current_minute {
        entry.count = 0;
        entry.minute_bucket = current_minute;
    }

    if entry.count >= limit {
        return Err(Error::Generic(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }

    entry.count += 1;
    Ok(())
}

fn record_failure(map: &DashMap<String, RateLimitEntry>, key: &str) {
    let current_minute = Utc::now().timestamp() / 60;

    let mut entry = map
        .entry(key.to_string())
        .or_insert(RateLimitEntry { count: 0, minute_bucket: current_minute });

    if entry.minute_bucket != current_minute {
        entry.count = 1;
        entry.minute_bucket = current_minute;
    } else {
        entry.count += 1;
    }
}

/// Called BEFORE authentication. Checks and increments global + per-IP counters.
/// Per-IP is only active on CLOUD_HOSTED or when LOGIN_RATE_LIMIT_PER_IP is explicitly set.
pub fn check_and_increment_login_attempt(ip: Option<&str>, email: &str) -> Result<()> {
    let current_minute = Utc::now().timestamp() / 60;
    maybe_evict(
        &[&IP_RATE_LIMIT, &ACCOUNT_RATE_LIMIT, &GLOBAL_RATE_LIMIT],
        current_minute,
    );

    // Global limit: always on
    check_and_increment(&GLOBAL_RATE_LIMIT, "global", *GLOBAL_LIMIT, current_minute)?;

    // Per-IP limit: CLOUD_HOSTED or explicit opt-in
    if *CLOUD_HOSTED || *PER_IP_LIMIT_EXPLICIT {
        if let Some(ip) = ip {
            check_and_increment(&IP_RATE_LIMIT, ip, *PER_IP_LIMIT, current_minute)?;
        }
    }

    // Per-account check (read-only, does not increment — failures are recorded separately)
    if *CLOUD_HOSTED || *PER_ACCOUNT_LIMIT_EXPLICIT {
        let entry = ACCOUNT_RATE_LIMIT.get(email);
        if let Some(entry) = entry {
            if entry.minute_bucket == current_minute && entry.count >= *PER_ACCOUNT_LIMIT {
                return Err(Error::Generic(
                    StatusCode::TOO_MANY_REQUESTS,
                    "Too many login attempts. Please try again later.".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Called AFTER authentication failure. Records per-account failure.
/// Per-account is only active on CLOUD_HOSTED or when LOGIN_RATE_LIMIT_PER_ACCOUNT is explicitly set.
pub fn record_login_failure(email: &str) {
    if *CLOUD_HOSTED || *PER_ACCOUNT_LIMIT_EXPLICIT {
        record_failure(&ACCOUNT_RATE_LIMIT, email);
    }
}
