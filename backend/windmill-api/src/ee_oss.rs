use anyhow::anyhow;
#[cfg(feature = "enterprise")]
use std::sync::Arc;
#[cfg(feature = "enterprise")]
use tokio::sync::RwLock;

pub async fn validate_license_key(_license_key: String) -> anyhow::Result<(String, bool)> {
    // Implementation is not open source
    Err(anyhow!("License can't be validated in Windmill CE"))
}

#[cfg(feature = "enterprise")]
pub async fn jwt_ext_auth(
    _w_id: Option<&String>,
    _token: &str,
    _external_jwks: Option<Arc<RwLock<ExternalJwks>>>,
) -> anyhow::Result<(crate::db::ApiAuthed, usize)> {
    // Implementation is not open source

    Err(anyhow!("External JWT auth is not open source"))
}

#[cfg(feature = "enterprise")]
pub struct ExternalJwks;

#[cfg(feature = "enterprise")]
impl ExternalJwks {
    pub async fn load() -> Option<Arc<RwLock<Self>>> {
        // Implementation is not open source
        None
    }
}
