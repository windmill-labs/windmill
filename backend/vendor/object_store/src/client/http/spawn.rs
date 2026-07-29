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

use crate::client::{
    HttpError, HttpErrorKind, HttpRequest, HttpResponse, HttpResponseBody, HttpService,
};
use async_trait::async_trait;
use bytes::Bytes;
use http::Response;
use http_body_util::BodyExt;
use hyper::body::{Body, Frame};
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

/// Spawn error
#[derive(Debug, Error)]
#[error("SpawnError")]
struct SpawnError {}

impl From<SpawnError> for HttpError {
    fn from(value: SpawnError) -> Self {
        Self::new(HttpErrorKind::Interrupted, value)
    }
}

/// Wraps a provided [`HttpService`] and runs it on a separate tokio runtime
///
/// See example on [`SpawnedReqwestConnector`]
///
/// [`SpawnedReqwestConnector`]: crate::client::http::SpawnedReqwestConnector
#[derive(Debug)]
pub struct SpawnService<T: HttpService + Clone> {
    inner: T,
    runtime: Handle,
}

impl<T: HttpService + Clone> SpawnService<T> {
    /// Creates a new [`SpawnService`] from the provided
    pub fn new(inner: T, runtime: Handle) -> Self {
        Self { inner, runtime }
    }
}

#[async_trait]
impl<T: HttpService + Clone> HttpService for SpawnService<T> {
    async fn call(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        let inner = self.inner.clone();
        let (send, recv) = tokio::sync::oneshot::channel();

        // We use an unbounded channel to prevent backpressure across the runtime boundary
        // which could in turn starve the underlying IO operations
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let handle = SpawnHandle(self.runtime.spawn(async move {
            let r = match HttpService::call(&inner, req).await {
                Ok(resp) => resp,
                Err(e) => {
                    let _ = send.send(Err(e));
                    return;
                }
            };

            let (parts, mut body) = r.into_parts();
            if send.send(Ok(parts)).is_err() {
                return;
            }

            while let Some(x) = body.frame().await {
                sender.send(x).unwrap();
            }
        }));

        let parts = recv.await.map_err(|_| SpawnError {})??;

        Ok(Response::from_parts(
            parts,
            HttpResponseBody::new(SpawnBody {
                stream: receiver,
                _worker: handle,
            }),
        ))
    }
}

/// A wrapper around a [`JoinHandle`] that aborts on drop
struct SpawnHandle(JoinHandle<()>);
impl Drop for SpawnHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

type StreamItem = Result<Frame<Bytes>, HttpError>;

struct SpawnBody {
    stream: tokio::sync::mpsc::UnboundedReceiver<StreamItem>,
    _worker: SpawnHandle,
}

impl Body for SpawnBody {
    type Data = Bytes;
    type Error = HttpError;

    fn poll_frame(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<StreamItem>> {
        self.stream.poll_recv(cx)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::mock_server::MockServer;
    use crate::client::retry::RetryExt;
    use crate::client::HttpClient;
    use crate::RetryConfig;

    async fn test_client(client: HttpClient) {
        let (send, recv) = tokio::sync::oneshot::channel();

        let mock = MockServer::new().await;
        mock.push(Response::new("BANANAS".to_string()));

        let url = mock.url().to_string();
        let thread = std::thread::spawn(|| {
            futures::executor::block_on(async move {
                let retry = RetryConfig::default();
                let ret = client.get(url).send_retry(&retry).await.unwrap();
                let payload = ret.into_body().bytes().await.unwrap();
                assert_eq!(payload.as_ref(), b"BANANAS");
                let _ = send.send(());
            })
        });
        recv.await.unwrap();
        thread.join().unwrap();
    }

    #[tokio::test]
    async fn test_spawn() {
        let client = HttpClient::new(SpawnService::new(reqwest::Client::new(), Handle::current()));
        test_client(client).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_no_spawn() {
        let client = HttpClient::new(reqwest::Client::new());
        test_client(client).await;
    }
}
