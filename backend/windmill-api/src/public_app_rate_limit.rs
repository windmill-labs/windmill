/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use hyper::StatusCode;
use std::sync::LazyLock;
use windmill_common::error::{Error, Result};
use windmill_common::per_minute_counter::PerMinuteCounter;

static RATE_LIMIT_COUNTER: LazyLock<PerMinuteCounter<String>> =
    LazyLock::new(PerMinuteCounter::new);

pub fn check_and_increment(workspace_id: &str, limit: i32) -> Result<()> {
    // Clamp before the cast: `as u32` on a negative limit wraps into an effectively unlimited
    // allowance, where a non-positive limit must reject every execution.
    if RATE_LIMIT_COUNTER.try_increment(workspace_id.to_string(), limit.max(0) as u32) {
        return Ok(());
    }

    Err(Error::Generic(
        StatusCode::TOO_MANY_REQUESTS,
        format!(
            "Rate limit exceeded for public app executions in workspace '{}'. \
             Limit: {} per minute per server.",
            workspace_id, limit
        ),
    ))
}
