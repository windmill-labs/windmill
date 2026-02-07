/*
 * Author: Claude
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache TTL for variables and resources (30seconds)
const CACHE_TTL_SECS: u64 = 30;

/// Cache entry with timestamp and value (following raw script cache pattern)
#[derive(Clone, Debug)]
pub struct CacheEntry<T> {
    pub timestamp: u64,
    pub value: T,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            value,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.timestamp + CACHE_TTL_SECS
    }
}

lazy_static::lazy_static! {
    /// Cache for individual variable values: key = "workspace_id:path"
    pub static ref VARIABLE_CACHE: Cache<String, CacheEntry<String>> = Cache::new(1000);

    /// Cache for resource values: key = "workspace_id:path"
    pub static ref RESOURCE_CACHE: Cache<String, CacheEntry<Value>> = Cache::new(1000);
}

/// Generate cache key for variables and resources
pub fn cache_key(workspace_id: &str, path: &str) -> String {
    format!("{}:{}", workspace_id, path)
}

/// Get cached variable if available and not expired
pub fn get_cached_variable(workspace_id: &str, path: &str) -> Option<String> {
    let key = cache_key(workspace_id, path);
    VARIABLE_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            VARIABLE_CACHE.remove(&key);
            None
        } else {
            tracing::debug!("Cache hit for variable {}", key);
            Some(entry.value.clone())
        }
    })
}

/// Cache variable data
pub fn cache_variable(workspace_id: &str, path: &str, email: &str, variable: String) {
    let key = format!("{}:{}", email, cache_key(workspace_id, path));
    let entry = CacheEntry::new(variable);
    VARIABLE_CACHE.insert(key.clone(), entry);
    tracing::debug!("Cached variable {}", key);
}

/// Get cached resource if available and not expired
pub fn get_cached_resource(workspace_id: &str, path: &str) -> Option<Value> {
    let key = cache_key(workspace_id, path);
    RESOURCE_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            RESOURCE_CACHE.remove(&key);
            None
        } else {
            tracing::debug!("Cache hit for resource {}", key);
            Some(entry.value.clone())
        }
    })
}

/// Cache resource data
pub fn cache_resource(workspace_id: &str, path: &str, resource: Value) {
    let key = cache_key(workspace_id, path);
    let entry = CacheEntry::new(resource);
    RESOURCE_CACHE.insert(key.clone(), entry);
    tracing::debug!("Cached resource {}", key);
}

/// Invalidate specific variable from cache
pub fn invalidate_variable_cache(workspace_id: &str, path: &str) {
    let key = cache_key(workspace_id, path);
    VARIABLE_CACHE.remove(&key);
    tracing::info!("Variable cache invalidated for {}", key);
}

/// Invalidate specific resource from cache
pub fn invalidate_resource_cache(workspace_id: &str, path: &str) {
    let key = cache_key(workspace_id, path);
    RESOURCE_CACHE.remove(&key);
    tracing::info!("Resource cache invalidated for {}", key);
}

/// Clear all caches (for testing/debugging)
#[allow(dead_code)]
pub fn clear_all_caches() {
    VARIABLE_CACHE.clear();
    RESOURCE_CACHE.clear();
    tracing::debug!("All variable/resource caches cleared");
}
