#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ee::*;

#[cfg(not(feature = "private"))]
use anyhow::anyhow;
#[cfg(not(feature = "private"))]
pub async fn validate_license_key(
    _license_key: String,
    _db: Option<&windmill_common::DB>,
) -> anyhow::Result<(String, bool)> {
    // Implementation is not open source
    Err(anyhow!("License can't be validated in Windmill CE"))
}
