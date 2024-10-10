use anyhow::anyhow;
#[cfg(feature = "enterprise")]
use windmill_common::error::{Error, Result};

pub async fn set_license_key(_license_key: String) -> anyhow::Result<bool> {
    // Implementation is not open source
    Err(anyhow!("License cannot be set in Windmill CE"))
}

#[cfg(feature = "enterprise")]
pub async fn verify_license_key() -> bool {
    // Implementation is not open source
    false
}
