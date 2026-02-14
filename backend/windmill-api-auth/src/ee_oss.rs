/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ee::*;

#[cfg(all(feature = "enterprise", not(feature = "private")))]
use {std::sync::Arc, tokio::sync::RwLock};

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn jwt_ext_auth(
    _w_id: Option<&String>,
    _token: &str,
    _external_jwks: Option<Arc<RwLock<ExternalJwks>>>,
    _db: &windmill_common::DB,
) -> anyhow::Result<(crate::ApiAuthed, usize, Option<uuid::Uuid>)> {
    // Implementation is not open source
    Err(anyhow::anyhow!("External JWT auth is not open source"))
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub struct ExternalJwks;

#[cfg(all(feature = "enterprise", not(feature = "private")))]
impl ExternalJwks {
    pub async fn load() -> Option<Arc<RwLock<Self>>> {
        // Implementation is not open source
        None
    }
}
