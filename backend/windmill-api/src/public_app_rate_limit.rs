/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use chrono::Utc;
use dashmap::DashMap;
use hyper::StatusCode;
use std::sync::LazyLock;
use windmill_common::error::{Error, Result};

struct RateLimitEntry {
    count: i32,
    minute_bucket: i64,
}

static RATE_LIMIT_COUNTER: LazyLock<DashMap<String, RateLimitEntry>> = LazyLock::new(DashMap::new);

pub fn check_and_increment(workspace_id: &str, limit: i32) -> Result<()> {
    let current_minute = Utc::now().timestamp() / 60;

    let mut entry = RATE_LIMIT_COUNTER
        .entry(workspace_id.to_string())
        .or_insert(RateLimitEntry { count: 0, minute_bucket: current_minute });

    if entry.minute_bucket != current_minute {
        entry.count = 0;
        entry.minute_bucket = current_minute;
    }

    if entry.count >= limit {
        return Err(Error::Generic(
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Rate limit exceeded for public app executions in workspace '{}'. \
                 Limit: {} per minute per server.",
                workspace_id, limit
            ),
        ));
    }

    entry.count += 1;
    Ok(())
}
