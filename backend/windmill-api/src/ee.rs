use anyhow::anyhow;

pub async fn validate_license_key(_license_key: String) -> anyhow::Result<String> {
    // Implementation is not open source
    Err(anyhow!("License can't be validated in Windmill CE"))
}
