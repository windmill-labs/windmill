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

//! A throttling object store wrapper
use parking_lot::Mutex;
use std::ops::Range;
use std::{convert::TryInto, sync::Arc};

use crate::multipart::{MultipartStore, PartId};
use crate::{
    path::Path, GetResult, GetResultPayload, ListResult, MultipartId, MultipartUpload, ObjectMeta,
    ObjectStore, PutMultipartOpts, PutOptions, PutPayload, PutResult, Result,
};
use crate::{GetOptions, UploadPart};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream::BoxStream, FutureExt, StreamExt};
use std::time::Duration;

/// Configuration settings for throttled store
#[derive(Debug, Default, Clone, Copy)]
pub struct ThrottleConfig {
    /// Sleep duration for every call to [`delete`](ThrottledStore::delete).
    ///
    /// Sleeping is done before the underlying store is called and independently of the success of
    /// the operation.
    pub wait_delete_per_call: Duration,

    /// Sleep duration for every byte received during [`get`](ThrottledStore::get).
    ///
    /// Sleeping is performed after the underlying store returned and only for successful gets. The
    /// sleep duration is additive to [`wait_get_per_call`](Self::wait_get_per_call).
    ///
    /// Note that the per-byte sleep only happens as the user consumes the output bytes. Should
    /// there be an intermediate failure (i.e. after partly consuming the output bytes), the
    /// resulting sleep time will be partial as well.
    pub wait_get_per_byte: Duration,

    /// Sleep duration for every call to [`get`](ThrottledStore::get).
    ///
    /// Sleeping is done before the underlying store is called and independently of the success of
    /// the operation. The sleep duration is additive to
    /// [`wait_get_per_byte`](Self::wait_get_per_byte).
    pub wait_get_per_call: Duration,

    /// Sleep duration for every call to [`list`](ThrottledStore::list).
    ///
    /// Sleeping is done before the underlying store is called and independently of the success of
    /// the operation. The sleep duration is additive to
    /// [`wait_list_per_entry`](Self::wait_list_per_entry).
    pub wait_list_per_call: Duration,

    /// Sleep duration for every entry received during [`list`](ThrottledStore::list).
    ///
    /// Sleeping is performed after the underlying store returned and only for successful lists.
    /// The sleep duration is additive to [`wait_list_per_call`](Self::wait_list_per_call).
    ///
    /// Note that the per-entry sleep only happens as the user consumes the output entries. Should
    /// there be an intermediate failure (i.e. after partly consuming the output entries), the
    /// resulting sleep time will be partial as well.
    pub wait_list_per_entry: Duration,

    /// Sleep duration for every call to
    /// [`list_with_delimiter`](ThrottledStore::list_with_delimiter).
    ///
    /// Sleeping is done before the underlying store is called and independently of the success of
    /// the operation. The sleep duration is additive to
    /// [`wait_list_with_delimiter_per_entry`](Self::wait_list_with_delimiter_per_entry).
    pub wait_list_with_delimiter_per_call: Duration,

    /// Sleep duration for every entry received during
    /// [`list_with_delimiter`](ThrottledStore::list_with_delimiter).
    ///
    /// Sleeping is performed after the underlying store returned and only for successful gets. The
    /// sleep duration is additive to
    /// [`wait_list_with_delimiter_per_call`](Self::wait_list_with_delimiter_per_call).
    pub wait_list_with_delimiter_per_entry: Duration,

    /// Sleep duration for every call to [`put`](ThrottledStore::put).
    ///
    /// Sleeping is done before the underlying store is called and independently of the success of
    /// the operation.
    pub wait_put_per_call: Duration,
}

/// Sleep only if non-zero duration
async fn sleep(duration: Duration) {
    if !duration.is_zero() {
        tokio::time::sleep(duration).await
    }
}

/// Store wrapper that wraps an inner store with some `sleep` calls.
///
/// This can be used for performance testing.
///
/// **Note that the behavior of the wrapper is deterministic and might not reflect real-world
/// conditions!**
#[derive(Debug)]
pub struct ThrottledStore<T> {
    inner: T,
    config: Arc<Mutex<ThrottleConfig>>,
}

impl<T> ThrottledStore<T> {
    /// Create new wrapper with zero waiting times.
    pub fn new(inner: T, config: ThrottleConfig) -> Self {
        Self {
            inner,
            config: Arc::new(Mutex::new(config)),
        }
    }

    /// Mutate config.
    pub fn config_mut<F>(&self, f: F)
    where
        F: Fn(&mut ThrottleConfig),
    {
        let mut guard = self.config.lock();
        f(&mut guard)
    }

    /// Return copy of current config.
    pub fn config(&self) -> ThrottleConfig {
        *self.config.lock()
    }
}

impl<T: ObjectStore> std::fmt::Display for ThrottledStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ThrottledStore({})", self.inner)
    }
}

#[async_trait]
impl<T: ObjectStore> ObjectStore for ThrottledStore<T> {
    async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
        sleep(self.config().wait_put_per_call).await;
        self.inner.put(location, payload).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        sleep(self.config().wait_put_per_call).await;
        self.inner.put_opts(location, payload, opts).await
    }

    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        let upload = self.inner.put_multipart(location).await?;
        Ok(Box::new(ThrottledUpload {
            upload,
            sleep: self.config().wait_put_per_call,
        }))
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        let upload = self.inner.put_multipart_opts(location, opts).await?;
        Ok(Box::new(ThrottledUpload {
            upload,
            sleep: self.config().wait_put_per_call,
        }))
    }

    async fn get(&self, location: &Path) -> Result<GetResult> {
        sleep(self.config().wait_get_per_call).await;

        // need to copy to avoid moving / referencing `self`
        let wait_get_per_byte = self.config().wait_get_per_byte;

        let result = self.inner.get(location).await?;
        Ok(throttle_get(result, wait_get_per_byte))
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        sleep(self.config().wait_get_per_call).await;

        // need to copy to avoid moving / referencing `self`
        let wait_get_per_byte = self.config().wait_get_per_byte;

        let result = self.inner.get_opts(location, options).await?;
        Ok(throttle_get(result, wait_get_per_byte))
    }

    async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
        let config = self.config();

        let sleep_duration =
            config.wait_get_per_call + config.wait_get_per_byte * (range.end - range.start) as u32;

        sleep(sleep_duration).await;

        self.inner.get_range(location, range).await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        let config = self.config();

        let total_bytes: u64 = ranges.iter().map(|range| range.end - range.start).sum();
        let sleep_duration =
            config.wait_get_per_call + config.wait_get_per_byte * total_bytes as u32;

        sleep(sleep_duration).await;

        self.inner.get_ranges(location, ranges).await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        sleep(self.config().wait_put_per_call).await;
        self.inner.head(location).await
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        sleep(self.config().wait_delete_per_call).await;

        self.inner.delete(location).await
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let stream = self.inner.list(prefix);
        let config = Arc::clone(&self.config);
        futures::stream::once(async move {
            let config = *config.lock();
            let wait_list_per_entry = config.wait_list_per_entry;
            sleep(config.wait_list_per_call).await;
            throttle_stream(stream, move |_| wait_list_per_entry)
        })
        .flatten()
        .boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let stream = self.inner.list_with_offset(prefix, offset);
        let config = Arc::clone(&self.config);
        futures::stream::once(async move {
            let config = *config.lock();
            let wait_list_per_entry = config.wait_list_per_entry;
            sleep(config.wait_list_per_call).await;
            throttle_stream(stream, move |_| wait_list_per_entry)
        })
        .flatten()
        .boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        sleep(self.config().wait_list_with_delimiter_per_call).await;

        match self.inner.list_with_delimiter(prefix).await {
            Ok(list_result) => {
                let entries_len = usize_to_u32_saturate(list_result.objects.len());
                sleep(self.config().wait_list_with_delimiter_per_entry * entries_len).await;
                Ok(list_result)
            }
            Err(err) => Err(err),
        }
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        sleep(self.config().wait_put_per_call).await;

        self.inner.copy(from, to).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        sleep(self.config().wait_put_per_call).await;

        self.inner.rename(from, to).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        sleep(self.config().wait_put_per_call).await;

        self.inner.copy_if_not_exists(from, to).await
    }

    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        sleep(self.config().wait_put_per_call).await;

        self.inner.rename_if_not_exists(from, to).await
    }
}

/// Saturated `usize` to `u32` cast.
fn usize_to_u32_saturate(x: usize) -> u32 {
    x.try_into().unwrap_or(u32::MAX)
}

fn throttle_get(result: GetResult, wait_get_per_byte: Duration) -> GetResult {
    #[allow(clippy::infallible_destructuring_match)]
    let s = match result.payload {
        GetResultPayload::Stream(s) => s,
        #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
        GetResultPayload::File(_, _) => unimplemented!(),
    };

    let stream = throttle_stream(s, move |bytes| {
        let bytes_len: u32 = usize_to_u32_saturate(bytes.len());
        wait_get_per_byte * bytes_len
    });

    GetResult {
        payload: GetResultPayload::Stream(stream),
        ..result
    }
}

fn throttle_stream<T: Send + 'static, E: Send + 'static, F>(
    stream: BoxStream<'_, Result<T, E>>,
    delay: F,
) -> BoxStream<'_, Result<T, E>>
where
    F: Fn(&T) -> Duration + Send + Sync + 'static,
{
    stream
        .then(move |result| {
            let delay = result.as_ref().ok().map(&delay).unwrap_or_default();
            sleep(delay).then(|_| futures::future::ready(result))
        })
        .boxed()
}

#[async_trait]
impl<T: MultipartStore> MultipartStore for ThrottledStore<T> {
    async fn create_multipart(&self, path: &Path) -> Result<MultipartId> {
        self.inner.create_multipart(path).await
    }

    async fn put_part(
        &self,
        path: &Path,
        id: &MultipartId,
        part_idx: usize,
        data: PutPayload,
    ) -> Result<PartId> {
        sleep(self.config().wait_put_per_call).await;
        self.inner.put_part(path, id, part_idx, data).await
    }

    async fn complete_multipart(
        &self,
        path: &Path,
        id: &MultipartId,
        parts: Vec<PartId>,
    ) -> Result<PutResult> {
        self.inner.complete_multipart(path, id, parts).await
    }

    async fn abort_multipart(&self, path: &Path, id: &MultipartId) -> Result<()> {
        self.inner.abort_multipart(path, id).await
    }
}

#[derive(Debug)]
struct ThrottledUpload {
    upload: Box<dyn MultipartUpload>,
    sleep: Duration,
}

#[async_trait]
impl MultipartUpload for ThrottledUpload {
    fn put_part(&mut self, data: PutPayload) -> UploadPart {
        let duration = self.sleep;
        let put = self.upload.put_part(data);
        Box::pin(async move {
            sleep(duration).await;
            put.await
        })
    }

    async fn complete(&mut self) -> Result<PutResult> {
        self.upload.complete().await
    }

    async fn abort(&mut self) -> Result<()> {
        self.upload.abort().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{integration::*, memory::InMemory, GetResultPayload};
    use futures::TryStreamExt;
    use tokio::time::Duration;
    use tokio::time::Instant;

    const WAIT_TIME: Duration = Duration::from_millis(100);
    const ZERO: Duration = Duration::from_millis(0); // Duration::default isn't constant

    macro_rules! assert_bounds {
        ($d:expr, $lower:expr) => {
            assert_bounds!($d, $lower, $lower + 2);
        };
        ($d:expr, $lower:expr, $upper:expr) => {
            let d = $d;
            let lower = $lower * WAIT_TIME;
            let upper = $upper * WAIT_TIME;
            assert!(d >= lower, "{:?} must be >= than {:?}", d, lower);
            assert!(d < upper, "{:?} must be < than {:?}", d, upper);
        };
    }

    #[tokio::test]
    async fn throttle_test() {
        let inner = InMemory::new();
        let store = ThrottledStore::new(inner, ThrottleConfig::default());

        put_get_delete_list(&store).await;
        list_uses_directories_correctly(&store).await;
        list_with_delimiter(&store).await;
        rename_and_copy(&store).await;
        copy_if_not_exists(&store).await;
        stream_get(&store).await;
        multipart(&store, &store).await;
    }

    #[tokio::test]
    async fn delete_test() {
        let inner = InMemory::new();
        let store = ThrottledStore::new(inner, ThrottleConfig::default());

        assert_bounds!(measure_delete(&store, None).await, 0);
        assert_bounds!(measure_delete(&store, Some(0)).await, 0);
        assert_bounds!(measure_delete(&store, Some(10)).await, 0);

        store.config_mut(|cfg| cfg.wait_delete_per_call = WAIT_TIME);
        assert_bounds!(measure_delete(&store, None).await, 1);
        assert_bounds!(measure_delete(&store, Some(0)).await, 1);
        assert_bounds!(measure_delete(&store, Some(10)).await, 1);
    }

    #[tokio::test]
    // macos github runner is so slow it can't complete within WAIT_TIME*2
    #[cfg(target_os = "linux")]
    async fn get_test() {
        let inner = InMemory::new();
        let store = ThrottledStore::new(inner, ThrottleConfig::default());

        assert_bounds!(measure_get(&store, None).await, 0);
        assert_bounds!(measure_get(&store, Some(0)).await, 0);
        assert_bounds!(measure_get(&store, Some(10)).await, 0);

        store.config_mut(|cfg| cfg.wait_get_per_call = WAIT_TIME);
        assert_bounds!(measure_get(&store, None).await, 1);
        assert_bounds!(measure_get(&store, Some(0)).await, 1);
        assert_bounds!(measure_get(&store, Some(10)).await, 1);

        store.config_mut(|cfg| {
            cfg.wait_get_per_call = ZERO;
            cfg.wait_get_per_byte = WAIT_TIME;
        });
        assert_bounds!(measure_get(&store, Some(2)).await, 2);

        store.config_mut(|cfg| {
            cfg.wait_get_per_call = WAIT_TIME;
            cfg.wait_get_per_byte = WAIT_TIME;
        });
        assert_bounds!(measure_get(&store, Some(2)).await, 3);
    }

    #[tokio::test]
    // macos github runner is so slow it can't complete within WAIT_TIME*2
    #[cfg(target_os = "linux")]
    async fn list_test() {
        let inner = InMemory::new();
        let store = ThrottledStore::new(inner, ThrottleConfig::default());

        assert_bounds!(measure_list(&store, 0).await, 0);
        assert_bounds!(measure_list(&store, 10).await, 0);

        store.config_mut(|cfg| cfg.wait_list_per_call = WAIT_TIME);
        assert_bounds!(measure_list(&store, 0).await, 1);
        assert_bounds!(measure_list(&store, 10).await, 1);

        store.config_mut(|cfg| {
            cfg.wait_list_per_call = ZERO;
            cfg.wait_list_per_entry = WAIT_TIME;
        });
        assert_bounds!(measure_list(&store, 2).await, 2);

        store.config_mut(|cfg| {
            cfg.wait_list_per_call = WAIT_TIME;
            cfg.wait_list_per_entry = WAIT_TIME;
        });
        assert_bounds!(measure_list(&store, 2).await, 3);
    }

    #[tokio::test]
    // macos github runner is so slow it can't complete within WAIT_TIME*2
    #[cfg(target_os = "linux")]
    async fn list_with_delimiter_test() {
        let inner = InMemory::new();
        let store = ThrottledStore::new(inner, ThrottleConfig::default());

        assert_bounds!(measure_list_with_delimiter(&store, 0).await, 0);
        assert_bounds!(measure_list_with_delimiter(&store, 10).await, 0);

        store.config_mut(|cfg| cfg.wait_list_with_delimiter_per_call = WAIT_TIME);
        assert_bounds!(measure_list_with_delimiter(&store, 0).await, 1);
        assert_bounds!(measure_list_with_delimiter(&store, 10).await, 1);

        store.config_mut(|cfg| {
            cfg.wait_list_with_delimiter_per_call = ZERO;
            cfg.wait_list_with_delimiter_per_entry = WAIT_TIME;
        });
        assert_bounds!(measure_list_with_delimiter(&store, 2).await, 2);

        store.config_mut(|cfg| {
            cfg.wait_list_with_delimiter_per_call = WAIT_TIME;
            cfg.wait_list_with_delimiter_per_entry = WAIT_TIME;
        });
        assert_bounds!(measure_list_with_delimiter(&store, 2).await, 3);
    }

    #[tokio::test]
    async fn put_test() {
        let inner = InMemory::new();
        let store = ThrottledStore::new(inner, ThrottleConfig::default());

        assert_bounds!(measure_put(&store, 0).await, 0);
        assert_bounds!(measure_put(&store, 10).await, 0);

        store.config_mut(|cfg| cfg.wait_put_per_call = WAIT_TIME);
        assert_bounds!(measure_put(&store, 0).await, 1);
        assert_bounds!(measure_put(&store, 10).await, 1);

        store.config_mut(|cfg| cfg.wait_put_per_call = ZERO);
        assert_bounds!(measure_put(&store, 0).await, 0);
    }

    async fn place_test_object(store: &ThrottledStore<InMemory>, n_bytes: Option<usize>) -> Path {
        let path = Path::from("foo");

        if let Some(n_bytes) = n_bytes {
            let data: Vec<_> = std::iter::repeat(1u8).take(n_bytes).collect();
            store.put(&path, data.into()).await.unwrap();
        } else {
            // ensure object is absent
            store.delete(&path).await.unwrap();
        }

        path
    }

    #[allow(dead_code)]
    async fn place_test_objects(store: &ThrottledStore<InMemory>, n_entries: usize) -> Path {
        let prefix = Path::from("foo");

        // clean up store
        let entries: Vec<_> = store.list(Some(&prefix)).try_collect().await.unwrap();

        for entry in entries {
            store.delete(&entry.location).await.unwrap();
        }

        // create new entries
        for i in 0..n_entries {
            let path = prefix.child(i.to_string().as_str());
            store.put(&path, "bar".into()).await.unwrap();
        }

        prefix
    }

    async fn measure_delete(store: &ThrottledStore<InMemory>, n_bytes: Option<usize>) -> Duration {
        let path = place_test_object(store, n_bytes).await;

        let t0 = Instant::now();
        store.delete(&path).await.unwrap();

        t0.elapsed()
    }

    #[allow(dead_code)]
    #[cfg(target_os = "linux")]
    async fn measure_get(store: &ThrottledStore<InMemory>, n_bytes: Option<usize>) -> Duration {
        let path = place_test_object(store, n_bytes).await;

        let t0 = Instant::now();
        let res = store.get(&path).await;
        if n_bytes.is_some() {
            // need to consume bytes to provoke sleep times
            let s = match res.unwrap().payload {
                GetResultPayload::Stream(s) => s,
                GetResultPayload::File(_, _) => unimplemented!(),
            };

            s.map_ok(|b| bytes::BytesMut::from(&b[..]))
                .try_concat()
                .await
                .unwrap();
        } else {
            assert!(res.is_err());
        }

        t0.elapsed()
    }

    #[allow(dead_code)]
    async fn measure_list(store: &ThrottledStore<InMemory>, n_entries: usize) -> Duration {
        let prefix = place_test_objects(store, n_entries).await;

        let t0 = Instant::now();
        store
            .list(Some(&prefix))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        t0.elapsed()
    }

    #[allow(dead_code)]
    async fn measure_list_with_delimiter(
        store: &ThrottledStore<InMemory>,
        n_entries: usize,
    ) -> Duration {
        let prefix = place_test_objects(store, n_entries).await;

        let t0 = Instant::now();
        store.list_with_delimiter(Some(&prefix)).await.unwrap();

        t0.elapsed()
    }

    async fn measure_put(store: &ThrottledStore<InMemory>, n_bytes: usize) -> Duration {
        let data: Vec<_> = std::iter::repeat(1u8).take(n_bytes).collect();

        let t0 = Instant::now();
        store.put(&Path::from("foo"), data.into()).await.unwrap();

        t0.elapsed()
    }
}
