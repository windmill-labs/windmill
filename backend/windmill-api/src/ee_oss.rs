#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ee::*;

#[cfg(not(feature = "private"))]
use anyhow::anyhow;
#[cfg(all(feature = "enterprise", not(feature = "private")))]
use std::sync::Arc;
#[cfg(all(feature = "enterprise", not(feature = "private")))]
use tokio::sync::RwLock;

#[cfg(not(feature = "private"))]
pub async fn validate_license_key(_license_key: String) -> anyhow::Result<(String, bool)> {
    // Implementation is not open source
    Err(anyhow!("License can't be validated in Windmill CE"))
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn jwt_ext_auth(
    _w_id: Option<&String>,
    _token: &str,
    _external_jwks: Option<Arc<RwLock<ExternalJwks>>>,
) -> anyhow::Result<(crate::db::ApiAuthed, usize)> {
    // Implementation is not open source

    Err(anyhow!("External JWT auth is not open source"))
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
