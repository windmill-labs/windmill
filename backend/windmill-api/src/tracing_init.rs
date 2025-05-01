/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use ::tracing::{field, Span};
use hyper::Response;
use tower_http::trace::{MakeSpan, OnFailure, OnResponse};
use uuid::Uuid;

lazy_static::lazy_static! {
    static ref LOG_REQUESTS: bool = std::env::var("LOG_REQUESTS")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(true);
}

#[derive(Clone)]
pub struct MyOnResponse {}

impl<B> OnResponse<B> for MyOnResponse {
    fn on_response(
        self,
        response: &Response<B>,
        latency: std::time::Duration,
        _span: &tracing::Span,
    ) {
        if *LOG_REQUESTS {
            let latency = latency.as_millis();
            let status = response.status().as_u16();
            if response.status().is_success() || response.status().is_redirection() {
                tracing::info!(latency = latency, status = status, "response")
            } else if response.status().as_u16() == 404 {
                tracing::warn!(latency = latency, status = status, "response")
            } else {
                tracing::error!(latency = latency, status = status, "response")
            }
        }
    }
}

#[derive(Clone)]
pub struct MyOnFailure {}

impl<B> OnFailure<B> for MyOnFailure {
    fn on_failure(&mut self, _b: B, _latency: std::time::Duration, _span: &tracing::Span) {
        // tracing::error!(latency = latency.as_millis(), "response")
    }
}

lazy_static::lazy_static! {
    static ref TRACING_HEADER: String = std::env::var("TRACING_HEADER")
    .ok().unwrap_or_else(|| "x-tracing-id".to_string());
}
#[derive(Clone)]
pub struct MyMakeSpan {}

impl<B> MakeSpan<B> for MyMakeSpan {
    fn make_span(&mut self, request: &hyper::Request<B>) -> Span {
        let tracing_id = request
            .headers()
            .get(TRACING_HEADER.as_str())
            .and_then(|x| x.to_str().map(|x| x.to_string()).ok())
            .unwrap_or(Uuid::new_v4().to_string());
        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            username = field::Empty,
            workspace_id = field::Empty,
            traceId = tracing_id,
            email = field::Empty,
        )
    }
}
