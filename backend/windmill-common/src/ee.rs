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
    Teams { teams_channel: TeamsChannel },
}

#[derive(Deserialize)]
pub struct TeamsChannel {
    pub team_id: String,
    pub team_name: String,
    pub channel_id: String,
    pub channel_name: String,
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
pub async fn maybe_renew_license_key_on_start(
    _http_client: &reqwest::Client,
    _db: &crate::db::DB,
    force_renew_now: bool,
) -> bool {
    // Implementation is not open source
    force_renew_now
}

#[cfg(feature = "enterprise")]
pub enum RenewReason {
    Manual,
    Schedule,
    OnStart,
}

#[cfg(feature = "enterprise")]
pub async fn renew_license_key(
    _http_client: &reqwest::Client,
    _db: &crate::db::DB,
    _key: Option<String>,
    _reason: RenewReason,
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

#[cfg(feature = "enterprise")]
pub async fn jobs_waiting_alerts(_db: &DB) {}

#[cfg(feature = "enterprise")]
pub async fn low_disk_alerts(_db: &DB, _: bool, _: bool, _: Vec<String>) {}
