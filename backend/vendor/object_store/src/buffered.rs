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

//! Utilities for performing tokio-style buffered IO

use crate::path::Path;
use crate::{
    Attributes, ObjectMeta, ObjectStore, PutMultipartOpts, PutOptions, PutPayloadMut, TagSet,
    WriteMultipart,
};
use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt};
use futures::ready;
use std::cmp::Ordering;
use std::io::{Error, ErrorKind, SeekFrom};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

/// The default buffer size used by [`BufReader`]
pub const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024;

/// An async-buffered reader compatible with the tokio IO traits
///
/// Internally this maintains a buffer of the requested size, and uses [`ObjectStore::get_range`]
/// to populate its internal buffer once depleted. This buffer is cleared on seek.
///
/// Whilst simple, this interface will typically be outperformed by the native [`ObjectStore`]
/// methods that better map to the network APIs. This is because most object stores have
/// very [high first-byte latencies], on the order of 100-200ms, and so avoiding unnecessary
/// round-trips is critical to throughput.
///
/// Systems looking to sequentially scan a file should instead consider using [`ObjectStore::get`],
/// or [`ObjectStore::get_opts`], or [`ObjectStore::get_range`] to read a particular range.
///
/// Systems looking to read multiple ranges of a file should instead consider using
/// [`ObjectStore::get_ranges`], which will optimise the vectored IO.
///
/// [high first-byte latencies]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html
pub struct BufReader {
    /// The object store to fetch data from
    store: Arc<dyn ObjectStore>,
    /// The size of the object
    size: u64,
    /// The path to the object
    path: Path,
    /// The current position in the object
    cursor: u64,
    /// The number of bytes to read in a single request
    capacity: usize,
    /// The buffered data if any
    buffer: Buffer,
}

impl std::fmt::Debug for BufReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufReader")
            .field("path", &self.path)
            .field("size", &self.size)
            .field("capacity", &self.capacity)
            .finish()
    }
}

enum Buffer {
    Empty,
    Pending(BoxFuture<'static, std::io::Result<Bytes>>),
    Ready(Bytes),
}

impl BufReader {
    /// Create a new [`BufReader`] from the provided [`ObjectMeta`] and [`ObjectStore`]
    pub fn new(store: Arc<dyn ObjectStore>, meta: &ObjectMeta) -> Self {
        Self::with_capacity(store, meta, DEFAULT_BUFFER_SIZE)
    }

    /// Create a new [`BufReader`] from the provided [`ObjectMeta`], [`ObjectStore`], and `capacity`
    pub fn with_capacity(store: Arc<dyn ObjectStore>, meta: &ObjectMeta, capacity: usize) -> Self {
        Self {
            path: meta.location.clone(),
            size: meta.size as _,
            store,
            capacity,
            cursor: 0,
            buffer: Buffer::Empty,
        }
    }

    fn poll_fill_buf_impl(
        &mut self,
        cx: &mut Context<'_>,
        amnt: usize,
    ) -> Poll<std::io::Result<&[u8]>> {
        let buf = &mut self.buffer;
        loop {
            match buf {
                Buffer::Empty => {
                    let store = Arc::clone(&self.store);
                    let path = self.path.clone();
                    let start = self.cursor.min(self.size) as _;
                    let end = self.cursor.saturating_add(amnt as u64).min(self.size) as _;

                    if start == end {
                        return Poll::Ready(Ok(&[]));
                    }

                    *buf = Buffer::Pending(Box::pin(async move {
                        Ok(store.get_range(&path, start..end).await?)
                    }))
                }
                Buffer::Pending(fut) => match ready!(fut.poll_unpin(cx)) {
                    Ok(b) => *buf = Buffer::Ready(b),
                    Err(e) => return Poll::Ready(Err(e)),
                },
                Buffer::Ready(r) => return Poll::Ready(Ok(r)),
            }
        }
    }
}

impl AsyncSeek for BufReader {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.cursor = match position {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => checked_add_signed(self.size, offset).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Seeking {offset} from end of {} byte file would result in overflow",
                        self.size
                    ),
                )
            })?,
            SeekFrom::Current(offset) => {
                checked_add_signed(self.cursor, offset).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Seeking {offset} from current offset of {} would result in overflow",
                            self.cursor
                        ),
                    )
                })?
            }
        };
        self.buffer = Buffer::Empty;
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Poll::Ready(Ok(self.cursor))
    }
}

impl AsyncRead for BufReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // Read the maximum of the internal buffer and `out`
        let to_read = out.remaining().max(self.capacity);
        let r = match ready!(self.poll_fill_buf_impl(cx, to_read)) {
            Ok(buf) => {
                let to_consume = out.remaining().min(buf.len());
                out.put_slice(&buf[..to_consume]);
                self.consume(to_consume);
                Ok(())
            }
            Err(e) => Err(e),
        };
        Poll::Ready(r)
    }
}

impl AsyncBufRead for BufReader {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        let capacity = self.capacity;
        self.get_mut().poll_fill_buf_impl(cx, capacity)
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        match &mut self.buffer {
            Buffer::Empty => assert_eq!(amt, 0, "cannot consume from empty buffer"),
            Buffer::Ready(b) => match b.len().cmp(&amt) {
                Ordering::Less => panic!("{amt} exceeds buffer sized of {}", b.len()),
                Ordering::Greater => *b = b.slice(amt..),
                Ordering::Equal => self.buffer = Buffer::Empty,
            },
            Buffer::Pending(_) => panic!("cannot consume from pending buffer"),
        }
        self.cursor += amt as u64;
    }
}

/// An async buffered writer compatible with the tokio IO traits
///
/// This writer adaptively uses [`ObjectStore::put`] or
/// [`ObjectStore::put_multipart`] depending on the amount of data that has
/// been written.
///
/// Up to `capacity` bytes will be buffered in memory, and flushed on shutdown
/// using [`ObjectStore::put`]. If `capacity` is exceeded, data will instead be
/// streamed using [`ObjectStore::put_multipart`]
pub struct BufWriter {
    capacity: usize,
    max_concurrency: usize,
    attributes: Option<Attributes>,
    tags: Option<TagSet>,
    extensions: Option<::http::Extensions>,
    state: BufWriterState,
    store: Arc<dyn ObjectStore>,
}

impl std::fmt::Debug for BufWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufWriter")
            .field("capacity", &self.capacity)
            .finish()
    }
}

enum BufWriterState {
    /// Buffer up to capacity bytes
    Buffer(Path, PutPayloadMut),
    /// [`ObjectStore::put_multipart`]
    Prepare(BoxFuture<'static, crate::Result<WriteMultipart>>),
    /// Write to a multipart upload
    Write(Option<WriteMultipart>),
    /// [`ObjectStore::put`]
    Flush(BoxFuture<'static, crate::Result<()>>),
}

impl BufWriter {
    /// Create a new [`BufWriter`] from the provided [`ObjectStore`] and [`Path`]
    pub fn new(store: Arc<dyn ObjectStore>, path: Path) -> Self {
        Self::with_capacity(store, path, 10 * 1024 * 1024)
    }

    /// Create a new [`BufWriter`] from the provided [`ObjectStore`], [`Path`] and `capacity`
    pub fn with_capacity(store: Arc<dyn ObjectStore>, path: Path, capacity: usize) -> Self {
        Self {
            capacity,
            store,
            max_concurrency: 8,
            attributes: None,
            tags: None,
            extensions: None,
            state: BufWriterState::Buffer(path, PutPayloadMut::new()),
        }
    }

    /// Override the maximum number of in-flight requests for this writer
    ///
    /// Defaults to 8
    pub fn with_max_concurrency(self, max_concurrency: usize) -> Self {
        Self {
            max_concurrency,
            ..self
        }
    }

    /// Set the attributes of the uploaded object
    pub fn with_attributes(self, attributes: Attributes) -> Self {
        Self {
            attributes: Some(attributes),
            ..self
        }
    }

    /// Set the tags of the uploaded object
    pub fn with_tags(self, tags: TagSet) -> Self {
        Self {
            tags: Some(tags),
            ..self
        }
    }

    /// Set the extensions of the uploaded object
    ///
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    pub fn with_extensions(self, extensions: ::http::Extensions) -> Self {
        Self {
            extensions: Some(extensions),
            ..self
        }
    }

    /// Write data to the writer in [`Bytes`].
    ///
    /// Unlike [`AsyncWrite::poll_write`], `put` can write data without extra copying.
    ///
    /// This API is recommended while the data source generates [`Bytes`].
    pub async fn put(&mut self, bytes: Bytes) -> crate::Result<()> {
        loop {
            return match &mut self.state {
                BufWriterState::Write(Some(write)) => {
                    write.wait_for_capacity(self.max_concurrency).await?;
                    write.put(bytes);
                    Ok(())
                }
                BufWriterState::Write(None) | BufWriterState::Flush(_) => {
                    panic!("Already shut down")
                }
                // NOTE
                //
                // This case should never happen in practice, but rust async API does
                // make it possible for users to call `put` before `poll_write` returns `Ready`.
                //
                // We allow such usage by `await` the future and continue the loop.
                BufWriterState::Prepare(f) => {
                    self.state = BufWriterState::Write(f.await?.into());
                    continue;
                }
                BufWriterState::Buffer(path, b) => {
                    if b.content_length().saturating_add(bytes.len()) < self.capacity {
                        b.push(bytes);
                        Ok(())
                    } else {
                        let buffer = std::mem::take(b);
                        let path = std::mem::take(path);
                        let opts = PutMultipartOpts {
                            attributes: self.attributes.take().unwrap_or_default(),
                            tags: self.tags.take().unwrap_or_default(),
                            extensions: self.extensions.take().unwrap_or_default(),
                        };
                        let upload = self.store.put_multipart_opts(&path, opts).await?;
                        let mut chunked =
                            WriteMultipart::new_with_chunk_size(upload, self.capacity);
                        for chunk in buffer.freeze() {
                            chunked.put(chunk);
                        }
                        chunked.put(bytes);
                        self.state = BufWriterState::Write(Some(chunked));
                        Ok(())
                    }
                }
            };
        }
    }

    /// Abort this writer, cleaning up any partially uploaded state
    ///
    /// # Panic
    ///
    /// Panics if this writer has already been shutdown or aborted
    pub async fn abort(&mut self) -> crate::Result<()> {
        match &mut self.state {
            BufWriterState::Buffer(_, _) | BufWriterState::Prepare(_) => Ok(()),
            BufWriterState::Flush(_) => panic!("Already shut down"),
            BufWriterState::Write(x) => x.take().unwrap().abort().await,
        }
    }
}

impl AsyncWrite for BufWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let cap = self.capacity;
        let max_concurrency = self.max_concurrency;
        loop {
            return match &mut self.state {
                BufWriterState::Write(Some(write)) => {
                    ready!(write.poll_for_capacity(cx, max_concurrency))?;
                    write.write(buf);
                    Poll::Ready(Ok(buf.len()))
                }
                BufWriterState::Write(None) | BufWriterState::Flush(_) => {
                    panic!("Already shut down")
                }
                BufWriterState::Prepare(f) => {
                    self.state = BufWriterState::Write(ready!(f.poll_unpin(cx)?).into());
                    continue;
                }
                BufWriterState::Buffer(path, b) => {
                    if b.content_length().saturating_add(buf.len()) >= cap {
                        let buffer = std::mem::take(b);
                        let path = std::mem::take(path);
                        let opts = PutMultipartOpts {
                            attributes: self.attributes.take().unwrap_or_default(),
                            tags: self.tags.take().unwrap_or_default(),
                            extensions: self.extensions.take().unwrap_or_default(),
                        };
                        let store = Arc::clone(&self.store);
                        self.state = BufWriterState::Prepare(Box::pin(async move {
                            let upload = store.put_multipart_opts(&path, opts).await?;
                            let mut chunked = WriteMultipart::new_with_chunk_size(upload, cap);
                            for chunk in buffer.freeze() {
                                chunked.put(chunk);
                            }
                            Ok(chunked)
                        }));
                        continue;
                    }
                    b.extend_from_slice(buf);
                    Poll::Ready(Ok(buf.len()))
                }
            };
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            return match &mut self.state {
                BufWriterState::Write(_) | BufWriterState::Buffer(_, _) => Poll::Ready(Ok(())),
                BufWriterState::Flush(_) => panic!("Already shut down"),
                BufWriterState::Prepare(f) => {
                    self.state = BufWriterState::Write(ready!(f.poll_unpin(cx)?).into());
                    continue;
                }
            };
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            match &mut self.state {
                BufWriterState::Prepare(f) => {
                    self.state = BufWriterState::Write(ready!(f.poll_unpin(cx)?).into());
                }
                BufWriterState::Buffer(p, b) => {
                    let buf = std::mem::take(b);
                    let path = std::mem::take(p);
                    let opts = PutOptions {
                        attributes: self.attributes.take().unwrap_or_default(),
                        tags: self.tags.take().unwrap_or_default(),
                        ..Default::default()
                    };
                    let store = Arc::clone(&self.store);
                    self.state = BufWriterState::Flush(Box::pin(async move {
                        store.put_opts(&path, buf.into(), opts).await?;
                        Ok(())
                    }));
                }
                BufWriterState::Flush(f) => return f.poll_unpin(cx).map_err(std::io::Error::from),
                BufWriterState::Write(x) => {
                    let upload = x.take().ok_or_else(|| {
                        std::io::Error::new(
                            ErrorKind::InvalidInput,
                            "Cannot shutdown a writer that has already been shut down",
                        )
                    })?;
                    self.state = BufWriterState::Flush(
                        async move {
                            upload.finish().await?;
                            Ok(())
                        }
                        .boxed(),
                    )
                }
            }
        }
    }
}

/// Port of standardised function as requires Rust 1.66
///
/// <https://github.com/rust-lang/rust/pull/87601/files#diff-b9390ee807a1dae3c3128dce36df56748ad8d23c6e361c0ebba4d744bf6efdb9R1533>
#[inline]
fn checked_add_signed(a: u64, rhs: i64) -> Option<u64> {
    let (res, overflowed) = a.overflowing_add(rhs as _);
    let overflow = overflowed ^ (rhs < 0);
    (!overflow).then_some(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::InMemory;
    use crate::path::Path;
    use crate::{Attribute, GetOptions};
    use itertools::Itertools;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_buf_reader() {
        let store = Arc::new(InMemory::new()) as Arc<dyn ObjectStore>;

        let existent = Path::from("exists.txt");
        const BYTES: usize = 4096;

        let data: Bytes = b"12345678".iter().cycle().copied().take(BYTES).collect();
        store.put(&existent, data.clone().into()).await.unwrap();

        let meta = store.head(&existent).await.unwrap();

        let mut reader = BufReader::new(Arc::clone(&store), &meta);
        let mut out = Vec::with_capacity(BYTES);
        let read = reader.read_to_end(&mut out).await.unwrap();

        assert_eq!(read, BYTES);
        assert_eq!(&out, &data);

        let err = reader.seek(SeekFrom::Current(i64::MIN)).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Seeking -9223372036854775808 from current offset of 4096 would result in overflow"
        );

        reader.rewind().await.unwrap();

        let err = reader.seek(SeekFrom::Current(-1)).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Seeking -1 from current offset of 0 would result in overflow"
        );

        // Seeking beyond the bounds of the file is permitted but should return no data
        reader.seek(SeekFrom::Start(u64::MAX)).await.unwrap();
        let buf = reader.fill_buf().await.unwrap();
        assert!(buf.is_empty());

        let err = reader.seek(SeekFrom::Current(1)).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Seeking 1 from current offset of 18446744073709551615 would result in overflow"
        );

        for capacity in [200, 1024, 4096, DEFAULT_BUFFER_SIZE] {
            let store = Arc::clone(&store);
            let mut reader = BufReader::with_capacity(store, &meta, capacity);

            let mut bytes_read = 0;
            loop {
                let buf = reader.fill_buf().await.unwrap();
                if buf.is_empty() {
                    assert_eq!(bytes_read, BYTES);
                    break;
                }
                assert!(buf.starts_with(b"12345678"));
                bytes_read += 8;
                reader.consume(8);
            }

            let mut buf = Vec::with_capacity(76);
            reader.seek(SeekFrom::Current(-76)).await.unwrap();
            reader.read_to_end(&mut buf).await.unwrap();
            assert_eq!(&buf, &data[BYTES - 76..]);

            reader.rewind().await.unwrap();
            let buffer = reader.fill_buf().await.unwrap();
            assert_eq!(buffer, &data[..capacity.min(BYTES)]);

            reader.seek(SeekFrom::Start(325)).await.unwrap();
            let buffer = reader.fill_buf().await.unwrap();
            assert_eq!(buffer, &data[325..(325 + capacity).min(BYTES)]);

            reader.seek(SeekFrom::End(0)).await.unwrap();
            let buffer = reader.fill_buf().await.unwrap();
            assert!(buffer.is_empty());
        }
    }

    // Note: `BufWriter::with_tags` functionality is tested in `crate::tests::tagging`
    #[tokio::test]
    async fn test_buf_writer() {
        let store = Arc::new(InMemory::new()) as Arc<dyn ObjectStore>;
        let path = Path::from("file.txt");
        let attributes = Attributes::from_iter([
            (Attribute::ContentType, "text/html"),
            (Attribute::CacheControl, "max-age=604800"),
        ]);

        // Test put
        let mut writer = BufWriter::with_capacity(Arc::clone(&store), path.clone(), 30)
            .with_attributes(attributes.clone());
        writer.write_all(&[0; 20]).await.unwrap();
        writer.flush().await.unwrap();
        writer.write_all(&[0; 5]).await.unwrap();
        writer.shutdown().await.unwrap();
        let response = store
            .get_opts(
                &path,
                GetOptions {
                    head: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(response.meta.size, 25);
        assert_eq!(response.attributes, attributes);

        // Test multipart
        let mut writer = BufWriter::with_capacity(Arc::clone(&store), path.clone(), 30)
            .with_attributes(attributes.clone());
        writer.write_all(&[0; 20]).await.unwrap();
        writer.flush().await.unwrap();
        writer.write_all(&[0; 20]).await.unwrap();
        writer.shutdown().await.unwrap();
        let response = store
            .get_opts(
                &path,
                GetOptions {
                    head: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(response.meta.size, 40);
        assert_eq!(response.attributes, attributes);
    }

    #[tokio::test]
    async fn test_buf_writer_with_put() {
        let store = Arc::new(InMemory::new()) as Arc<dyn ObjectStore>;
        let path = Path::from("file.txt");

        // Test put
        let mut writer = BufWriter::with_capacity(Arc::clone(&store), path.clone(), 30);
        writer
            .put(Bytes::from((0..20).collect_vec()))
            .await
            .unwrap();
        writer
            .put(Bytes::from((20..25).collect_vec()))
            .await
            .unwrap();
        writer.shutdown().await.unwrap();
        let response = store
            .get_opts(
                &path,
                GetOptions {
                    head: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(response.meta.size, 25);
        assert_eq!(response.bytes().await.unwrap(), (0..25).collect_vec());

        // Test multipart
        let mut writer = BufWriter::with_capacity(Arc::clone(&store), path.clone(), 30);
        writer
            .put(Bytes::from((0..20).collect_vec()))
            .await
            .unwrap();
        writer
            .put(Bytes::from((20..40).collect_vec()))
            .await
            .unwrap();
        writer.shutdown().await.unwrap();
        let response = store
            .get_opts(
                &path,
                GetOptions {
                    head: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(response.meta.size, 40);
        assert_eq!(response.bytes().await.unwrap(), (0..40).collect_vec());
    }
}
