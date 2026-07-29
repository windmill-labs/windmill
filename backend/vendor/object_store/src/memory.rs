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

//! An in-memory object store implementation
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{stream::BoxStream, StreamExt};
use parking_lot::RwLock;

use crate::multipart::{MultipartStore, PartId};
use crate::util::InvalidGetRange;
use crate::{
    path::Path, Attributes, GetRange, GetResult, GetResultPayload, ListResult, MultipartId,
    MultipartUpload, ObjectMeta, ObjectStore, PutMode, PutMultipartOpts, PutOptions, PutResult,
    Result, UpdateVersion, UploadPart,
};
use crate::{GetOptions, PutPayload};

/// A specialized `Error` for in-memory object store-related errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("No data in memory found. Location: {path}")]
    NoDataInMemory { path: String },

    #[error("Invalid range: {source}")]
    Range { source: InvalidGetRange },

    #[error("Object already exists at that location: {path}")]
    AlreadyExists { path: String },

    #[error("ETag required for conditional update")]
    MissingETag,

    #[error("MultipartUpload not found: {id}")]
    UploadNotFound { id: String },

    #[error("Missing part at index: {part}")]
    MissingPart { part: usize },
}

impl From<Error> for super::Error {
    fn from(source: Error) -> Self {
        match source {
            Error::NoDataInMemory { ref path } => Self::NotFound {
                path: path.into(),
                source: source.into(),
            },
            Error::AlreadyExists { ref path } => Self::AlreadyExists {
                path: path.into(),
                source: source.into(),
            },
            _ => Self::Generic {
                store: "InMemory",
                source: Box::new(source),
            },
        }
    }
}

/// In-memory storage suitable for testing or for opting out of using a cloud
/// storage provider.
#[derive(Debug, Default)]
pub struct InMemory {
    storage: SharedStorage,
}

#[derive(Debug, Clone)]
struct Entry {
    data: Bytes,
    last_modified: DateTime<Utc>,
    attributes: Attributes,
    e_tag: usize,
}

impl Entry {
    fn new(
        data: Bytes,
        last_modified: DateTime<Utc>,
        e_tag: usize,
        attributes: Attributes,
    ) -> Self {
        Self {
            data,
            last_modified,
            e_tag,
            attributes,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Storage {
    next_etag: usize,
    map: BTreeMap<Path, Entry>,
    uploads: HashMap<usize, PartStorage>,
}

#[derive(Debug, Default, Clone)]
struct PartStorage {
    parts: Vec<Option<Bytes>>,
}

type SharedStorage = Arc<RwLock<Storage>>;

impl Storage {
    fn insert(&mut self, location: &Path, bytes: Bytes, attributes: Attributes) -> usize {
        let etag = self.next_etag;
        self.next_etag += 1;
        let entry = Entry::new(bytes, Utc::now(), etag, attributes);
        self.overwrite(location, entry);
        etag
    }

    fn overwrite(&mut self, location: &Path, entry: Entry) {
        self.map.insert(location.clone(), entry);
    }

    fn create(&mut self, location: &Path, entry: Entry) -> Result<()> {
        use std::collections::btree_map;
        match self.map.entry(location.clone()) {
            btree_map::Entry::Occupied(_) => Err(Error::AlreadyExists {
                path: location.to_string(),
            }
            .into()),
            btree_map::Entry::Vacant(v) => {
                v.insert(entry);
                Ok(())
            }
        }
    }

    fn update(&mut self, location: &Path, v: UpdateVersion, entry: Entry) -> Result<()> {
        match self.map.get_mut(location) {
            // Return Precondition instead of NotFound for consistency with stores
            None => Err(crate::Error::Precondition {
                path: location.to_string(),
                source: format!("Object at location {location} not found").into(),
            }),
            Some(e) => {
                let existing = e.e_tag.to_string();
                let expected = v.e_tag.ok_or(Error::MissingETag)?;
                if existing == expected {
                    *e = entry;
                    Ok(())
                } else {
                    Err(crate::Error::Precondition {
                        path: location.to_string(),
                        source: format!("{existing} does not match {expected}").into(),
                    })
                }
            }
        }
    }

    fn upload_mut(&mut self, id: &MultipartId) -> Result<&mut PartStorage> {
        let parts = id
            .parse()
            .ok()
            .and_then(|x| self.uploads.get_mut(&x))
            .ok_or_else(|| Error::UploadNotFound { id: id.into() })?;
        Ok(parts)
    }

    fn remove_upload(&mut self, id: &MultipartId) -> Result<PartStorage> {
        let parts = id
            .parse()
            .ok()
            .and_then(|x| self.uploads.remove(&x))
            .ok_or_else(|| Error::UploadNotFound { id: id.into() })?;
        Ok(parts)
    }
}

impl std::fmt::Display for InMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InMemory")
    }
}

#[async_trait]
impl ObjectStore for InMemory {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let mut storage = self.storage.write();
        let etag = storage.next_etag;
        let entry = Entry::new(payload.into(), Utc::now(), etag, opts.attributes);

        match opts.mode {
            PutMode::Overwrite => storage.overwrite(location, entry),
            PutMode::Create => storage.create(location, entry)?,
            PutMode::Update(v) => storage.update(location, v, entry)?,
        }
        storage.next_etag += 1;

        Ok(PutResult {
            e_tag: Some(etag.to_string()),
            version: None,
        })
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        Ok(Box::new(InMemoryUpload {
            location: location.clone(),
            attributes: opts.attributes,
            parts: vec![],
            storage: Arc::clone(&self.storage),
        }))
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let entry = self.entry(location)?;
        let e_tag = entry.e_tag.to_string();

        let meta = ObjectMeta {
            location: location.clone(),
            last_modified: entry.last_modified,
            size: entry.data.len() as u64,
            e_tag: Some(e_tag),
            version: None,
        };
        options.check_preconditions(&meta)?;

        let (range, data) = match options.range {
            Some(range) => {
                let r = range
                    .as_range(entry.data.len() as u64)
                    .map_err(|source| Error::Range { source })?;
                (
                    r.clone(),
                    entry.data.slice(r.start as usize..r.end as usize),
                )
            }
            None => (0..entry.data.len() as u64, entry.data),
        };
        let stream = futures::stream::once(futures::future::ready(Ok(data)));

        Ok(GetResult {
            payload: GetResultPayload::Stream(stream.boxed()),
            attributes: entry.attributes,
            meta,
            range,
        })
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        let entry = self.entry(location)?;
        ranges
            .iter()
            .map(|range| {
                let r = GetRange::Bounded(range.clone())
                    .as_range(entry.data.len() as u64)
                    .map_err(|source| Error::Range { source })?;
                let r_end = usize::try_from(r.end).map_err(|_e| Error::Range {
                    source: InvalidGetRange::TooLarge {
                        requested: r.end,
                        max: usize::MAX as u64,
                    },
                })?;
                let r_start = usize::try_from(r.start).map_err(|_e| Error::Range {
                    source: InvalidGetRange::TooLarge {
                        requested: r.start,
                        max: usize::MAX as u64,
                    },
                })?;
                Ok(entry.data.slice(r_start..r_end))
            })
            .collect()
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let entry = self.entry(location)?;

        Ok(ObjectMeta {
            location: location.clone(),
            last_modified: entry.last_modified,
            size: entry.data.len() as u64,
            e_tag: Some(entry.e_tag.to_string()),
            version: None,
        })
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        self.storage.write().map.remove(location);
        Ok(())
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let root = Path::default();
        let prefix = prefix.unwrap_or(&root);

        let storage = self.storage.read();
        let values: Vec<_> = storage
            .map
            .range((prefix)..)
            .take_while(|(key, _)| key.as_ref().starts_with(prefix.as_ref()))
            .filter(|(key, _)| {
                // Don't return for exact prefix match
                key.prefix_match(prefix)
                    .map(|mut x| x.next().is_some())
                    .unwrap_or(false)
            })
            .map(|(key, value)| {
                Ok(ObjectMeta {
                    location: key.clone(),
                    last_modified: value.last_modified,
                    size: value.data.len() as u64,
                    e_tag: Some(value.e_tag.to_string()),
                    version: None,
                })
            })
            .collect();

        futures::stream::iter(values).boxed()
    }

    /// The memory implementation returns all results, as opposed to the cloud
    /// versions which limit their results to 1k or more because of API
    /// limitations.
    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let root = Path::default();
        let prefix = prefix.unwrap_or(&root);

        let mut common_prefixes = BTreeSet::new();

        // Only objects in this base level should be returned in the
        // response. Otherwise, we just collect the common prefixes.
        let mut objects = vec![];
        for (k, v) in self.storage.read().map.range((prefix)..) {
            if !k.as_ref().starts_with(prefix.as_ref()) {
                break;
            }

            let mut parts = match k.prefix_match(prefix) {
                Some(parts) => parts,
                None => continue,
            };

            // Pop first element
            let common_prefix = match parts.next() {
                Some(p) => p,
                // Should only return children of the prefix
                None => continue,
            };

            if parts.next().is_some() {
                common_prefixes.insert(prefix.child(common_prefix));
            } else {
                let object = ObjectMeta {
                    location: k.clone(),
                    last_modified: v.last_modified,
                    size: v.data.len() as u64,
                    e_tag: Some(v.e_tag.to_string()),
                    version: None,
                };
                objects.push(object);
            }
        }

        Ok(ListResult {
            objects,
            common_prefixes: common_prefixes.into_iter().collect(),
        })
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let entry = self.entry(from)?;
        self.storage
            .write()
            .insert(to, entry.data, entry.attributes);
        Ok(())
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let entry = self.entry(from)?;
        let mut storage = self.storage.write();
        if storage.map.contains_key(to) {
            return Err(Error::AlreadyExists {
                path: to.to_string(),
            }
            .into());
        }
        storage.insert(to, entry.data, entry.attributes);
        Ok(())
    }
}

#[async_trait]
impl MultipartStore for InMemory {
    async fn create_multipart(&self, _path: &Path) -> Result<MultipartId> {
        let mut storage = self.storage.write();
        let etag = storage.next_etag;
        storage.next_etag += 1;
        storage.uploads.insert(etag, Default::default());
        Ok(etag.to_string())
    }

    async fn put_part(
        &self,
        _path: &Path,
        id: &MultipartId,
        part_idx: usize,
        payload: PutPayload,
    ) -> Result<PartId> {
        let mut storage = self.storage.write();
        let upload = storage.upload_mut(id)?;
        if part_idx <= upload.parts.len() {
            upload.parts.resize(part_idx + 1, None);
        }
        upload.parts[part_idx] = Some(payload.into());
        Ok(PartId {
            content_id: Default::default(),
        })
    }

    async fn complete_multipart(
        &self,
        path: &Path,
        id: &MultipartId,
        _parts: Vec<PartId>,
    ) -> Result<PutResult> {
        let mut storage = self.storage.write();
        let upload = storage.remove_upload(id)?;

        let mut cap = 0;
        for (part, x) in upload.parts.iter().enumerate() {
            cap += x.as_ref().ok_or(Error::MissingPart { part })?.len();
        }
        let mut buf = Vec::with_capacity(cap);
        for x in &upload.parts {
            buf.extend_from_slice(x.as_ref().unwrap())
        }
        let etag = storage.insert(path, buf.into(), Default::default());
        Ok(PutResult {
            e_tag: Some(etag.to_string()),
            version: None,
        })
    }

    async fn abort_multipart(&self, _path: &Path, id: &MultipartId) -> Result<()> {
        self.storage.write().remove_upload(id)?;
        Ok(())
    }
}

impl InMemory {
    /// Create new in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a fork of the store, with the current content copied into the
    /// new store.
    pub fn fork(&self) -> Self {
        let storage = self.storage.read();
        let storage = Arc::new(RwLock::new(storage.clone()));
        Self { storage }
    }

    fn entry(&self, location: &Path) -> Result<Entry> {
        let storage = self.storage.read();
        let value = storage
            .map
            .get(location)
            .cloned()
            .ok_or_else(|| Error::NoDataInMemory {
                path: location.to_string(),
            })?;

        Ok(value)
    }
}

#[derive(Debug)]
struct InMemoryUpload {
    location: Path,
    attributes: Attributes,
    parts: Vec<PutPayload>,
    storage: Arc<RwLock<Storage>>,
}

#[async_trait]
impl MultipartUpload for InMemoryUpload {
    fn put_part(&mut self, payload: PutPayload) -> UploadPart {
        self.parts.push(payload);
        Box::pin(futures::future::ready(Ok(())))
    }

    async fn complete(&mut self) -> Result<PutResult> {
        let cap = self.parts.iter().map(|x| x.content_length()).sum();
        let mut buf = Vec::with_capacity(cap);
        let parts = self.parts.iter().flatten();
        parts.for_each(|x| buf.extend_from_slice(x));
        let etag = self.storage.write().insert(
            &self.location,
            buf.into(),
            std::mem::take(&mut self.attributes),
        );

        Ok(PutResult {
            e_tag: Some(etag.to_string()),
            version: None,
        })
    }

    async fn abort(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::integration::*;

    use super::*;

    #[tokio::test]
    async fn in_memory_test() {
        let integration = InMemory::new();

        put_get_delete_list(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        stream_get(&integration).await;
        put_opts(&integration, true).await;
        multipart(&integration, &integration).await;
        put_get_attributes(&integration).await;
    }

    #[tokio::test]
    async fn box_test() {
        let integration: Box<dyn ObjectStore> = Box::new(InMemory::new());

        put_get_delete_list(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        stream_get(&integration).await;
    }

    #[tokio::test]
    async fn arc_test() {
        let integration: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

        put_get_delete_list(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        stream_get(&integration).await;
    }

    #[tokio::test]
    async fn unknown_length() {
        let integration = InMemory::new();

        let location = Path::from("some_file");

        let data = Bytes::from("arbitrary data");

        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        let read_data = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(&*read_data, data);
    }

    const NON_EXISTENT_NAME: &str = "nonexistentname";

    #[tokio::test]
    async fn nonexistent_location() {
        let integration = InMemory::new();

        let location = Path::from(NON_EXISTENT_NAME);

        let err = get_nonexistent_object(&integration, Some(location))
            .await
            .unwrap_err();
        if let crate::Error::NotFound { path, source } = err {
            let source_variant = source.downcast_ref::<Error>();
            assert!(
                matches!(source_variant, Some(Error::NoDataInMemory { .. }),),
                "got: {source_variant:?}"
            );
            assert_eq!(path, NON_EXISTENT_NAME);
        } else {
            panic!("unexpected error type: {err:?}");
        }
    }
}
