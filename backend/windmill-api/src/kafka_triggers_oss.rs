use crate::db::DB;
use axum::Router;

pub fn workspaced_service() -> Router {
    crate::kafka_triggers_ee::workspaced_service()
}

pub fn start_kafka_consumers(
    _db: DB,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    crate::kafka_triggers_ee::start_kafka_consumers(_db, _killpill_rx)
}
