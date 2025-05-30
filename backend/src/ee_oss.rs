#[cfg(not(feature = "private"))]
pub async fn set_license_key(_license_key: String) -> () {
    // Implementation is not open source
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn verify_license_key() -> () {
    // Implementation is not open source
}
