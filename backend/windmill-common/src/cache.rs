//! Windmill cache system.
//!
//! # Features
//! - `scoped_cache`: Use scoped cache instead of global cache.
//!   1. The cache is made thread-local, so each thread has its own entries.
//!   2. The cache is made temporary, so it is deleted when the program exits.
//!   This shall only be used for testing, e.g. [`sqlx::test`] spawn a database per test,
//!   and there is only one test per thread, so using thread-local cache avoid unexpected results.

use crate::{
    apps::AppScriptId, error, flows::FlowNodeId, flows::FlowValue, scripts::ScriptHash,
    scripts::ScriptLang,
};

#[cfg(feature = "scoped_cache")]
use std::thread::ThreadId;
use std::{
    future::Future,
    hash::Hash,
    panic::Location,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use sqlx::{
    types::{Json, JsonRawValue as RawValue},
    PgExecutor,
};
use uuid::Uuid;

pub use const_format::concatcp;
pub use lazy_static::lazy_static;
pub use quick_cache::sync::Cache;

#[cfg(not(feature = "scoped_cache"))]
lazy_static! {
    /// Cache directory for windmill server/worker(s).
    /// 1. If `XDG_CACHE_HOME` is set, use `"${XDG_CACHE_HOME}/windmill"`.
    /// 2. If `HOME` is set, use `"${HOME}/.cache/windmill"`.
    /// 3. Otherwise, use `"{std::env::temp_dir()}/windmill/cache"`.
    pub static ref CACHE_PATH: PathBuf = {
        std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .map(|cache| cache.join("windmill"))
            .unwrap_or_else(|_| std::env::temp_dir().join("windmill/cache"))
    };
}

#[cfg(feature = "scoped_cache")]
lazy_static! {
    /// Temporary directory for thread-local cache.
    pub static ref CACHE_PATH_TMP: tempfile::TempDir = {
        tempfile::tempdir().expect("Failed to create temporary directory")
    };

    /// Cache directory for windmill server/worker(s).
    pub static ref CACHE_PATH: PathBuf = CACHE_PATH_TMP.as_ref().to_path_buf();
}

/// An item that can be imported/exported from/into the file-system.
pub trait Item: Sized {
    /// Returns the path of the item within the given `root` path.
    fn path(&self, root: impl AsRef<Path>) -> PathBuf;
}

/// Bytes storage.
pub trait Storage {
    /// Get bytes for `item`.
    fn get(&self, item: impl Item) -> std::io::Result<Vec<u8>>;
    /// Put bytes for `item`.
    fn put(&self, item: impl Item, data: impl AsRef<[u8]>) -> std::io::Result<()>;

    /// Get utf8 string for `item`.
    #[inline(always)]
    fn get_utf8(&self, item: impl Item) -> error::Result<String> {
        Ok(String::from_utf8(self.get(item)?)?)
    }

    /// Get json for `item`.
    #[inline(always)]
    fn get_json<T: for<'de> Deserialize<'de>>(&self, item: impl Item) -> error::Result<T> {
        Ok(serde_json::from_slice(&self.get(item)?)?)
    }

    /// Get json raw value for `item`.
    #[inline(always)]
    fn get_json_raw(&self, item: impl Item) -> error::Result<Box<RawValue>> {
        Ok(RawValue::from_string(self.get_utf8(item)?)?)
    }
}

/// A type that can be imported from [`Storage`].
pub trait Import: Sized {
    fn import(src: &impl Storage) -> error::Result<Self>;
}

/// A type that can be exported to [`Storage`].
pub trait Export: Clone {
    /// The untrusted type that can be imported from [`Storage`].
    type Untrusted: Import;

    /// Resolve the untrusted type into the trusted type.
    fn resolve(src: Self::Untrusted) -> error::Result<Self>;
    /// Export the trusted type into storage.
    fn export(&self, dst: &impl Storage) -> error::Result<()>;
}

/// A file-system backed concurrent cache.
pub struct FsBackedCache<Key, Val, Root> {
    #[cfg(not(feature = "scoped_cache"))]
    cache: Cache<Key, Val>,
    #[cfg(feature = "scoped_cache")]
    cache: Cache<(ThreadId, Key), Val>,
    root: Root,
}

impl<Key: Eq + Hash + Item + Clone, Val: Export, Root: AsRef<Path>> FsBackedCache<Key, Val, Root> {
    /// Create a new file-system backed cache with `items_capacity` capacity.
    /// The cache will be stored in the `root` directory.
    pub fn new(root: Root, items_capacity: usize) -> Self {
        Self { cache: Cache::new(items_capacity), root }
    }

    /// Build a path for the given `key`.
    pub fn path(&self, key: &Key) -> PathBuf {
        #[cfg(feature = "scoped_cache")]
        let key = &(std::thread::current().id(), key.clone());
        key.path(&self.root)
    }

    /// Clear the cache.
    pub fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
        self.cache.clear();
    }

    /// Remove the item with the given `key` from the cache.
    pub fn remove(&self, key: &Key) -> Option<(Key, Val)> {
        let _ = std::fs::remove_dir_all(self.path(key));
        #[cfg(feature = "scoped_cache")]
        let key = &(std::thread::current().id(), key.clone());
        let res = self.cache.remove(key);
        #[cfg(feature = "scoped_cache")]
        let res = res.map(|(k, v)| (k.1, v));
        res
    }

    /// Gets or inserts an item in the cache with key `key`.
    pub async fn get_or_insert_async<'a, F>(&'a self, key: Key, with: F) -> error::Result<Val>
    where
        F: Future<Output = error::Result<Val::Untrusted>>,
    {
        let import_or_fetch = async {
            let path = &self.path(&key);
            // Retrieve the data from the cache directory or the database.
            if std::fs::metadata(path).is_ok() {
                // Cache path exists, read its contents.
                match <Val as Export>::Untrusted::import(path).and_then(Val::resolve) {
                    Ok(data) => return Ok(data),
                    Err(err) => tracing::warn!(
                        "Failed to import from file-system, fetch source: {path:?}: {err:?}"
                    ),
                }
            }
            // Cache path doesn't exist or import failed, generate the content.
            let data = Val::resolve(with.await?)?;
            // Try to export data to the file-system.
            // If failed, remove the directory but still return the data.
            if let Err(err) = std::fs::create_dir_all(path)
                .map_err(Into::into)
                .and_then(|_| data.export(&path))
            {
                tracing::warn!("Failed to export to file-system: {path:?}: {err:?}");
                let _ = std::fs::remove_dir_all(path);
            }
            Ok(data)
        };
        #[cfg(feature = "scoped_cache")]
        let key = (std::thread::current().id(), key.clone());
        self.cache.get_or_insert_async(&key, import_or_fetch).await
    }
}

/// Like [`lazy_static`]`, but for file-system backed caches.
///
/// # Example
/// ```rust
/// use windmill_common::make_static;
///
/// make_static! {
///     /// String cache with a maximum capacity of 1000 items stored in the
///     /// "subdirectory" directory.
///    static ref CACHE: { u64 => String } in "subdirectory" <= 1000;
///    /// Another cache.
///    static ref ANOTHER_CACHE: { u64 => Vec<String> } in "another" <= 100;
/// }
/// ```
#[macro_export]
macro_rules! make_static {
    { $( $(#[$attr:meta])* static ref $name:ident: { $Key:ty => $Val:ty } in $root:literal <= $cap:literal; )+ } => {
        $crate::cache::lazy_static! {
            $(
                $(#[$attr])*
                static ref $name: $crate::cache::FsBackedCache<$Key, $Val, ::std::path::PathBuf> =
                    $crate::cache::FsBackedCache::new(
                        $crate::cache::CACHE_PATH.join($root),
                        $cap
                    );
            )+
        }
    };
}

// re-export:
pub use make_static;

/// Create an anonymous file-system backed cache for one-time use.
///
/// # Example
/// ```rust
/// use windmill_common::anon;
/// let cache = anon!({ u64 => String } in "subdirectory" <= 1000);
/// ```
#[macro_export]
macro_rules! anon {
    ({ $Key:ty => $Val:ty } in $root:literal <= $cap:literal) => {{
        $crate::cache::make_static! {
            static ref __ANON__: { $Key => $Val } in $root <= $cap;
        }

        &__ANON__
    }};
}

// re-export:
pub use anon;

pub mod future {
    use super::*;

    /// Extension trait for futures that can be cached.
    pub trait FutureCachedExt<T: Import>: Future<Output = error::Result<T>> + Sized {
        /// Get or insert the future result in the cache.
        ///
        /// # Example
        /// ```rust
        /// use windmill_common::cache::{self, future::FutureCachedExt};
        /// use sqlx::types::Json;
        ///
        /// #[allow(unused)]
        /// async {
        ///     let result = std::future::ready(Ok(42u64))
        ///         .cached(cache::anon!({ u64 => u64 } in "test" <= 1), 42u64)
        ///         .await;
        ///
        ///     assert_eq!(result.unwrap(), 42u64);
        /// };
        /// ```
        fn cached<Key: Eq + Hash + Item + Clone, Val: Export<Untrusted = T>, Root: AsRef<Path>>(
            self,
            cache: &FsBackedCache<Key, Val, Root>,
            key: Key,
        ) -> impl Future<Output = error::Result<Val>> {
            cache.get_or_insert_async(key.to_owned(), self)
        }
    }

    impl<T: Import, F: Future<Output = error::Result<T>> + Sized> FutureCachedExt<T> for F {}
}

/// Flow data: i.e. a cached `raw_flow`.
/// Contains the original json raw value and a pre-parsed [`FlowValue`].
#[derive(Debug, Clone)]
pub struct FlowData {
    pub raw_flow: Box<RawValue>,
    pub flow: FlowValue,
}

impl FlowData {
    pub fn from_raw(raw_flow: Box<RawValue>) -> error::Result<Self> {
        let flow = serde_json::from_str(raw_flow.get())?;
        Ok(Self { raw_flow, flow })
    }

    pub fn value(&self) -> &FlowValue {
        &self.flow
    }
}

#[derive(Debug, Clone)]
pub struct ScriptData {
    pub lock: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone)]
pub enum RawData {
    Flow(Arc<FlowData>),
    Script(Arc<ScriptData>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptMetadata {
    pub language: Option<ScriptLang>,
    pub envs: Option<Vec<String>>,
    pub codebase: Option<String>,
}

#[derive(Debug)]
pub struct RawScript {
    pub content: String,
    pub lock: Option<String>,
    pub meta: Option<ScriptMetadata>,
}

#[derive(Debug)]
pub struct RawFlow {
    pub raw_flow: Box<RawValue>,
}

#[derive(Debug)]
pub struct RawNode {
    pub raw_code: Option<String>,
    pub raw_lock: Option<String>,
    pub raw_flow: Option<Box<RawValue>>,
}

#[derive(Debug, Clone)]
struct Entry<T>(Arc<T>);

#[derive(Debug, Clone)]
struct ScriptFull {
    pub data: Arc<ScriptData>,
    pub meta: Arc<ScriptMetadata>,
}

fn unwrap_or_error<Key: std::fmt::Debug, Val>(
    at: &'static Location,
    entity: &'static str,
    key: Key,
) -> impl FnOnce(Option<Val>) -> error::Result<Val> {
    move |optional| {
        optional
            .ok_or_else(|| error::Error::InternalErrAt(at, format!("{key:?}: {entity} not found")))
    }
}

/// Clear all caches.
pub fn clear() {
    flow::clear();
    script::clear();
    app::clear();
    job::clear();
}

pub mod flow {
    use super::*;

    make_static! {
        /// Flow node cache.
        /// FIXME: Use `Arc<Node>` for cheap cloning.
        static ref NODES: { FlowNodeId => RawData } in "flow" <= 1000;
        /// Flow version value cache (version id => value).
        static ref FLOWS: { i64 => Entry<FlowData> } in "flows" <= 1000;
        /// Flow version lite value cache (version id => value).
        static ref FLOWS_LITE: { i64 => Entry<FlowData> } in "flowslite" <= 1000;
    }

    /// Clear the flow cache.
    pub fn clear() {
        NODES.clear();
        FLOWS.clear();
        FLOWS_LITE.clear();
    }

    /// Fetch the flow node script referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    #[track_caller]
    pub fn fetch_script<'c>(
        e: impl PgExecutor<'c>,
        node: FlowNodeId,
    ) -> impl Future<Output = error::Result<Arc<ScriptData>>> {
        let fetch_node = fetch_node(e, node);
        async move {
            fetch_node.await.and_then(|data| match data {
                RawData::Script(data) => Ok(data),
                RawData::Flow(_) => Err(error::Error::internal_err(format!(
                    "Flow node ({:x}) isn't a script node.",
                    node.0
                ))),
            })
        }
    }

    /// Fetch the flow node flow value referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    #[track_caller]
    pub fn fetch_flow<'c>(
        e: impl PgExecutor<'c>,
        node: FlowNodeId,
    ) -> impl Future<Output = error::Result<Arc<FlowData>>> {
        let fetch_node = fetch_node(e, node);
        async move {
            fetch_node.await.and_then(|data| match data {
                RawData::Flow(data) => Ok(data),
                RawData::Script(_) => Err(error::Error::internal_err(format!(
                    "Flow node ({:x}) isn't a flow node.",
                    node.0
                ))),
            })
        }
    }

    /// Fetch the flow node referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    #[track_caller]
    pub(super) fn fetch_node<'c>(
        e: impl PgExecutor<'c>,
        node: FlowNodeId,
    ) -> impl Future<Output = error::Result<RawData>> {
        let loc = Location::caller();
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        NODES.get_or_insert_async(node, async move {
            sqlx::query!(
                "SELECT \
                    code AS \"raw_code: String\", \
                    lock AS \"raw_lock: String\", \
                    flow AS \"raw_flow: Json<Box<RawValue>>\" \
                FROM flow_node WHERE id = $1 LIMIT 1",
                node.0,
            )
            .fetch_optional(e)
            .await
            .map_err(Into::into)
            .and_then(unwrap_or_error(&loc, "Flow node", node))
            .map(|r| RawNode {
                raw_code: r.raw_code,
                raw_lock: r.raw_lock,
                raw_flow: r.raw_flow.map(|Json(raw_flow)| raw_flow),
            })
        })
    }

    #[track_caller]
    pub fn fetch_version<'c>(
        e: impl PgExecutor<'c>,
        id: i64,
    ) -> impl Future<Output = error::Result<Arc<FlowData>>> {
        let loc = Location::caller();
        let fut = FLOWS.get_or_insert_async(id, async move {
            sqlx::query_scalar!(
                "SELECT value AS \"value!: Json<Box<RawValue>>\"
                FROM flow_version WHERE id = $1 LIMIT 1",
                id,
            )
            .fetch_optional(e)
            .await
            .map_err(Into::into)
            .and_then(unwrap_or_error(&loc, "Flow version", id))
            .map(|Json(raw_flow)| RawFlow { raw_flow })
        });
        fut.map_ok(|Entry(data)| data)
    }

    #[track_caller]
    pub fn fetch_version_lite<'c>(
        e: impl PgExecutor<'c>,
        id: i64,
    ) -> impl Future<Output = error::Result<Arc<FlowData>>> {
        let loc = Location::caller();
        let fut = FLOWS_LITE.get_or_insert_async(id, async move {
            sqlx::query_scalar!(
                "SELECT value AS \"value!: Json<Box<RawValue>>\"
                FROM flow_version_lite WHERE id = $1 LIMIT 1",
                id,
            )
            .fetch_optional(e)
            .await
            .map_err(Into::into)
            .and_then(unwrap_or_error(&loc, "Flow version \"lite\"", id))
            .map(|Json(raw_flow)| RawFlow { raw_flow })
        });
        fut.map_ok(|Entry(data)| data)
    }
}

pub mod script {
    use super::*;

    make_static! {
        /// Scripts cache.
        /// FIXME: Use `Arc<Val>` for cheap cloning.
        static ref CACHE: { ScriptHash => ScriptFull } in "script" <= 1000;
    }

    /// Clear the script cache.
    pub fn clear() {
        CACHE.clear();
    }

    /// Fetch the script referenced by `hash` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    #[track_caller]
    pub fn fetch<'c>(
        e: impl PgExecutor<'c>,
        hash: ScriptHash,
    ) -> impl Future<Output = error::Result<(Arc<ScriptData>, Arc<ScriptMetadata>)>> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        let loc = Location::caller();
        let fut = CACHE.get_or_insert_async(hash, async move {
            sqlx::query!(
                "SELECT \
                    content AS \"content!: String\",
                    lock AS \"lock: String\", \
                    language AS \"language: Option<ScriptLang>\", \
                    envs AS \"envs: Vec<String>\", \
                    codebase LIKE '%.tar' as use_tar \
                FROM script WHERE hash = $1 LIMIT 1",
                hash.0
            )
            .fetch_optional(e)
            .await
            .map_err(Into::into)
            .and_then(unwrap_or_error(&loc, "Script", hash))
            .map(|r| RawScript {
                content: r.content,
                lock: r.lock,
                meta: Some(ScriptMetadata {
                    language: r.language,
                    envs: r.envs,
                    codebase: if let Some(use_tar) = r.use_tar {
                        let sh = hash.to_string();
                        if use_tar {
                            Some(format!("{sh}.tar"))
                        } else {
                            Some(sh)
                        }
                    } else {
                        None
                    },
                }),
            })
        });
        fut.map_ok(|ScriptFull { data, meta }| (data, meta))
    }

    /// Invalidate the script cache for the given `hash`.
    pub fn invalidate(hash: ScriptHash) {
        let _ = CACHE.remove(&hash);
    }
}

pub mod app {
    use super::*;

    make_static! {
        /// App scripts cache.
        static ref CACHE: { AppScriptId => Entry<ScriptData> } in "app" <= 1000;
    }

    /// Clear the app scripts cache.
    pub fn clear() {
        CACHE.clear();
    }

    /// Fetch the app script referenced by `id` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    #[track_caller]
    pub fn fetch_script<'c>(
        e: impl PgExecutor<'c>,
        id: AppScriptId,
    ) -> impl Future<Output = error::Result<Arc<ScriptData>>> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        let loc = Location::caller();
        let fut = CACHE.get_or_insert_async(id, async move {
            sqlx::query!(
                "SELECT lock, code FROM app_script WHERE id = $1 LIMIT 1",
                id.0,
            )
            .fetch_optional(e)
            .await
            .map_err(Into::into)
            .and_then(unwrap_or_error(&loc, "Application script", id))
            .map(|r| RawScript { content: r.code, lock: r.lock, meta: None })
        });
        fut.map_ok(|Entry(data)| data)
    }
}

pub mod job {
    use super::*;
    use crate::jobs::JobKind;

    #[cfg(not(feature = "scoped_cache"))]
    lazy_static! {
        /// Very small in-memory cache for "preview" jobs raw data.
        static ref PREVIEWS: Cache<Uuid, RawData> = Cache::new(50);
    }

    #[cfg(feature = "scoped_cache")]
    lazy_static! {
        /// Very small in-memory cache for "preview" jobs raw data.
        static ref PREVIEWS: Cache<(ThreadId, Uuid), RawData> = Cache::new(50);
    }

    /// Clear the job cache.
    pub fn clear() {
        PREVIEWS.clear();
    }

    #[track_caller]
    pub fn fetch_preview_flow<'a, 'c>(
        e: impl PgExecutor<'c> + 'a,
        job: &'a Uuid,
        // original raw values from `queue` or `completed_job` tables:
        // kept for backward compatibility.
        raw_flow: Option<Json<Box<RawValue>>>,
    ) -> impl Future<Output = error::Result<Arc<FlowData>>> + 'a {
        let fetch_preview = fetch_preview(e, job, None, None, raw_flow);
        async move {
            fetch_preview.await.and_then(|data| match data {
                RawData::Flow(data) => Ok(data),
                RawData::Script(_) => Err(error::Error::internal_err(format!(
                    "Job ({job}) isn't a flow job."
                ))),
            })
        }
    }

    #[track_caller]
    pub fn fetch_preview_script<'a, 'c>(
        e: impl PgExecutor<'c> + 'a,
        job: &'a Uuid,
        // original raw values from `queue` or `completed_job` tables:
        // kept for backward compatibility.
        raw_lock: Option<String>,
        raw_code: Option<String>,
    ) -> impl Future<Output = error::Result<Arc<ScriptData>>> + 'a {
        let fetch_preview = fetch_preview(e, job, raw_lock, raw_code, None);
        async move {
            fetch_preview.await.and_then(|data| match data {
                RawData::Script(data) => Ok(data),
                RawData::Flow(_) => Err(error::Error::internal_err(format!(
                    "Job ({job}) isn't a script job."
                ))),
            })
        }
    }

    #[track_caller]
    pub fn fetch_preview<'a, 'c>(
        e: impl PgExecutor<'c> + 'a,
        job: &'a Uuid,
        // original raw values from `queue` or `completed_job` tables:
        // kept for backward compatibility.
        raw_lock: Option<String>,
        raw_code: Option<String>,
        raw_flow: Option<Json<Box<RawValue>>>,
    ) -> impl Future<Output = error::Result<RawData>> + 'a {
        let loc = Location::caller();
        let fetch = async move {
            match (raw_lock, raw_code, raw_flow) {
                (None, None, None) => sqlx::query!(
                    "SELECT raw_code, raw_lock, raw_flow AS \"raw_flow: Json<Box<RawValue>>\" \
                    FROM v2_job WHERE id = $1 LIMIT 1",
                    job
                )
                .fetch_optional(e)
                .await
                .map_err(Into::into)
                .and_then(unwrap_or_error(&loc, "Preview", job))
                .map(|r| (r.raw_lock, r.raw_code, r.raw_flow)),
                (lock, code, flow) => Ok((lock, code, flow)),
            }
            .and_then(|(lock, code, flow)| match flow {
                Some(Json(flow)) => FlowData::from_raw(flow).map(Arc::new).map(RawData::Flow),
                _ => Ok(RawData::Script(Arc::new(ScriptData {
                    code: code.unwrap_or_default(),
                    lock,
                }))),
            })
        };
        #[cfg(not(feature = "scoped_cache"))]
        return PREVIEWS.get_or_insert_async(job, fetch);
        #[cfg(feature = "scoped_cache")]
        async move {
            let job = &(std::thread::current().id(), job.clone());
            PREVIEWS.get_or_insert_async(job, fetch).await
        }
    }

    #[track_caller]
    pub fn fetch_script<'c>(
        e: impl PgExecutor<'c>,
        kind: JobKind,
        hash: Option<ScriptHash>,
    ) -> impl Future<Output = error::Result<Arc<ScriptData>>> {
        use JobKind::*;
        let loc = Location::caller();
        async move {
            match (kind, hash.map(|ScriptHash(id)| id)) {
                (FlowScript, Some(id)) => flow::fetch_script(e, FlowNodeId(id)).await,
                (Script | Dependencies, Some(hash)) => script::fetch(e, ScriptHash(hash))
                    .await
                    .map(|(data, _meta)| data),
                (AppScript, Some(id)) => app::fetch_script(e, AppScriptId(id)).await,
                _ => Err(error::Error::internal_err(format!(
                    "Isn't a script job: {:?}",
                    kind
                ))),
            }
            .map_err(error::relocate_internal(loc))
        }
    }

    #[track_caller]
    pub fn fetch_flow<'c>(
        e: impl PgExecutor<'c> + Copy,
        kind: JobKind,
        hash: Option<ScriptHash>,
    ) -> impl Future<Output = error::Result<Arc<FlowData>>> {
        use JobKind::*;
        let loc = Location::caller();
        async move {
            match (kind, hash.map(|ScriptHash(id)| id)) {
                (FlowDependencies, Some(id)) => flow::fetch_version(e, id).await,
                (FlowNode, Some(id)) => flow::fetch_flow(e, FlowNodeId(id)).await,
                (Flow, Some(id)) => match flow::fetch_version_lite(e, id).await {
                    Ok(raw_flow) => Ok(raw_flow),
                    Err(_) => flow::fetch_version(e, id).await,
                },
                _ => Err(error::Error::internal_err(format!(
                    "Isn't a flow job {:?}",
                    kind
                ))),
            }
            .map_err(error::relocate_internal(loc))
        }
    }
}

const _: () = {
    impl Import for RawFlow {
        fn import(src: &impl Storage) -> error::Result<Self> {
            Ok(Self { raw_flow: src.get_json_raw("flow.json")? })
        }
    }

    impl Export for FlowData {
        type Untrusted = RawFlow;

        fn resolve(src: Self::Untrusted) -> error::Result<Self> {
            FlowData::from_raw(src.raw_flow)
        }

        fn export(&self, dst: &impl Storage) -> error::Result<()> {
            Ok(dst.put("flow.json", self.raw_flow.get().as_bytes())?)
        }
    }

    impl Import for RawScript {
        fn import(src: &impl Storage) -> error::Result<Self> {
            let content = src.get_utf8("code.txt")?;
            let lock = src.get_utf8("lock.txt").ok();
            let meta = src.get_json("info.json").ok();
            Ok(Self { content, lock, meta })
        }
    }

    impl Export for ScriptData {
        type Untrusted = RawScript;

        fn resolve(src: Self::Untrusted) -> error::Result<Self> {
            Ok(ScriptData { code: src.content, lock: src.lock })
        }

        fn export(&self, dst: &impl Storage) -> error::Result<()> {
            dst.put("code.txt", self.code.as_bytes())?;
            if let Some(lock) = self.lock.as_ref() {
                dst.put("lock.txt", lock.as_bytes())?;
            }
            Ok(())
        }
    }

    impl Export for ScriptFull {
        type Untrusted = RawScript;

        fn resolve(mut src: Self::Untrusted) -> error::Result<Self> {
            let Some(meta) = src.meta.take() else {
                return Err(error::Error::internal_err("Invalid script src".to_string()));
            };
            Ok(ScriptFull {
                data: Arc::new(ScriptData { code: src.content, lock: src.lock }),
                meta: Arc::new(meta),
            })
        }

        fn export(&self, dst: &impl Storage) -> error::Result<()> {
            self.data.export(dst)?;
            self.meta.export(dst)?;
            Ok(())
        }
    }

    impl Import for RawNode {
        fn import(src: &impl Storage) -> error::Result<Self> {
            let code = src.get_utf8("code.txt").ok();
            let lock = src.get_utf8("lock.txt").ok();
            let flow = src.get_json_raw("flow.json").ok();
            Ok(Self { raw_code: code, raw_lock: lock, raw_flow: flow })
        }
    }

    impl Export for RawData {
        type Untrusted = RawNode;

        fn resolve(src: Self::Untrusted) -> error::Result<Self> {
            match src {
                RawNode { raw_flow: Some(flow), .. } => {
                    FlowData::from_raw(flow).map(Arc::new).map(Self::Flow)
                }
                RawNode { raw_code: Some(code), raw_lock: lock, .. } => {
                    Ok(Self::Script(Arc::new(ScriptData { code, lock })))
                }
                _ => Err(error::Error::internal_err(
                    "Invalid raw data src".to_string(),
                )),
            }
        }

        fn export(&self, dst: &impl Storage) -> error::Result<()> {
            match self {
                RawData::Flow(data) => data.export(dst),
                RawData::Script(data) => data.export(dst),
            }
        }
    }

    impl<T: Export> Export for Entry<T> {
        type Untrusted = T::Untrusted;

        fn resolve(src: Self::Untrusted) -> error::Result<Self> {
            Ok(Entry(Arc::new(T::resolve(src)?)))
        }

        fn export(&self, dst: &impl Storage) -> error::Result<()> {
            self.0.export(dst)
        }
    }

    impl<T: for<'de> Deserialize<'de> + Serialize> Import for T {
        fn import(src: &impl Storage) -> error::Result<Self> {
            let data = src.get("self.json")?;
            Ok(serde_json::from_slice(&data)?)
        }
    }

    impl<T: Clone + for<'de> Deserialize<'de> + Serialize> Export for T {
        type Untrusted = Self;

        fn resolve(src: Self::Untrusted) -> error::Result<Self> {
            Ok(src)
        }

        fn export(&self, dst: &impl Storage) -> error::Result<()> {
            Ok(dst.put("self.json", serde_json::to_vec(self)?)?)
        }
    }

    impl<T: AsRef<Path>> Storage for T {
        fn get(&self, item: impl Item) -> std::io::Result<Vec<u8>> {
            use std::fs::OpenOptions;
            use std::io::Read;

            OpenOptions::new()
                .read(true)
                .open(item.path(self))
                .and_then(|mut file| {
                    let mut buf = vec![];
                    file.read_to_end(&mut buf)?;
                    Ok(buf)
                })
        }

        fn put(&self, item: impl Item, data: impl AsRef<[u8]>) -> std::io::Result<()> {
            use std::fs::OpenOptions;
            use std::io::Write;

            OpenOptions::new()
                .write(true)
                .create(true)
                .open(item.path(self))
                .and_then(|mut file| file.write_all(data.as_ref()))
        }
    }

    macro_rules! impl_item {
        ($( ($t:ty, |$x:ident| $join:expr) ),*) => {
            $(
                impl Item for $t {
                    fn path(&self, root: impl AsRef<Path>) -> PathBuf {
                        let $x = self;
                        root.as_ref().join($join)
                    }
                }
            )*
        };
    }

    impl_item! {
        (&'static str, |x| x),
        (i64, |x| format!("{:016x}", *x as u64)),
        (u64, |x| format!("{:016x}", x)),
        (Uuid, |x| format!("{:032x}", x.as_u128())),
        (ScriptHash, |x| format!("{:016x}", x.0)),
        (FlowNodeId, |x| format!("{:016x}", x.0)),
        (AppScriptId, |x| format!("{:016x}", x.0))
    }

    #[cfg(feature = "scoped_cache")]
    impl<T: Item> Item for (ThreadId, T) {
        fn path(&self, root: impl AsRef<Path>) -> PathBuf {
            let (id, item) = self;
            item.path(root.as_ref().join(format!("{id:?}")))
        }
    }
};
