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

//! Common logic for interacting with remote object stores
use std::{
    fmt::Display,
    ops::{Range, RangeBounds},
};

use super::Result;
use bytes::Bytes;
use futures::{stream::StreamExt, Stream, TryStreamExt};

#[cfg(any(feature = "azure", feature = "http"))]
pub(crate) static RFC1123_FMT: &str = "%a, %d %h %Y %T GMT";

// deserialize dates according to rfc1123
#[cfg(any(feature = "azure", feature = "http"))]
pub(crate) fn deserialize_rfc1123<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    let naive =
        chrono::NaiveDateTime::parse_from_str(&s, RFC1123_FMT).map_err(serde::de::Error::custom)?;
    Ok(chrono::TimeZone::from_utc_datetime(&chrono::Utc, &naive))
}

#[cfg(any(feature = "aws", feature = "azure"))]
pub(crate) fn hmac_sha256(secret: impl AsRef<[u8]>, bytes: impl AsRef<[u8]>) -> ring::hmac::Tag {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret.as_ref());
    ring::hmac::sign(&key, bytes.as_ref())
}

/// Collect a stream into [`Bytes`] avoiding copying in the event of a single chunk
pub async fn collect_bytes<S, E>(mut stream: S, size_hint: Option<u64>) -> Result<Bytes, E>
where
    E: Send,
    S: Stream<Item = Result<Bytes, E>> + Send + Unpin,
{
    let first = stream.next().await.transpose()?.unwrap_or_default();

    // Avoid copying if single response
    match stream.next().await.transpose()? {
        None => Ok(first),
        Some(second) => {
            let size_hint = size_hint.unwrap_or_else(|| first.len() as u64 + second.len() as u64);

            let mut buf = Vec::with_capacity(size_hint as usize);
            buf.extend_from_slice(&first);
            buf.extend_from_slice(&second);
            while let Some(maybe_bytes) = stream.next().await {
                buf.extend_from_slice(&maybe_bytes?);
            }

            Ok(buf.into())
        }
    }
}

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
/// Takes a function and spawns it to a tokio blocking pool if available
pub(crate) async fn maybe_spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(runtime) => runtime.spawn_blocking(f).await?,
        Err(_) => f(),
    }
}

/// Range requests with a gap less than or equal to this,
/// will be coalesced into a single request by [`coalesce_ranges`]
pub const OBJECT_STORE_COALESCE_DEFAULT: u64 = 1024 * 1024;

/// Up to this number of range requests will be performed in parallel by [`coalesce_ranges`]
pub(crate) const OBJECT_STORE_COALESCE_PARALLEL: usize = 10;

/// Takes a function `fetch` that can fetch a range of bytes and uses this to
/// fetch the provided byte `ranges`
///
/// To improve performance it will:
///
/// * Combine ranges less than `coalesce` bytes apart into a single call to `fetch`
/// * Make multiple `fetch` requests in parallel (up to maximum of 10)
///
pub async fn coalesce_ranges<F, E, Fut>(
    ranges: &[Range<u64>],
    fetch: F,
    coalesce: u64,
) -> Result<Vec<Bytes>, E>
where
    F: Send + FnMut(Range<u64>) -> Fut,
    E: Send,
    Fut: std::future::Future<Output = Result<Bytes, E>> + Send,
{
    let fetch_ranges = merge_ranges(ranges, coalesce);

    let fetched: Vec<_> = futures::stream::iter(fetch_ranges.iter().cloned())
        .map(fetch)
        .buffered(OBJECT_STORE_COALESCE_PARALLEL)
        .try_collect()
        .await?;

    Ok(ranges
        .iter()
        .map(|range| {
            let idx = fetch_ranges.partition_point(|v| v.start <= range.start) - 1;
            let fetch_range = &fetch_ranges[idx];
            let fetch_bytes = &fetched[idx];

            let start = range.start - fetch_range.start;
            let end = range.end - fetch_range.start;
            let range = (start as usize)..(end as usize).min(fetch_bytes.len());
            fetch_bytes.slice(range)
        })
        .collect())
}

/// Returns a sorted list of ranges that cover `ranges`
fn merge_ranges(ranges: &[Range<u64>], coalesce: u64) -> Vec<Range<u64>> {
    if ranges.is_empty() {
        return vec![];
    }

    let mut ranges = ranges.to_vec();
    ranges.sort_unstable_by_key(|range| range.start);

    let mut ret = Vec::with_capacity(ranges.len());
    let mut start_idx = 0;
    let mut end_idx = 1;

    while start_idx != ranges.len() {
        let mut range_end = ranges[start_idx].end;

        while end_idx != ranges.len()
            && ranges[end_idx]
                .start
                .checked_sub(range_end)
                .map(|delta| delta <= coalesce)
                .unwrap_or(true)
        {
            range_end = range_end.max(ranges[end_idx].end);
            end_idx += 1;
        }

        let start = ranges[start_idx].start;
        let end = range_end;
        ret.push(start..end);

        start_idx = end_idx;
        end_idx += 1;
    }

    ret
}

/// Request only a portion of an object's bytes
///
/// These can be created from [usize] ranges, like
///
/// ```rust
/// # use object_store::GetRange;
/// let range1: GetRange = (50..150).into();
/// let range2: GetRange = (50..=150).into();
/// let range3: GetRange = (50..).into();
/// let range4: GetRange = (..150).into();
/// ```
///
/// Implementations may wish to inspect [`GetResult`] for the exact byte
/// range returned.
///
/// [`GetResult`]: crate::GetResult
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GetRange {
    /// Request a specific range of bytes
    ///
    /// If the given range is zero-length or starts after the end of the object,
    /// an error will be returned. Additionally, if the range ends after the end
    /// of the object, the entire remainder of the object will be returned.
    /// Otherwise, the exact requested range will be returned.
    ///
    /// Note that range is u64 (i.e., not usize),
    /// as `object_store` supports 32-bit architectures such as WASM
    Bounded(Range<u64>),
    /// Request all bytes starting from a given byte offset
    Offset(u64),
    /// Request up to the last n bytes
    Suffix(u64),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidGetRange {
    #[error("Wanted range starting at {requested}, but object was only {length} bytes long")]
    StartTooLarge { requested: u64, length: u64 },

    #[error("Range started at {start} and ended at {end}")]
    Inconsistent { start: u64, end: u64 },

    #[error("Range {requested} is larger than system memory limit {max}")]
    TooLarge { requested: u64, max: u64 },
}

impl GetRange {
    /// Check if the range is valid.
    pub fn is_valid(&self) -> Result<(), InvalidGetRange> {
        if let Self::Bounded(r) = self {
            if r.end <= r.start {
                return Err(InvalidGetRange::Inconsistent {
                    start: r.start,
                    end: r.end,
                });
            }
            if (r.end - r.start) > usize::MAX as u64 {
                return Err(InvalidGetRange::TooLarge {
                    requested: r.start,
                    max: usize::MAX as u64,
                });
            }
        }
        Ok(())
    }

    /// Convert to a [`Range`] if [valid](Self::is_valid).
    pub fn as_range(&self, len: u64) -> Result<Range<u64>, InvalidGetRange> {
        self.is_valid()?;
        match self {
            Self::Bounded(r) => {
                if r.start >= len {
                    Err(InvalidGetRange::StartTooLarge {
                        requested: r.start,
                        length: len,
                    })
                } else if r.end > len {
                    Ok(r.start..len)
                } else {
                    Ok(r.clone())
                }
            }
            Self::Offset(o) => {
                if *o >= len {
                    Err(InvalidGetRange::StartTooLarge {
                        requested: *o,
                        length: len,
                    })
                } else {
                    Ok(*o..len)
                }
            }
            Self::Suffix(n) => Ok(len.saturating_sub(*n)..len),
        }
    }
}

impl Display for GetRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bounded(r) => write!(f, "bytes={}-{}", r.start, r.end - 1),
            Self::Offset(o) => write!(f, "bytes={o}-"),
            Self::Suffix(n) => write!(f, "bytes=-{n}"),
        }
    }
}

impl<T: RangeBounds<u64>> From<T> for GetRange {
    fn from(value: T) -> Self {
        use std::ops::Bound::*;
        let first = match value.start_bound() {
            Included(i) => *i,
            Excluded(i) => i + 1,
            Unbounded => 0,
        };
        match value.end_bound() {
            Included(i) => Self::Bounded(first..(i + 1)),
            Excluded(i) => Self::Bounded(first..*i),
            Unbounded => Self::Offset(first),
        }
    }
}
// http://docs.aws.amazon.com/general/latest/gr/sigv4-create-canonical-request.html
//
// Do not URI-encode any of the unreserved characters that RFC 3986 defines:
// A-Z, a-z, 0-9, hyphen ( - ), underscore ( _ ), period ( . ), and tilde ( ~ ).
#[cfg(any(feature = "aws", feature = "gcp"))]
pub(crate) const STRICT_ENCODE_SET: percent_encoding::AsciiSet = percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Computes the SHA256 digest of `body` returned as a hex encoded string
#[cfg(any(feature = "aws", feature = "gcp"))]
pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, bytes);
    hex_encode(digest.as_ref())
}

/// Returns `bytes` as a lower-case hex encoded string
#[cfg(any(feature = "aws", feature = "gcp"))]
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // String writing is infallible
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::Error;

    use super::*;
    use rand::{rng, Rng};
    use std::ops::Range;

    /// Calls coalesce_ranges and validates the returned data is correct
    ///
    /// Returns the fetched ranges
    async fn do_fetch(ranges: Vec<Range<u64>>, coalesce: u64) -> Vec<Range<u64>> {
        let max = ranges.iter().map(|x| x.end).max().unwrap_or(0);
        let src: Vec<_> = (0..max).map(|x| x as u8).collect();

        let mut fetches = vec![];
        let coalesced = coalesce_ranges::<_, Error, _>(
            &ranges,
            |range| {
                fetches.push(range.clone());
                let start = usize::try_from(range.start).unwrap();
                let end = usize::try_from(range.end).unwrap();
                futures::future::ready(Ok(Bytes::from(src[start..end].to_vec())))
            },
            coalesce,
        )
        .await
        .unwrap();

        assert_eq!(ranges.len(), coalesced.len());
        for (range, bytes) in ranges.iter().zip(coalesced) {
            assert_eq!(
                bytes.as_ref(),
                &src[usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()]
            );
        }
        fetches
    }

    #[tokio::test]
    async fn test_coalesce_ranges() {
        let fetches = do_fetch(vec![], 0).await;
        assert!(fetches.is_empty());

        let fetches = do_fetch(vec![0..3; 1], 0).await;
        assert_eq!(fetches, vec![0..3]);

        let fetches = do_fetch(vec![0..2, 3..5], 0).await;
        assert_eq!(fetches, vec![0..2, 3..5]);

        let fetches = do_fetch(vec![0..1, 1..2], 0).await;
        assert_eq!(fetches, vec![0..2]);

        let fetches = do_fetch(vec![0..1, 2..72], 1).await;
        assert_eq!(fetches, vec![0..72]);

        let fetches = do_fetch(vec![0..1, 56..72, 73..75], 1).await;
        assert_eq!(fetches, vec![0..1, 56..75]);

        let fetches = do_fetch(vec![0..1, 5..6, 7..9, 2..3, 4..6], 1).await;
        assert_eq!(fetches, vec![0..9]);

        let fetches = do_fetch(vec![0..1, 5..6, 7..9, 2..3, 4..6], 1).await;
        assert_eq!(fetches, vec![0..9]);

        let fetches = do_fetch(vec![0..1, 6..7, 8..9, 10..14, 9..10], 4).await;
        assert_eq!(fetches, vec![0..1, 6..14]);
    }

    #[tokio::test]
    async fn test_coalesce_fuzz() {
        let mut rand = rng();
        for _ in 0..100 {
            let object_len = rand.random_range(10..250);
            let range_count = rand.random_range(0..10);
            let ranges: Vec<_> = (0..range_count)
                .map(|_| {
                    let start = rand.random_range(0..object_len);
                    let max_len = 20.min(object_len - start);
                    let len = rand.random_range(0..max_len);
                    start..start + len
                })
                .collect();

            let coalesce = rand.random_range(1..5);
            let fetches = do_fetch(ranges.clone(), coalesce).await;

            for fetch in fetches.windows(2) {
                assert!(
                    fetch[0].start <= fetch[1].start,
                    "fetches should be sorted, {:?} vs {:?}",
                    fetch[0],
                    fetch[1]
                );

                let delta = fetch[1].end - fetch[0].end;
                assert!(
                    delta > coalesce,
                    "fetches should not overlap by {}, {:?} vs {:?} for {:?}",
                    coalesce,
                    fetch[0],
                    fetch[1],
                    ranges
                );
            }
        }
    }

    #[test]
    fn getrange_str() {
        assert_eq!(GetRange::Offset(0).to_string(), "bytes=0-");
        assert_eq!(GetRange::Bounded(10..19).to_string(), "bytes=10-18");
        assert_eq!(GetRange::Suffix(10).to_string(), "bytes=-10");
    }

    #[test]
    fn getrange_from() {
        assert_eq!(Into::<GetRange>::into(10..15), GetRange::Bounded(10..15),);
        assert_eq!(Into::<GetRange>::into(10..=15), GetRange::Bounded(10..16),);
        assert_eq!(Into::<GetRange>::into(10..), GetRange::Offset(10),);
        assert_eq!(Into::<GetRange>::into(..=15), GetRange::Bounded(0..16));
    }

    #[test]
    fn test_as_range() {
        let range = GetRange::Bounded(2..5);
        assert_eq!(range.as_range(5).unwrap(), 2..5);

        let range = range.as_range(4).unwrap();
        assert_eq!(range, 2..4);

        let range = GetRange::Bounded(3..3);
        let err = range.as_range(2).unwrap_err().to_string();
        assert_eq!(err, "Range started at 3 and ended at 3");

        let range = GetRange::Bounded(2..2);
        let err = range.as_range(3).unwrap_err().to_string();
        assert_eq!(err, "Range started at 2 and ended at 2");

        let range = GetRange::Suffix(3);
        assert_eq!(range.as_range(3).unwrap(), 0..3);
        assert_eq!(range.as_range(2).unwrap(), 0..2);

        let range = GetRange::Suffix(0);
        assert_eq!(range.as_range(0).unwrap(), 0..0);

        let range = GetRange::Offset(2);
        let err = range.as_range(2).unwrap_err().to_string();
        assert_eq!(
            err,
            "Wanted range starting at 2, but object was only 2 bytes long"
        );

        let err = range.as_range(1).unwrap_err().to_string();
        assert_eq!(
            err,
            "Wanted range starting at 2, but object was only 1 bytes long"
        );

        let range = GetRange::Offset(1);
        assert_eq!(range.as_range(2).unwrap(), 1..2);
    }
}
