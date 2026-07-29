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

//! An object store implementation for generic HTTP servers
//!
//! This follows [rfc2518] commonly known as [WebDAV]
//!
//! Basic get support will work out of the box with most HTTP servers,
//! even those that don't explicitly support [rfc2518]
//!
//! Other operations such as list, delete, copy, etc... will likely
//! require server-side configuration. A list of HTTP servers with support
//! can be found [here](https://wiki.archlinux.org/title/WebDAV#Server)
//!
//! Multipart uploads are not currently supported
//!
//! [rfc2518]: https://datatracker.ietf.org/doc/html/rfc2518
//! [WebDAV]: https://en.wikipedia.org/wiki/WebDAV

use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use url::Url;

use crate::client::get::GetClientExt;
use crate::client::header::get_etag;
use crate::client::{http_connector, HttpConnector};
use crate::http::client::Client;
use crate::path::Path;
use crate::{
    ClientConfigKey, ClientOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta,
    ObjectStore, PutMode, PutMultipartOpts, PutOptions, PutPayload, PutResult, Result, RetryConfig,
};

mod client;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Must specify a URL")]
    MissingUrl,

    #[error("Unable parse source url. Url: {}, Error: {}", url, source)]
    UnableToParseUrl {
        source: url::ParseError,
        url: String,
    },

    #[error("Unable to extract metadata from headers: {}", source)]
    Metadata {
        source: crate::client::header::Error,
    },
}

impl From<Error> for crate::Error {
    fn from(err: Error) -> Self {
        Self::Generic {
            store: "HTTP",
            source: Box::new(err),
        }
    }
}

/// An [`ObjectStore`] implementation for generic HTTP servers
///
/// See [`crate::http`] for more information
#[derive(Debug)]
pub struct HttpStore {
    client: Arc<Client>,
}

impl std::fmt::Display for HttpStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpStore")
    }
}

#[async_trait]
impl ObjectStore for HttpStore {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        if opts.mode != PutMode::Overwrite {
            // TODO: Add support for If header - https://datatracker.ietf.org/doc/html/rfc2518#section-9.4
            return Err(crate::Error::NotImplemented);
        }

        let response = self.client.put(location, payload, opts.attributes).await?;
        let e_tag = match get_etag(response.headers()) {
            Ok(e_tag) => Some(e_tag),
            Err(crate::client::header::Error::MissingEtag) => None,
            Err(source) => return Err(Error::Metadata { source }.into()),
        };

        Ok(PutResult {
            e_tag,
            version: None,
        })
    }

    async fn put_multipart_opts(
        &self,
        _location: &Path,
        _opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        Err(crate::Error::NotImplemented)
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        self.client.get_opts(location, options).await
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        self.client.delete(location).await
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix_len = prefix.map(|p| p.as_ref().len()).unwrap_or_default();
        let prefix = prefix.cloned();
        let client = Arc::clone(&self.client);
        futures::stream::once(async move {
            let status = client.list(prefix.as_ref(), "infinity").await?;

            let iter = status
                .response
                .into_iter()
                .filter(|r| !r.is_dir())
                .map(move |response| {
                    response.check_ok()?;
                    response.object_meta(client.base_url())
                })
                // Filter out exact prefix matches
                .filter_ok(move |r| r.location.as_ref().len() > prefix_len);

            Ok::<_, crate::Error>(futures::stream::iter(iter))
        })
        .try_flatten()
        .boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let status = self.client.list(prefix, "1").await?;
        let prefix_len = prefix.map(|p| p.as_ref().len()).unwrap_or(0);

        let mut objects: Vec<ObjectMeta> = Vec::with_capacity(status.response.len());
        let mut common_prefixes = Vec::with_capacity(status.response.len());
        for response in status.response {
            response.check_ok()?;
            match response.is_dir() {
                false => {
                    let meta = response.object_meta(self.client.base_url())?;
                    // Filter out exact prefix matches
                    if meta.location.as_ref().len() > prefix_len {
                        objects.push(meta);
                    }
                }
                true => {
                    let path = response.path(self.client.base_url())?;
                    // Exclude the current object
                    if path.as_ref().len() > prefix_len {
                        common_prefixes.push(path);
                    }
                }
            }
        }

        Ok(ListResult {
            common_prefixes,
            objects,
        })
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        self.client.copy(from, to, true).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.client.copy(from, to, false).await
    }
}

/// Configure a connection to a generic HTTP server
#[derive(Debug, Default, Clone)]
pub struct HttpBuilder {
    url: Option<String>,
    client_options: ClientOptions,
    retry_config: RetryConfig,
    http_connector: Option<Arc<dyn HttpConnector>>,
}

impl HttpBuilder {
    /// Create a new [`HttpBuilder`] with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the retry configuration
    pub fn with_retry(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Set individual client configuration without overriding the entire config
    pub fn with_config(mut self, key: ClientConfigKey, value: impl Into<String>) -> Self {
        self.client_options = self.client_options.with_config(key, value);
        self
    }

    /// Sets the client options, overriding any already set
    pub fn with_client_options(mut self, options: ClientOptions) -> Self {
        self.client_options = options;
        self
    }

    /// The [`HttpConnector`] to use
    ///
    /// On non-WASM32 platforms uses [`reqwest`] by default, on WASM32 platforms must be provided
    pub fn with_http_connector<C: HttpConnector>(mut self, connector: C) -> Self {
        self.http_connector = Some(Arc::new(connector));
        self
    }

    /// Build an [`HttpStore`] with the configured options
    pub fn build(self) -> Result<HttpStore> {
        let url = self.url.ok_or(Error::MissingUrl)?;
        let parsed = Url::parse(&url).map_err(|source| Error::UnableToParseUrl { url, source })?;

        let client = http_connector(self.http_connector)?.connect(&self.client_options)?;

        Ok(HttpStore {
            client: Arc::new(Client::new(
                parsed,
                client,
                self.client_options,
                self.retry_config,
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::integration::*;
    use crate::tests::*;

    use super::*;

    #[tokio::test]
    async fn http_test() {
        maybe_skip_integration!();
        let url = std::env::var("HTTP_URL").expect("HTTP_URL must be set");
        let options = ClientOptions::new().with_allow_http(true);
        let integration = HttpBuilder::new()
            .with_url(url)
            .with_client_options(options)
            .build()
            .unwrap();

        put_get_delete_list(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
    }
}
