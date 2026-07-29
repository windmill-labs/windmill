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

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use crate::local::LocalFileSystem;
use crate::memory::InMemory;
use crate::path::Path;
use crate::ObjectStore;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to recognise URL \"{}\"", url)]
    Unrecognised { url: Url },

    #[error(transparent)]
    Path {
        #[from]
        source: crate::path::Error,
    },
}

impl From<Error> for super::Error {
    fn from(e: Error) -> Self {
        Self::Generic {
            store: "URL",
            source: Box::new(e),
        }
    }
}

/// Recognizes various URL formats, identifying the relevant [`ObjectStore`]
///
/// See [`ObjectStoreScheme::parse`] for more details
///
/// # Supported formats:
/// - `file:///path/to/my/file` -> [`LocalFileSystem`]
/// - `memory:///` -> [`InMemory`]
/// - `s3://bucket/path` -> [`AmazonS3`](crate::aws::AmazonS3) (also supports `s3a`)
/// - `gs://bucket/path` -> [`GoogleCloudStorage`](crate::gcp::GoogleCloudStorage)
/// - `az://account/container/path` -> [`MicrosoftAzure`](crate::azure::MicrosoftAzure) (also supports `adl`, `azure`, `abfs`, `abfss`)
/// - `http://mydomain/path` -> [`HttpStore`](crate::http::HttpStore)
/// - `https://mydomain/path` -> [`HttpStore`](crate::http::HttpStore)
///
/// There are also special cases for AWS and Azure for `https://{host?}/path` paths:
/// - `dfs.core.windows.net`, `blob.core.windows.net`, `dfs.fabric.microsoft.com`, `blob.fabric.microsoft.com` -> [`MicrosoftAzure`](crate::azure::MicrosoftAzure)
/// - `amazonaws.com` -> [`AmazonS3`](crate::aws::AmazonS3)
/// - `r2.cloudflarestorage.com` -> [`AmazonS3`](crate::aws::AmazonS3)
///
#[non_exhaustive] // permit new variants
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ObjectStoreScheme {
    /// Url corresponding to [`LocalFileSystem`]
    Local,
    /// Url corresponding to [`InMemory`]
    Memory,
    /// Url corresponding to [`AmazonS3`](crate::aws::AmazonS3)
    AmazonS3,
    /// Url corresponding to [`GoogleCloudStorage`](crate::gcp::GoogleCloudStorage)
    GoogleCloudStorage,
    /// Url corresponding to [`MicrosoftAzure`](crate::azure::MicrosoftAzure)
    MicrosoftAzure,
    /// Url corresponding to [`HttpStore`](crate::http::HttpStore)
    Http,
}

impl ObjectStoreScheme {
    /// Create an [`ObjectStoreScheme`] from the provided [`Url`]
    ///
    /// Returns the [`ObjectStoreScheme`] and the remaining [`Path`]
    ///
    /// # Example
    /// ```
    /// # use url::Url;
    /// # use object_store::ObjectStoreScheme;
    /// let url: Url = "file:///path/to/my/file".parse().unwrap();
    /// let (scheme, path) = ObjectStoreScheme::parse(&url).unwrap();
    /// assert_eq!(scheme, ObjectStoreScheme::Local);
    /// assert_eq!(path.as_ref(), "path/to/my/file");
    ///
    /// let url: Url = "https://blob.core.windows.net/path/to/my/file".parse().unwrap();
    /// let (scheme, path) = ObjectStoreScheme::parse(&url).unwrap();
    /// assert_eq!(scheme, ObjectStoreScheme::MicrosoftAzure);
    /// assert_eq!(path.as_ref(), "path/to/my/file");
    ///
    /// let url: Url = "https://example.com/path/to/my/file".parse().unwrap();
    /// let (scheme, path) = ObjectStoreScheme::parse(&url).unwrap();
    /// assert_eq!(scheme, ObjectStoreScheme::Http);
    /// assert_eq!(path.as_ref(), "path/to/my/file");
    /// ```
    pub fn parse(url: &Url) -> Result<(Self, Path), Error> {
        let strip_bucket = || Some(url.path().strip_prefix('/')?.split_once('/')?.1);

        let (scheme, path) = match (url.scheme(), url.host_str()) {
            ("file", None) => (Self::Local, url.path()),
            ("memory", None) => (Self::Memory, url.path()),
            ("s3" | "s3a", Some(_)) => (Self::AmazonS3, url.path()),
            ("gs", Some(_)) => (Self::GoogleCloudStorage, url.path()),
            ("az" | "adl" | "azure" | "abfs" | "abfss", Some(_)) => {
                (Self::MicrosoftAzure, url.path())
            }
            ("http", Some(_)) => (Self::Http, url.path()),
            ("https", Some(host)) => {
                if host.ends_with("dfs.core.windows.net")
                    || host.ends_with("blob.core.windows.net")
                    || host.ends_with("dfs.fabric.microsoft.com")
                    || host.ends_with("blob.fabric.microsoft.com")
                {
                    (Self::MicrosoftAzure, url.path())
                } else if host.ends_with("amazonaws.com") {
                    match host.starts_with("s3") {
                        true => (Self::AmazonS3, strip_bucket().unwrap_or_default()),
                        false => (Self::AmazonS3, url.path()),
                    }
                } else if host.ends_with("r2.cloudflarestorage.com") {
                    (Self::AmazonS3, strip_bucket().unwrap_or_default())
                } else {
                    (Self::Http, url.path())
                }
            }
            _ => return Err(Error::Unrecognised { url: url.clone() }),
        };

        Ok((scheme, Path::from_url_path(path)?))
    }
}

#[cfg(feature = "cloud")]
macro_rules! builder_opts {
    ($builder:ty, $url:expr, $options:expr) => {{
        let builder = $options.into_iter().fold(
            <$builder>::new().with_url($url.to_string()),
            |builder, (key, value)| match key.as_ref().parse() {
                Ok(k) => builder.with_config(k, value),
                Err(_) => builder,
            },
        );
        Box::new(builder.build()?) as _
    }};
}

/// Create an [`ObjectStore`] based on the provided `url`
///
/// Returns
/// - An [`ObjectStore`] of the corresponding type
/// - The [`Path`] into the [`ObjectStore`] of the addressed resource
pub fn parse_url(url: &Url) -> Result<(Box<dyn ObjectStore>, Path), super::Error> {
    parse_url_opts(url, std::iter::empty::<(&str, &str)>())
}

/// Create an [`ObjectStore`] based on the provided `url` and options
///
/// Returns
/// - An [`ObjectStore`] of the corresponding type
/// - The [`Path`] into the [`ObjectStore`] of the addressed resource
pub fn parse_url_opts<I, K, V>(
    url: &Url,
    options: I,
) -> Result<(Box<dyn ObjectStore>, Path), super::Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: Into<String>,
{
    let _options = options;
    let (scheme, path) = ObjectStoreScheme::parse(url)?;
    let path = Path::parse(path)?;

    let store = match scheme {
        #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
        ObjectStoreScheme::Local => Box::new(LocalFileSystem::new()) as _,
        ObjectStoreScheme::Memory => Box::new(InMemory::new()) as _,
        #[cfg(feature = "aws")]
        ObjectStoreScheme::AmazonS3 => {
            builder_opts!(crate::aws::AmazonS3Builder, url, _options)
        }
        #[cfg(feature = "gcp")]
        ObjectStoreScheme::GoogleCloudStorage => {
            builder_opts!(crate::gcp::GoogleCloudStorageBuilder, url, _options)
        }
        #[cfg(feature = "azure")]
        ObjectStoreScheme::MicrosoftAzure => {
            builder_opts!(crate::azure::MicrosoftAzureBuilder, url, _options)
        }
        #[cfg(feature = "http")]
        ObjectStoreScheme::Http => {
            let url = &url[..url::Position::BeforePath];
            builder_opts!(crate::http::HttpBuilder, url, _options)
        }
        #[cfg(not(all(
            feature = "aws",
            feature = "azure",
            feature = "gcp",
            feature = "http",
            not(target_arch = "wasm32")
        )))]
        s => {
            return Err(super::Error::Generic {
                store: "parse_url",
                source: format!("feature for {s:?} not enabled").into(),
            })
        }
    };

    Ok((store, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let cases = [
            ("file:/path", (ObjectStoreScheme::Local, "path")),
            ("file:///path", (ObjectStoreScheme::Local, "path")),
            ("memory:/path", (ObjectStoreScheme::Memory, "path")),
            ("memory:///", (ObjectStoreScheme::Memory, "")),
            ("s3://bucket/path", (ObjectStoreScheme::AmazonS3, "path")),
            ("s3a://bucket/path", (ObjectStoreScheme::AmazonS3, "path")),
            (
                "https://s3.region.amazonaws.com/bucket",
                (ObjectStoreScheme::AmazonS3, ""),
            ),
            (
                "https://s3.region.amazonaws.com/bucket/path",
                (ObjectStoreScheme::AmazonS3, "path"),
            ),
            (
                "https://bucket.s3.region.amazonaws.com",
                (ObjectStoreScheme::AmazonS3, ""),
            ),
            (
                "https://ACCOUNT_ID.r2.cloudflarestorage.com/bucket",
                (ObjectStoreScheme::AmazonS3, ""),
            ),
            (
                "https://ACCOUNT_ID.r2.cloudflarestorage.com/bucket/path",
                (ObjectStoreScheme::AmazonS3, "path"),
            ),
            (
                "abfs://container/path",
                (ObjectStoreScheme::MicrosoftAzure, "path"),
            ),
            (
                "abfs://file_system@account_name.dfs.core.windows.net/path",
                (ObjectStoreScheme::MicrosoftAzure, "path"),
            ),
            (
                "abfss://file_system@account_name.dfs.core.windows.net/path",
                (ObjectStoreScheme::MicrosoftAzure, "path"),
            ),
            (
                "https://account.dfs.core.windows.net",
                (ObjectStoreScheme::MicrosoftAzure, ""),
            ),
            (
                "https://account.blob.core.windows.net",
                (ObjectStoreScheme::MicrosoftAzure, ""),
            ),
            (
                "gs://bucket/path",
                (ObjectStoreScheme::GoogleCloudStorage, "path"),
            ),
            (
                "gs://test.example.com/path",
                (ObjectStoreScheme::GoogleCloudStorage, "path"),
            ),
            ("http://mydomain/path", (ObjectStoreScheme::Http, "path")),
            ("https://mydomain/path", (ObjectStoreScheme::Http, "path")),
            (
                "s3://bucket/foo%20bar",
                (ObjectStoreScheme::AmazonS3, "foo bar"),
            ),
            (
                "https://foo/bar%20baz",
                (ObjectStoreScheme::Http, "bar baz"),
            ),
            (
                "file:///bar%252Efoo",
                (ObjectStoreScheme::Local, "bar%2Efoo"),
            ),
            (
                "abfss://file_system@account.dfs.fabric.microsoft.com/",
                (ObjectStoreScheme::MicrosoftAzure, ""),
            ),
            (
                "abfss://file_system@account.dfs.fabric.microsoft.com/",
                (ObjectStoreScheme::MicrosoftAzure, ""),
            ),
            (
                "https://account.dfs.fabric.microsoft.com/",
                (ObjectStoreScheme::MicrosoftAzure, ""),
            ),
            (
                "https://account.dfs.fabric.microsoft.com/container",
                (ObjectStoreScheme::MicrosoftAzure, "container"),
            ),
            (
                "https://account.blob.fabric.microsoft.com/",
                (ObjectStoreScheme::MicrosoftAzure, ""),
            ),
            (
                "https://account.blob.fabric.microsoft.com/container",
                (ObjectStoreScheme::MicrosoftAzure, "container"),
            ),
        ];

        for (s, (expected_scheme, expected_path)) in cases {
            let url = Url::parse(s).unwrap();
            let (scheme, path) = ObjectStoreScheme::parse(&url).unwrap();

            assert_eq!(scheme, expected_scheme, "{s}");
            assert_eq!(path, Path::parse(expected_path).unwrap(), "{s}");
        }

        let neg_cases = [
            "unix:/run/foo.socket",
            "file://remote/path",
            "memory://remote/",
        ];
        for s in neg_cases {
            let url = Url::parse(s).unwrap();
            assert!(ObjectStoreScheme::parse(&url).is_err());
        }
    }

    #[test]
    fn test_url_spaces() {
        let url = Url::parse("file:///my file with spaces").unwrap();
        assert_eq!(url.path(), "/my%20file%20with%20spaces");
        let (_, path) = parse_url(&url).unwrap();
        assert_eq!(path.as_ref(), "my file with spaces");
    }

    #[tokio::test]
    #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
    async fn test_url_http() {
        use crate::client::mock_server::MockServer;
        use http::{header::USER_AGENT, Response};

        let server = MockServer::new().await;

        server.push_fn(|r| {
            assert_eq!(r.uri().path(), "/foo/bar");
            assert_eq!(r.headers().get(USER_AGENT).unwrap(), "test_url");
            Response::new(String::new())
        });

        let test = format!("{}/foo/bar", server.url());
        let opts = [("user_agent", "test_url"), ("allow_http", "true")];
        let url = test.parse().unwrap();
        let (store, path) = parse_url_opts(&url, opts).unwrap();
        assert_eq!(path.as_ref(), "foo/bar");
        store.get(&path).await.unwrap();

        server.shutdown().await;
    }
}
