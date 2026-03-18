#[cfg(feature = "private")]
pub use crate::runner_group_ee::*;

#[cfg(not(feature = "private"))]
pub async fn create_runner_group_map(
    _killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    _db: &windmill_common::db::DB,
    _worker_dir: &str,
    _base_internal_url: &str,
    _worker_name: &str,
    _job_completed_tx: &crate::JobCompletedSender,
) -> (
    std::collections::HashMap<
        String,
        tokio::sync::mpsc::Sender<windmill_queue::DedicatedWorkerJob>,
    >,
    Vec<tokio::task::JoinHandle<()>>,
) {
    (std::collections::HashMap::new(), vec![])
}
