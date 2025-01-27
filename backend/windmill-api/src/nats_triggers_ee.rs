use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NatsResourceAuth {}

pub fn workspaced_service() -> Router {
    Router::new()
}

pub fn start_nats_consumers(_db: DB, mut _killpill_rx: tokio::sync::broadcast::Receiver<()>) -> () {
    // implementation is not open source
}

#[derive(Serialize, Deserialize)]
pub enum NatsTriggerConfigConnection {}
