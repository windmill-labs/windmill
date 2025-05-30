#[cfg(all(feature = "enterprise", not(feature = "private")))]
use crate::db::DB;
#[cfg(not(feature = "private"))]
use crate::ee_oss::LicensePlan::Community;
#[cfg(all(feature = "enterprise", not(feature = "private")))]
use crate::error;
#[cfg(not(feature = "private"))]
use serde::Deserialize;
#[cfg(not(feature = "private"))]
use std::sync::Arc;
#[cfg(not(feature = "private"))]
use tokio::sync::RwLock;

#[cfg(not(feature = "private"))]
lazy_static::lazy_static! {
  pub static ref LICENSE_KEY_VALID: Arc<RwLock<bool>> = Arc::new(RwLock::new(true));
  pub static ref LICENSE_KEY_ID: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
  pub static ref LICENSE_KEY: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
}

#[cfg(not(feature = "private"))]
pub enum LicensePlan {
    Community,
    Pro,
    Enterprise,
}

#[cfg(not(feature = "private"))]
pub async fn get_license_plan() -> LicensePlan {
    // Implementation is not open source
    return Community;
}

#[derive(Deserialize)]
#[serde(untagged)]
#[cfg(not(feature = "private"))]
pub enum CriticalErrorChannel {
    Email { email: String },
    Slack { slack_channel: String },
    Teams { teams_channel: TeamsChannel },
}

#[derive(Deserialize)]
#[cfg(not(feature = "private"))]
pub struct TeamsChannel {
    pub team_id: String,
    pub team_name: String,
    pub channel_id: String,
    pub channel_name: String,
}

#[cfg(not(feature = "private"))]
pub enum CriticalAlertKind {
    #[cfg(feature = "enterprise")]
    CriticalError,
    #[cfg(feature = "enterprise")]
    RecoveredCriticalError,
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn send_critical_alert(
    _error_message: String,
    _db: &DB,
    _kind: CriticalAlertKind,
    _channels: Option<Vec<CriticalErrorChannel>>,
) {
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn maybe_renew_license_key_on_start(
    _http_client: &reqwest::Client,
    _db: &crate::db::DB,
    force_renew_now: bool,
) -> bool {
    // Implementation is not open source
    force_renew_now
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub enum RenewReason {
    Manual,
    Schedule,
    OnStart,
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn renew_license_key(
    _http_client: &reqwest::Client,
    _db: &crate::db::DB,
    _key: Option<String>,
    _reason: RenewReason,
) -> String {
    // Implementation is not open source
    "".to_string()
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn create_customer_portal_session(
    _http_client: &reqwest::Client,
    _key: Option<String>,
) -> error::Result<String> {
    // Implementation is not open source
    Ok("".to_string())
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn worker_groups_alerts(_db: &DB) {}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn jobs_waiting_alerts(_db: &DB) {}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn low_disk_alerts(
    _db: &DB,
    _server_mode: bool,
    _worker_mode: bool,
    _workers: Vec<String>,
) {
    // Implementation is not open source
}
