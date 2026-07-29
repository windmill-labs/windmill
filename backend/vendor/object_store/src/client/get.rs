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

use std::ops::Range;

use crate::client::header::{header_meta, HeaderConfig};
use crate::client::HttpResponse;
use crate::path::Path;
use crate::{Attribute, Attributes, GetOptions, GetRange, GetResult, GetResultPayload, Result};
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use http::header::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_RANGE,
    CONTENT_TYPE,
};
use http::StatusCode;
use reqwest::header::ToStrError;

/// A client that can perform a get request
#[async_trait]
pub(crate) trait GetClient: Send + Sync + 'static {
    const STORE: &'static str;

    /// Configure the [`HeaderConfig`] for this client
    const HEADER_CONFIG: HeaderConfig;

    async fn get_request(&self, path: &Path, options: GetOptions) -> Result<HttpResponse>;
}

/// Extension trait for [`GetClient`] that adds common retrieval functionality
#[async_trait]
pub(crate) trait GetClientExt {
    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult>;
}

#[async_trait]
impl<T: GetClient> GetClientExt for T {
    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let range = options.range.clone();
        if let Some(r) = range.as_ref() {
            r.is_valid().map_err(|e| crate::Error::Generic {
                store: T::STORE,
                source: Box::new(e),
            })?;
        }
        let response = self.get_request(location, options).await?;
        get_result::<T>(location, range, response).map_err(|e| crate::Error::Generic {
            store: T::STORE,
            source: Box::new(e),
        })
    }
}

struct ContentRange {
    /// The range of the object returned
    range: Range<u64>,
    /// The total size of the object being requested
    size: u64,
}

impl ContentRange {
    /// Parse a content range of the form `bytes <range-start>-<range-end>/<size>`
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Range>
    fn from_str(s: &str) -> Option<Self> {
        let rem = s.trim().strip_prefix("bytes ")?;
        let (range, size) = rem.split_once('/')?;
        let size = size.parse().ok()?;

        let (start_s, end_s) = range.split_once('-')?;

        let start = start_s.parse().ok()?;
        let end: u64 = end_s.parse().ok()?;

        Some(Self {
            size,
            range: start..end + 1,
        })
    }
}

/// A specialized `Error` for get-related errors
#[derive(Debug, thiserror::Error)]
enum GetResultError {
    #[error(transparent)]
    Header {
        #[from]
        source: crate::client::header::Error,
    },

    #[error(transparent)]
    InvalidRangeRequest {
        #[from]
        source: crate::util::InvalidGetRange,
    },

    #[error("Received non-partial response when range requested")]
    NotPartial,

    #[error("Content-Range header not present in partial response")]
    NoContentRange,

    #[error("Failed to parse value for CONTENT_RANGE header: \"{value}\"")]
    ParseContentRange { value: String },

    #[error("Content-Range header contained non UTF-8 characters")]
    InvalidContentRange { source: ToStrError },

    #[error("Cache-Control header contained non UTF-8 characters")]
    InvalidCacheControl { source: ToStrError },

    #[error("Content-Disposition header contained non UTF-8 characters")]
    InvalidContentDisposition { source: ToStrError },

    #[error("Content-Encoding header contained non UTF-8 characters")]
    InvalidContentEncoding { source: ToStrError },

    #[error("Content-Language header contained non UTF-8 characters")]
    InvalidContentLanguage { source: ToStrError },

    #[error("Content-Type header contained non UTF-8 characters")]
    InvalidContentType { source: ToStrError },

    #[error("Metadata value for \"{key:?}\" contained non UTF-8 characters")]
    InvalidMetadata { key: String },

    #[error("Requested {expected:?}, got {actual:?}")]
    UnexpectedRange {
        expected: Range<u64>,
        actual: Range<u64>,
    },
}

fn get_result<T: GetClient>(
    location: &Path,
    range: Option<GetRange>,
    response: HttpResponse,
) -> Result<GetResult, GetResultError> {
    let mut meta = header_meta(location, response.headers(), T::HEADER_CONFIG)?;

    // ensure that we receive the range we asked for
    let range = if let Some(expected) = range {
        if response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(GetResultError::NotPartial);
        }

        let val = response
            .headers()
            .get(CONTENT_RANGE)
            .ok_or(GetResultError::NoContentRange)?;

        let value = val
            .to_str()
            .map_err(|source| GetResultError::InvalidContentRange { source })?;

        let value = ContentRange::from_str(value).ok_or_else(|| {
            let value = value.into();
            GetResultError::ParseContentRange { value }
        })?;

        let actual = value.range;

        // Update size to reflect full size of object (#5272)
        meta.size = value.size;

        let expected = expected.as_range(meta.size)?;

        if actual != expected {
            return Err(GetResultError::UnexpectedRange { expected, actual });
        }

        actual
    } else {
        0..meta.size
    };

    macro_rules! parse_attributes {
        ($headers:expr, $(($header:expr, $attr:expr, $map_err:expr)),*) => {{
            let mut attributes = Attributes::new();
            $(
            if let Some(x) = $headers.get($header) {
                let x = x.to_str().map_err($map_err)?;
                attributes.insert($attr, x.to_string().into());
            }
            )*
            attributes
        }}
    }

    let mut attributes = parse_attributes!(
        response.headers(),
        (CACHE_CONTROL, Attribute::CacheControl, |source| {
            GetResultError::InvalidCacheControl { source }
        }),
        (
            CONTENT_DISPOSITION,
            Attribute::ContentDisposition,
            |source| GetResultError::InvalidContentDisposition { source }
        ),
        (CONTENT_ENCODING, Attribute::ContentEncoding, |source| {
            GetResultError::InvalidContentEncoding { source }
        }),
        (CONTENT_LANGUAGE, Attribute::ContentLanguage, |source| {
            GetResultError::InvalidContentLanguage { source }
        }),
        (CONTENT_TYPE, Attribute::ContentType, |source| {
            GetResultError::InvalidContentType { source }
        })
    );

    // Add attributes that match the user-defined metadata prefix (e.g. x-amz-meta-)
    if let Some(prefix) = T::HEADER_CONFIG.user_defined_metadata_prefix {
        for (key, val) in response.headers() {
            if let Some(suffix) = key.as_str().strip_prefix(prefix) {
                if let Ok(val_str) = val.to_str() {
                    attributes.insert(
                        Attribute::Metadata(suffix.to_string().into()),
                        val_str.to_string().into(),
                    );
                } else {
                    return Err(GetResultError::InvalidMetadata {
                        key: key.to_string(),
                    });
                }
            }
        }
    }

    let stream = response
        .into_body()
        .bytes_stream()
        .map_err(|source| crate::Error::Generic {
            store: T::STORE,
            source: Box::new(source),
        })
        .boxed();

    Ok(GetResult {
        range,
        meta,
        attributes,
        payload: GetResultPayload::Stream(stream),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::*;

    struct TestClient {}

    #[async_trait]
    impl GetClient for TestClient {
        const STORE: &'static str = "TEST";

        const HEADER_CONFIG: HeaderConfig = HeaderConfig {
            etag_required: false,
            last_modified_required: false,
            version_header: None,
            user_defined_metadata_prefix: Some("x-test-meta-"),
        };

        async fn get_request(&self, _: &Path, _: GetOptions) -> Result<HttpResponse> {
            unimplemented!()
        }
    }

    fn make_response(
        object_size: usize,
        range: Option<Range<usize>>,
        status: StatusCode,
        content_range: Option<&str>,
        headers: Option<Vec<(&str, &str)>>,
    ) -> HttpResponse {
        let mut builder = http::Response::builder();
        if let Some(range) = content_range {
            builder = builder.header(CONTENT_RANGE, range);
        }

        let body = match range {
            Some(range) => vec![0_u8; range.end - range.start],
            None => vec![0_u8; object_size],
        };

        if let Some(headers) = headers {
            for (key, value) in headers {
                builder = builder.header(key, value);
            }
        }

        builder
            .status(status)
            .header(CONTENT_LENGTH, object_size)
            .body(body.into())
            .unwrap()
    }

    #[tokio::test]
    async fn test_get_result() {
        let path = Path::from("test");

        let resp = make_response(12, None, StatusCode::OK, None, None);
        let res = get_result::<TestClient>(&path, None, resp).unwrap();
        assert_eq!(res.meta.size, 12);
        assert_eq!(res.range, 0..12);
        let bytes = res.bytes().await.unwrap();
        assert_eq!(bytes.len(), 12);

        let get_range = GetRange::from(2..3);

        let resp = make_response(
            12,
            Some(2..3),
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 2-2/12"),
            None,
        );
        let res = get_result::<TestClient>(&path, Some(get_range.clone()), resp).unwrap();
        assert_eq!(res.meta.size, 12);
        assert_eq!(res.range, 2..3);
        let bytes = res.bytes().await.unwrap();
        assert_eq!(bytes.len(), 1);

        let resp = make_response(12, Some(2..3), StatusCode::OK, None, None);
        let err = get_result::<TestClient>(&path, Some(get_range.clone()), resp).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Received non-partial response when range requested"
        );

        let resp = make_response(
            12,
            Some(2..3),
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 2-3/12"),
            None,
        );
        let err = get_result::<TestClient>(&path, Some(get_range.clone()), resp).unwrap_err();
        assert_eq!(err.to_string(), "Requested 2..3, got 2..4");

        let resp = make_response(
            12,
            Some(2..3),
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 2-2/*"),
            None,
        );
        let err = get_result::<TestClient>(&path, Some(get_range.clone()), resp).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Failed to parse value for CONTENT_RANGE header: \"bytes 2-2/*\""
        );

        let resp = make_response(12, Some(2..3), StatusCode::PARTIAL_CONTENT, None, None);
        let err = get_result::<TestClient>(&path, Some(get_range.clone()), resp).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Content-Range header not present in partial response"
        );

        let resp = make_response(
            2,
            Some(2..3),
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 2-3/2"),
            None,
        );
        let err = get_result::<TestClient>(&path, Some(get_range.clone()), resp).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Wanted range starting at 2, but object was only 2 bytes long"
        );

        let resp = make_response(
            6,
            Some(2..6),
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 2-5/6"),
            None,
        );
        let res = get_result::<TestClient>(&path, Some(GetRange::Suffix(4)), resp).unwrap();
        assert_eq!(res.meta.size, 6);
        assert_eq!(res.range, 2..6);
        let bytes = res.bytes().await.unwrap();
        assert_eq!(bytes.len(), 4);

        let resp = make_response(
            6,
            Some(2..6),
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 2-3/6"),
            None,
        );
        let err = get_result::<TestClient>(&path, Some(GetRange::Suffix(4)), resp).unwrap_err();
        assert_eq!(err.to_string(), "Requested 2..6, got 2..4");

        let resp = make_response(
            12,
            None,
            StatusCode::OK,
            None,
            Some(vec![("x-test-meta-foo", "bar")]),
        );
        let res = get_result::<TestClient>(&path, None, resp).unwrap();
        assert_eq!(res.meta.size, 12);
        assert_eq!(res.range, 0..12);
        assert_eq!(
            res.attributes.get(&Attribute::Metadata("foo".into())),
            Some(&"bar".into())
        );
        let bytes = res.bytes().await.unwrap();
        assert_eq!(bytes.len(), 12);
    }
}
