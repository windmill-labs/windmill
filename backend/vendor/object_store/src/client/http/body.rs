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

use crate::client::{HttpError, HttpErrorKind};
use crate::{collect_bytes, PutPayload};
use bytes::Bytes;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

/// An HTTP Request
pub type HttpRequest = http::Request<HttpRequestBody>;

/// The [`Body`] of an [`HttpRequest`]
#[derive(Debug, Clone)]
pub struct HttpRequestBody(Inner);

impl HttpRequestBody {
    /// An empty [`HttpRequestBody`]
    pub fn empty() -> Self {
        Self(Inner::Bytes(Bytes::new()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn into_reqwest(self) -> reqwest::Body {
        match self.0 {
            Inner::Bytes(b) => b.into(),
            Inner::PutPayload(_, payload) => reqwest::Body::wrap_stream(futures::stream::iter(
                payload.into_iter().map(Ok::<_, HttpError>),
            )),
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) fn into_reqwest(self) -> reqwest::Body {
        match self.0 {
            Inner::Bytes(b) => b.into(),
            Inner::PutPayload(_, payload) => Bytes::from(payload).into(),
        }
    }

    /// Returns true if this body is empty
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Inner::Bytes(x) => x.is_empty(),
            Inner::PutPayload(_, x) => x.iter().any(|x| !x.is_empty()),
        }
    }

    /// Returns the total length of the [`Bytes`] in this body
    pub fn content_length(&self) -> usize {
        match &self.0 {
            Inner::Bytes(x) => x.len(),
            Inner::PutPayload(_, x) => x.content_length(),
        }
    }

    /// If this body consists of a single contiguous [`Bytes`], returns it
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match &self.0 {
            Inner::Bytes(x) => Some(x),
            _ => None,
        }
    }
}

impl From<Bytes> for HttpRequestBody {
    fn from(value: Bytes) -> Self {
        Self(Inner::Bytes(value))
    }
}

impl From<Vec<u8>> for HttpRequestBody {
    fn from(value: Vec<u8>) -> Self {
        Self(Inner::Bytes(value.into()))
    }
}

impl From<String> for HttpRequestBody {
    fn from(value: String) -> Self {
        Self(Inner::Bytes(value.into()))
    }
}

impl From<PutPayload> for HttpRequestBody {
    fn from(value: PutPayload) -> Self {
        Self(Inner::PutPayload(0, value))
    }
}

#[derive(Debug, Clone)]
enum Inner {
    Bytes(Bytes),
    PutPayload(usize, PutPayload),
}

impl Body for HttpRequestBody {
    type Data = Bytes;
    type Error = HttpError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(match &mut self.0 {
            Inner::Bytes(bytes) => {
                let out = bytes.split_off(0);
                if out.is_empty() {
                    None
                } else {
                    Some(Ok(Frame::data(out)))
                }
            }
            Inner::PutPayload(offset, payload) => {
                let slice = payload.as_ref();
                if *offset == slice.len() {
                    None
                } else {
                    Some(Ok(Frame::data(
                        slice[std::mem::replace(offset, *offset + 1)].clone(),
                    )))
                }
            }
        })
    }

    fn is_end_stream(&self) -> bool {
        match self.0 {
            Inner::Bytes(ref bytes) => bytes.is_empty(),
            Inner::PutPayload(offset, ref body) => offset == body.as_ref().len(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self.0 {
            Inner::Bytes(ref bytes) => SizeHint::with_exact(bytes.len() as u64),
            Inner::PutPayload(offset, ref payload) => {
                let iter = payload.as_ref().iter().skip(offset);
                SizeHint::with_exact(iter.map(|x| x.len() as u64).sum())
            }
        }
    }
}

/// An HTTP response
pub type HttpResponse = http::Response<HttpResponseBody>;

/// The body of an [`HttpResponse`]
#[derive(Debug)]
pub struct HttpResponseBody(BoxBody<Bytes, HttpError>);

impl HttpResponseBody {
    /// Create an [`HttpResponseBody`] from the provided [`Body`]
    ///
    /// Note: [`BodyExt::map_err`] can be used to alter error variants
    pub fn new<B>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = HttpError> + Send + Sync + 'static,
    {
        Self(BoxBody::new(body))
    }

    /// Collects this response into a [`Bytes`]
    pub async fn bytes(self) -> Result<Bytes, HttpError> {
        let size_hint = self.0.size_hint().lower();
        let s = self.0.into_data_stream();
        collect_bytes(s, Some(size_hint)).await
    }

    /// Returns a stream of this response data
    pub fn bytes_stream(self) -> BoxStream<'static, Result<Bytes, HttpError>> {
        self.0.into_data_stream().boxed()
    }

    /// Returns the response as a [`String`]
    pub(crate) async fn text(self) -> Result<String, HttpError> {
        let b = self.bytes().await?;
        String::from_utf8(b.into()).map_err(|e| HttpError::new(HttpErrorKind::Decode, e))
    }

    #[cfg(any(feature = "aws", feature = "gcp", feature = "azure"))]
    pub(crate) async fn json<B: serde::de::DeserializeOwned>(self) -> Result<B, HttpError> {
        let b = self.bytes().await?;
        serde_json::from_slice(&b).map_err(|e| HttpError::new(HttpErrorKind::Decode, e))
    }
}

impl Body for HttpResponseBody {
    type Data = Bytes;
    type Error = HttpError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }
}

impl From<Bytes> for HttpResponseBody {
    fn from(value: Bytes) -> Self {
        Self::new(Full::new(value).map_err(|e| match e {}))
    }
}

impl From<Vec<u8>> for HttpResponseBody {
    fn from(value: Vec<u8>) -> Self {
        Bytes::from(value).into()
    }
}

impl From<String> for HttpResponseBody {
    fn from(value: String) -> Self {
        Bytes::from(value).into()
    }
}
