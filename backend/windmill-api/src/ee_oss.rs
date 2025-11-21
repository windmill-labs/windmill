#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ee::*;
#[cfg(all(
    feature = "enterprise",
    any(feature = "nats", feature = "kafka", feature = "sqs_trigger"),
    not(feature = "private")
))]
use {crate::db::ApiAuthed, windmill_common::DB};

#[cfg(not(feature = "private"))]
use anyhow::anyhow;
#[cfg(all(feature = "enterprise", not(feature = "private")))]
use {std::sync::Arc, tokio::sync::RwLock};
#[cfg(not(feature = "private"))]
pub async fn validate_license_key(
    _license_key: String,
    _db: Option<&crate::db::DB>,
) -> anyhow::Result<(String, bool)> {
    // Implementation is not open source
    Err(anyhow!("License can't be validated in Windmill CE"))
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn jwt_ext_auth(
    _w_id: Option<&String>,
    _token: &str,
    _external_jwks: Option<Arc<RwLock<ExternalJwks>>>,
    _db: &crate::db::DB,
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

#[cfg(all(
    feature = "enterprise",
    any(feature = "nats", feature = "kafka", feature = "sqs_trigger"),
    not(feature = "private")
))]
pub async fn interpolate(
    _authed: &ApiAuthed,
    _db: &DB,
    _w_id: &str,
    _s: String,
) -> Result<String, anyhow::Error> {
    // Implementation is not open source
    Err(anyhow!(
        "Interpolation is not available in open source version"
    ))
}
