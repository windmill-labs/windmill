use crate::error;

use std::path::{Path, PathBuf};

use quick_cache::sync::Cache;
use sqlx::PgExecutor;

/// Cache directory for windmill server/worker(s).
pub const CACHE_DIR: &str = "/tmp/windmill/cache/";

pub mod flow {
    use super::*;
    use crate::flows::{FlowNodeId, FlowValue};

    /// Cache directory for windmill server/worker(s) flow nodes.
    pub const CACHE_DIR: &str = const_format::concatcp!(super::CACHE_DIR, "flow");

    lazy_static::lazy_static! {
        /// Flow node cache.
        /// FIXME: This should be a static but [`Cache`] does not have a const constructor.
        /// FIXME: Use `Arc<Val>` for cheap cloning.
        static ref CACHE: Cache<FlowNodeId, Val> = Cache::new(1000);
    }

    /// Flow node cache value.
    #[derive(Debug, Clone, Default)]
    struct Val {
        lock: Option<String>,
        code: Option<String>,
        flow: Option<FlowValue>,
    }

    /// Fetch the flow node script referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch_script(e: impl PgExecutor<'_>, node: FlowNodeId)
        -> error::Result<(Option<String>, String)>
    {
        fetch(e, node).await.and_then(|Val { lock, code, .. }| Ok((lock, code.ok_or_else(|| {
            error::Error::InternalErr(format!("Flow node ({:x}) isn't a script node.", node.0))
        })?)))
    }

    /// Fetch the flow node flow value referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch_flow(e: impl PgExecutor<'_>, node: FlowNodeId)
        -> error::Result<FlowValue>
    {
        fetch(e, node).await.and_then(|Val { flow, .. }| flow.ok_or_else(|| {
            error::Error::InternalErr(format!("Flow node ({:x}) isn't a flow value node.", node.0))
        }))
    }

    /// Fetch the flow node referenced by `node` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    async fn fetch(e: impl PgExecutor<'_>, node: FlowNodeId) -> error::Result<Val> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        CACHE.get_or_insert_async(
            &node,
            fs::import_or_insert_with(CACHE_DIR, node.0 as u64, async {
                sqlx::query!(
                    "SELECT \
                        lock AS \"lock: String\", \
                        code AS \"code: String\", \
                        flow::text AS \"flow: Box<str>\" \
                    FROM flow_node WHERE id = $1 LIMIT 1",
                    node.0,
                )
                .fetch_one(e)
                .await
                .map_err(Into::into)
                .and_then(|r| Ok(Val {
                    lock: r.lock.and_then(|x| if x.is_empty() { None } else { Some(x) }),
                    code: r.code,
                    flow: match r.flow {
                        None => None,
                        Some(flow) => serde_json::from_str(&flow).map_err(|err| {
                            error::Error::InternalErr(format!("Unable to parse flow value: {err:?}"))
                        })?,
                    }
                }))
            })
        ).await
    }

    // ----------------------------------------------------------------------------------------------
    // impl `fs::Bundle` for `Val`.

    #[derive(Copy, Clone)]
    enum Item {
        Lock,
        Code,
        Flow,
    }

    impl fs::Item for Item {
        fn path(&self, root: &Path) -> PathBuf {
            match self {
                Self::Lock => root.join("lock.txt"),
                Self::Code => root.join("code.txt"),
                Self::Flow => root.join("flow.json"),
            }
        }
    }

    impl fs::Bundle for Val {
        type Item = Item;

        fn items() -> &'static [Self::Item] {
            &[Item::Lock, Item::Code, Item::Flow]
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            match item {
                Item::Lock => self.lock = Some(String::from_utf8(data)?),
                Item::Code => self.code = Some(String::from_utf8(data)?),
                Item::Flow => self.flow = Some(serde_json::from_slice(&data)?),
            }
            Ok(())
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match item {
                Item::Lock => Ok(self.lock.as_ref().map(|s| s.as_bytes().to_vec())),
                Item::Code => Ok(self.code.as_ref().map(|s| s.as_bytes().to_vec())),
                Item::Flow => Ok(self.flow.as_ref().map(|f| serde_json::to_vec(f)).transpose()?),
            }
        }
    }
}

pub mod script {
    use super::*;
    use crate::scripts::{ScriptHash, ScriptLang};

    /// Cache directory for windmill server/worker(s) scripts.
    pub const CACHE_DIR: &str = const_format::concatcp!(super::CACHE_DIR, "script");

    lazy_static::lazy_static! {
        /// Scripts cache.
        /// FIXME: This should be a static but [`Cache`] does not have a const constructor.
        /// FIXME: Use `Arc<Val>` for cheap cloning.
        static ref CACHE: Cache<ScriptHash, Val> = Cache::new(1000);
    }

    /// Script cache value.
    #[derive(Debug, Clone, Default)]
    pub struct Val {
        pub lock: Option<String>,
        pub code: String,
        pub language: Option<ScriptLang>,
        pub envs: Option<Vec<String>>,
        pub codebase: Option<String>,
    }

    /// Fetch the script referenced by `hash` from the cache.
    /// If not present, import from the file-system cache or fetch it from the database and write
    /// it to the file system and cache.
    /// This should be preferred over fetching the database directly.
    pub async fn fetch(e: impl PgExecutor<'_>, hash: ScriptHash, workspace_id: &str)
        -> error::Result<Val>
    {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        CACHE.get_or_insert_async(
            &hash,
            fs::import_or_insert_with(CACHE_DIR, hash.0 as u64, async {
                sqlx::query!(
                    "SELECT \
                        lock AS \"lock: String\", \
                        content AS \"code!: String\",
                        language AS \"language: Option<ScriptLang>\", \
                        envs AS \"envs: Vec<String>\", \
                        codebase AS \"codebase: String\" \
                    FROM script WHERE hash = $1 AND workspace_id = $2 LIMIT 1",
                    hash.0,
                    workspace_id,
                )
                .fetch_one(e)
                .await
                .map_err(Into::into)
                .map(|r| Val {
                    lock: r.lock.and_then(|x| if x.is_empty() { None } else { Some(x) }),
                    code: r.code,
                    language: r.language,
                    envs: r.envs,
                    codebase: r.codebase,
                })
            })
        )
        .await
    }

    // ----------------------------------------------------------------------------------------------
    // impl `fs::Bundle` for `Val`.

    #[derive(Copy, Clone)]
    pub enum Item {
        Lock,
        Code,
        Info,
    }

    impl fs::Item for Item {
        fn path(&self, root: &Path) -> PathBuf {
            match self {
                Item::Lock => root.join("lock.txt"),
                Item::Code => root.join("code.txt"),
                Item::Info => root.join("info.json"),
            }
        }
    }

    impl fs::Bundle for Val {
        type Item = Item;

        fn items() -> &'static [Self::Item] {
            &[Item::Lock, Item::Code, Item::Info]
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            match item {
                Item::Lock => self.lock = Some(String::from_utf8(data)?),
                Item::Code => self.code = String::from_utf8(data)?,
                Item::Info => (self.language, self.envs, self.codebase) = serde_json::from_slice(&data)?,
            }
            Ok(())
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match item {
                Item::Lock => Ok(self.lock.as_ref().map(|s| s.as_bytes().to_vec())),
                Item::Code => Ok(Some(self.code.as_bytes().to_vec())),
                Item::Info => Ok(Some(serde_json::to_vec(&(&self.language, &self.envs, &self.codebase))?)),
            }
        }
    }
}

mod fs {
    use super::*;

    use std::future::Future;

    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};

    /// A bundle of items that can be imported/exported from/into the file-system.
    pub trait Bundle: Default {
        /// Item type of the bundle.
        type Item: Item;
        /// Returns a slice of all items than **can** exists within the bundle.
        fn items() -> &'static [Self::Item];
        /// Import the given `data` into the `item`.
        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()>;
        /// Export the `item` into a `Vec<u8>`.
        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>>;
    }

    /// An item that can be imported/exported from/into the file-system.
    pub trait Item: Copy + 'static {
        /// Returns the path of the item within the given `root` path.
        fn path(&self, root: &Path) -> PathBuf;
    }

    /// Import or insert a bundle within the given combination of `{root}/{key}/`.
    pub async fn import_or_insert_with<T, F>(root: &str, key: u64, f: F)
        -> error::Result<T>
    where
        T: Bundle,
        F: Future<Output = error::Result<T>>,
    {
        // Generate the file path from `root` path and `key`.
        let path = Path::new(root).join(format!("{:016x}", key));
        // Retrieve the data from the cache directory or the database.
        if fs::metadata(&path).is_ok() {
            // Cache path exists, read its contents.
            let import = || -> error::Result<T> {
                let mut data = T::default();
                for item in T::items() {
                    let mut buf = vec![];
                    let Ok(mut file) = OpenOptions::new().read(true).open(item.path(&path))
                    else { continue };
                    file.read_to_end(&mut buf)?;
                    data.import(*item, buf)?;
                }
                tracing::debug!("Imported from file-system: {:?}", path);
                Ok(data)
            };
            match import() {
                Ok(data) => return Ok(data),
                Err(err) => tracing::warn!(
                    "Failed to import from file-system, fetch source..: {path:?}: {err:?}"
                )
            }
        }
        // Cache path doesn't exist or import failed, generate the content.
        let data = f.await?;
        let export = |data: &T| -> error::Result<()> {
            fs::create_dir_all(&path)?;
            // Write the generated data to the file.
            for item in T::items() {
                let Some(buf) = data.export(*item)?
                else { continue };
                let mut file = OpenOptions::new().write(true).create(true).open(item.path(&path))?;
                file.write_all(&buf)?;
            }
            tracing::debug!("Exported to file-system: {:?}", path);
            Ok(())
        };
        // Try to export data to the file-system.
        // If failed, remove the directory but still return the data.
        if let Err(err) = export(&data) {
            tracing::warn!("Failed to export to file-system: {path:?}: {err:?}");
            let _ = fs::remove_dir_all(&path);
        }
        Ok(data)
    }
}
