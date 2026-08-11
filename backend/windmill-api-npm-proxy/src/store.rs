//! Where the proxy keeps what it reads out of a package archive.
//!
//! A published version's archive never changes, so the files read out of it are cacheable
//! indefinitely and are worth keeping somewhere other than the API process's heap. They go
//! to disk, and to the instance object store when one is configured, following the tiering
//! `windmill-worker`'s dependency cache already uses: local disk first, object store on a
//! local miss (which then populates disk), the registry on a miss in both.
//!
//! Only the manifest and the `.d.ts` files are kept, because those are what the endpoints
//! serve. Everything else stays reachable by walking the archive on demand.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use windmill_common::error::{Error, Result};
use windmill_common::worker::ROOT_CACHE_DIR;

/// Bound on the whole cache directory. A sweep runs after a package is written and removes
/// least-recently-used package directories until the total is back under it. Overrunning
/// the disk of a small API container would be a worse failure than the memory this
/// replaces, so this is a real bound rather than a hint.
const DISK_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Walking the tree costs a `stat` per file, so the sweep is rate limited rather than run
/// on every write.
const SWEEP_INTERVAL_SECS: i64 = 600;

static LAST_SWEEP: AtomicI64 = AtomicI64::new(0);

const MANIFEST_FILE: &str = "manifest.json";
const FILES_DIR: &str = "files";

/// What `/filetree` answers with, written once beside the files it describes.
#[derive(Serialize, Deserialize)]
pub(crate) struct Manifest {
    /// In archive order, `/`-prefixed.
    pub names: Vec<String>,
    /// The package's entry point, from its packaged `package.json`.
    pub main: String,
}

/// Cache root for one package version. Registry-scoped: the `npmrc` setting can be pointed
/// at a different registry that serves a different artifact under the same name and
/// version, and the cached files must not outlive that change.
pub(crate) fn package_dir(registry_url: &str, package: &str, version: &str) -> PathBuf {
    PathBuf::from(&*ROOT_CACHE_DIR)
        .join("npm_proxy")
        .join(registry_key(registry_url))
        // A scoped package carries a `/`, and a version is registry-supplied: neither may
        // introduce a path segment of its own.
        .join(escape_segment(package))
        .join(escape_segment(version))
}

/// Key for the same package version in the object store, which is shared across replicas
/// and so has to be scoped exactly as the local path is.
pub(crate) fn object_key(registry_url: &str, package: &str, version: &str) -> String {
    format!(
        "npm_proxy/{}/{}/{}.tar",
        registry_key(registry_url),
        escape_segment(package),
        escape_segment(version)
    )
}

fn registry_key(registry_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(registry_url.as_bytes());
    hex::encode(&hasher.finalize()[..8])
}

fn escape_segment(segment: &str) -> String {
    segment.replace(['/', '\\'], "%2F")
}

/// Reject anything that would land outside the package directory. Archive entry paths are
/// registry-supplied, so this is the boundary between "a file in a tarball" and "a path on
/// our disk".
fn safe_relative_path(dir: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    Some(dir.join(FILES_DIR).join(relative))
}

pub(crate) async fn read_manifest(dir: &Path) -> Option<Manifest> {
    let raw = tokio::fs::read(dir.join(MANIFEST_FILE)).await.ok()?;
    serde_json::from_slice(&raw).ok()
}

pub(crate) async fn read_file(dir: &Path, relative: &str) -> Option<Vec<u8>> {
    tokio::fs::read(safe_relative_path(dir, relative)?).await.ok()
}

/// A package version being written. Files land in a sibling temp directory and the whole
/// thing is published with a rename, so a concurrent reader never sees a half-written
/// package and a killed process leaves no directory that looks complete.
///
/// Synchronous throughout: it is filled from the archive walk, which already runs on a
/// blocking thread.
pub(crate) struct PendingPackage {
    tmp: PathBuf,
    final_dir: PathBuf,
}

impl PendingPackage {
    pub(crate) fn new(dir: &Path) -> Result<Self> {
        let tmp = PathBuf::from(format!("{}.tmp.{}", dir.display(), uuid::Uuid::new_v4()));
        std::fs::create_dir_all(tmp.join(FILES_DIR))
            .map_err(|e| Error::InternalErr(format!("Failed to create {tmp:?}: {e}")))?;
        Ok(Self { tmp, final_dir: dir.to_path_buf() })
    }

    pub(crate) fn write_file(&self, relative: &str, content: &[u8]) -> Result<()> {
        let path = safe_relative_path(&self.tmp, relative).ok_or_else(|| {
            Error::BadRequest(format!("Package archive holds an unsafe path: {relative}"))
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::InternalErr(format!("Failed to create {parent:?}: {e}")))?;
        }
        std::fs::write(&path, content)
            .map_err(|e| Error::InternalErr(format!("Failed to write {path:?}: {e}")))
    }

    pub(crate) fn publish(self, manifest: &Manifest) -> Result<PathBuf> {
        let raw = serde_json::to_vec(manifest)
            .map_err(|e| Error::InternalErr(format!("Failed to encode manifest: {e}")))?;
        std::fs::write(self.tmp.join(MANIFEST_FILE), raw)
            .map_err(|e| Error::InternalErr(format!("Failed to write manifest: {e}")))?;

        if let Some(parent) = self.final_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Losing the race is success: the other writer published the same immutable
        // content, so keep theirs and drop ours.
        match std::fs::rename(&self.tmp, &self.final_dir) {
            Ok(()) => {}
            Err(_) if self.final_dir.join(MANIFEST_FILE).exists() => {
                let _ = std::fs::remove_dir_all(&self.tmp);
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&self.tmp);
                return Err(Error::InternalErr(format!(
                    "Failed to publish {:?}: {e}",
                    self.final_dir
                )));
            }
        }
        Ok(self.final_dir.clone())
    }
}

/// Pull a package version out of the instance object store into the local cache. `Ok(false)`
/// means "not there", which is a miss rather than a failure.
pub(crate) async fn pull_from_object_store(_dir: &Path, _key: &str) -> Result<bool> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    {
        let Some(os) = windmill_object_store::get_object_store().await else {
            return Ok(false);
        };
        let Ok(bytes) = windmill_object_store::attempt_fetch_bytes(os, _key).await else {
            return Ok(false);
        };

        let tmp = format!("{}.tmp.{}", _dir.display(), uuid::Uuid::new_v4());
        if let Err(e) = windmill_common::worker::extract_tar(bytes, &tmp).await {
            let _ = tokio::fs::remove_dir_all(&tmp).await;
            return Err(e);
        }
        if let Some(parent) = _dir.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        match windmill_common::worker::atomic_publish_dir(&tmp, &_dir.to_string_lossy()) {
            Ok(()) => return Ok(true),
            Err(e) => {
                let _ = tokio::fs::remove_dir_all(&tmp).await;
                return Err(e);
            }
        }
    }
    #[allow(unreachable_code)]
    Ok(false)
}

/// Publish a freshly extracted package version to the instance object store so the other
/// replicas do not each have to fetch it. Best effort: the request it came from has already
/// been served.
pub(crate) fn push_to_object_store(_dir: PathBuf, _key: String) {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    tokio::spawn(async move {
        let Some(os) = windmill_object_store::get_object_store().await else {
            return;
        };
        let tarred = tokio::task::spawn_blocking(move || {
            let mut tar = tar::Builder::new(Vec::new());
            tar.append_dir_all(".", &_dir)?;
            tar.into_inner()
        })
        .await;

        match tarred {
            Ok(Ok(bytes)) => {
                if let Err(e) = os
                    .put(
                        &windmill_object_store::object_store_reexports::Path::from(_key.clone()),
                        bytes.into(),
                    )
                    .await
                {
                    tracing::warn!("failed to push {_key} to the object store: {e:?}");
                }
            }
            Ok(Err(e)) => tracing::warn!("failed to tar {_key}: {e:?}"),
            Err(e) => tracing::warn!("failed to tar {_key}: {e:?}"),
        }
    });
}

/// Remove least-recently-used package directories until the cache is back under its bound.
/// Best effort throughout: a cache that cannot be pruned is a reason to log, not to fail a
/// request that has already been served.
pub(crate) fn sweep_if_due() {
    let now = chrono::Utc::now().timestamp();
    let last = LAST_SWEEP.load(Ordering::Relaxed);
    if now - last < SWEEP_INTERVAL_SECS {
        return;
    }
    if LAST_SWEEP
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    tokio::task::spawn_blocking(|| {
        if let Err(e) = sweep(DISK_BYTES) {
            tracing::warn!("npm proxy cache sweep failed: {e}");
        }
    });
}

fn sweep(budget: u64) -> std::io::Result<()> {
    let root = PathBuf::from(&*ROOT_CACHE_DIR).join("npm_proxy");
    if !root.exists() {
        return Ok(());
    }

    // registry/package/version, so package directories sit three levels down
    let mut packages: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    let mut total = 0u64;
    for registry in child_dirs(&root)? {
        for package in child_dirs(&registry)? {
            for version in child_dirs(&package)? {
                let (size, accessed) = measure(&version)?;
                total += size;
                packages.push((accessed, size, version));
            }
        }
    }

    if total <= budget {
        return Ok(());
    }

    packages.sort_by_key(|(accessed, _, _)| *accessed);
    for (_, size, dir) in packages {
        if total <= budget {
            break;
        }
        if std::fs::remove_dir_all(&dir).is_ok() {
            total = total.saturating_sub(size);
        }
    }
    Ok(())
}

fn child_dirs(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            out.push(entry.path());
        }
    }
    Ok(out)
}

/// Total size of a package directory and the most recent access across it. Access time is
/// what makes the sweep an LRU rather than a TTL: a package read every session should
/// outlive one fetched once and never read again.
fn measure(dir: &Path) -> std::io::Result<(u64, SystemTime)> {
    let mut size = 0;
    let mut accessed = SystemTime::UNIX_EPOCH;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                size += meta.len();
                if let Ok(at) = meta.accessed().or_else(|_| meta.modified()) {
                    accessed = accessed.max(at);
                }
            }
        }
    }
    Ok((size, accessed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_may_not_escape_its_package_directory() {
        let dir = Path::new("/cache/pkg/1.0.0");

        assert!(safe_relative_path(dir, "types/index.d.ts").is_some());
        assert_eq!(safe_relative_path(dir, "../../../etc/passwd"), None);
        assert_eq!(safe_relative_path(dir, "/etc/passwd"), None);
        assert_eq!(safe_relative_path(dir, "types/../../escape.d.ts"), None);
    }

    #[test]
    fn a_scoped_package_stays_one_directory() {
        let scoped = package_dir("https://registry.example.com", "@types/node", "20.0.0");
        assert!(scoped.ends_with("@types%2Fnode/20.0.0"));
    }

    #[test]
    fn the_registry_scopes_the_cache() {
        let one = package_dir("https://one.example.com", "lodash", "4.17.21");
        let two = package_dir("https://two.example.com", "lodash", "4.17.21");
        assert_ne!(one, two);
        assert_ne!(
            object_key("https://one.example.com", "lodash", "4.17.21"),
            object_key("https://two.example.com", "lodash", "4.17.21")
        );
    }

    #[test]
    fn a_package_is_published_whole_or_not_at_all() {
        let root = std::env::temp_dir().join(format!("npm-proxy-{}", uuid::Uuid::new_v4()));
        let dir = root.join("pkg").join("1.0.0");

        let pending = PendingPackage::new(&dir).unwrap();
        pending.write_file("types/index.d.ts", b"declare const x: number").unwrap();
        // nothing is visible at the final path until publish
        assert!(!dir.exists());

        pending
            .publish(&Manifest { names: vec!["/types/index.d.ts".into()], main: "index.js".into() })
            .unwrap();
        assert!(dir.join(MANIFEST_FILE).exists());
        assert!(dir.join(FILES_DIR).join("types/index.d.ts").exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unsafe_path_is_refused_rather_than_written() {
        let root = std::env::temp_dir().join(format!("npm-proxy-{}", uuid::Uuid::new_v4()));
        let pending = PendingPackage::new(&root.join("pkg").join("1.0.0")).unwrap();

        assert!(pending.write_file("../../escape.d.ts", b"x").is_err());

        let _ = std::fs::remove_dir_all(&root);
    }
}
