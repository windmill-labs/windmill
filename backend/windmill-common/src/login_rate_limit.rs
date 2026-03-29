use chrono::Utc;
use dashmap::DashMap;
use hyper::StatusCode;
use std::sync::LazyLock;

use crate::error::{Error, Result};

const DEFAULT_PER_IP_LIMIT: i32 = 10;
const DEFAULT_PER_ACCOUNT_LIMIT: i32 = 5;

struct RateLimitEntry {
    count: i32,
    minute_bucket: i64,
}

static IP_RATE_LIMIT: LazyLock<DashMap<String, RateLimitEntry>> = LazyLock::new(DashMap::new);
static ACCOUNT_RATE_LIMIT: LazyLock<DashMap<String, RateLimitEntry>> = LazyLock::new(DashMap::new);

static PER_IP_LIMIT: LazyLock<i32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_IP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PER_IP_LIMIT)
});

static PER_ACCOUNT_LIMIT: LazyLock<i32> = LazyLock::new(|| {
    std::env::var("LOGIN_RATE_LIMIT_PER_ACCOUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PER_ACCOUNT_LIMIT)
});

pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> String {
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            let trimmed = ip.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(ips) = forwarded_for.to_str() {
            if let Some(first_ip) = ips.split(',').next() {
                let trimmed = first_ip.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    "unknown".to_string()
}

fn check_rate_limit(map: &DashMap<String, RateLimitEntry>, key: &str, limit: i32) -> Result<()> {
    let current_minute = Utc::now().timestamp() / 60;

    let entry = map
        .entry(key.to_string())
        .or_insert(RateLimitEntry { count: 0, minute_bucket: current_minute });

    if entry.minute_bucket != current_minute {
        return Ok(());
    }

    if entry.count >= limit {
        return Err(Error::Generic(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }

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

pub fn check_login_rate_limit(ip: &str, email: &str) -> Result<()> {
    check_rate_limit(&IP_RATE_LIMIT, ip, *PER_IP_LIMIT)?;
    check_rate_limit(&ACCOUNT_RATE_LIMIT, email, *PER_ACCOUNT_LIMIT)?;
    Ok(())
}

pub fn record_login_failure(ip: &str, email: &str) {
    record_failure(&IP_RATE_LIMIT, ip);
    record_failure(&ACCOUNT_RATE_LIMIT, email);
}
