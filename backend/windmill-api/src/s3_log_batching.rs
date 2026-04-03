// Ducklake issues many rapid, identical S3 proxy requests (same object, method, status)
// within milliseconds, flooding the logs. This module debounces those logs:
// - The first request in a batch logs immediately.
// - Subsequent identical requests within 500ms are silently counted.
// - When 500ms pass with no new request (or 5s max), a single summary line is emitted
//   with the total count and average latency, then the batch is cleared.
// A background task (spawned in lib.rs) calls flush_s3_batches() every 250ms to emit
// and clean up expired batches.

use axum::{extract::Request, middleware::Next, response::Response};
use dashmap::DashMap;
use std::time::Instant;

lazy_static::lazy_static! {
    static ref S3_LOG_BATCHES: DashMap<S3ProxyKey, BatchState> = DashMap::new();
}

#[derive(Clone)]
pub(crate) struct S3ProxyRequest {
    pub uri: String,
    pub method: String,
}

#[derive(Hash, Eq, PartialEq)]
struct S3ProxyKey {
    workspace: String,
    object_key: String,
    method: String,
    status: u16,
}

struct BatchState {
    total_count: u64,
    total_latency_ms: u128,
    first_seen: Instant,
    last_seen: Instant,
    uri: String,
}

// URI format: /api/w/{workspace}/s3_proxy/{object_key}
fn extract_workspace_and_object_key(uri: &str) -> (&str, &str) {
    // Extract workspace from /api/w/{workspace}/...
    let workspace = uri
        .strip_prefix("/api/w/")
        .and_then(|s| s.split('/').next())
        .unwrap_or("");
    let object_key = uri
        .find("/s3_proxy/")
        .map(|i| &uri[i + "/s3_proxy/".len()..])
        .unwrap_or(uri);
    (workspace, object_key)
}

pub(crate) fn record_s3_log(uri: &str, method: &str, status: u16, latency_ms: u128) {
    let (workspace, object_key) = extract_workspace_and_object_key(uri);
    let key = S3ProxyKey {
        workspace: workspace.to_string(),
        object_key: object_key.to_string(),
        method: method.to_string(),
        status,
    };

    let mut entry = S3_LOG_BATCHES.entry(key).or_insert_with(|| BatchState {
        total_count: 0,
        total_latency_ms: 0,
        first_seen: Instant::now(),
        last_seen: Instant::now(),
        uri: uri.to_string(),
    });

    entry.total_count += 1;
    entry.total_latency_ms += latency_ms;
    entry.last_seen = Instant::now();

    if entry.total_count == 1 {
        tracing::info!(latency = latency_ms, status = status, "response")
    }
}

pub fn flush_s3_batches() {
    let now = Instant::now();
    S3_LOG_BATCHES.retain(|key, batch| {
        let idle = now.duration_since(batch.last_seen).as_millis() > 500;
        let max_age = now.duration_since(batch.first_seen).as_secs() > 5;

        if idle || max_age {
            if batch.total_count > 1 {
                let avg_latency = batch.total_latency_ms / batch.total_count as u128;
                tracing::info!(
                    count = batch.total_count,
                    avg_latency = avg_latency,
                    status = key.status,
                    method = %key.method,
                    uri = %batch.uri,
                    "s3_proxy batched response"
                );
            }
            false
        } else {
            true
        }
    });
}

pub async fn s3_proxy_log_middleware(request: Request, next: Next) -> Response {
    let s3_info = if request.uri().path().contains("/s3_proxy/") {
        Some(S3ProxyRequest {
            uri: request.uri().to_string(),
            method: request.method().to_string(),
        })
    } else {
        None
    };
    let mut response = next.run(request).await;
    if let Some(info) = s3_info {
        response.extensions_mut().insert(info);
    }
    response
}
