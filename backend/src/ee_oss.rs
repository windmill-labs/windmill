pub async fn set_license_key(license_key: String) -> () {
    crate::ee::set_license_key(license_key).await
}

#[cfg(feature = "enterprise")]
pub async fn verify_license_key() -> () {
    crate::ee::verify_license_key().await
}
