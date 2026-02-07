/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! OnceLock bridges for functions that live in windmill-api but are needed by windmill-store.

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use windmill_common::db::DB;
use windmill_common::error;

// ------------ _refresh_token bridge ------------

type RefreshTokenFn = Box<
    dyn Fn(
            DB,
            String, // path
            String, // w_id
            i32,    // account_id
        ) -> Pin<Box<dyn Future<Output = error::Result<String>> + Send>>
        + Send
        + Sync,
>;

static REFRESH_TOKEN_FN: OnceLock<RefreshTokenFn> = OnceLock::new();

pub fn set_refresh_token_fn<F, Fut>(f: F)
where
    F: Fn(DB, String, String, i32) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = error::Result<String>> + Send + 'static,
{
    REFRESH_TOKEN_FN
        .set(Box::new(move |db, path, w_id, id| {
            Box::pin(f(db.clone(), path, w_id, id))
        }))
        .ok();
}

/// Call the registered _refresh_token function. Returns an error if the bridge is not registered.
pub async fn refresh_token(
    db: &DB,
    path: &str,
    w_id: &str,
    account_id: i32,
) -> error::Result<String> {
    let f = REFRESH_TOKEN_FN.get().ok_or_else(|| {
        error::Error::BadRequest(
            "OAuth2 token refresh is not available (bridge not registered)".to_string(),
        )
    })?;
    f(db.clone(), path.to_string(), w_id.to_string(), account_id).await
}
