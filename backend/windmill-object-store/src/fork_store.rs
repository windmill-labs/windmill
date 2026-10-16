//! An [`ObjectStore`] over one workspace storage that applies the workspace's
//! [`ForkStorageKeys`]: callers keep using logical keys.

use std::sync::Arc;

use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use object_store::path::Path;
use object_store::{
    GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore, PutMultipartOpts,
    PutOptions, PutPayload, PutResult, Result,
};
use windmill_common::fork_storage::{fork_storage_keys, ForkStorageKeys};

use crate::{build_object_store_client, ObjectStoreResource};

/// The client for `w_id`'s storage `storage` (None = default). Every server-side access to a
/// workspace storage by logical key goes through this, so fork isolation holds on each path.
pub async fn workspace_object_store(
    db: &windmill_common::DB,
    w_id: &str,
    storage: Option<&str>,
    resource: &ObjectStoreResource,
) -> windmill_common::error::Result<Arc<dyn ObjectStore>> {
    let inner = build_object_store_client(resource).await?;
    let keys = fork_storage_keys(db, w_id, storage).await?;
    Ok(Arc::new(ForkStore { inner, keys }))
}

#[derive(Debug)]
pub struct ForkStore {
    inner: Arc<dyn ObjectStore>,
    keys: Arc<ForkStorageKeys>,
}

impl ForkStore {
    pub fn new(inner: Arc<dyn ObjectStore>, keys: Arc<ForkStorageKeys>) -> Self {
        Self { inner, keys }
    }

    pub fn keys(&self) -> &ForkStorageKeys {
        &self.keys
    }
}

impl std::fmt::Display for ForkStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ForkStore({})", self.inner)
    }
}

fn denied(location: &Path, e: windmill_common::error::Error) -> object_store::Error {
    object_store::Error::PermissionDenied { path: location.to_string(), source: Box::new(e) }
}

fn parse(key: String) -> Result<Path> {
    Path::parse(key).map_err(|source| object_store::Error::InvalidPath { source })
}

impl ForkStore {
    fn write_path(&self, location: &Path) -> Result<Path> {
        parse(
            self.keys
                .write_key(location.as_ref())
                .map_err(|e| denied(location, e))?,
        )
    }

    fn read_paths(&self, location: &Path) -> Result<Vec<Path>> {
        self.keys
            .read_keys(location.as_ref())
            .map_err(|e| denied(location, e))?
            .into_iter()
            .map(parse)
            .collect()
    }

    /// The physical path of the first of `location`'s read paths that exists (the last one
    /// when none does).
    pub async fn resolve_read(&self, location: &Path) -> Result<Path> {
        parse(self.resolve_read_key(location.as_ref()).await?)
    }

    /// [`Self::resolve_read`] on a raw key, for callers that forward the key themselves.
    pub async fn resolve_read_key(&self, key: &str) -> Result<String> {
        let location = Path::from(key);
        let mut keys = self.keys.read_keys(key).map_err(|e| denied(&location, e))?;
        let last = keys.pop().expect("read_keys returns at least one key");
        for candidate in keys {
            match self.inner.head(&parse(candidate.clone())?).await {
                Ok(_) => return Ok(candidate),
                Err(object_store::Error::NotFound { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(last)
    }
}

#[async_trait::async_trait]
impl ObjectStore for ForkStore {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        self.inner
            .put_opts(&self.write_path(location)?, payload, opts)
            .await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        self.inner
            .put_multipart_opts(&self.write_path(location)?, opts)
            .await
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let paths = self.read_paths(location)?;
        let last = paths.len() - 1;
        for (i, path) in paths.into_iter().enumerate() {
            match self.inner.get_opts(&path, options.clone()).await {
                Err(object_store::Error::NotFound { .. }) if i < last => continue,
                Ok(mut result) => {
                    result.meta.location = location.clone();
                    return Ok(result);
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!("read_keys returns at least one key")
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let paths = self.read_paths(location)?;
        let last = paths.len() - 1;
        for (i, path) in paths.into_iter().enumerate() {
            match self.inner.head(&path).await {
                Err(object_store::Error::NotFound { .. }) if i < last => continue,
                Ok(meta) => return Ok(ObjectMeta { location: location.clone(), ..meta }),
                Err(e) => return Err(e),
            }
        }
        unreachable!("read_keys returns at least one key")
    }

    /// Removes this workspace's copy only: an ancestor's object stays readable through it.
    async fn delete(&self, location: &Path) -> Result<()> {
        self.inner.delete(&self.write_path(location)?).await
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix = prefix.map(|p| p.as_ref().to_string()).unwrap_or_default();
        let mapping = match self.keys.list(&prefix) {
            Ok(m) => m,
            Err(e) => {
                let err = denied(&Path::from(prefix), e);
                return futures::stream::once(async move { Err(err) }).boxed();
            }
        };
        let physical = Path::from(mapping.physical_prefix.clone());
        self.inner
            .list(Some(&physical))
            .try_filter_map(move |meta| {
                let logical = mapping.logical_key(meta.location.as_ref());
                futures::future::ready(Ok(logical
                    .and_then(|l| Path::parse(l).ok())
                    .map(|location| ObjectMeta { location, ..meta })))
            })
            .boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix = prefix.map(|p| p.as_ref().to_string()).unwrap_or_default();
        let mapping = match self.keys.list(&prefix) {
            Ok(m) => m,
            Err(e) => {
                let err = denied(&Path::from(prefix), e);
                return futures::stream::once(async move { Err(err) }).boxed();
            }
        };
        let physical = Path::from(mapping.physical_prefix.clone());
        let physical_offset = match parse(mapping.physical_key(offset.as_ref())) {
            Ok(p) => p,
            Err(e) => return futures::stream::once(async move { Err(e) }).boxed(),
        };
        self.inner
            .list_with_offset(Some(&physical), &physical_offset)
            .try_filter_map(move |meta| {
                let logical = mapping.logical_key(meta.location.as_ref());
                futures::future::ready(Ok(logical
                    .and_then(|l| Path::parse(l).ok())
                    .map(|location| ObjectMeta { location, ..meta })))
            })
            .boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let prefix_str = prefix.map(|p| p.as_ref().to_string()).unwrap_or_default();
        let mapping = self
            .keys
            .list(&prefix_str)
            .map_err(|e| denied(&Path::from(prefix_str.clone()), e))?;
        let physical = Path::from(mapping.physical_prefix.clone());
        let listed = self.inner.list_with_delimiter(Some(&physical)).await?;
        let logical = |p: &Path| {
            mapping
                .logical_key(p.as_ref())
                .and_then(|l| Path::parse(l).ok())
        };
        Ok(ListResult {
            common_prefixes: listed.common_prefixes.iter().filter_map(logical).collect(),
            objects: listed
                .objects
                .into_iter()
                .filter_map(|meta| {
                    logical(&meta.location).map(|location| ObjectMeta { location, ..meta })
                })
                .collect(),
        })
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let from = self.resolve_read(from).await?;
        self.inner.copy(&from, &self.write_path(to)?).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let from = self.resolve_read(from).await?;
        self.inner
            .copy_if_not_exists(&from, &self.write_path(to)?)
            .await
    }
}
