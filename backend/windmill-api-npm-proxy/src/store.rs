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
/// Upload parts allowed in flight at once. The unit is parts, not bytes, so this times the
/// chunk size is what an upload can hold.
#[cfg(all(feature = "enterprise", feature = "parquet"))]
const UPLOAD_PARTS_IN_FLIGHT: usize = 2;
/// How long a scratch directory has to sit untouched before the sweep calls it abandoned.
/// A live writer's scratch is seconds old; deleting one mid-write would publish a package
/// missing whatever it had already written.
const SCRATCH_STALE_SECS: u64 = 3600;

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
    let escaped = segment.replace(['/', '\\'], "%2F");
    // A segment of only dots is a traversal, not a name: `..` would resolve the package
    // directory to its own parent, which the comment above claims cannot happen.
    if escaped.chars().all(|c| c == '.') {
        return escaped.replace('.', "%2E");
    }
    escaped
}

/// Reject anything that would land outside the package directory. Archive entry paths are
/// registry-supplied, so this is the boundary between "a file in a tarball" and "a path on
/// our disk".
fn safe_relative_path(dir: &Path, relative: &str) -> Option<PathBuf> {
    if !is_safe_relative(relative) {
        return None;
    }
    Some(dir.join(FILES_DIR).join(relative))
}

/// Whether an archive entry's path is one we would ever write. Checked before either sink,
/// not inside one: an archive carrying a traversal is malicious rather than a cache that
/// happens to be unwritable, and the two must not degrade the same way.
pub(crate) fn is_safe_relative(relative: &str) -> bool {
    Path::new(relative)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
}

/// Whether a package directory is complete. The manifest is written last, so its presence
/// is what distinguishes a published package from anything else at that path.
pub(crate) async fn has_manifest(dir: &Path) -> bool {
    tokio::fs::metadata(dir.join(MANIFEST_FILE)).await.is_ok()
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

    /// Returns the published directory and the manifest's size, which the caller adds to
    /// its write accounting: a package of mostly non-retained entries writes a large file
    /// list and would otherwise advance the sweep trigger by almost nothing.
    pub(crate) fn publish(self, manifest: &Manifest) -> Result<(PathBuf, u64)> {
        let raw = serde_json::to_vec(manifest)
            .map_err(|e| Error::InternalErr(format!("Failed to encode manifest: {e}")))?;
        let manifest_bytes = raw.len() as u64;
        std::fs::write(self.scratch.path.join(MANIFEST_FILE), raw)
            .map_err(|e| Error::InternalErr(format!("Failed to write manifest: {e}")))?;

        let final_dir = self.final_dir.clone();
        let tmp = self.scratch.into_published();
        publish_dir(&tmp, &final_dir)?;
        Ok((final_dir, manifest_bytes))
    }
}

/// Take a directory off the live cache path in one step, returning a guard that removes it
/// on drop. `None` means the path could not be moved and still stands.
///
/// Deleting a package directory where it stands is not safe to interrupt: `remove_dir_all`
/// unlinks in readdir order, so a process that stops partway can leave `manifest.json`
/// standing over files that are already gone. Reads take that for a complete package
/// forever after, and since the manifest short-circuits the lookup, the object store is
/// never consulted to repair it. A rename is atomic, so the live path only ever holds a
/// whole package or nothing, and the detached copy carries the scratch name: if this
/// process dies before the guard runs, the sweep accounts for it and reclaims it.
fn detach(dir: &Path) -> Option<Scratch> {
    let aside = Scratch::beside(dir);
    std::fs::rename(dir, &aside.path).ok().map(|()| aside)
}

/// Move a finished tree onto its final path.
///
/// A destination that already holds a manifest is another writer publishing the same
/// immutable content, so theirs wins and this tree is dropped. A destination *without* one
/// is debris, left by an eviction or a writer killed between the two, and it has to be
/// replaced rather than deferred to: `rename` refuses a non-empty directory, so treating
/// the failure as a concurrent publish would discard this valid tree and leave the path
/// permanently unreadable, with nothing on the read side able to repair it.
///
/// The debris is detached rather than deleted in place so the final path is never absent
/// for longer than a rename.
fn publish_dir(tmp: &Path, final_dir: &Path) -> Result<()> {
    if let Some(parent) = final_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::rename(tmp, final_dir).is_ok() {
        return Ok(());
    }
    if final_dir.join(MANIFEST_FILE).exists() {
        let _ = std::fs::remove_dir_all(tmp);
        return Ok(());
    }

    let _debris = detach(final_dir);
    match std::fs::rename(tmp, final_dir) {
        Ok(()) => Ok(()),
        // Another writer got there in the window above, which is still the same content.
        Err(_) if final_dir.join(MANIFEST_FILE).exists() => {
            let _ = std::fs::remove_dir_all(tmp);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(tmp);
            Err(Error::InternalErr(format!(
                "Failed to publish {final_dir:?}: {e}"
            )))
        }
    }
}

/// Pull a package version out of the instance object store into the local cache. `Ok(None)`
/// means "not there", which is a miss rather than a failure.
///
/// The object streams to a temp file and is unpacked from it on a blocking thread: holding
/// a package's worth of bytes per concurrent miss would put back the request-scaled heap
/// this module exists to remove, and unpacking is synchronous work whatever its signature
/// says.
pub(crate) async fn pull_from_object_store(_dir: &Path, _key: &str) -> Result<Option<u64>> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    {
        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        let Some(os) = windmill_object_store::get_object_store().await else {
            return Ok(None);
        };
        let path = windmill_object_store::object_store_reexports::Path::from(_key);
        let Ok(result) = os.get(&path).await else {
            return Ok(None);
        };

        // The temp file is a sibling of the package directory, so its parent has to exist
        // before anything is written: a wiped cache has no tree at all.
        if let Some(parent) = _dir.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::InternalErr(format!("Failed to create {parent:?}: {e}"))
            })?;
        }
        // Both the download and the unpacked tree live inside the scratch directory, so a
        // concurrent pull cannot collide with this one and dropping it takes the download
        // with it. `with_extension` would have replaced the uuid, giving every pull the
        // same path.
        let scratch = Scratch::beside(_dir);
        tokio::fs::create_dir_all(&scratch.path).await.map_err(|e| {
            Error::InternalErr(format!("Failed to create {:?}: {e}", scratch.path))
        })?;
        let tar_path = scratch.path.join("archive.tar");
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

        // The scratch guard moves into the blocking task: leaving it in the request future
        // means cancelling the request drops it, recursively removing the directory while
        // this is still unpacking into it or renaming out of it.
        let destination = _dir.to_path_buf();
        let published = tokio::task::spawn_blocking(move || -> Result<Option<u64>> {
            let unpack_to = scratch.path.join("content");
            let file = std::fs::File::open(&tar_path)
                .map_err(|e| Error::InternalErr(format!("Failed to open {tar_path:?}: {e}")))?;
            tar::Archive::new(file)
                .unpack(&unpack_to)
                .map_err(|e| Error::InternalErr(format!("Failed to unpack: {e}")))?;

            // An object that unpacks without a manifest is not a package. Publishing it
            // would make every later pull short-circuit the registry against a tree nothing
            // can read.
            if !unpack_to.join(MANIFEST_FILE).exists() {
                return Ok(None);
            }
            // `unpack` restores the mtimes the archive was built with, so a pulled tree
            // carries the time it was first filled somewhere else. The sweep reads the
            // newest mtime in a package as its recency, which would make a package pulled
            // because something just asked for it the first candidate for eviction.
            if let Ok(file) = std::fs::OpenOptions::new()
                .append(true)
                .open(unpack_to.join(MANIFEST_FILE))
            {
                let _ = file.set_modified(SystemTime::now());
            }
            let unpacked = measure(&unpack_to).map(|(bytes, _)| bytes).unwrap_or(0);
            publish_dir(&unpack_to, &destination)?;
            // The download is still in the scratch directory, which goes with this drop.
            drop(scratch);
            // A miss rather than a hit if the destination is somehow still not a package:
            // reporting a hit here hands the caller a directory whose manifest cannot be
            // read, which is a failed request every time instead of one registry fetch.
            if !destination.join(MANIFEST_FILE).exists() {
                return Ok(None);
            }
            Ok(Some(unpacked))
        })
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to unpack {_key}: {e}")))??;

        if published.is_none() {
            tracing::warn!("{_key} unpacked without a manifest, ignoring it");
        }
        return Ok(published);
    }
    #[allow(unreachable_code)]
    Ok(None)
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

    // Inside a scratch directory rather than beside the package: a sibling file survives
    // both the early returns below and the sweep, which only ever walks directories.
    let scratch = Scratch::beside(dir);
    tokio::fs::create_dir_all(&scratch.path)
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to create {:?}: {e}", scratch.path)))?;
    let tar_path = scratch.path.join("archive.tar");
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

    const CHUNK: usize = 8 * 1024 * 1024;
    let mut file = tokio::fs::File::open(&tar_path)
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to open {tar_path:?}: {e}")))?;
    let path = windmill_object_store::object_store_reexports::Path::from(key);
    let mut writer = WriteMultipart::new(
        os.put_multipart(&path)
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to start upload of {key}: {e}")))?,
    );
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let read = file
            .read(&mut chunk)
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to read {tar_path:?}: {e}")))?;
        if read == 0 {
            break;
        }
        // Parts in flight, not bytes: `write` queues without waiting, so a slow store
        // would otherwise leave the whole tar outstanding in upload tasks.
        writer
            .wait_for_capacity(UPLOAD_PARTS_IN_FLIGHT)
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to upload {key}: {e}")))?;
        writer.write(&chunk[..read]);
    }
    writer
        .finish()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to upload {key}: {e}")))?;
    Ok(())
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

    // registry/package/version, so package directories sit three levels down. A scratch
    // directory left by a write that died mid-flight will never be read, and counts against
    // the cap until something removes it.
    let mut packages: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    let mut total = 0u64;
    for registry in child_dirs(root)? {
        for package in child_dirs(&registry)? {
            for version in child_dirs(&package)? {
                // A scratch directory is either a live write, which must not be deleted
                // underneath its writer, or abandoned debris. Never an eviction candidate,
                // but its bytes are on the disk either way and have to count.
                if is_scratch(&version) {
                    if is_stale(&version) {
                        let _ = std::fs::remove_dir_all(&version);
                    } else {
                        total += measure(&version).map(|(size, _)| size).unwrap_or(0);
                    }
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
        // Detached, not deleted in place: an eviction is exactly the long unlink that gets
        // interrupted, and one that stops after the declarations but before the manifest
        // leaves a directory every later read trusts and nothing repairs. A candidate that
        // cannot be moved is left whole and skipped, since the cap is a bound this rechecks
        // on a later sweep while a half-deleted package is permanent.
        if let Some(_aside) = detach(&dir) {
            // The guard removes it, off the live path.
            total = total.saturating_sub(size);
        }
    }
    Ok(())
}

/// Publishing renames, so a directory still carrying the scratch marker is either a write
/// in flight or one that died. Age is what separates them: concurrent cold fills are
/// exactly when the sweep is most likely to run, and a live scratch must survive it.
fn is_scratch(dir: &Path) -> bool {
    // The exact shape `Scratch` writes, not any name containing the marker: a version
    // string is registry-supplied, and one like `1.0.0-alpha.tmp.1` would otherwise be
    // re-fetched forever and never counted against the cap.
    dir.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.rsplit_once(".tmp."))
        .is_some_and(|(_, suffix)| uuid::Uuid::parse_str(suffix).is_ok())
}

fn is_stale(dir: &Path) -> bool {
    std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .map(|at| {
            at.elapsed()
                .map(|since| since.as_secs() > SCRATCH_STALE_SECS)
                .unwrap_or(false)
        })
        .unwrap_or(false)
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

        // the guard the extraction consults before either sink
        assert!(is_safe_relative("types/index.d.ts"));
        assert!(!is_safe_relative("../../../etc/passwd"));
        assert!(!is_safe_relative("/etc/passwd"));
        assert!(!is_safe_relative("types/../../escape.d.ts"));

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

    /// A registry that answered for a package called `..` would otherwise resolve the
    /// package directory to its own parent.
    #[test]
    fn a_traversal_cannot_be_a_package_name() {
        let root = package_dir("https://registry.example.com", "lodash", "1.0.0");
        let traversal = package_dir("https://registry.example.com", "..", "..");

        assert!(traversal.starts_with(root.parent().unwrap().parent().unwrap()));
        assert!(traversal.ends_with("%2E%2E/%2E%2E"));
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

        // a writer that died mid-flight long enough ago to be abandoned, and two packages
        let debris = format!("1.0.0.tmp.{}", uuid::Uuid::new_v4());
        make("ghost", &debris, 1024);
        let ghost = root.join("registry").join("ghost").join(&debris);
        let long_ago = SystemTime::now() - std::time::Duration::from_secs(SCRATCH_STALE_SECS * 2);
        std::fs::File::open(&ghost)
            .and_then(|f| f.set_modified(long_ago))
            .unwrap();
        let old = make("old", "1.0.0", 64 * 1024);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let fresh = make("fresh", "1.0.0", 64 * 1024);

        sweep(&root, 96 * 1024).unwrap();

        assert!(!ghost.exists(), "an abandoned scratch directory should be removed");
        // a version is registry-supplied: one shaped like debris is still a package
        assert!(!is_scratch(&root.join("registry").join("pkg").join("1.0.0-alpha.tmp.1")));
        assert!(fresh.exists(), "the more recently used package should survive");
        assert!(!old.exists(), "the least recently used should go first");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Concurrent cold fills are exactly when a sweep is most likely to run, and deleting
    /// a scratch directory mid-write would publish a package missing what it had written.
    #[test]
    fn a_scratch_directory_still_being_written_survives_a_sweep() {
        let root = std::env::temp_dir().join(format!("npm-proxy-live-{}", uuid::Uuid::new_v4()));
        let dir = root.join("registry").join("pkg").join("1.0.0");
        let pending = PendingPackage::new(&dir).unwrap();
        pending.write_file("index.d.ts", b"declare const x: number").unwrap();

        sweep(&root, 0).unwrap();

        assert!(pending.scratch.path.exists(), "a live writer's scratch must survive");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The live path holds a whole package or nothing. A directory recursively deleted where
    /// it stands passes through states that are neither, and the one where the manifest
    /// outlives the files it describes is read as valid for as long as the cache exists.
    #[test]
    fn a_package_leaves_the_live_path_in_one_step() {
        let root = std::env::temp_dir().join(format!("npm-proxy-{}", uuid::Uuid::new_v4()));
        let dir = root.join("pkg").join("1.0.0");
        std::fs::create_dir_all(dir.join(FILES_DIR)).unwrap();
        std::fs::write(dir.join(FILES_DIR).join("index.d.ts"), b"declare const x: number").unwrap();
        std::fs::write(dir.join(MANIFEST_FILE), b"{}").unwrap();

        let aside = detach(&dir).expect("a package directory can be detached");

        assert!(!dir.exists(), "the live path is gone the moment it is detached");
        assert!(aside.path.join(MANIFEST_FILE).exists(), "the whole tree went with it");
        assert!(is_scratch(&aside.path), "and dying here leaves debris the sweep reclaims");
        let detached = aside.path.clone();
        drop(aside);
        assert!(!detached.exists(), "the guard removes it");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// An eviction killed partway through leaves a directory that is not a package. Reads
    /// skip it, so only a publish can clear it, and `rename` refuses a non-empty
    /// destination: deferring to it instead would discard every tree published afterwards
    /// and leave the path unreadable for good.
    #[test]
    fn a_destination_left_without_a_manifest_is_replaced() {
        let root = std::env::temp_dir().join(format!("npm-proxy-{}", uuid::Uuid::new_v4()));
        let dir = root.join("pkg").join("1.0.0");
        std::fs::create_dir_all(dir.join(FILES_DIR)).unwrap();
        std::fs::write(dir.join(FILES_DIR).join("half-deleted.d.ts"), b"stale").unwrap();

        let pending = PendingPackage::new(&dir).unwrap();
        pending.write_file("index.d.ts", b"declare const x: number").unwrap();
        pending
            .publish(&Manifest { names: vec!["/index.d.ts".into()], main: "index.js".into() })
            .unwrap();

        assert!(dir.join(MANIFEST_FILE).exists(), "the valid tree must win");
        assert!(dir.join(FILES_DIR).join("index.d.ts").exists());
        assert!(!dir.join(FILES_DIR).join("half-deleted.d.ts").exists(), "debris must go");
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
