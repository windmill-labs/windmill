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

//! A [`ChunkedStore`] that can be used to test streaming behaviour

use std::fmt::{Debug, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use futures::stream::BoxStream;
use futures::StreamExt;

use crate::path::Path;
use crate::{
    GetOptions, GetResult, GetResultPayload, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    PutMultipartOpts, PutOptions, PutResult,
};
use crate::{PutPayload, Result};

/// Wraps a [`ObjectStore`] and makes its get response return chunks
/// in a controllable manner.
///
/// A `ChunkedStore` makes the memory consumption and performance of
/// the wrapped [`ObjectStore`] worse. It is intended for use within
/// tests, to control the chunks in the produced output streams. For
/// example, it is used to verify the delimiting logic in
/// newline_delimited_stream.
#[derive(Debug)]
pub struct ChunkedStore {
    inner: Arc<dyn ObjectStore>,
    chunk_size: usize, // chunks are in memory, so we use usize not u64
}

impl ChunkedStore {
    /// Creates a new [`ChunkedStore`] with the specified chunk_size
    pub fn new(inner: Arc<dyn ObjectStore>, chunk_size: usize) -> Self {
        Self { inner, chunk_size }
    }
}

impl Display for ChunkedStore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChunkedStore({})", self.inner)
    }
}

#[async_trait]
impl ObjectStore for ChunkedStore {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        self.inner.put_opts(location, payload, opts).await
    }

    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        self.inner.put_multipart(location).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        self.inner.put_multipart_opts(location, opts).await
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let r = self.inner.get_opts(location, options).await?;
        let stream = match r.payload {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            GetResultPayload::File(file, path) => {
                crate::local::chunked_stream(file, path, r.range.clone(), self.chunk_size)
            }
            GetResultPayload::Stream(stream) => {
                let buffer = BytesMut::new();
                futures::stream::unfold(
                    (stream, buffer, false, self.chunk_size),
                    |(mut stream, mut buffer, mut exhausted, chunk_size)| async move {
                        // Keep accumulating bytes until we reach capacity as long as
                        // the stream can provide them:
                        if exhausted {
                            return None;
                        }
                        while buffer.len() < chunk_size {
                            match stream.next().await {
                                None => {
                                    exhausted = true;
                                    let slice = buffer.split_off(0).freeze();
                                    return Some((
                                        Ok(slice),
                                        (stream, buffer, exhausted, chunk_size),
                                    ));
                                }
                                Some(Ok(bytes)) => {
                                    buffer.put(bytes);
                                }
                                Some(Err(e)) => {
                                    return Some((
                                        Err(crate::Error::Generic {
                                            store: "ChunkedStore",
                                            source: Box::new(e),
                                        }),
                                        (stream, buffer, exhausted, chunk_size),
                                    ))
                                }
                            };
                        }
                        // Return the chunked values as the next value in the stream
                        let slice = buffer.split_to(chunk_size).freeze();
                        Some((Ok(slice), (stream, buffer, exhausted, chunk_size)))
                    },
                )
                .boxed()
            }
        };
        Ok(GetResult {
            payload: GetResultPayload::Stream(stream),
            ..r
        })
    }

    async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
        self.inner.get_range(location, range).await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        self.inner.head(location).await
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        self.inner.delete(location).await
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        self.inner.copy(from, to).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.inner.copy_if_not_exists(from, to).await
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    #[cfg(feature = "fs")]
    use crate::integration::*;
    #[cfg(feature = "fs")]
    use crate::local::LocalFileSystem;
    use crate::memory::InMemory;
    use crate::path::Path;

    use super::*;

    #[tokio::test]
    async fn test_chunked_basic() {
        let location = Path::parse("test").unwrap();
        let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        store.put(&location, vec![0; 1001].into()).await.unwrap();

        for chunk_size in [10, 20, 31] {
            let store = ChunkedStore::new(Arc::clone(&store), chunk_size);
            let mut s = match store.get(&location).await.unwrap().payload {
                GetResultPayload::Stream(s) => s,
                _ => unreachable!(),
            };

            let mut remaining = 1001;
            while let Some(next) = s.next().await {
                let size = next.unwrap().len() as u64;
                let expected = remaining.min(chunk_size as u64);
                assert_eq!(size, expected);
                remaining -= expected;
            }
            assert_eq!(remaining, 0);
        }
    }

    #[cfg(feature = "fs")]
    #[tokio::test]
    async fn test_chunked() {
        let temporary = tempfile::tempdir().unwrap();
        let integrations: &[Arc<dyn ObjectStore>] = &[
            Arc::new(InMemory::new()),
            Arc::new(LocalFileSystem::new_with_prefix(temporary.path()).unwrap()),
        ];

        for integration in integrations {
            let integration = ChunkedStore::new(Arc::clone(integration), 100);

            put_get_delete_list(&integration).await;
            get_opts(&integration).await;
            list_uses_directories_correctly(&integration).await;
            list_with_delimiter(&integration).await;
            rename_and_copy(&integration).await;
            copy_if_not_exists(&integration).await;
            stream_get(&integration).await;
        }
    }
}
