use crate::ee::LicensePlan::Community;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
  pub static ref LICENSE_KEY_VALID: Arc<RwLock<bool>> = Arc::new(RwLock::new(true));
  pub static ref LICENSE_KEY_ID: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
  pub static ref LICENSE_KEY: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
}

pub enum LicensePlan {
    Community,
    Pro,
    Enterprise,
}

pub async fn get_license_plan() -> LicensePlan {
    // Implementation is not open source
    return Community;
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum CriticalErrorChannel {}

pub async fn trigger_critical_error_channels(_error_message: String) {}

#[cfg(feature = "enterprise")]
pub async fn renew_license_key(_http_client: &reqwest::Client, _db: &crate::db::DB) -> String {
    // Implementation is not open source
    "License cannot be renewed in Windmill CE".to_string()
}

#[cfg(feature = "enterprise")]
pub async fn schedule_key_renewal(_http_client: &reqwest::Client, _db: &crate::db::DB) -> () {
    // Implementation is not open source
}
