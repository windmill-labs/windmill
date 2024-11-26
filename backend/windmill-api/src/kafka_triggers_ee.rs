use crate::db::DB;
use axum::Router;

pub fn workspaced_service() -> Router {
    Router::new()
}

pub async fn start_kafka_consumers(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    // implementation is not open source
}
