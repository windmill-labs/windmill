// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! An object store wrapper handling a constant path prefix
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use std::ops::Range;

use crate::path::Path;
use crate::{
    GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore, PutMultipartOpts,
    PutOptions, PutPayload, PutResult, Result,
};

/// Store wrapper that applies a constant prefix to all paths handled by the store.
#[derive(Debug, Clone)]
pub struct PrefixStore<T: ObjectStore> {
    prefix: Path,
    inner: T,
}

impl<T: ObjectStore> std::fmt::Display for PrefixStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrefixObjectStore({})", self.prefix.as_ref())
    }
}

impl<T: ObjectStore> PrefixStore<T> {
    /// Create a new instance of [`PrefixStore`]
    pub fn new(store: T, prefix: impl Into<Path>) -> Self {
        Self {
            prefix: prefix.into(),
            inner: store,
        }
    }

    /// Create the full path from a path relative to prefix
    fn full_path(&self, location: &Path) -> Path {
        self.prefix.parts().chain(location.parts()).collect()
    }

    /// Strip the constant prefix from a given path
    fn strip_prefix(&self, path: Path) -> Path {
        // Note cannot use match because of borrow checker
        if let Some(suffix) = path.prefix_match(&self.prefix) {
            return suffix.collect();
        }
        path
    }

    /// Strip the constant prefix from a given ObjectMeta
    fn strip_meta(&self, meta: ObjectMeta) -> ObjectMeta {
        ObjectMeta {
            last_modified: meta.last_modified,
            size: meta.size,
            location: self.strip_prefix(meta.location),
            e_tag: meta.e_tag,
            version: None,
        }
    }
}

// Note: This is a relative hack to move these two functions to pure functions so they don't rely
// on the `self` lifetime. Expected to be cleaned up before merge.
//
/// Strip the constant prefix from a given path
fn strip_prefix(prefix: &Path, path: Path) -> Path {
    // Note cannot use match because of borrow checker
    if let Some(suffix) = path.prefix_match(prefix) {
        return suffix.collect();
    }
    path
}

/// Strip the constant prefix from a given ObjectMeta
fn strip_meta(prefix: &Path, meta: ObjectMeta) -> ObjectMeta {
    ObjectMeta {
        last_modified: meta.last_modified,
        size: meta.size,
        location: strip_prefix(prefix, meta.location),
        e_tag: meta.e_tag,
        version: None,
    }
}
#[async_trait::async_trait]
impl<T: ObjectStore> ObjectStore for PrefixStore<T> {
    async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
        let full_path = self.full_path(location);
        self.inner.put(&full_path, payload).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let full_path = self.full_path(location);
        self.inner.put_opts(&full_path, payload, opts).await
    }

    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        let full_path = self.full_path(location);
        self.inner.put_multipart(&full_path).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        let full_path = self.full_path(location);
        self.inner.put_multipart_opts(&full_path, opts).await
    }

    async fn get(&self, location: &Path) -> Result<GetResult> {
        let full_path = self.full_path(location);
        self.inner.get(&full_path).await
    }

    async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
        let full_path = self.full_path(location);
        self.inner.get_range(&full_path, range).await
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let full_path = self.full_path(location);
        self.inner.get_opts(&full_path, options).await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        let full_path = self.full_path(location);
        self.inner.get_ranges(&full_path, ranges).await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let full_path = self.full_path(location);
        let meta = self.inner.head(&full_path).await?;
        Ok(self.strip_meta(meta))
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        let full_path = self.full_path(location);
        self.inner.delete(&full_path).await
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix = self.full_path(prefix.unwrap_or(&Path::default()));
        let s = self.inner.list(Some(&prefix));
        let slf_prefix = self.prefix.clone();
        s.map_ok(move |meta| strip_meta(&slf_prefix, meta)).boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let offset = self.full_path(offset);
        let prefix = self.full_path(prefix.unwrap_or(&Path::default()));
        let s = self.inner.list_with_offset(Some(&prefix), &offset);
        let slf_prefix = self.prefix.clone();
        s.map_ok(move |meta| strip_meta(&slf_prefix, meta)).boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let prefix = self.full_path(prefix.unwrap_or(&Path::default()));
        self.inner
            .list_with_delimiter(Some(&prefix))
            .await
            .map(|lst| ListResult {
                common_prefixes: lst
                    .common_prefixes
                    .into_iter()
                    .map(|p| self.strip_prefix(p))
                    .collect(),
                objects: lst
                    .objects
                    .into_iter()
                    .map(|meta| self.strip_meta(meta))
                    .collect(),
            })
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.full_path(from);
        let full_to = self.full_path(to);
        self.inner.copy(&full_from, &full_to).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.full_path(from);
        let full_to = self.full_path(to);
        self.inner.rename(&full_from, &full_to).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.full_path(from);
        let full_to = self.full_path(to);
        self.inner.copy_if_not_exists(&full_from, &full_to).await
    }

    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.full_path(from);
        let full_to = self.full_path(to);
        self.inner.rename_if_not_exists(&full_from, &full_to).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::*;
    use crate::local::LocalFileSystem;

    use tempfile::TempDir;

    #[tokio::test]
    async fn prefix_test() {
        let root = TempDir::new().unwrap();
        let inner = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        let integration = PrefixStore::new(inner, "prefix");

        put_get_delete_list(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        stream_get(&integration).await;
    }

    #[tokio::test]
    async fn prefix_test_applies_prefix() {
        let tmpdir = TempDir::new().unwrap();
        let local = LocalFileSystem::new_with_prefix(tmpdir.path()).unwrap();

        let location = Path::from("prefix/test_file.json");
        let data = Bytes::from("arbitrary data");

        local.put(&location, data.clone().into()).await.unwrap();

        let prefix = PrefixStore::new(local, "prefix");
        let location_prefix = Path::from("test_file.json");

        let content_list = flatten_list_stream(&prefix, None).await.unwrap();
        assert_eq!(content_list, &[location_prefix.clone()]);

        let root = Path::from("/");
        let content_list = flatten_list_stream(&prefix, Some(&root)).await.unwrap();
        assert_eq!(content_list, &[location_prefix.clone()]);

        let read_data = prefix
            .get(&location_prefix)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(&*read_data, data);

        let target_prefix = Path::from("/test_written.json");
        prefix
            .put(&target_prefix, data.clone().into())
            .await
            .unwrap();

        prefix.delete(&location_prefix).await.unwrap();

        let local = LocalFileSystem::new_with_prefix(tmpdir.path()).unwrap();

        let err = local.get(&location).await.unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

        let location = Path::from("prefix/test_written.json");
        let read_data = local.get(&location).await.unwrap().bytes().await.unwrap();
        assert_eq!(&*read_data, data)
    }
}
