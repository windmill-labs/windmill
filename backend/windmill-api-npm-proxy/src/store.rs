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
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
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
/// on every write. Bytes written since the last one force it early, or a burst of large
/// packages could overshoot the cap by the whole interval's worth of writes.
const SWEEP_INTERVAL_SECS: i64 = 600;
const SWEEP_AFTER_WRITTEN: u64 = 256 * 1024 * 1024;

static LAST_SWEEP: AtomicI64 = AtomicI64::new(0);
static WRITTEN_SINCE_SWEEP: AtomicU64 = AtomicU64::new(0);

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

/// Mark a package as used, so the sweep evicts what is genuinely cold. Best effort: a
/// cache that cannot be written is still a cache that can be read.
pub(crate) fn touch(dir: &Path) {
    let manifest = dir.join(MANIFEST_FILE);
    tokio::task::spawn_blocking(move || {
        if let Ok(file) = std::fs::OpenOptions::new().append(true).open(&manifest) {
            let _ = file.set_modified(SystemTime::now());
        }
    });
}

pub(crate) async fn read_file(dir: &Path, relative: &str) -> Option<Vec<u8>> {
    tokio::fs::read(safe_relative_path(dir, relative)?).await.ok()
}

/// A temp directory beside its destination, removed on drop unless it was published. A
/// failed extraction that left its directory behind would accumulate against the disk cap
/// without ever being served.
pub(crate) struct Scratch {
    pub path: PathBuf,
    published: bool,
}

impl Scratch {
    pub(crate) fn beside(dir: &Path) -> Self {
        Self {
            path: PathBuf::from(format!("{}.tmp.{}", dir.display(), uuid::Uuid::new_v4())),
            published: false,
        }
    }

    fn into_published(mut self) -> PathBuf {
        self.published = true;
        self.path.clone()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// A package version being written. Files land in a sibling temp directory and the whole
/// thing is published with a rename, so a concurrent reader never sees a half-written
/// package and a killed process leaves no directory that looks complete.
///
/// Synchronous throughout: it is filled from the archive walk, which already runs on a
/// blocking thread.
pub(crate) struct PendingPackage {
    scratch: Scratch,
    final_dir: PathBuf,
}

impl PendingPackage {
    pub(crate) fn new(dir: &Path) -> Result<Self> {
        let scratch = Scratch::beside(dir);
        std::fs::create_dir_all(scratch.path.join(FILES_DIR)).map_err(|e| {
            Error::InternalErr(format!("Failed to create {:?}: {e}", scratch.path))
        })?;
        Ok(Self { scratch, final_dir: dir.to_path_buf() })
    }

    pub(crate) fn write_file(&self, relative: &str, content: &[u8]) -> Result<()> {
        let path = safe_relative_path(&self.scratch.path, relative).ok_or_else(|| {
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
        std::fs::write(self.scratch.path.join(MANIFEST_FILE), raw)
            .map_err(|e| Error::InternalErr(format!("Failed to write manifest: {e}")))?;

        let final_dir = self.final_dir.clone();
        if let Some(parent) = final_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Losing the race is success: the other writer published the same immutable
        // content, so keep theirs and drop ours.
        let tmp = self.scratch.into_published();
        match std::fs::rename(&tmp, &final_dir) {
            Ok(()) => {}
            Err(_) if final_dir.join(MANIFEST_FILE).exists() => {
                let _ = std::fs::remove_dir_all(&tmp);
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(Error::InternalErr(format!(
                    "Failed to publish {final_dir:?}: {e}"
                )));
            }
        }
        Ok(final_dir)
    }
}

/// Pull a package version out of the instance object store into the local cache. `Ok(false)`
/// means "not there", which is a miss rather than a failure.
///
/// The object streams to a temp file and is unpacked from it on a blocking thread: holding
/// a package's worth of bytes per concurrent miss would put back the request-scaled heap
/// this module exists to remove, and unpacking is synchronous work whatever its signature
/// says.
pub(crate) async fn pull_from_object_store(_dir: &Path, _key: &str) -> Result<bool> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    {
        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        let Some(os) = windmill_object_store::get_object_store().await else {
            return Ok(false);
        };
        let path = windmill_object_store::object_store_reexports::Path::from(_key);
        let Ok(result) = os.get(&path).await else {
            return Ok(false);
        };

        // The temp file is a sibling of the package directory, so its parent has to exist
        // before anything is written: a wiped cache has no tree at all.
        if let Some(parent) = _dir.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::InternalErr(format!("Failed to create {parent:?}: {e}"))
            })?;
        }
        let scratch = Scratch::beside(_dir);
        let tar_path = scratch.path.with_extension("tar");
        let mut file = tokio::fs::File::create(&tar_path)
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to create {tar_path:?}: {e}")))?;
        let mut stream = result.into_stream();
        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| Error::InternalErr(format!("Failed to read {_key}: {e}")))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| Error::InternalErr(format!("Failed to write {tar_path:?}: {e}")))?;
        }
        file.flush()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to flush {tar_path:?}: {e}")))?;
        drop(file);

        let unpack_to = scratch.path.clone();
        let unpacked = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&tar_path)?;
            let mut archive = tar::Archive::new(file);
            let result = archive.unpack(&unpack_to);
            let _ = std::fs::remove_file(&tar_path);
            result
        })
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to unpack {_key}: {e}")))?;
        unpacked.map_err(|e| Error::InternalErr(format!("Failed to unpack {_key}: {e}")))?;

        windmill_common::worker::atomic_publish_dir(
            &scratch.into_published().to_string_lossy(),
            &_dir.to_string_lossy(),
        )?;
        return Ok(true);
    }
    #[allow(unreachable_code)]
    Ok(false)
}

/// Publish a freshly extracted package version to the instance object store so the other
/// replicas do not each have to fetch it. Best effort throughout: the request it came from
/// has already been served.
///
/// The tar is built into a temp file and uploaded in parts, for the same reason the pull
/// streams: a package's worth of bytes per concurrent upload is the heap usage this module
/// exists to remove. One upload at a time, since none of this is on a request's path.
pub(crate) fn push_to_object_store(_dir: PathBuf, _key: String) {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    tokio::spawn(async move {
        static UPLOADS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
        let _permit = UPLOADS.acquire().await;

        if let Err(e) = push_inner(&_dir, &_key).await {
            tracing::warn!("failed to push {_key} to the object store: {e:?}");
        }
    });
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn push_inner(dir: &Path, key: &str) -> Result<()> {
    use tokio::io::AsyncReadExt;
    use windmill_object_store::object_store_reexports::WriteMultipart;

    let Some(os) = windmill_object_store::get_object_store().await else {
        return Ok(());
    };

    let tar_path = PathBuf::from(format!("{}.upload.{}.tar", dir.display(), uuid::Uuid::new_v4()));
    let source = dir.to_path_buf();
    let build_at = tar_path.clone();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&build_at)?;
        let mut tar = tar::Builder::new(file);
        tar.append_dir_all(".", &source)?;
        tar.into_inner().map(|_| ())
    })
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to tar {key}: {e}")))?
    .map_err(|e| Error::InternalErr(format!("Failed to tar {key}: {e}")))?;

    let upload = async {
        let mut file = tokio::fs::File::open(&tar_path).await?;
        let path = windmill_object_store::object_store_reexports::Path::from(key);
        let mut writer = WriteMultipart::new(
            os.put_multipart(&path)
                .await
                .map_err(std::io::Error::other)?,
        );
        let mut chunk = vec![0u8; 8 * 1024 * 1024];
        loop {
            let read = file.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            writer.write(&chunk[..read]);
        }
        writer.finish().await.map_err(std::io::Error::other)?;
        Ok::<_, std::io::Error>(())
    }
    .await;

    let _ = tokio::fs::remove_file(&tar_path).await;
    upload.map_err(|e| Error::InternalErr(format!("Failed to upload {key}: {e}")))
}

/// Remove least-recently-used package directories until the cache is back under its bound.
/// Best effort throughout: a cache that cannot be pruned is a reason to log, not to fail a
/// request that has already been served.
pub(crate) fn sweep_if_due(written: u64) {
    let pending = WRITTEN_SINCE_SWEEP.fetch_add(written, Ordering::Relaxed) + written;
    let now = chrono::Utc::now().timestamp();
    let last = LAST_SWEEP.load(Ordering::Relaxed);
    if now - last < SWEEP_INTERVAL_SECS && pending < SWEEP_AFTER_WRITTEN {
        return;
    }
    if LAST_SWEEP
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    WRITTEN_SINCE_SWEEP.store(0, Ordering::Relaxed);
    tokio::task::spawn_blocking(|| {
        let root = PathBuf::from(&*ROOT_CACHE_DIR).join("npm_proxy");
        if let Err(e) = sweep(&root, DISK_BYTES) {
            tracing::warn!("npm proxy cache sweep failed: {e}");
        }
    });
}

fn sweep(root: &Path, budget: u64) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    // registry/package/version, so package directories sit three levels down. A temp
    // directory at any level is the debris of a write that died mid-flight: nothing will
    // ever read it, and it counts against the cap until something removes it.
    let mut packages: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    let mut total = 0u64;
    for registry in child_dirs(root)? {
        for package in child_dirs(&registry)? {
            for version in child_dirs(&package)? {
                if is_debris(&version) {
                    let _ = std::fs::remove_dir_all(&version);
                    continue;
                }
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

/// A scratch or upload directory whose writer is gone. Publishing renames, so anything
/// still carrying the marker after the process that made it has moved on is abandoned.
fn is_debris(dir: &Path) -> bool {
    dir.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.contains(".tmp.") || n.contains(".upload."))
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

/// Space a package directory occupies, and when it was last used. Modification time, not
/// access time: cache volumes are mounted `relatime` by default, so atime does not move on
/// a read and an LRU built on it would silently be a FIFO. `touch` is what marks a hit.
///
/// Allocated blocks rather than file lengths, because a package of many tiny declarations
/// costs a block each: `@types/lodash` reports a fraction of what it takes on disk if you
/// believe its file sizes.
fn measure(dir: &Path) -> std::io::Result<(u64, SystemTime)> {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;

    let mut size = 0;
    let mut accessed = SystemTime::UNIX_EPOCH;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            #[cfg(unix)]
            {
                size += meta.blocks() * 512;
            }
            #[cfg(not(unix))]
            {
                size += meta.len();
            }
            if meta.is_dir() {
                stack.push(entry.path());
            } else if let Ok(at) = meta.modified() {
                accessed = accessed.max(at);
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

    /// The cap is what keeps a cache directory from outgrowing a small container disk, so
    /// it has to survive both abandoned writes and a set that is merely over budget.
    #[test]
    fn the_sweep_drops_debris_and_the_least_recently_used() {
        let root = std::env::temp_dir().join(format!("npm-proxy-sweep-{}", uuid::Uuid::new_v4()));
        let make = |package: &str, version: &str, bytes: usize| {
            let dir = root.join("registry").join(package).join(version).join(FILES_DIR);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("index.d.ts"), vec![b'x'; bytes]).unwrap();
            dir
        };

        // a writer that died mid-flight, and two real packages
        make("ghost", "1.0.0.tmp.abandoned", 1024);
        let old = make("old", "1.0.0", 64 * 1024);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let fresh = make("fresh", "1.0.0", 64 * 1024);

        sweep(&root, 96 * 1024).unwrap();

        assert!(!root.join("registry").join("ghost").join("1.0.0.tmp.abandoned").exists());
        assert!(fresh.exists(), "the more recently used package should survive");
        assert!(!old.exists(), "the least recently used should go first");

        let _ = std::fs::remove_dir_all(&root);
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
