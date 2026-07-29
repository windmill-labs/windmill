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

use crate::client::{HttpClient, HttpError, HttpErrorKind, HttpRequest, HttpRequestBody};
use http::header::{InvalidHeaderName, InvalidHeaderValue};
use http::uri::InvalidUri;
use http::{HeaderName, HeaderValue, Method, Uri};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RequestBuilderError {
    #[error("Invalid URI")]
    InvalidUri(#[from] InvalidUri),

    #[error("Invalid Header Value")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("Invalid Header Name")]
    InvalidHeaderName(#[from] InvalidHeaderName),

    #[error("JSON serialization error")]
    SerdeJson(#[from] serde_json::Error),

    #[error("URL serialization error")]
    SerdeUrl(#[from] serde_urlencoded::ser::Error),
}

impl From<RequestBuilderError> for HttpError {
    fn from(value: RequestBuilderError) -> Self {
        Self::new(HttpErrorKind::Request, value)
    }
}

impl From<std::convert::Infallible> for RequestBuilderError {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

pub(crate) struct HttpRequestBuilder {
    client: HttpClient,
    request: Result<HttpRequest, RequestBuilderError>,
}

impl HttpRequestBuilder {
    pub(crate) fn new(client: HttpClient) -> Self {
        Self {
            client,
            request: Ok(HttpRequest::new(HttpRequestBody::empty())),
        }
    }

    #[cfg(any(feature = "aws", feature = "azure"))]
    pub(crate) fn from_parts(client: HttpClient, request: HttpRequest) -> Self {
        Self {
            client,
            request: Ok(request),
        }
    }

    pub(crate) fn method(mut self, method: Method) -> Self {
        if let Ok(r) = &mut self.request {
            *r.method_mut() = method;
        }
        self
    }

    pub(crate) fn uri<U>(mut self, url: U) -> Self
    where
        U: TryInto<Uri>,
        U::Error: Into<RequestBuilderError>,
    {
        match (url.try_into(), &mut self.request) {
            (Ok(uri), Ok(r)) => *r.uri_mut() = uri,
            (Err(e), Ok(_)) => self.request = Err(e.into()),
            (_, Err(_)) => {}
        }
        self
    }

    pub(crate) fn extensions(mut self, extensions: ::http::Extensions) -> Self {
        if let Ok(r) = &mut self.request {
            *r.extensions_mut() = extensions;
        }
        self
    }

    pub(crate) fn header<K, V>(mut self, name: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        K::Error: Into<RequestBuilderError>,
        V: TryInto<HeaderValue>,
        V::Error: Into<RequestBuilderError>,
    {
        match (name.try_into(), value.try_into(), &mut self.request) {
            (Ok(name), Ok(value), Ok(r)) => {
                r.headers_mut().insert(name, value);
            }
            (Err(e), _, Ok(_)) => self.request = Err(e.into()),
            (_, Err(e), Ok(_)) => self.request = Err(e.into()),
            (_, _, Err(_)) => {}
        }
        self
    }

    #[cfg(feature = "aws")]
    pub(crate) fn headers(mut self, headers: http::HeaderMap) -> Self {
        use http::header::{Entry, OccupiedEntry};

        if let Ok(ref mut req) = self.request {
            // IntoIter of HeaderMap yields (Option<HeaderName>, HeaderValue).
            // The first time a name is yielded, it will be Some(name), and if
            // there are more values with the same name, the next yield will be
            // None.

            let mut prev_entry: Option<OccupiedEntry<'_, _>> = None;
            for (key, value) in headers {
                match key {
                    Some(key) => match req.headers_mut().entry(key) {
                        Entry::Occupied(mut e) => {
                            e.insert(value);
                            prev_entry = Some(e);
                        }
                        Entry::Vacant(e) => {
                            let e = e.insert_entry(value);
                            prev_entry = Some(e);
                        }
                    },
                    None => match prev_entry {
                        Some(ref mut entry) => {
                            entry.append(value);
                        }
                        None => unreachable!("HeaderMap::into_iter yielded None first"),
                    },
                }
            }
        }
        self
    }

    #[cfg(feature = "gcp")]
    pub(crate) fn bearer_auth(mut self, token: &str) -> Self {
        let value = HeaderValue::try_from(format!("Bearer {}", token));
        match (value, &mut self.request) {
            (Ok(mut v), Ok(r)) => {
                v.set_sensitive(true);
                r.headers_mut().insert(http::header::AUTHORIZATION, v);
            }
            (Err(e), Ok(_)) => self.request = Err(e.into()),
            (_, Err(_)) => {}
        }
        self
    }

    #[cfg(any(feature = "aws", feature = "gcp"))]
    pub(crate) fn json<S: serde::Serialize>(mut self, s: S) -> Self {
        match (serde_json::to_vec(&s), &mut self.request) {
            (Ok(json), Ok(request)) => {
                *request.body_mut() = json.into();
            }
            (Err(e), Ok(_)) => self.request = Err(e.into()),
            (_, Err(_)) => {}
        }
        self
    }

    #[cfg(any(test, feature = "aws", feature = "gcp", feature = "azure"))]
    pub(crate) fn query<T: serde::Serialize + ?Sized>(mut self, query: &T) -> Self {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            let mut out = format!("{}?", req.uri().path());
            let start_position = out.len();
            let mut encoder = form_urlencoded::Serializer::for_suffix(&mut out, start_position);
            let serializer = serde_urlencoded::Serializer::new(&mut encoder);

            if let Err(err) = query.serialize(serializer) {
                error = Some(err.into());
            }

            match http::uri::PathAndQuery::from_maybe_shared(out) {
                Ok(p) => {
                    let mut parts = req.uri().clone().into_parts();
                    parts.path_and_query = Some(p);
                    *req.uri_mut() = Uri::from_parts(parts).unwrap();
                }
                Err(err) => error = Some(err.into()),
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    #[cfg(any(feature = "gcp", feature = "azure"))]
    pub(crate) fn form<T: serde::Serialize>(mut self, form: T) -> Self {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match serde_urlencoded::to_string(form) {
                Ok(body) => {
                    req.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    *req.body_mut() = body.into();
                }
                Err(err) => error = Some(err.into()),
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    #[cfg(any(feature = "aws", feature = "gcp", feature = "azure"))]
    pub(crate) fn body(mut self, b: impl Into<HttpRequestBody>) -> Self {
        if let Ok(r) = &mut self.request {
            *r.body_mut() = b.into();
        }
        self
    }

    pub(crate) fn into_parts(self) -> (HttpClient, Result<HttpRequest, RequestBuilderError>) {
        (self.client, self.request)
    }
}

#[cfg(any(test, feature = "azure"))]
pub(crate) fn add_query_pairs<I, K, V>(uri: &mut Uri, query_pairs: I)
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<(K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut parts = uri.clone().into_parts();

    let mut out = match parts.path_and_query {
        Some(p) => match p.query() {
            Some(query) => format!("{}?{}", p.path(), query),
            None => format!("{}?", p.path()),
        },
        None => "/?".to_string(),
    };
    let mut serializer = if out.ends_with('?') {
        let start_position = out.len();
        form_urlencoded::Serializer::for_suffix(&mut out, start_position)
    } else {
        form_urlencoded::Serializer::new(&mut out)
    };

    serializer.extend_pairs(query_pairs);

    parts.path_and_query = Some(out.try_into().unwrap());
    *uri = Uri::from_parts(parts).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_query_pairs() {
        let mut uri = Uri::from_static("https://foo@example.com/bananas");

        add_query_pairs(&mut uri, [("foo", "1")]);
        assert_eq!(uri.to_string(), "https://foo@example.com/bananas?foo=1");

        add_query_pairs(&mut uri, [("bingo", "foo"), ("auth", "test")]);
        assert_eq!(
            uri.to_string(),
            "https://foo@example.com/bananas?foo=1&bingo=foo&auth=test"
        );

        add_query_pairs(&mut uri, [("t1", "funky shenanigans"), ("a", "😀")]);
        assert_eq!(
            uri.to_string(),
            "https://foo@example.com/bananas?foo=1&bingo=foo&auth=test&t1=funky+shenanigans&a=%F0%9F%98%80"
        );
    }

    #[test]
    fn test_add_query_pairs_no_path() {
        let mut uri = Uri::from_static("https://foo@example.com");
        add_query_pairs(&mut uri, [("foo", "1")]);
        assert_eq!(uri.to_string(), "https://foo@example.com/?foo=1");
    }

    #[test]
    fn test_request_builder_query() {
        let client = HttpClient::new(reqwest::Client::new());
        assert_request_uri(
            HttpRequestBuilder::new(client.clone()).uri("http://example.com/bananas"),
            "http://example.com/bananas",
        );

        assert_request_uri(
            HttpRequestBuilder::new(client.clone())
                .uri("http://example.com/bananas")
                .query(&[("foo", "1")]),
            "http://example.com/bananas?foo=1",
        );

        assert_request_uri(
            HttpRequestBuilder::new(client.clone())
                .uri("http://example.com")
                .query(&[("foo", "1")]),
            "http://example.com/?foo=1",
        );
    }

    fn assert_request_uri(builder: HttpRequestBuilder, expected: &str) {
        assert_eq!(builder.into_parts().1.unwrap().uri().to_string(), expected)
    }
}
