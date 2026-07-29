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

use futures::future::BoxFuture;
use futures::FutureExt;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::{JoinHandle, JoinSet};

pub(crate) type ResponseFn =
    Box<dyn FnOnce(Request<Incoming>) -> BoxFuture<'static, Response<String>> + Send>;

/// A mock server
pub(crate) struct MockServer {
    responses: Arc<Mutex<VecDeque<ResponseFn>>>,
    shutdown: oneshot::Sender<()>,
    handle: JoinHandle<()>,
    url: String,
}

impl MockServer {
    pub(crate) async fn new() -> Self {
        let responses: Arc<Mutex<VecDeque<ResponseFn>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(10)));

        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();

        let (shutdown, mut rx) = oneshot::channel::<()>();

        let url = format!("http://{}", listener.local_addr().unwrap());

        let r = Arc::clone(&responses);
        let handle = tokio::spawn(async move {
            let mut set = JoinSet::new();

            loop {
                let (stream, _) = tokio::select! {
                    conn = listener.accept() => conn.unwrap(),
                    _ = &mut rx => break,
                };

                let r = Arc::clone(&r);
                set.spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            TokioIo::new(stream),
                            service_fn(move |req| {
                                let r = Arc::clone(&r);
                                let next = r.lock().pop_front();
                                async move {
                                    Ok::<_, Infallible>(match next {
                                        Some(r) => r(req).await,
                                        None => Response::new("Hello World".to_string()),
                                    })
                                }
                            }),
                        )
                        .await;
                });
            }

            set.abort_all();
        });

        Self {
            responses,
            shutdown,
            handle,
            url,
        }
    }

    /// The url of the mock server
    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    /// Add a response
    pub(crate) fn push(&self, response: Response<String>) {
        self.push_fn(|_| response)
    }

    /// Add a response function
    pub(crate) fn push_fn<F>(&self, f: F)
    where
        F: FnOnce(Request<Incoming>) -> Response<String> + Send + 'static,
    {
        let f = Box::new(|req| async move { f(req) }.boxed());
        self.responses.lock().push_back(f)
    }

    pub(crate) fn push_async_fn<F, Fut>(&self, f: F)
    where
        F: FnOnce(Request<Incoming>) -> Fut + Send + 'static,
        Fut: Future<Output = Response<String>> + Send + 'static,
    {
        self.responses.lock().push_back(Box::new(|r| f(r).boxed()))
    }

    /// Shutdown the mock server
    pub(crate) async fn shutdown(self) {
        let _ = self.shutdown.send(());
        self.handle.await.unwrap()
    }
}
