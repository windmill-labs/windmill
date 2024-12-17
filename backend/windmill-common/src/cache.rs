use crate::apps::AppScriptId;
use crate::error;
use crate::flows::FlowNodeId;
use crate::flows::FlowValue;
use crate::scripts::ScriptHash;
use crate::scripts::ScriptLang;

use std::future::Future;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::future::TryFutureExt;
use quick_cache::Equivalent;
use serde::{Deserialize, Serialize};
use sqlx::types::{Json, JsonRawValue as RawValue};
use sqlx::PgExecutor;

pub use const_format::concatcp;
pub use lazy_static::lazy_static;
pub use quick_cache::sync::Cache;

/// Cache directory for windmill server/worker(s).
pub const CACHE_DIR: &str = "/tmp/windmill/cache/";

/// A file-system backed concurrent cache.
pub struct FsBackedCache<Key, Val> {
    cache: Cache<Key, Val>,
    root: &'static str,
}

impl<Key: Eq + Hash + fs::Item, Val: Clone> FsBackedCache<Key, Val> {
    /// Create a new file-system backed cache with `items_capacity` capacity.
    /// The cache will be stored in the `root` directory.
    pub fn new(root: &'static str, items_capacity: usize) -> Self {
        Self { cache: Cache::new(items_capacity), root }
    }

    /// Build a path for the given `key`.
    pub fn path(&self, key: &Key) -> PathBuf {
        key.path(self.root)
    }

    /// Remove the item with the given `key` from the cache.
    pub fn remove(&self, key: &Key) -> Option<(Key, Val)> {
        let _ = std::fs::remove_dir_all(self.path(key));
        self.cache.remove(key)
    }

    /// Gets or inserts an item in the cache with key `key`.
    pub async fn get_or_insert_async<'a, T: fs::Bundle, Q, F>(
        &'a self,
        key: &Q,
        map: impl Fn(T) -> Val,
        with: F,
    ) -> error::Result<Val>
    where
        Q: Hash + Equivalent<Key> + ToOwned<Owned = Key>,
        F: Future<Output = error::Result<T>>,
    {
        self.cache
            .get_or_insert_async(key, async {
                let key = key.to_owned();
                fs::import_or_insert_with(self.path(&key), with)
                    .await
                    .map(map)
            })
            .await
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
                static ref $name: $crate::cache::FsBackedCache<$Key, $Val> =
                    $crate::cache::FsBackedCache::new(
                        $crate::cache::concatcp!($crate::cache::CACHE_DIR, $root),
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
    pub trait FutureCachedExt<T: fs::Bundle>: Future<Output = error::Result<T>> + Sized {
        /// Get or insert the future result in the cache.
        ///
        /// # Example
        /// ```rust
        /// use windmill_common::cache::{self, future::FutureCachedExt};
        /// use sqlx::types::Json;
        ///
        /// #[allow(unused)]
        /// async {
        ///     let result = std::future::ready(Ok(Json(42)))
        ///         .cached(cache::anon!({ u64 => Json<u64> } in "test" <= 1), &42, |x| x)
        ///         .await;
        ///
        ///     assert_eq!(result.unwrap(), Json(42));
        /// };
        /// ```
        fn cached<Key: Eq + Hash + fs::Item, Val: Clone, Q>(
            self,
            cache: &FsBackedCache<Key, Val>,
            key: &Q,
            map: impl Fn(T) -> Val,
        ) -> impl Future<Output = error::Result<Val>>
        where
            Q: Hash + Equivalent<Key> + ToOwned<Owned = Key>,
        {
            cache.get_or_insert_async(key, map, self)
        }
    }

    impl<T: fs::Bundle, F: Future<Output = error::Result<T>> + Sized> FutureCachedExt<T> for F {}
}

/// Flow data: i.e. a cached `raw_flow`.
/// Contains the original json raw value and a pre-parsed [`FlowValue`].
#[derive(Debug, Clone)]
pub struct FlowData {
    pub raw_flow: Box<RawValue>,
    pub flow: Result<FlowValue, String>,
}

impl FlowData {
    pub fn from_utf8(vec: Vec<u8>) -> error::Result<Self> {
        Ok(Self::from_raw(RawValue::from_string(String::from_utf8(
            vec,
        )?)?))
    }

    pub fn from_raw(raw_flow: Box<RawValue>) -> Self {
        let flow = serde_json::from_str(raw_flow.get())
            .map_err(|e| format!("Invalid flow value: {:?}", e));
        Self { raw_flow, flow }
    }

    pub fn value(&self) -> error::Result<&FlowValue> {
        self.flow
            .as_ref()
            .map_err(|err| error::Error::InternalErr(err.clone()))
    }
}

impl Default for FlowData {
    fn default() -> Self {
        Self { raw_flow: Default::default(), flow: Err(Default::default()) }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScriptData {
    pub lock: Option<String>,
    pub code: String,
}

impl ScriptData {
    pub fn from_raw(lock: Option<String>, code: Option<String>) -> Self {
        let lock = lock.and_then(|x| if x.is_empty() { None } else { Some(x) });
        let code = code.unwrap_or_default();
        Self { lock, code }
    }
}

#[derive(Debug, Clone)]
pub enum RawData {
    Flow(Arc<FlowData>),
    Script(Arc<ScriptData>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ScriptMetadata {
    pub language: Option<ScriptLang>,
    pub envs: Option<Vec<String>>,
    pub codebase: Option<String>,
}

const _: () = {
    impl fs::Bundle for FlowData {
        type Item = &'static str;

        fn items() -> impl Iterator<Item = Self::Item> {
            ["flow.json"].into_iter()
        }

        fn import(&mut self, _: Self::Item, data: Vec<u8>) -> error::Result<()> {
            *self = Self::from_utf8(data)?;
            Ok(())
        }

        fn export(&self, _: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match self.raw_flow.get().is_empty() {
                false => Ok(Some(self.raw_flow.get().as_bytes().to_vec())),
                true => Ok(None),
            }
        }
    }

    impl fs::Bundle for ScriptData {
        type Item = &'static str;

        fn items() -> impl Iterator<Item = Self::Item> {
            ["lock.txt", "code.txt"].into_iter()
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            match item {
                "lock.txt" => self.lock = Some(String::from_utf8(data)?),
                "code.txt" => self.code = String::from_utf8(data)?,
                _ => {}
            }
            Ok(())
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match item {
                "lock.txt" => Ok(self.lock.as_ref().map(|s| s.as_bytes().to_vec())),
                "code.txt" if !self.code.is_empty() => Ok(Some(self.code.as_bytes().to_vec())),
                _ => Ok(None),
            }
        }
    }

    impl fs::Bundle for ScriptMetadata {
        type Item = &'static str;

        fn items() -> impl Iterator<Item = Self::Item> {
            ["info.json"].into_iter()
        }

        fn import(&mut self, _: Self::Item, data: Vec<u8>) -> error::Result<()> {
            *self = serde_json::from_slice(&data)?;
            Ok(())
        }

        fn export(&self, _: Self::Item) -> error::Result<Option<Vec<u8>>> {
            Ok(Some(serde_json::to_vec(self)?))
        }
    }
};

pub mod flow {
    use super::*;

    make_static! {
        /// Flow node cache.
        /// FIXME: Use `Arc<Node>` for cheap cloning.
        static ref NODES: { FlowNodeId => RawData } in "flow" <= 1000;
        /// Flow version value cache (version id => value).
        static ref FLOWS: { i64 => Arc<FlowData> } in "flows" <= 1000;
        /// Flow version lite value cache (version id => value).
        static ref FLOWS_LITE: { i64 => Arc<FlowData> } in "flowslite" <= 1000;
    }

    /// Fetch the flow node script referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch_script(
        e: impl PgExecutor<'_>,
        node: FlowNodeId,
    ) -> error::Result<Arc<ScriptData>> {
        fetch_node(e, node).await.and_then(|data| match data {
            RawData::Script(data) => Ok(data),
            RawData::Flow(_) => Err(error::Error::InternalErr(format!(
                "Flow node ({:x}) isn't a script node.",
                node.0
            ))),
        })
    }

    /// Fetch the flow node flow value referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch_flow(
        e: impl PgExecutor<'_>,
        node: FlowNodeId,
    ) -> error::Result<Arc<FlowData>> {
        fetch_node(e, node).await.and_then(|data| match data {
            RawData::Flow(data) => Ok(data),
            RawData::Script(_) => Err(error::Error::InternalErr(format!(
                "Flow node ({:x}) isn't a flow node.",
                node.0
            ))),
        })
    }

    /// Fetch the flow node referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub(super) async fn fetch_node(
        e: impl PgExecutor<'_>,
        node: FlowNodeId,
    ) -> error::Result<RawData> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        NODES
            .get_or_insert_async(
                &node,
                |(script, flow)| match flow {
                    Some(flow) => RawData::Flow(Arc::new(flow)),
                    _ => RawData::Script(Arc::new(script)),
                },
                async {
                    sqlx::query!(
                        "SELECT \
                            lock AS \"lock: String\", \
                            code AS \"code: String\", \
                            flow AS \"flow: Json<Box<RawValue>>\" \
                        FROM flow_node WHERE id = $1 LIMIT 1",
                        node.0,
                    )
                    .fetch_one(e)
                    .await
                    .map_err(Into::into)
                    .map(|r| {
                        (
                            ScriptData::from_raw(r.lock, r.code),
                            r.flow.map(|Json(raw_flow)| FlowData::from_raw(raw_flow)),
                        )
                    })
                },
            )
            .await
    }

    pub async fn fetch_version(e: impl PgExecutor<'_>, id: i64) -> error::Result<Arc<FlowData>> {
        FLOWS
            .get_or_insert_async(&id, Arc::new, async {
                sqlx::query_scalar!(
                    "SELECT value AS \"value!: Json<Box<RawValue>>\"
                    FROM flow_version WHERE id = $1 LIMIT 1",
                    id,
                )
                .fetch_one(e)
                .await
                .map_err(Into::into)
                .map(|Json(raw_flow)| FlowData::from_raw(raw_flow))
            })
            .await
    }

    pub async fn fetch_version_lite(
        e: impl PgExecutor<'_>,
        id: i64,
    ) -> error::Result<Arc<FlowData>> {
        FLOWS_LITE
            .get_or_insert_async(&id, Arc::new, async {
                sqlx::query_scalar!(
                    "SELECT value AS \"value!: Json<Box<RawValue>>\"
                    FROM flow_version_lite WHERE id = $1 LIMIT 1",
                    id,
                )
                .fetch_one(e)
                .await
                .map_err(Into::into)
                .map(|Json(raw_flow)| FlowData::from_raw(raw_flow))
            })
            .await
    }
}

pub mod script {
    use super::*;

    make_static! {
        /// Scripts cache.
        /// FIXME: Use `Arc<Val>` for cheap cloning.
        static ref CACHE: { ScriptHash => (Arc<ScriptData>, Arc<ScriptMetadata>) } in "script" <= 1000;
    }

    /// Fetch the script referenced by `hash` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch(
        e: impl PgExecutor<'_>,
        hash: ScriptHash,
    ) -> error::Result<(Arc<ScriptData>, Arc<ScriptMetadata>)> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        CACHE
            .get_or_insert_async(
                &hash,
                |(data, metadata)| (Arc::new(data), Arc::new(metadata)),
                async {
                    sqlx::query!(
                        "SELECT \
                            lock AS \"lock: String\", \
                            content AS \"code!: String\",
                            language AS \"language: Option<ScriptLang>\", \
                            envs AS \"envs: Vec<String>\", \
                            codebase AS \"codebase: String\" \
                        FROM script WHERE hash = $1 LIMIT 1",
                        hash.0
                    )
                    .fetch_one(e)
                    .await
                    .map_err(Into::into)
                    .map(|r| {
                        (
                            ScriptData::from_raw(r.lock, Some(r.code)),
                            ScriptMetadata {
                                language: r.language,
                                envs: r.envs,
                                codebase: r.codebase,
                            },
                        )
                    })
                },
            )
            .await
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
        static ref CACHE: { AppScriptId => Arc<ScriptData> } in "app" <= 1000;
    }

    /// Fetch the app script referenced by `id` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch_script(
        e: impl PgExecutor<'_>,
        id: AppScriptId,
    ) -> error::Result<Arc<ScriptData>> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        CACHE
            .get_or_insert_async(&id, Arc::new, async {
                sqlx::query!(
                    "SELECT lock, code FROM app_script WHERE id = $1 LIMIT 1",
                    id.0,
                )
                .fetch_one(e)
                .await
                .map_err(Into::into)
                .map(|r| ScriptData::from_raw(r.lock, Some(r.code)))
            })
            .await
    }
}

pub mod job {
    use super::*;
    use crate::jobs::JobKind;

    use uuid::Uuid;

    lazy_static! {
        /// Very small in-memory cache for "preview" jobs raw data.
        static ref PREVIEWS: Cache<Uuid, RawData> = Cache::new(50);
    }

    pub async fn fetch_preview_flow(
        e: impl PgExecutor<'_>,
        job: &Uuid,
        // original raw values from `queue` or `completed_job` tables:
        // kept for backward compatibility.
        raw_flow: Option<Json<Box<RawValue>>>,
    ) -> error::Result<Arc<FlowData>> {
        fetch_preview(e, job, None, None, raw_flow)
            .await
            .and_then(|data| match data {
                RawData::Flow(data) => Ok(data),
                RawData::Script(_) => Err(error::Error::InternalErr(format!(
                    "Job ({job}) isn't a flow job."
                ))),
            })
    }

    pub async fn fetch_preview_script(
        e: impl PgExecutor<'_>,
        job: &Uuid,
        // original raw values from `queue` or `completed_job` tables:
        // kept for backward compatibility.
        raw_lock: Option<String>,
        raw_code: Option<String>,
    ) -> error::Result<Arc<ScriptData>> {
        fetch_preview(e, job, raw_lock, raw_code, None)
            .await
            .and_then(|data| match data {
                RawData::Script(data) => Ok(data),
                RawData::Flow(_) => Err(error::Error::InternalErr(format!(
                    "Job ({job}) isn't a script job."
                ))),
            })
    }

    pub async fn fetch_preview(
        e: impl PgExecutor<'_>,
        job: &Uuid,
        // original raw values from `queue` or `completed_job` tables:
        // kept for backward compatibility.
        raw_lock: Option<String>,
        raw_code: Option<String>,
        raw_flow: Option<Json<Box<RawValue>>>,
    ) -> error::Result<RawData> {
        PREVIEWS
            .get_or_insert_async(job, async {
                match (raw_lock, raw_code, raw_flow) {
                    (None, None, None) => sqlx::query!(
                        "SELECT raw_code, raw_lock, raw_flow AS \"raw_flow: Json<Box<RawValue>>\" \
                        FROM job WHERE id = $1 LIMIT 1",
                        job
                    )
                    .fetch_one(e)
                    .map_err(Into::into)
                    .await
                    .map(|r| (r.raw_lock, r.raw_code, r.raw_flow)),
                    (lock, code, flow) => Ok((lock, code, flow)),
                }
                .map(|(lock, code, flow)| match flow {
                    Some(Json(flow)) => RawData::Flow(Arc::new(FlowData::from_raw(flow))),
                    _ => RawData::Script(Arc::new(ScriptData::from_raw(lock, code))),
                })
            })
            .await
    }

    pub async fn fetch_script(
        e: impl PgExecutor<'_>,
        kind: JobKind,
        hash: Option<ScriptHash>,
    ) -> error::Result<Arc<ScriptData>> {
        use JobKind::*;
        match (kind, hash.map(|ScriptHash(id)| id)) {
            (FlowScript, Some(id)) => flow::fetch_script(e, FlowNodeId(id)).await,
            (Script | Dependencies, Some(hash)) => script::fetch(e, ScriptHash(hash))
                .await
                .map(|(raw_script, _metadata)| raw_script),
            (AppScript, Some(id)) => app::fetch_script(e, AppScriptId(id)).await,
            _ => Err(error::Error::InternalErr(format!(
                "Isn't a script job: {:?}",
                kind
            ))),
        }
    }

    pub async fn fetch_flow(
        e: impl PgExecutor<'_> + Copy,
        kind: JobKind,
        hash: Option<ScriptHash>,
    ) -> error::Result<Arc<FlowData>> {
        use JobKind::*;
        match (kind, hash.map(|ScriptHash(id)| id)) {
            (FlowDependencies, Some(id)) => flow::fetch_version(e, id).await,
            (FlowNode, Some(id)) => flow::fetch_flow(e, FlowNodeId(id)).await,
            (Flow, Some(id)) => match flow::fetch_version_lite(e, id).await {
                Ok(raw_flow) => Ok(raw_flow),
                Err(_) => flow::fetch_version(e, id).await,
            },
            _ => Err(error::Error::InternalErr(format!(
                "Isn't a flow job {:?}",
                kind
            ))),
        }
    }
}

mod fs {
    use super::*;

    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};

    use uuid::Uuid;

    /// A bundle of items that can be imported/exported from/into the file-system.
    pub trait Bundle: Default {
        /// Item type of the bundle.
        type Item: Item + Copy;
        /// Returns a slice of all items than **can** exists within the bundle.
        fn items() -> impl Iterator<Item = Self::Item>;
        /// Import the given `data` into the `item`.
        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()>;
        /// Export the `item` into a `Vec<u8>`.
        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>>;
    }

    /// An item that can be imported/exported from/into the file-system.
    pub trait Item: Sized {
        /// Returns the path of the item within the given `root` path.
        fn path(&self, root: impl AsRef<Path>) -> PathBuf;
    }

    /// Import or insert a bundle within the given combination of `{root}/{key}/`.
    pub async fn import_or_insert_with<T, F>(path: impl AsRef<Path>, f: F) -> error::Result<T>
    where
        T: Bundle,
        F: Future<Output = error::Result<T>>,
    {
        let path = path.as_ref();
        // Retrieve the data from the cache directory or the database.
        if fs::metadata(path).is_ok() {
            // Cache path exists, read its contents.
            let import = || -> error::Result<T> {
                let mut data = T::default();
                for item in T::items() {
                    let mut buf = vec![];
                    let Ok(mut file) = OpenOptions::new().read(true).open(item.path(path)) else {
                        continue;
                    };
                    file.read_to_end(&mut buf)?;
                    data.import(item, buf)?;
                }
                tracing::debug!("Imported from file-system: {:?}", path);
                Ok(data)
            };
            match import() {
                Ok(data) => return Ok(data),
                Err(err) => tracing::warn!(
                    "Failed to import from file-system, fetch source..: {path:?}: {err:?}"
                ),
            }
        }
        // Cache path doesn't exist or import failed, generate the content.
        let data = f.await?;
        let export = |data: &T| -> error::Result<()> {
            fs::create_dir_all(path)?;
            // Write the generated data to the file.
            for item in T::items() {
                let Some(buf) = data.export(item)? else {
                    continue;
                };
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(item.path(path))?;
                file.write_all(&buf)?;
            }
            tracing::debug!("Exported to file-system: {:?}", path);
            Ok(())
        };
        // Try to export data to the file-system.
        // If failed, remove the directory but still return the data.
        if let Err(err) = export(&data) {
            tracing::warn!("Failed to export to file-system: {path:?}: {err:?}");
            let _ = fs::remove_dir_all(path);
        }
        Ok(data)
    }

    // Implement `Bundle`.

    // Empty bundle.
    impl Bundle for () {
        type Item = &'static str;

        fn items() -> impl Iterator<Item = Self::Item> {
            [].into_iter()
        }

        fn import(&mut self, _: Self::Item, _: Vec<u8>) -> error::Result<()> {
            Ok(())
        }

        fn export(&self, _: Self::Item) -> error::Result<Option<Vec<u8>>> {
            Ok(None)
        }
    }

    // JSON bundle.
    impl<T: for<'de> Deserialize<'de> + Serialize + Default> Bundle for Json<T> {
        type Item = &'static str;

        fn items() -> impl Iterator<Item = Self::Item> {
            ["self.json"].into_iter()
        }

        fn import(&mut self, _: Self::Item, data: Vec<u8>) -> error::Result<()> {
            self.0 = serde_json::from_slice(&data)?;
            Ok(())
        }

        fn export(&self, _: Self::Item) -> error::Result<Option<Vec<u8>>> {
            Ok(Some(serde_json::to_vec(&self.0)?))
        }
    }

    // Optional bundle.
    impl<T: Bundle> Bundle for Option<T> {
        type Item = T::Item;

        fn items() -> impl Iterator<Item = Self::Item> {
            T::items()
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            let mut x = T::default();
            x.import(item, data)?;
            *self = Some(x);
            Ok(())
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match self {
                Some(x) => x.export(item),
                _ => Ok(None),
            }
        }
    }

    // Bundle pair.
    impl<I: Item + Copy + PartialEq, A: Bundle<Item = I>, B: Bundle<Item = I>> Bundle for (A, B) {
        type Item = I;

        fn items() -> impl Iterator<Item = Self::Item> {
            A::items().chain(B::items())
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            match A::items().any(|i| i == item) {
                true => self.0.import(item, data),
                _ => self.1.import(item, data),
            }
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match A::items().any(|i| i == item) {
                true => self.0.export(item),
                _ => self.1.export(item),
            }
        }
    }

    // Implement `Item`.

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

    #[cfg(test)]
    #[test]
    fn test_items() {
        let p = "test".path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/test"));
        let p = i64::MAX.path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/7fffffffffffffff"));
        let p = u64::MAX.path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/ffffffffffffffff"));
        let p = Uuid::from_u128(u128::MAX).path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/ffffffffffffffffffffffffffffffff"));
        let p = ScriptHash(i64::MAX).path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/7fffffffffffffff"));
        let p = FlowNodeId(i64::MAX).path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/7fffffffffffffff"));
        let p = AppScriptId(i64::MAX).path("/tmp");
        assert_eq!(p, PathBuf::from("/tmp/7fffffffffffffff"));
    }
}
