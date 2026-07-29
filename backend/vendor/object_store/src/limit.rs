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

//! An object store that limits the maximum concurrency of the wrapped implementation

use crate::{
    BoxStream, GetOptions, GetResult, GetResultPayload, ListResult, MultipartUpload, ObjectMeta,
    ObjectStore, Path, PutMultipartOpts, PutOptions, PutPayload, PutResult, Result, StreamExt,
    UploadPart,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{FutureExt, Stream};
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Store wrapper that wraps an inner store and limits the maximum number of concurrent
/// object store operations. Where each call to an [`ObjectStore`] member function is
/// considered a single operation, even if it may result in more than one network call
///
/// ```
/// # use object_store::memory::InMemory;
/// # use object_store::limit::LimitStore;
///
/// // Create an in-memory `ObjectStore` limited to 20 concurrent requests
/// let store = LimitStore::new(InMemory::new(), 20);
/// ```
///
#[derive(Debug)]
pub struct LimitStore<T: ObjectStore> {
    inner: Arc<T>,
    max_requests: usize,
    semaphore: Arc<Semaphore>,
}

impl<T: ObjectStore> LimitStore<T> {
    /// Create new limit store that will limit the maximum
    /// number of outstanding concurrent requests to
    /// `max_requests`
    pub fn new(inner: T, max_requests: usize) -> Self {
        Self {
            inner: Arc::new(inner),
            max_requests,
            semaphore: Arc::new(Semaphore::new(max_requests)),
        }
    }
}

impl<T: ObjectStore> std::fmt::Display for LimitStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LimitStore({}, {})", self.max_requests, self.inner)
    }
}

#[async_trait]
impl<T: ObjectStore> ObjectStore for LimitStore<T> {
    async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.put(location, payload).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.put_opts(location, payload, opts).await
    }
    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        let upload = self.inner.put_multipart(location).await?;
        Ok(Box::new(LimitUpload {
            semaphore: Arc::clone(&self.semaphore),
            upload,
        }))
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        let upload = self.inner.put_multipart_opts(location, opts).await?;
        Ok(Box::new(LimitUpload {
            semaphore: Arc::clone(&self.semaphore),
            upload,
        }))
    }

    async fn get(&self, location: &Path) -> Result<GetResult> {
        let permit = Arc::clone(&self.semaphore).acquire_owned().await.unwrap();
        let r = self.inner.get(location).await?;
        Ok(permit_get_result(r, permit))
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let permit = Arc::clone(&self.semaphore).acquire_owned().await.unwrap();
        let r = self.inner.get_opts(location, options).await?;
        Ok(permit_get_result(r, permit))
    }

    async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.get_range(location, range).await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.get_ranges(location, ranges).await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.head(location).await
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.delete(location).await
    }

    fn delete_stream<'a>(
        &'a self,
        locations: BoxStream<'a, Result<Path>>,
    ) -> BoxStream<'a, Result<Path>> {
        self.inner.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix = prefix.cloned();
        let inner = Arc::clone(&self.inner);
        let fut = Arc::clone(&self.semaphore)
            .acquire_owned()
            .map(move |permit| {
                let s = inner.list(prefix.as_ref());
                PermitWrapper::new(s, permit.unwrap())
            });
        fut.into_stream().flatten().boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix = prefix.cloned();
        let offset = offset.clone();
        let inner = Arc::clone(&self.inner);
        let fut = Arc::clone(&self.semaphore)
            .acquire_owned()
            .map(move |permit| {
                let s = inner.list_with_offset(prefix.as_ref(), &offset);
                PermitWrapper::new(s, permit.unwrap())
            });
        fut.into_stream().flatten().boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.copy(from, to).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.rename(from, to).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.copy_if_not_exists(from, to).await
    }

    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.inner.rename_if_not_exists(from, to).await
    }
}

fn permit_get_result(r: GetResult, permit: OwnedSemaphorePermit) -> GetResult {
    let payload = match r.payload {
        #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
        v @ GetResultPayload::File(_, _) => v,
        GetResultPayload::Stream(s) => {
            GetResultPayload::Stream(PermitWrapper::new(s, permit).boxed())
        }
    };
    GetResult { payload, ..r }
}

/// Combines an [`OwnedSemaphorePermit`] with some other type
struct PermitWrapper<T> {
    inner: T,
    #[allow(dead_code)]
    permit: OwnedSemaphorePermit,
}

impl<T> PermitWrapper<T> {
    fn new(inner: T, permit: OwnedSemaphorePermit) -> Self {
        Self { inner, permit }
    }
}

impl<T: Stream + Unpin> Stream for PermitWrapper<T> {
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An [`MultipartUpload`] wrapper that limits the maximum number of concurrent requests
#[derive(Debug)]
pub struct LimitUpload {
    upload: Box<dyn MultipartUpload>,
    semaphore: Arc<Semaphore>,
}

impl LimitUpload {
    /// Create a new [`LimitUpload`] limiting `upload` to `max_concurrency` concurrent requests
    pub fn new(upload: Box<dyn MultipartUpload>, max_concurrency: usize) -> Self {
        Self {
            upload,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }
}

#[async_trait]
impl MultipartUpload for LimitUpload {
    fn put_part(&mut self, data: PutPayload) -> UploadPart {
        let upload = self.upload.put_part(data);
        let s = Arc::clone(&self.semaphore);
        Box::pin(async move {
            let _permit = s.acquire().await.unwrap();
            upload.await
        })
    }

    async fn complete(&mut self) -> Result<PutResult> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.upload.complete().await
    }

    async fn abort(&mut self) -> Result<()> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.upload.abort().await
    }
}

#[cfg(test)]
mod tests {
    use crate::integration::*;
    use crate::limit::LimitStore;
    use crate::memory::InMemory;
    use crate::ObjectStore;
    use futures::stream::StreamExt;
    use std::pin::Pin;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn limit_test() {
        let max_requests = 10;
        let memory = InMemory::new();
        let integration = LimitStore::new(memory, max_requests);

        put_get_delete_list(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        stream_get(&integration).await;

        let mut streams = Vec::with_capacity(max_requests);
        for _ in 0..max_requests {
            let mut stream = integration.list(None).peekable();
            Pin::new(&mut stream).peek().await; // Ensure semaphore is acquired
            streams.push(stream);
        }

        let t = Duration::from_millis(20);

        // Expect to not be able to make another request
        let fut = integration.list(None).collect::<Vec<_>>();
        assert!(timeout(t, fut).await.is_err());

        // Drop one of the streams
        streams.pop();

        // Can now make another request
        integration.list(None).collect::<Vec<_>>().await;
    }
}
