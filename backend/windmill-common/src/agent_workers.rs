use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct QueueInitJob {
    pub content: String,
}

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use sha2::{Digest, Sha256};

use crate::{jwt::decode_without_verify, worker::HttpClient, DB};

lazy_static! {
    pub static ref BASE_INTERNAL_URL: String =
        std::env::var("BASE_INTERNAL_URL").unwrap_or("http://localhost:8080".to_string());
    pub static ref AGENT_TOKEN: String = std::env::var("AGENT_TOKEN").unwrap_or_default();
    pub static ref DECODED_AGENT_TOKEN: Option<AgentAuth> = {
        if AGENT_TOKEN.is_empty() {
            None
        } else {
            decode_without_verify::<AgentAuth>(AGENT_TOKEN.trim_start_matches(AGENT_JWT_PREFIX))
                .ok()
        }
    };
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

pub const AGENT_JWT_PREFIX: &str = "jwt_agent_";

pub fn build_agent_http_client(worker_suffix: &str) -> HttpClient {
    let client = ClientBuilder::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "User-Agent",                          // Replace with your desired header name
                    "Windmill-Agent/1.0".parse().unwrap(), // Replace with your desired header value
                );
                let token = format!(
                    "{}{}_{}",
                    AGENT_JWT_PREFIX,
                    worker_suffix,
                    AGENT_TOKEN.trim_start_matches(AGENT_JWT_PREFIX),
                );
                headers.insert(
                    "Authorization",
                    format!("Bearer {}", token).parse().unwrap(),
                );
                headers
            })
            .build()
            .expect("Failed to create HTTP client"),
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(5),
    ))
    .build();
    HttpClient(client)
}

#[derive(Deserialize, Serialize)]
pub struct PingJobStatus {
    pub mem_peak: Option<i32>,
    pub current_mem: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PingJobStatusResponse {
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub already_completed: bool,
}

// #[derive(Serialize, Deserialize)]
// pub struct PullJobRequest {
//     pub worker_name: String,
// }

// Agent token blacklist functionality

#[derive(Debug, Clone)]
struct BlacklistCacheEntry {
    is_blacklisted: bool,
    expires_at: Instant,
}

lazy_static! {
    static ref BLACKLIST_CACHE: Mutex<HashMap<String, BlacklistCacheEntry>> =
        Mutex::new(HashMap::new());
}

const BLACKLIST_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub async fn is_token_blacklisted(db: &DB, token: &str) -> Result<bool, sqlx::Error> {
    let token_hash = hash_token(token);

    // Check cache first
    {
        let mut cache = BLACKLIST_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&token_hash) {
            if entry.expires_at > Instant::now() {
                return Ok(entry.is_blacklisted);
            }
            // Remove expired entry
            cache.remove(&token_hash);
        }
    }

    // Query database for blacklist status
    let now = Utc::now().naive_utc();
    let is_blacklisted = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM agent_token_blacklist WHERE token_hash = $1 AND expires_at > $2)",
        token_hash,
        now
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    // Update cache
    {
        let mut cache = BLACKLIST_CACHE.lock().unwrap();
        cache.insert(
            token_hash,
            BlacklistCacheEntry {
                is_blacklisted,
                expires_at: Instant::now() + BLACKLIST_CACHE_TTL,
            },
        );

        // Clean up expired entries to prevent memory leak
        cache.retain(|_, entry| entry.expires_at > Instant::now());
    }

    Ok(is_blacklisted)
}

pub async fn blacklist_token(
    db: &DB,
    token: &str,
    expires_at: DateTime<Utc>,
    blacklisted_by: &str,
) -> Result<(), sqlx::Error> {
    let token_hash = hash_token(token);

    sqlx::query!(
        "INSERT INTO agent_token_blacklist (token_hash, expires_at, blacklisted_by) 
         VALUES ($1, $2, $3) 
         ON CONFLICT (token_hash) DO UPDATE SET 
            expires_at = EXCLUDED.expires_at,
            blacklisted_at = NOW(),
            blacklisted_by = EXCLUDED.blacklisted_by",
        token_hash,
        expires_at.naive_utc(),
        blacklisted_by
    )
    .execute(db)
    .await?;

    // Invalidate cache entry
    {
        let mut cache = BLACKLIST_CACHE.lock().unwrap();
        cache.remove(&token_hash);
    }

    Ok(())
}

pub async fn blacklist_token_with_optional_expiry(
    db: &DB,
    token: &str,
    expires_at: Option<DateTime<Utc>>,
    blacklisted_by: &str,
) -> Result<(), sqlx::Error> {
    // Determine the expiration time
    let final_expires_at = match expires_at {
        Some(expiry) => expiry,
        None => {
            // Try to extract expiration from JWT token
            match extract_jwt_expiration(token) {
                Some(jwt_expiry) => jwt_expiry,
                None => {
                    // If we can't extract expiration, use a default (e.g., 1 year from now)
                    Utc::now() + chrono::Duration::days(365)
                }
            }
        }
    };

    // Use the existing blacklist_token function
    blacklist_token(db, token, final_expires_at, blacklisted_by).await
}

pub async fn remove_token_from_blacklist(db: &DB, token: &str) -> Result<bool, sqlx::Error> {
    let token_hash = hash_token(token);

    let result = sqlx::query!(
        "DELETE FROM agent_token_blacklist WHERE token_hash = $1",
        token_hash
    )
    .execute(db)
    .await?;

    // Invalidate cache entry
    {
        let mut cache = BLACKLIST_CACHE.lock().unwrap();
        cache.remove(&token_hash);
    }

    Ok(result.rows_affected() > 0)
}

pub fn extract_jwt_expiration(token: &str) -> Option<DateTime<Utc>> {
    // Remove prefix if present
    let clean_token = token.trim_start_matches(AGENT_JWT_PREFIX);

    // Try to decode the JWT and extract exp claim
    if let Ok(claims) = decode_without_verify::<serde_json::Value>(clean_token) {
        if let Some(exp_value) = claims.get("exp") {
            if let Some(exp_timestamp) = exp_value.as_u64() {
                // Convert Unix timestamp to DateTime<Utc>
                if let Some(datetime) = DateTime::from_timestamp(exp_timestamp as i64, 0) {
                    return Some(datetime);
                }
            }
        }
    }

    None
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlacklistTokenRequest {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}
