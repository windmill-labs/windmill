#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ee::*;

#[cfg(all(feature = "enterprise", not(feature = "private")))]
use crate::db::DB;
#[cfg(not(feature = "private"))]
use crate::ee_oss::LicensePlan::Community;
#[cfg(all(feature = "enterprise", not(feature = "private")))]
use crate::error;
#[cfg(not(feature = "private"))]
use serde::Deserialize;
#[cfg(not(feature = "private"))]
use std::sync::atomic::AtomicBool;

#[cfg(not(feature = "private"))]
lazy_static::lazy_static! {
  pub static ref LICENSE_KEY_VALID: AtomicBool = AtomicBool::new(true);
  pub static ref LICENSE_KEY_ID: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee("".to_string());
  pub static ref LICENSE_KEY: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee("".to_string());
  pub static ref LICENSE_OFFLINE_METADATA: arc_swap::ArcSwap<Option<OfflineMetadata>> = arc_swap::ArcSwap::from_pointee(None);
  pub static ref LICENSE_OFFLINE_OVER_CU_CAP: AtomicBool = AtomicBool::new(false);
  pub static ref LICENSE_OFFLINE_OVER_SEAT_CAP: AtomicBool = AtomicBool::new(false);
  pub static ref LICENSE_OFFLINE_LAST_STATUS: arc_swap::ArcSwap<Option<OfflineCapStatus>> = arc_swap::ArcSwap::from_pointee(None);
  pub static ref LICENSE_OFFLINE_LAST_CHECKED_AT: arc_swap::ArcSwap<Option<chrono::DateTime<chrono::Utc>>> = arc_swap::ArcSwap::from_pointee(None);
}

#[cfg(not(feature = "private"))]
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct OfflineMetadata {
    pub v: u32,
    pub kind: String,
    pub hash: String,
    pub seats: i64,
    pub cu_limit: f64,
}

#[cfg(not(feature = "private"))]
impl OfflineMetadata {
    pub fn is_offline(&self) -> bool {
        self.kind == "offline"
    }
}

#[cfg(not(feature = "private"))]
#[derive(Clone, Debug, serde::Serialize)]
pub struct OfflineCapStatus {
    pub seats_used: f64,
    pub seats_cap: i64,
    pub author_count: i64,
    pub operator_count: i64,
    pub current_cu: f64,
    pub cu_cap: f64,
    pub cu_over_cap: bool,
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn check_seat_cap_for_new_user(
    _db: &DB,
    _email: &str,
    _new_user_is_operator: bool,
) -> anyhow::Result<Option<String>> {
    Ok(None)
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn check_seat_cap_for_reactivation(
    _db: &DB,
    _email: &str,
) -> anyhow::Result<Option<String>> {
    Ok(None)
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn compute_instance_hash(_db: &DB) -> anyhow::Result<Option<String>> {
    // Implementation is not open source
    Ok(None)
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn enforce_offline_caps(_db: &DB) -> anyhow::Result<Option<OfflineCapStatus>> {
    // Implementation is not open source
    Ok(None)
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn alert_on_online_license_expired(_db: &DB) {
    // Implementation is not open source
}

#[cfg(not(feature = "private"))]
#[derive(PartialEq, Eq)]
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
