/*
 * Author: Claude
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use windmill_common::db::Authable;

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

/// Hash the caller's full authorization context into a stable identity string.
///
/// Email alone is **not** a sufficient scope: the same email can resolve to different
/// effective permissions (`username`, groups, folders, scopes, admin/operator) through
/// job- or owner-scoped tokens that share an email but carry a narrower `permissioned_as`.
/// Every input that determines what the caller may read is folded in, mirroring
/// `job_read_access_cache_key` in windmill-api, so a lower-privilege context can never
/// reuse a higher-privilege context's cache entry. Variable-length fields are
/// length-prefixed to keep the encoding injective.
pub fn auth_identity<A: Authable + ?Sized>(authed: &A) -> String {
    let mut hasher = Sha256::new();
    let field = |hasher: &mut Sha256, bytes: &[u8]| {
        hasher.update((bytes.len() as u32).to_be_bytes());
        hasher.update(bytes);
    };
    hasher.update([authed.is_admin() as u8, authed.is_operator() as u8]);
    field(&mut hasher, authed.email().as_bytes());
    field(&mut hasher, authed.username().as_bytes());
    let mut groups: Vec<&str> = authed.groups().iter().map(String::as_str).collect();
    groups.sort_unstable();
    hasher.update((groups.len() as u32).to_be_bytes());
    for g in groups {
        field(&mut hasher, g.as_bytes());
    }
    let mut folders: Vec<&str> = authed.folders().iter().map(|f| f.0.as_str()).collect();
    folders.sort_unstable();
    hasher.update((folders.len() as u32).to_be_bytes());
    for f in folders {
        field(&mut hasher, f.as_bytes());
    }
    match authed.scopes() {
        // u32::MAX length-prefix marks "no scopes" so it can't collide with an empty list.
        None => hasher.update(u32::MAX.to_be_bytes()),
        Some(scopes) => {
            let mut scopes: Vec<&str> = scopes.iter().map(String::as_str).collect();
            scopes.sort_unstable();
            hasher.update((scopes.len() as u32).to_be_bytes());
            for s in scopes {
                field(&mut hasher, s.as_bytes());
            }
        }
    }
    hex::encode(hasher.finalize())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal [`Authable`] double so we can assert which authorization fields the
    /// cache identity is sensitive to, without standing up a full auth stack.
    struct FakeAuthed {
        email: String,
        username: String,
        is_admin: bool,
        is_operator: bool,
        groups: Vec<String>,
        folders: Vec<(String, bool, bool)>,
        scopes: Option<Vec<String>>,
    }

    impl FakeAuthed {
        fn base() -> Self {
            Self {
                email: "alice@x.dev".to_string(),
                username: "alice".to_string(),
                is_admin: false,
                is_operator: false,
                groups: vec!["all".to_string()],
                folders: vec![("shared".to_string(), false, false)],
                scopes: None,
            }
        }
    }

    impl Authable for FakeAuthed {
        fn email(&self) -> &str {
            &self.email
        }
        fn username(&self) -> &str {
            &self.username
        }
        fn is_admin(&self) -> bool {
            self.is_admin
        }
        fn is_operator(&self) -> bool {
            self.is_operator
        }
        fn groups(&self) -> &[String] {
            &self.groups
        }
        fn folders(&self) -> &[(String, bool, bool)] {
            &self.folders
        }
        fn scopes(&self) -> Option<&[String]> {
            self.scopes.as_deref()
        }
    }

    // Email alone must NOT determine the cache identity: two contexts that share an email
    // but resolve to different effective permissions must get distinct identities, so a
    // lower-privilege context can never reuse a higher-privilege one's cached secret.
    #[test]
    fn auth_identity_is_not_just_email() {
        let base = auth_identity(&FakeAuthed::base());

        let mut more_folders = FakeAuthed::base();
        more_folders
            .folders
            .push(("secret".to_string(), false, false));
        assert_ne!(base, auth_identity(&more_folders), "folders must matter");

        let mut more_groups = FakeAuthed::base();
        more_groups.groups.push(("devs").to_string());
        assert_ne!(base, auth_identity(&more_groups), "groups must matter");

        let mut other_user = FakeAuthed::base();
        other_user.username = "bob".to_string();
        assert_ne!(base, auth_identity(&other_user), "username must matter");

        let mut admin = FakeAuthed::base();
        admin.is_admin = true;
        assert_ne!(base, auth_identity(&admin), "is_admin must matter");

        let mut operator = FakeAuthed::base();
        operator.is_operator = true;
        assert_ne!(base, auth_identity(&operator), "is_operator must matter");

        let mut scoped = FakeAuthed::base();
        scoped.scopes = Some(vec!["resources:read:f/secret/x".to_string()]);
        assert_ne!(base, auth_identity(&scoped), "scopes must matter");
    }

    // Identical authorization contexts must produce the same identity (so the same caller
    // gets a cache hit), and ordering of groups/folders must not change the identity.
    #[test]
    fn auth_identity_is_stable_and_order_independent() {
        let a = FakeAuthed::base();
        assert_eq!(auth_identity(&a), auth_identity(&FakeAuthed::base()));

        let mut reordered = FakeAuthed::base();
        reordered.groups = vec!["all".to_string(), "devs".to_string()];
        let mut other_order = FakeAuthed::base();
        other_order.groups = vec!["devs".to_string(), "all".to_string()];
        assert_eq!(auth_identity(&reordered), auth_identity(&other_order));
    }
}
