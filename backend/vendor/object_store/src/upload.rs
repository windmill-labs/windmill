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

use std::task::{Context, Poll};

use crate::{PutPayload, PutPayloadMut, PutResult, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::future::BoxFuture;
use futures::ready;
use tokio::task::JoinSet;

/// An upload part request
pub type UploadPart = BoxFuture<'static, Result<()>>;

/// A trait allowing writing an object in fixed size chunks
///
/// Consecutive chunks of data can be written by calling [`MultipartUpload::put_part`] and polling
/// the returned futures to completion. Multiple futures returned by [`MultipartUpload::put_part`]
/// may be polled in parallel, allowing for concurrent uploads.
///
/// Once all part uploads have been polled to completion, the upload can be completed by
/// calling [`MultipartUpload::complete`]. This will make the entire uploaded object visible
/// as an atomic operation.It is implementation behind behaviour if [`MultipartUpload::complete`]
/// is called before all [`UploadPart`] have been polled to completion.
#[async_trait]
pub trait MultipartUpload: Send + std::fmt::Debug {
    /// Upload the next part
    ///
    /// Most stores require that all parts excluding the last are at least 5 MiB, and some
    /// further require that all parts excluding the last be the same size, e.g. [R2].
    /// Clients wanting to maximise compatibility should therefore perform writes in
    /// fixed size blocks larger than 5 MiB.
    ///
    /// Implementations may invoke this method multiple times and then await on the
    /// returned futures in parallel
    ///
    /// ```no_run
    /// # use futures::StreamExt;
    /// # use object_store::MultipartUpload;
    /// #
    /// # async fn test() {
    /// #
    /// let mut upload: Box<&dyn MultipartUpload> = todo!();
    /// let p1 = upload.put_part(vec![0; 10 * 1024 * 1024].into());
    /// let p2 = upload.put_part(vec![1; 10 * 1024 * 1024].into());
    /// futures::future::try_join(p1, p2).await.unwrap();
    /// upload.complete().await.unwrap();
    /// # }
    /// ```
    ///
    /// [R2]: https://developers.cloudflare.com/r2/objects/multipart-objects/#limitations
    fn put_part(&mut self, data: PutPayload) -> UploadPart;

    /// Complete the multipart upload
    ///
    /// It is implementation defined behaviour if this method is called before polling
    /// all [`UploadPart`] returned by [`MultipartUpload::put_part`] to completion. Additionally,
    /// it is implementation defined behaviour to call [`MultipartUpload::complete`]
    /// on an already completed or aborted [`MultipartUpload`].
    async fn complete(&mut self) -> Result<PutResult>;

    /// Abort the multipart upload
    ///
    /// If a [`MultipartUpload`] is dropped without calling [`MultipartUpload::complete`],
    /// some object stores will automatically clean up any previously uploaded parts.
    /// However, some stores, such as S3 and GCS, cannot perform cleanup on drop.
    /// As such [`MultipartUpload::abort`] can be invoked to perform this cleanup.
    ///
    /// It will not be possible to call `abort` in all failure scenarios, for example
    /// non-graceful shutdown of the calling application. It is therefore recommended
    /// object stores are configured with lifecycle rules to automatically cleanup
    /// unused parts older than some threshold. See [crate::aws] and [crate::gcp]
    /// for more information.
    ///
    /// It is implementation defined behaviour to call [`MultipartUpload::abort`]
    /// on an already completed or aborted [`MultipartUpload`]
    async fn abort(&mut self) -> Result<()>;
}

#[async_trait]
impl<W: MultipartUpload + ?Sized> MultipartUpload for Box<W> {
    fn put_part(&mut self, data: PutPayload) -> UploadPart {
        (**self).put_part(data)
    }

    async fn complete(&mut self) -> Result<PutResult> {
        (**self).complete().await
    }

    async fn abort(&mut self) -> Result<()> {
        (**self).abort().await
    }
}

/// A synchronous write API for uploading data in parallel in fixed size chunks
///
/// Uses multiple tokio tasks in a [`JoinSet`] to multiplex upload tasks in parallel
///
/// The design also takes inspiration from [`Sink`] with [`WriteMultipart::wait_for_capacity`]
/// allowing back pressure on producers, prior to buffering the next part. However, unlike
/// [`Sink`] this back pressure is optional, allowing integration with synchronous producers
///
/// [`Sink`]: futures::sink::Sink
#[derive(Debug)]
pub struct WriteMultipart {
    upload: Box<dyn MultipartUpload>,

    buffer: PutPayloadMut,

    chunk_size: usize,

    tasks: JoinSet<Result<()>>,
}

impl WriteMultipart {
    /// Create a new [`WriteMultipart`] that will upload using 5MB chunks
    pub fn new(upload: Box<dyn MultipartUpload>) -> Self {
        Self::new_with_chunk_size(upload, 5 * 1024 * 1024)
    }

    /// Create a new [`WriteMultipart`] that will upload in fixed `chunk_size` sized chunks
    pub fn new_with_chunk_size(upload: Box<dyn MultipartUpload>, chunk_size: usize) -> Self {
        Self {
            upload,
            chunk_size,
            buffer: PutPayloadMut::new(),
            tasks: Default::default(),
        }
    }

    /// Polls for there to be less than `max_concurrency` [`UploadPart`] in progress
    ///
    /// See [`Self::wait_for_capacity`] for an async version of this function
    pub fn poll_for_capacity(
        &mut self,
        cx: &mut Context<'_>,
        max_concurrency: usize,
    ) -> Poll<Result<()>> {
        while !self.tasks.is_empty() && self.tasks.len() >= max_concurrency {
            ready!(self.tasks.poll_join_next(cx)).unwrap()??
        }
        Poll::Ready(Ok(()))
    }

    /// Wait until there are less than `max_concurrency` [`UploadPart`] in progress
    ///
    /// See [`Self::poll_for_capacity`] for a [`Poll`] version of this function
    pub async fn wait_for_capacity(&mut self, max_concurrency: usize) -> Result<()> {
        futures::future::poll_fn(|cx| self.poll_for_capacity(cx, max_concurrency)).await
    }

    /// Write data to this [`WriteMultipart`]
    ///
    /// Data is buffered using [`PutPayloadMut::extend_from_slice`]. Implementations looking to
    /// write data from owned buffers may prefer [`Self::put`] as this avoids copying.
    ///
    /// Note this method is synchronous (not `async`) and will immediately
    /// start new uploads as soon as the internal `chunk_size` is hit,
    /// regardless of how many outstanding uploads are already in progress.
    ///
    /// Back pressure can optionally be applied to producers by calling
    /// [`Self::wait_for_capacity`] prior to calling this method
    pub fn write(&mut self, mut buf: &[u8]) {
        while !buf.is_empty() {
            let remaining = self.chunk_size - self.buffer.content_length();
            let to_read = buf.len().min(remaining);
            self.buffer.extend_from_slice(&buf[..to_read]);
            if to_read == remaining {
                let buffer = std::mem::take(&mut self.buffer);
                self.put_part(buffer.into())
            }
            buf = &buf[to_read..]
        }
    }

    /// Put a chunk of data into this [`WriteMultipart`] without copying
    ///
    /// Data is buffered using [`PutPayloadMut::push`]. Implementations looking to
    /// perform writes from non-owned buffers should prefer [`Self::write`] as this
    /// will allow multiple calls to share the same underlying allocation.
    ///
    /// See [`Self::write`] for information on backpressure
    pub fn put(&mut self, mut bytes: Bytes) {
        while !bytes.is_empty() {
            let remaining = self.chunk_size - self.buffer.content_length();
            if bytes.len() < remaining {
                self.buffer.push(bytes);
                return;
            }
            self.buffer.push(bytes.split_to(remaining));
            let buffer = std::mem::take(&mut self.buffer);
            self.put_part(buffer.into())
        }
    }

    pub(crate) fn put_part(&mut self, part: PutPayload) {
        self.tasks.spawn(self.upload.put_part(part));
    }

    /// Abort this upload, attempting to clean up any successfully uploaded parts
    pub async fn abort(mut self) -> Result<()> {
        self.tasks.shutdown().await;
        self.upload.abort().await
    }

    /// Flush final chunk, and await completion of all in-flight requests
    pub async fn finish(mut self) -> Result<PutResult> {
        if !self.buffer.is_empty() {
            let part = std::mem::take(&mut self.buffer);
            self.put_part(part.into())
        }

        self.wait_for_capacity(0).await?;

        match self.upload.complete().await {
            Err(e) => {
                self.tasks.shutdown().await;
                self.upload.abort().await?;
                Err(e)
            }
            Ok(result) => Ok(result),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use futures::FutureExt;
    use parking_lot::Mutex;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};

    use crate::memory::InMemory;
    use crate::path::Path;
    use crate::throttle::{ThrottleConfig, ThrottledStore};
    use crate::ObjectStore;

    use super::*;

    #[tokio::test]
    async fn test_concurrency() {
        let config = ThrottleConfig {
            wait_put_per_call: Duration::from_millis(1),
            ..Default::default()
        };

        let path = Path::from("foo");
        let store = ThrottledStore::new(InMemory::new(), config);
        let upload = store.put_multipart(&path).await.unwrap();
        let mut write = WriteMultipart::new_with_chunk_size(upload, 10);

        for _ in 0..20 {
            write.write(&[0; 5]);
        }
        assert!(write.wait_for_capacity(10).now_or_never().is_none());
        write.wait_for_capacity(10).await.unwrap()
    }

    #[derive(Debug, Default)]
    struct InstrumentedUpload {
        chunks: Arc<Mutex<Vec<PutPayload>>>,
    }

    #[async_trait]
    impl MultipartUpload for InstrumentedUpload {
        fn put_part(&mut self, data: PutPayload) -> UploadPart {
            self.chunks.lock().push(data);
            futures::future::ready(Ok(())).boxed()
        }

        async fn complete(&mut self) -> Result<PutResult> {
            Ok(PutResult {
                e_tag: None,
                version: None,
            })
        }

        async fn abort(&mut self) -> Result<()> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_write_multipart() {
        let mut rng = StdRng::seed_from_u64(42);

        for method in [0.0, 0.5, 1.0] {
            for _ in 0..10 {
                for chunk_size in [1, 17, 23] {
                    let upload = Box::<InstrumentedUpload>::default();
                    let chunks = Arc::clone(&upload.chunks);
                    let mut write = WriteMultipart::new_with_chunk_size(upload, chunk_size);

                    let mut expected = Vec::with_capacity(1024);

                    for _ in 0..50 {
                        let chunk_size = rng.random_range(0..30);
                        let data: Vec<_> = (0..chunk_size).map(|_| rng.random()).collect();
                        expected.extend_from_slice(&data);

                        match rng.random_bool(method) {
                            true => write.put(data.into()),
                            false => write.write(&data),
                        }
                    }
                    write.finish().await.unwrap();

                    let chunks = chunks.lock();

                    let actual: Vec<_> = chunks.iter().flatten().flatten().copied().collect();
                    assert_eq!(expected, actual);

                    for chunk in chunks.iter().take(chunks.len() - 1) {
                        assert_eq!(chunk.content_length(), chunk_size)
                    }

                    let last_chunk = chunks.last().unwrap().content_length();
                    assert!(last_chunk <= chunk_size, "{chunk_size}");
                }
            }
        }
    }
}
