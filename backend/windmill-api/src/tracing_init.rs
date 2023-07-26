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

#[derive(Clone)]
pub struct MyOnResponse {}

impl<B> OnResponse<B> for MyOnResponse {
    fn on_response(
        self,
        response: &Response<B>,
        latency: std::time::Duration,
        _span: &tracing::Span,
    ) {
        tracing::info!(
            latency = latency.as_millis(),
            status = response.status().as_u16(),
            "response"
        )
    }
}

#[derive(Clone)]
pub struct MyOnFailure {}

impl<B> OnFailure<B> for MyOnFailure {
    fn on_failure(&mut self, _b: B, _latency: std::time::Duration, _span: &tracing::Span) {
        // tracing::error!(latency = latency.as_millis(), "response")
    }
}
#[derive(Clone)]
pub struct MyMakeSpan {}

impl<B> MakeSpan<B> for MyMakeSpan {
    fn make_span(&mut self, request: &hyper::Request<B>) -> Span {
        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            username = field::Empty,
            workspace_id = field::Empty,
            email = field::Empty,
        )
    }
}
