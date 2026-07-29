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

use crate::Result;
use futures::Stream;
use std::future::Future;

/// Takes a paginated operation `op` that when called with:
///
/// - A state `S`
/// - An optional next token `Option<String>`
///
/// Returns
///
/// - A response value `T`
/// - The next state `S`
/// - The next continuation token `Option<String>`
///
/// And converts it into a `Stream<Result<T>>` which will first call `op(state, None)`, and yield
/// the returned response `T`. If the returned continuation token was `None` the stream will then
/// finish, otherwise it will continue to call `op(state, token)` with the values returned by the
/// previous call to `op`, until a continuation token of `None` is returned
///
pub(crate) fn stream_paginated<F, Fut, S, T, C>(
    client: C,
    state: S,
    op: F,
) -> impl Stream<Item = Result<T>>
where
    C: Clone,
    F: Fn(C, S, Option<String>) -> Fut + Copy,
    Fut: Future<Output = Result<(T, S, Option<String>)>>,
{
    enum PaginationState<T> {
        Start(T),
        HasMore(T, String),
        Done,
    }

    futures::stream::unfold(PaginationState::Start(state), move |state| {
        let client = client.clone();
        async move {
            let (s, page_token) = match state {
                PaginationState::Start(s) => (s, None),
                PaginationState::HasMore(s, page_token) if !page_token.is_empty() => {
                    (s, Some(page_token))
                }
                _ => {
                    return None;
                }
            };

            let (resp, s, continuation) = match op(client, s, page_token).await {
                Ok(resp) => resp,
                Err(e) => return Some((Err(e), PaginationState::Done)),
            };

            let next_state = match continuation {
                Some(token) => PaginationState::HasMore(s, token),
                None => PaginationState::Done,
            };

            Some((Ok(resp), next_state))
        }
    })
}
