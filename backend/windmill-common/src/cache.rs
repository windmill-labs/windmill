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
        static ref CACHE: Cache<FlowNodeId, Val> = Cache::new(1000);
    }

    /// Flow node cache value.
    #[derive(Debug, Clone, Default)]
    struct Val {
        lock: Option<String>,
        code: Option<String>,
        flow: Option<FlowValue>,
    }

    #[derive(Copy, Clone)]
    enum ValItem {
        Lock,
        Code,
        Flow,
    }

    impl fs::Item for ValItem {
        fn path(&self, root: &Path) -> PathBuf {
            match self {
                Self::Lock => root.join("lock.txt"),
                Self::Code => root.join("code.txt"),
                Self::Flow => root.join("flow.bin"),
            }
        }
    }

    impl fs::Bundle for Val {
        type Item = ValItem;

        fn items() -> &'static [Self::Item] {
            &[ValItem::Lock, ValItem::Code, ValItem::Flow]
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match item {
                ValItem::Lock => Ok(self.lock.as_ref().map(|s| s.as_bytes().to_vec())),
                ValItem::Code => Ok(self.code.as_ref().map(|s| s.as_bytes().to_vec())),
                ValItem::Flow => Ok(self.flow.as_ref().map(|f| bitcode::serialize(f)).transpose()?),
            }
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            match item {
                ValItem::Lock => self.lock = Some(String::from_utf8(data)?),
                ValItem::Code => self.code = Some(String::from_utf8(data)?),
                ValItem::Flow => self.flow = Some(bitcode::deserialize(&data)?),
            }
            Ok(())
        }
    }

    pub async fn fetch_code(e: impl PgExecutor<'_>, node: FlowNodeId)
        -> error::Result<(Option<String>, String)>
    {
        fetch(e, node).await.map(|Val { lock, code, .. }| (lock, code.unwrap_or_default()))
    }

    pub async fn fetch_flow(e: impl PgExecutor<'_>, node: FlowNodeId)
        -> error::Result<FlowValue>
    {
        fetch(e, node).await.map(|Val { flow, .. }| flow.unwrap())
    }

    async fn fetch(e: impl PgExecutor<'_>, node: FlowNodeId) -> error::Result<Val> {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        CACHE.get_or_insert_async(
            &node,
            fs::import_or_insert_with(CACHE_DIR, node.0 as u64, || async {
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
}

pub mod script {
    use super::*;
    use crate::scripts::{ScriptHash, ScriptLang};

    /// Cache directory for windmill server/worker(s) flow nodes.
    pub const CACHE_DIR: &str = const_format::concatcp!(super::CACHE_DIR, "script");

    lazy_static::lazy_static! {
        /// Scripts cache.
        /// FIXME: This should be a static but [`Cache`] does not have a const constructor.
        static ref CACHE: Cache<ScriptHash, Val> = Cache::new(1000);
    }

    /// Flow node cache value.
    #[derive(Debug, Clone, Default)]
    pub struct Val {
        pub lock: Option<String>,
        pub code: String,
        pub language: Option<ScriptLang>,
        pub envs: Option<Vec<String>>,
        pub codebase: Option<String>,
    }

    #[derive(Copy, Clone)]
    pub enum ValItem {
        Lock,
        Code,
        Metadata,
    }

    impl fs::Item for ValItem {
        fn path(&self, root: &Path) -> PathBuf {
            match self {
                Self::Lock => root.join("lock.txt"),
                Self::Code => root.join("code.txt"),
                Self::Metadata => root.join("metadata.bin"),
            }
        }
    }

    impl fs::Bundle for Val {
        type Item = ValItem;

        fn items() -> &'static [Self::Item] {
            &[ValItem::Lock, ValItem::Code, ValItem::Metadata]
        }

        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>> {
            match item {
                ValItem::Lock => Ok(self.lock.as_ref().map(|s| s.as_bytes().to_vec())),
                ValItem::Code => Ok(Some(self.code.as_bytes().to_vec())),
                ValItem::Metadata => Ok(Some(bitcode::serialize(&(&self.language, &self.envs, &self.codebase)).unwrap())),
            }
        }

        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()> {
            match item {
                ValItem::Lock => self.lock = Some(String::from_utf8(data)?),
                ValItem::Code => self.code = String::from_utf8(data)?,
                ValItem::Metadata => (self.language, self.envs, self.codebase) = bitcode::deserialize(&data)?,
            }
            Ok(())
        }
    }

    pub async fn fetch(e: impl PgExecutor<'_>, hash: ScriptHash, workspace_id: &str)
        -> error::Result<Val>
    {
        // If not present, `get_or_insert_async` will lock the key until the future completes,
        // so only one thread will be able to fetch the data from the database and write it to
        // the file system and cache, hence no race on the file system.
        CACHE.get_or_insert_async(
            &hash,
            fs::import_or_insert_with(CACHE_DIR, hash.0 as u64, || async {
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
}

mod fs {
    use super::*;

    use std::future::Future;

    use tokio::fs::{self, OpenOptions};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    pub trait Item: Copy + 'static {
        fn path(&self, root: &Path) -> PathBuf;
    }

    pub trait Bundle: Default {
        type Item: Item;
        fn items() -> &'static [Self::Item];
        fn export(&self, item: Self::Item) -> error::Result<Option<Vec<u8>>>;
        fn import(&mut self, item: Self::Item, data: Vec<u8>) -> error::Result<()>;
    }

    /// Get or insert a file with the given path.
    pub async fn import_or_insert_with<T, F>(root: &str, key: u64, f: impl FnOnce() -> F)
        -> error::Result<T>
    where
        T: Bundle,
        F: Future<Output = error::Result<T>>,
    {
        // Generate the file path from `root` path and `key`.
        let path = Path::new(root).join(format!("{:016x}", key));
        if fs::metadata(&path).await.is_ok() {
            // File exists, read its contents.
            let mut data = T::default();
            for item in T::items() {
                let mut buf = vec![];
                let Ok(mut file) = OpenOptions::new().read(true).open(item.path(&path)).await else { continue };
                file.read_to_end(&mut buf).await?;
                data.import(*item, buf)?;
            }
            return Ok(data);
        }
        // File doesn't exist, generate the content.
        let data = f().await?;
        // Write the generated data to the file.
        fs::create_dir_all(&path).await?;
        for item in T::items() {
            let Some(buf) = data.export(*item)? else { continue };
            let mut file = OpenOptions::new().write(true).create(true).open(item.path(&path)).await?;
            file.write_all(&buf).await?;
        }
        Ok(data)
    }
}
