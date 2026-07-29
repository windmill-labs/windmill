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

use crate::client::builder::{HttpRequestBuilder, RequestBuilderError};
use crate::client::{HttpRequest, HttpResponse, HttpResponseBody};
use crate::ClientOptions;
use async_trait::async_trait;
use http::{Method, Uri};
use http_body_util::BodyExt;
use std::error::Error;
use std::sync::Arc;
use tokio::runtime::Handle;

/// An HTTP protocol error
///
/// Clients should return this when an HTTP request fails to be completed, e.g. because
/// of a connection issue. This does **not** include HTTP requests that are return
/// non 2xx Status Codes, as these should instead be returned as an [`HttpResponse`]
/// with the appropriate status code set.
#[derive(Debug, thiserror::Error)]
#[error("HTTP error: {source}")]
pub struct HttpError {
    kind: HttpErrorKind,
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

/// Identifies the kind of [`HttpError`]
///
/// This is used, among other things, to determine if a request can be retried
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HttpErrorKind {
    /// An error occurred whilst connecting to the remote
    ///
    /// Will be automatically retried
    Connect,
    /// An error occurred whilst making the request
    ///
    /// Will be automatically retried
    Request,
    /// Request timed out
    ///
    /// Will be automatically retried if the request is idempotent
    Timeout,
    /// The request was aborted
    ///
    /// Will be automatically retried if the request is idempotent
    Interrupted,
    /// An error occurred whilst decoding the response
    ///
    /// Will not be automatically retried
    Decode,
    /// An unknown error occurred
    ///
    /// Will not be automatically retried
    Unknown,
}

impl HttpError {
    /// Create a new [`HttpError`] with the optional status code
    pub fn new<E>(kind: HttpErrorKind, e: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Box::new(e),
        }
    }

    pub(crate) fn reqwest(e: reqwest::Error) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let is_connect = || e.is_connect();
        #[cfg(target_arch = "wasm32")]
        let is_connect = || false;

        let mut kind = if e.is_timeout() {
            HttpErrorKind::Timeout
        } else if is_connect() {
            HttpErrorKind::Connect
        } else if e.is_decode() {
            HttpErrorKind::Decode
        } else {
            HttpErrorKind::Unknown
        };

        // Reqwest error variants aren't great, attempt to refine them
        let mut source = e.source();
        while let Some(e) = source {
            if let Some(e) = e.downcast_ref::<hyper::Error>() {
                if e.is_closed() || e.is_incomplete_message() || e.is_body_write_aborted() {
                    kind = HttpErrorKind::Request;
                } else if e.is_timeout() {
                    kind = HttpErrorKind::Timeout;
                }
                break;
            }
            if let Some(e) = e.downcast_ref::<std::io::Error>() {
                match e.kind() {
                    std::io::ErrorKind::TimedOut => kind = HttpErrorKind::Timeout,
                    std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::UnexpectedEof => kind = HttpErrorKind::Interrupted,
                    _ => {}
                }
                break;
            }
            source = e.source();
        }
        Self {
            kind,
            // We strip URL as it will be included by RetryError if not sensitive
            source: Box::new(e.without_url()),
        }
    }

    /// Returns the [`HttpErrorKind`]
    pub fn kind(&self) -> HttpErrorKind {
        self.kind
    }
}

/// An asynchronous function from a [`HttpRequest`] to a [`HttpResponse`].
#[async_trait]
pub trait HttpService: std::fmt::Debug + Send + Sync + 'static {
    /// Perform [`HttpRequest`] returning [`HttpResponse`]
    async fn call(&self, req: HttpRequest) -> Result<HttpResponse, HttpError>;
}

/// An HTTP client
#[derive(Debug, Clone)]
pub struct HttpClient(Arc<dyn HttpService>);

impl HttpClient {
    /// Create a new [`HttpClient`] from an [`HttpService`]
    pub fn new(service: impl HttpService + 'static) -> Self {
        Self(Arc::new(service))
    }

    /// Performs [`HttpRequest`] using this client
    pub async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        self.0.call(request).await
    }

    #[allow(unused)]
    pub(crate) fn get<U>(&self, url: U) -> HttpRequestBuilder
    where
        U: TryInto<Uri>,
        U::Error: Into<RequestBuilderError>,
    {
        self.request(Method::GET, url)
    }

    #[allow(unused)]
    pub(crate) fn post<U>(&self, url: U) -> HttpRequestBuilder
    where
        U: TryInto<Uri>,
        U::Error: Into<RequestBuilderError>,
    {
        self.request(Method::POST, url)
    }

    #[allow(unused)]
    pub(crate) fn put<U>(&self, url: U) -> HttpRequestBuilder
    where
        U: TryInto<Uri>,
        U::Error: Into<RequestBuilderError>,
    {
        self.request(Method::PUT, url)
    }

    #[allow(unused)]
    pub(crate) fn delete<U>(&self, url: U) -> HttpRequestBuilder
    where
        U: TryInto<Uri>,
        U::Error: Into<RequestBuilderError>,
    {
        self.request(Method::DELETE, url)
    }

    pub(crate) fn request<U>(&self, method: Method, url: U) -> HttpRequestBuilder
    where
        U: TryInto<Uri>,
        U::Error: Into<RequestBuilderError>,
    {
        HttpRequestBuilder::new(self.clone())
            .uri(url)
            .method(method)
    }
}

#[async_trait]
#[cfg(not(target_arch = "wasm32"))]
impl HttpService for reqwest::Client {
    async fn call(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        let (parts, body) = req.into_parts();

        let url = parts.uri.to_string().parse().unwrap();
        let mut req = reqwest::Request::new(parts.method, url);
        *req.headers_mut() = parts.headers;
        *req.body_mut() = Some(body.into_reqwest());

        let r = self.execute(req).await.map_err(HttpError::reqwest)?;
        let res: http::Response<reqwest::Body> = r.into();
        let (parts, body) = res.into_parts();

        let body = HttpResponseBody::new(body.map_err(HttpError::reqwest));
        Ok(HttpResponse::from_parts(parts, body))
    }
}

#[async_trait]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl HttpService for reqwest::Client {
    async fn call(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        use futures::{
            channel::{mpsc, oneshot},
            SinkExt, StreamExt, TryStreamExt,
        };
        use http_body_util::{Empty, StreamBody};
        use wasm_bindgen_futures::spawn_local;

        let (parts, body) = req.into_parts();
        let url = parts.uri.to_string().parse().unwrap();
        let mut req = reqwest::Request::new(parts.method, url);
        *req.headers_mut() = parts.headers;
        *req.body_mut() = Some(body.into_reqwest());

        let (mut tx, rx) = mpsc::channel(1);
        let (tx_parts, rx_parts) = oneshot::channel();
        let res_fut = self.execute(req);

        spawn_local(async move {
            match res_fut.await.map_err(HttpError::reqwest) {
                Err(err) => {
                    let _ = tx_parts.send(Err(err));
                    drop(tx);
                }
                Ok(res) => {
                    let (mut parts, _) = http::Response::new(Empty::<()>::new()).into_parts();
                    parts.headers = res.headers().clone();
                    parts.status = res.status();
                    let _ = tx_parts.send(Ok(parts));
                    let mut stream = res.bytes_stream().map_err(HttpError::reqwest);
                    while let Some(chunk) = stream.next().await {
                        if let Err(_e) = tx.send(chunk).await {
                            // Disconnected due to a transitive drop of the receiver
                            break;
                        }
                    }
                }
            }
        });

        let parts = rx_parts.await.unwrap()?;
        let safe_stream = rx.map(|chunk| {
            let frame = hyper::body::Frame::data(chunk?);
            Ok(frame)
        });
        let body = HttpResponseBody::new(StreamBody::new(safe_stream));

        Ok(HttpResponse::from_parts(parts, body))
    }
}

/// A factory for [`HttpClient`]
pub trait HttpConnector: std::fmt::Debug + Send + Sync + 'static {
    /// Create a new [`HttpClient`] with the provided [`ClientOptions`]
    fn connect(&self, options: &ClientOptions) -> crate::Result<HttpClient>;
}

/// [`HttpConnector`] using [`reqwest::Client`]
#[derive(Debug, Default)]
#[allow(missing_copy_implementations)]
#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
pub struct ReqwestConnector {}

#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
impl HttpConnector for ReqwestConnector {
    fn connect(&self, options: &ClientOptions) -> crate::Result<HttpClient> {
        let client = options.client()?;
        Ok(HttpClient::new(client))
    }
}

/// [`reqwest::Client`] connector that performs all I/O on the provided tokio
/// [`Runtime`] (thread pool).
///
/// This adapter is most useful when you wish to segregate I/O from CPU bound
/// work that may be happening on the [`Runtime`].
///
/// [`Runtime`]: tokio::runtime::Runtime
///
/// # Example: Spawning requests on separate runtime
///
/// ```
/// # use std::sync::Arc;
/// # use tokio::runtime::Runtime;
/// # use object_store::azure::MicrosoftAzureBuilder;
/// # use object_store::client::SpawnedReqwestConnector;
/// # use object_store::ObjectStore;
/// # fn get_io_runtime() -> Runtime {
/// #   tokio::runtime::Builder::new_current_thread().build().unwrap()
/// # }
/// # fn main() -> Result<(), object_store::Error> {
/// // create a tokio runtime for I/O.
/// let io_runtime: Runtime = get_io_runtime();
/// // configure a store using the runtime.
/// let handle = io_runtime.handle().clone(); // get a handle to the same runtime
/// let store: Arc<dyn ObjectStore> = Arc::new(
///   MicrosoftAzureBuilder::new()
///     .with_http_connector(SpawnedReqwestConnector::new(handle))
///     .with_container_name("my_container")
///     .with_account("my_account")
///     .build()?
///  );
/// // any requests made using store will be spawned on the io_runtime
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
#[allow(missing_copy_implementations)]
#[cfg(not(target_arch = "wasm32"))]
pub struct SpawnedReqwestConnector {
    runtime: Handle,
}

#[cfg(not(target_arch = "wasm32"))]
impl SpawnedReqwestConnector {
    /// Create a new [`SpawnedReqwestConnector`] with the provided [`Handle`] to
    /// a tokio [`Runtime`]
    ///
    /// [`Runtime`]: tokio::runtime::Runtime
    pub fn new(runtime: Handle) -> Self {
        Self { runtime }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl HttpConnector for SpawnedReqwestConnector {
    fn connect(&self, options: &ClientOptions) -> crate::Result<HttpClient> {
        let spawn_service = super::SpawnService::new(options.client()?, self.runtime.clone());
        Ok(HttpClient::new(spawn_service))
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
pub(crate) fn http_connector(
    custom: Option<Arc<dyn HttpConnector>>,
) -> crate::Result<Arc<dyn HttpConnector>> {
    match custom {
        Some(x) => Ok(x),
        None => Err(crate::Error::NotSupported {
            source: "WASI architectures must provide an HTTPConnector"
                .to_string()
                .into(),
        }),
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
pub(crate) fn http_connector(
    custom: Option<Arc<dyn HttpConnector>>,
) -> crate::Result<Arc<dyn HttpConnector>> {
    match custom {
        Some(x) => Ok(x),
        None => Ok(Arc::new(ReqwestConnector {})),
    }
}
