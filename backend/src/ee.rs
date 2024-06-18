use anyhow::anyhow;
#[cfg(feature = "enterprise")]
use windmill_common::error::{Error, Result};

pub async fn set_license_key(_license_key: String) -> anyhow::Result<()> {
    // Implementation is not open source
    Err(anyhow!("License cannot be set in Windmill CE"))
}

#[cfg(feature = "enterprise")]
pub async fn verify_license_key() -> Result<()> {
    // Implementation is not open source
    Err(Error::InternalErr(
        "License always invalid in Windmill CE".to_string(),
    ))
}
