use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
  pub static ref LICENSE_KEY_VALID: Arc<RwLock<bool>> = Arc::new(RwLock::new(true));
  pub static ref LICENSE_KEY_ID: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
  pub static ref LICENSE_KEY: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
}
