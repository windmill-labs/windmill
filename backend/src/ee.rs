#[cfg(feature = "enterprise")]
use windmill_common::error;

pub async fn set_license_key(license_key: String) -> anyhow::Result<()> {
    use windmill_api::ee::validate_license_key;
    use windmill_common::ee::{LICENSE_KEY, LICENSE_KEY_ID, LICENSE_KEY_VALID};

    let id = validate_license_key(license_key.clone()).await?;
    {
        let mut l = LICENSE_KEY_ID.write().await;
        *l = id.to_string()
    }

    {
        let mut l = LICENSE_KEY.write().await;
        *l = license_key
    }
    {
        let mut l = LICENSE_KEY_VALID.write().await;
        *l = true
    }

    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn verify_license_key() -> error::Result<()> {
    use windmill_common::ee::{LICENSE_KEY, LICENSE_KEY_VALID};
    use windmill_common::error::to_anyhow;

    let expiry_nb = LICENSE_KEY
        .read()
        .await
        .clone()
        .split(".")
        .nth(1)
        .unwrap_or_else(|| "")
        .parse::<u64>()
        .map_err(to_anyhow)?;
    if expiry_nb < chrono::Utc::now().timestamp() as u64 {
        tracing::error!(
            "License key expired: {} < {}",
            expiry_nb,
            chrono::Utc::now().timestamp() as u64
        );
        let mut l = LICENSE_KEY_VALID.write().await;
        *l = false;
    };
    Ok(())
}
