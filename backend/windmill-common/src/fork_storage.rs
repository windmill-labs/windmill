//! Where a workspace's object keys live in a storage it may share with its fork ancestors.
//!
//! A fork that isolates a storage (a `fork_storage_namespace` row) writes logical key `k` at
//! `__wm_forks/<fork segment>/k` and reads `k` from the first of its own and its ancestors'
//! locations that has it. Everything under `__wm_forks/` is reachable only through the areas of
//! the workspace's own chain, which is also what DuckLake's fork namespaces live in: those keys
//! arrive already prefixed and pass through unchanged.

use std::sync::Arc;

use quick_cache::sync::Cache;

use crate::error::{Error, Result};
use crate::workspaces::{
    fork_data_dir_segment, lfs_entry_storage_ref, workspace_with_fork_ancestors,
    FORK_DUCKLAKE_DATA_DIR,
};
use crate::DB;

/// How one workspace's logical keys map to physical keys in one of its storages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkStorageKeys {
    /// Prefix of every write: `__wm_forks/<segment>/` in an isolating fork, `""` otherwise.
    write_prefix: String,
    /// Prefixes a read tries in order, nearest first.
    read_prefixes: Vec<String>,
    /// `__wm_forks/<segment>/` of this workspace, writable as is.
    own_area: String,
    /// The fork areas of this workspace and its ancestors, readable as is.
    readable_areas: Vec<String>,
}

/// A listing request translated to the physical storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListMapping {
    pub physical_prefix: String,
    /// Stripped from each physical key to get the logical key returned to the caller.
    strip: String,
    /// Logical keys under `__wm_forks/` are dropped: reached only by listing that area itself.
    hide_fork_area: bool,
}

impl ListMapping {
    /// Whether listed keys differ from what the caller may see: false lets a listing response
    /// be passed through untouched.
    pub fn rewrites(&self) -> bool {
        let fork_dir = format!("{FORK_DUCKLAKE_DATA_DIR}/");
        !self.strip.is_empty()
            || (self.hide_fork_area && fork_dir.starts_with(&self.physical_prefix))
    }

    /// The physical form of a logical key of this listing, e.g. a start-after cursor.
    pub fn physical_key(&self, logical: &str) -> String {
        format!("{}{logical}", self.strip)
    }

    /// The logical key for a listed physical key, or None when the caller must not see it.
    pub fn logical_key(&self, physical: &str) -> Option<String> {
        let logical = physical.strip_prefix(self.strip.as_str())?;
        if self.hide_fork_area && in_fork_area(logical) {
            return None;
        }
        Some(logical.to_string())
    }
}

fn in_fork_area(key: &str) -> bool {
    key == FORK_DUCKLAKE_DATA_DIR
        || key
            .strip_prefix(FORK_DUCKLAKE_DATA_DIR)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn fork_area(w_id: &str) -> String {
    format!("{FORK_DUCKLAKE_DATA_DIR}/{}/", fork_data_dir_segment(w_id))
}

fn forbidden(key: &str) -> Error {
    Error::NotAuthorized(format!(
        "`{key}` is in another workspace's fork namespace (`{FORK_DUCKLAKE_DATA_DIR}/`)"
    ))
}

/// One workspace of a fork chain, nearest first, as the storage mapping needs it.
#[derive(Debug, Clone)]
pub struct ChainLink {
    pub w_id: String,
    /// Identity of the storage in this workspace's settings (`lfs_entry_storage_ref`).
    pub storage_ref: Option<String>,
    pub isolated: bool,
}

impl ForkStorageKeys {
    /// The mapping of `chain[0]`. Reads stop at the first ancestor whose storage is a different
    /// one: its keys are not in this bucket.
    pub fn from_chain(chain: &[ChainLink]) -> Self {
        let own = &chain[0];
        let mut read_prefixes = vec![];
        let mut reached_root = true;
        for link in chain {
            if link.storage_ref != own.storage_ref {
                reached_root = false;
                break;
            }
            if link.isolated {
                read_prefixes.push(fork_area(&link.w_id));
            }
        }
        if reached_root || read_prefixes.is_empty() {
            read_prefixes.push(String::new());
        }
        Self {
            write_prefix: read_prefixes[0].clone(),
            read_prefixes,
            own_area: fork_area(&own.w_id),
            readable_areas: chain.iter().map(|l| fork_area(&l.w_id)).collect(),
        }
    }

    /// Whether logical keys map to themselves (only the fork-area boundary applies).
    pub fn is_identity(&self) -> bool {
        self.write_prefix.is_empty() && self.read_prefixes.len() == 1
    }

    /// The physical key a write (put, multipart, delete, copy destination) of `key` goes to.
    pub fn write_key(&self, key: &str) -> Result<String> {
        let key = key.trim_start_matches('/');
        if in_fork_area(key) {
            return if key.starts_with(&self.own_area) {
                Ok(key.to_string())
            } else {
                Err(forbidden(key))
            };
        }
        Ok(format!("{}{key}", self.write_prefix))
    }

    /// The physical keys a read of `key` tries, in order, until one exists.
    pub fn read_keys(&self, key: &str) -> Result<Vec<String>> {
        let key = key.trim_start_matches('/');
        if in_fork_area(key) {
            return if self.readable_areas.iter().any(|a| key.starts_with(a)) {
                Ok(vec![key.to_string()])
            } else {
                Err(forbidden(key))
            };
        }
        Ok(self
            .read_prefixes
            .iter()
            .map(|p| format!("{p}{key}"))
            .collect())
    }

    /// A listing of `prefix` covers this workspace's own objects only: an isolating fork does
    /// not list its ancestors' keys.
    pub fn list(&self, prefix: &str) -> Result<ListMapping> {
        let prefix = prefix.trim_start_matches('/');
        if in_fork_area(prefix) {
            if !self.readable_areas.iter().any(|a| prefix.starts_with(a)) {
                return Err(forbidden(prefix));
            }
            return Ok(ListMapping {
                physical_prefix: prefix.to_string(),
                strip: String::new(),
                hide_fork_area: false,
            });
        }
        Ok(ListMapping {
            physical_prefix: format!("{}{prefix}", self.write_prefix),
            strip: self.write_prefix.clone(),
            hide_fork_area: true,
        })
    }
}

lazy_static::lazy_static! {
    static ref FORK_STORAGE_KEYS_CACHE: Cache<(String, String), (Arc<ForkStorageKeys>, i64)> =
        Cache::new(1000);
}

/// Drop the cached mappings of a workspace whose isolation rows or lineage just changed.
pub fn invalidate_fork_storage_keys_cache(w_id: &str) {
    FORK_STORAGE_KEYS_CACHE.retain(|(w, _), _| w != w_id);
}

/// `storage` as the registry keys it: '' for the default storage.
pub fn registry_storage_name(storage: Option<&str>) -> &str {
    match storage {
        None | Some("") | Some(windmill_types::s3::DEFAULT_STORAGE) => "",
        Some(s) => s,
    }
}

/// The storage entry named `storage` (registry form) in a `large_file_storage` value.
pub fn lfs_entry<'a>(lfs: &'a serde_json::Value, storage: &str) -> Option<&'a serde_json::Value> {
    if storage.is_empty() {
        Some(lfs)
    } else {
        lfs.get("secondary_storage")?.get(storage)
    }
}

/// How `w_id`'s keys map in `storage`. Reads lineage for any `w_id` with no authorization
/// check, like [`workspace_with_fork_ancestors`]: callers must be authorized for `w_id`.
pub async fn fork_storage_keys(
    db: &DB,
    w_id: &str,
    storage: Option<&str>,
) -> Result<Arc<ForkStorageKeys>> {
    let storage = registry_storage_name(storage).to_string();
    let cache_key = (w_id.to_string(), storage.clone());
    let now = chrono::Utc::now().timestamp();
    if let Some((keys, expiry)) = FORK_STORAGE_KEYS_CACHE.get(&cache_key) {
        if expiry > now {
            return Ok(keys);
        }
    }

    let chain_ids = workspace_with_fork_ancestors(db, w_id).await?;
    let rows = sqlx::query!(
        r#"SELECT c.id AS "id!", ws.large_file_storage,
                  EXISTS (SELECT 1 FROM fork_storage_namespace f
                          WHERE f.workspace_id = c.id AND f.storage = $2) AS "isolated!"
           FROM unnest($1::text[]) WITH ORDINALITY AS c(id, ord)
           LEFT JOIN workspace_settings ws ON ws.workspace_id = c.id
           ORDER BY c.ord"#,
        &chain_ids,
        &storage,
    )
    .fetch_all(db)
    .await?;
    let chain: Vec<ChainLink> = rows
        .into_iter()
        .map(|r| ChainLink {
            storage_ref: r
                .large_file_storage
                .as_ref()
                .and_then(|lfs| lfs_entry(lfs, &storage))
                .and_then(lfs_entry_storage_ref),
            w_id: r.id,
            isolated: r.isolated,
        })
        .collect();
    let keys = Arc::new(if chain.is_empty() {
        ForkStorageKeys::from_chain(&[ChainLink {
            w_id: w_id.to_string(),
            storage_ref: None,
            isolated: false,
        }])
    } else {
        ForkStorageKeys::from_chain(&chain)
    });
    FORK_STORAGE_KEYS_CACHE.insert(cache_key, (keys.clone(), now + 30));
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(w_id: &str, storage_ref: &str, isolated: bool) -> ChainLink {
        ChainLink { w_id: w_id.into(), storage_ref: Some(storage_ref.into()), isolated }
    }

    #[test]
    fn root_is_identity_but_keeps_fork_areas_out_of_reach() {
        let keys = ForkStorageKeys::from_chain(&[link("prod", "S3Storage:u/a/s3", false)]);
        assert!(keys.is_identity());
        assert_eq!(keys.write_key("a/b.csv").unwrap(), "a/b.csv");
        assert_eq!(keys.read_keys("a/b.csv").unwrap(), vec!["a/b.csv"]);
        let other = fork_area("wm-fork-x");
        assert!(keys.write_key(&format!("{other}a")).is_err());
        assert!(keys.read_keys(&format!("{other}a")).is_err());
        let listing = keys.list("").unwrap();
        assert_eq!(listing.logical_key("a/b.csv").as_deref(), Some("a/b.csv"));
        assert_eq!(listing.logical_key(&format!("{other}a")), None);
    }

    #[test]
    fn isolated_fork_writes_its_area_and_reads_through_to_ancestors() {
        let s = "S3Storage:u/a/s3";
        let chain = [
            link("wm-fork-b", s, true),
            link("wm-fork-a", s, false),
            link("dev", s, true),
            link("prod", s, false),
        ];
        let keys = ForkStorageKeys::from_chain(&chain);
        let (b, dev) = (fork_area("wm-fork-b"), fork_area("dev"));
        assert_eq!(keys.write_key("x.csv").unwrap(), format!("{b}x.csv"));
        assert_eq!(
            keys.read_keys("x.csv").unwrap(),
            vec![
                format!("{b}x.csv"),
                format!("{dev}x.csv"),
                "x.csv".to_string()
            ]
        );
        // DuckLake's own fork namespace and its ancestors' pass through as is.
        assert_eq!(
            keys.write_key(&format!("{b}lake/f")).unwrap(),
            format!("{b}lake/f")
        );
        let a = fork_area("wm-fork-a");
        assert_eq!(
            keys.read_keys(&format!("{a}lake/f")).unwrap(),
            vec![format!("{a}lake/f")]
        );
        assert!(keys.write_key(&format!("{a}lake/f")).is_err());
        let listing = keys.list("dir/").unwrap();
        assert_eq!(listing.physical_prefix, format!("{b}dir/"));
        assert_eq!(
            listing.logical_key(&format!("{b}dir/f")).as_deref(),
            Some("dir/f")
        );
    }

    #[test]
    fn shared_fork_writes_where_its_parent_does() {
        let s = "S3Storage:u/a/s3";
        let keys = ForkStorageKeys::from_chain(&[
            link("wm-fork-c", s, false),
            link("dev", s, true),
            link("prod", s, false),
        ]);
        let dev = fork_area("dev");
        assert_eq!(keys.write_key("x").unwrap(), format!("{dev}x"));
        assert_eq!(
            keys.read_keys("x").unwrap(),
            vec![format!("{dev}x"), "x".into()]
        );
    }

    #[test]
    fn reads_stop_at_an_ancestor_on_another_storage() {
        let keys = ForkStorageKeys::from_chain(&[
            link("wm-fork-d", "S3Storage:u/a/other", true),
            link("prod", "S3Storage:u/a/s3", false),
        ]);
        let d = fork_area("wm-fork-d");
        assert_eq!(keys.read_keys("x").unwrap(), vec![format!("{d}x")]);
    }

    #[test]
    fn fork_area_match_is_on_whole_segments() {
        assert!(in_fork_area("__wm_forks/a"));
        assert!(!in_fork_area("__wm_forks_backup/a"));
        let keys = ForkStorageKeys::from_chain(&[link("prod", "S3Storage:u/a/s3", false)]);
        assert_eq!(
            keys.write_key("__wm_forks_backup/a").unwrap(),
            "__wm_forks_backup/a"
        );
    }
}
