#[cfg(feature = "enterprise")]
use crate::db::ApiAuthed;
use anyhow::anyhow;

pub async fn validate_license_key(_license_key: String) -> anyhow::Result<String> {
    // Implementation is not open source
    Err(anyhow!("License can't be validated in Windmill CE"))
}

#[cfg(feature = "enterprise")]
pub async fn jwt_ext_auth(_w_id: Option<&String>, _token: &str) -> Option<(ApiAuthed, usize)> {
    // Implementation is not open source

    None
}
