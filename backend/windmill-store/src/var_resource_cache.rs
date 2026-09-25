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

/// A cached variable value plus whether it is a secret. `is_secret` is retained so a
/// cache hit can re-run the per-read side effects of a secret read (the
/// `variables.decrypt_secret` audit and running-job secret registration) that the
/// original miss performed — a hit must be observably equivalent to a miss.
#[derive(Clone, Debug)]
pub struct CachedVariable {
    pub value: String,
    pub is_secret: bool,
}

lazy_static::lazy_static! {
    /// Cache for individual variable values. Key: [`identity_cache_key`]
    /// (`identity:workspace_id:path`) — scoped to the caller's authorization context.
    pub static ref VARIABLE_CACHE: Cache<String, CacheEntry<CachedVariable>> = Cache::new(1000);

    /// Cache for interpolated resource values. Key: [`identity_cache_key`]
    /// (`identity:workspace_id:path`) — scoped to the caller's authorization context.
    pub static ref RESOURCE_CACHE: Cache<String, CacheEntry<Value>> = Cache::new(1000);
}

/// Generate cache key for variables and resources
pub fn cache_key(workspace_id: &str, path: &str) -> String {
    format!("{}:{}", workspace_id, path)
}

// Canonical home is `windmill_common::db`, shared with every other cache that
// short-circuits an RLS query; re-exported here for this module's callers.
pub use windmill_common::db::auth_identity;

/// Generate an identity-scoped cache key (`identity:workspace_id:path`).
///
/// Both the variable and resource caches store *already-decrypted* values that were
/// resolved under the caller's row-level-security context. The cache is consulted before
/// the per-folder RLS query runs, so an unscoped `workspace:path` key would let an entry
/// warmed by one caller (via `allow_cache=true`) be served to a different caller who has
/// no access to the underlying folder, leaking decrypted secrets within the TTL. `identity`
/// is [`auth_identity`] — the hash of the caller's full authorization context — so a hit
/// can only ever be returned to a caller whose authorized read populated it.
fn identity_cache_key(identity: &str, workspace_id: &str, path: &str) -> String {
    format!("{}:{}", identity, cache_key(workspace_id, path))
}

/// Get cached variable if available and not expired. Scoped to `identity`
/// ([`auth_identity`]); see [`identity_cache_key`]. Returns the value and its `is_secret`
/// flag so the caller can re-run a secret read's side effects on a hit.
pub fn get_cached_variable(
    workspace_id: &str,
    path: &str,
    identity: &str,
) -> Option<CachedVariable> {
    let key = identity_cache_key(identity, workspace_id, path);
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

/// Cache variable data, scoped to the caller identity. See [`get_cached_variable`].
pub fn cache_variable(workspace_id: &str, path: &str, identity: &str, variable: CachedVariable) {
    let key = identity_cache_key(identity, workspace_id, path);
    let entry = CacheEntry::new(variable);
    VARIABLE_CACHE.insert(key.clone(), entry);
    tracing::debug!("Cached variable {}", key);
}

/// Get cached resource if available and not expired.
///
/// Scoped to `identity` ([`auth_identity`]); see [`identity_cache_key`]. The cached value
/// is the *already-interpolated* resource — its `$var:`/`$res:` secrets are resolved and
/// decrypted inline — so it must never cross authorization boundaries.
pub fn get_cached_resource(workspace_id: &str, path: &str, identity: &str) -> Option<Value> {
    let key = identity_cache_key(identity, workspace_id, path);
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

/// Cache resource data, scoped to the caller identity. See [`get_cached_resource`].
pub fn cache_resource(workspace_id: &str, path: &str, identity: &str, resource: Value) {
    let key = identity_cache_key(identity, workspace_id, path);
    let entry = CacheEntry::new(resource);
    RESOURCE_CACHE.insert(key.clone(), entry);
    tracing::debug!("Cached resource {}", key);
}

/// Invalidate a variable from the cache.
///
/// NOTE: entries are keyed by [`identity_cache_key`] (`identity:workspace:path`), so this
/// `workspace:path` key cannot target them — it only removes a legacy unscoped entry, if
/// any. Per-identity entries are not enumerable here; rely on the 30s TTL for staleness,
/// or use [`clear_all_caches`] to force a full flush. Currently unused.
pub fn invalidate_variable_cache(workspace_id: &str, path: &str) {
    let key = cache_key(workspace_id, path);
    VARIABLE_CACHE.remove(&key);
    tracing::info!("Variable cache invalidated for {}", key);
}

/// Invalidate a resource from the cache. Same identity-scoping caveat as
/// [`invalidate_variable_cache`]. Currently unused.
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
