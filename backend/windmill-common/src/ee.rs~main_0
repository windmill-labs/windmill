#[cfg(feature = "enterprise")]
use crate::db::DB;
use crate::ee::LicensePlan::Community;
#[cfg(feature = "enterprise")]
use crate::error;
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
pub enum CriticalErrorChannel {
    Email { email: String },
    Slack { slack_channel: String },
}

pub enum CriticalAlertKind {
    #[cfg(feature = "enterprise")]
    CriticalError,
    #[cfg(feature = "enterprise")]
    RecoveredCriticalError,
}

#[cfg(feature = "enterprise")]
pub async fn send_critical_alert(
    _error_message: String,
    _db: &DB,
    _kind: CriticalAlertKind,
    _channels: Option<Vec<CriticalErrorChannel>>,
) {
}

#[cfg(feature = "enterprise")]
pub async fn schedule_key_renewal(_http_client: &reqwest::Client, _db: &crate::db::DB) -> () {
    // Implementation is not open source
}

#[cfg(feature = "enterprise")]
pub async fn renew_license_key(
    _http_client: &reqwest::Client,
    _db: &crate::db::DB,
    _key: Option<String>,
    _manual: bool,
) -> String {
    // Implementation is not open source
    "".to_string()
}

#[cfg(feature = "enterprise")]
pub async fn create_customer_portal_session(
    _http_client: &reqwest::Client,
    _key: Option<String>,
) -> error::Result<String> {
    // Implementation is not open source
    Ok("".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn worker_groups_alerts(_db: &DB) {}
