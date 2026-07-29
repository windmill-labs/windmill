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

//! Cloud Multipart Upload
//!
//! This crate provides an asynchronous interface for multipart file uploads to
//! cloud storage services. It's designed to offer efficient, non-blocking operations,
//! especially useful when dealing with large files or high-throughput systems.

use async_trait::async_trait;

use crate::path::Path;
use crate::{MultipartId, PutPayload, PutResult, Result};

/// Represents a part of a file that has been successfully uploaded in a multipart upload process.
#[derive(Debug, Clone)]
pub struct PartId {
    /// Id of this part
    pub content_id: String,
}

/// A low-level interface for interacting with multipart upload APIs
///
/// Most use-cases should prefer [`ObjectStore::put_multipart`] as this is supported by more
/// backends, including [`LocalFileSystem`], and automatically handles uploading fixed
/// size parts of sufficient size in parallel
///
/// [`ObjectStore::put_multipart`]: crate::ObjectStore::put_multipart
/// [`LocalFileSystem`]: crate::local::LocalFileSystem
#[async_trait]
pub trait MultipartStore: Send + Sync + 'static {
    /// Creates a new multipart upload, returning the [`MultipartId`]
    async fn create_multipart(&self, path: &Path) -> Result<MultipartId>;

    /// Uploads a new part with index `part_idx`
    ///
    /// `part_idx` should be an integer in the range `0..N` where `N` is the number of
    /// parts in the upload. Parts may be uploaded concurrently and in any order.
    ///
    /// Most stores require that all parts excluding the last are at least 5 MiB, and some
    /// further require that all parts excluding the last be the same size, e.g. [R2].
    /// [`WriteMultipart`] performs writes in fixed size blocks of 5 MiB, and clients wanting
    /// to maximise compatibility should look to do likewise.
    ///
    /// [R2]: https://developers.cloudflare.com/r2/objects/multipart-objects/#limitations
    /// [`WriteMultipart`]: crate::upload::WriteMultipart
    async fn put_part(
        &self,
        path: &Path,
        id: &MultipartId,
        part_idx: usize,
        data: PutPayload,
    ) -> Result<PartId>;

    /// Completes a multipart upload
    ///
    /// The `i`'th value of `parts` must be a [`PartId`] returned by a call to [`Self::put_part`]
    /// with a `part_idx` of `i`, and the same `path` and `id` as provided to this method. Calling
    /// this method with out of sequence or repeated [`PartId`], or [`PartId`] returned for other
    /// values of `path` or `id`, will result in implementation-defined behaviour
    async fn complete_multipart(
        &self,
        path: &Path,
        id: &MultipartId,
        parts: Vec<PartId>,
    ) -> Result<PutResult>;

    /// Aborts a multipart upload
    async fn abort_multipart(&self, path: &Path, id: &MultipartId) -> Result<()>;
}
