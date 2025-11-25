#[cfg(feature = "private")]
// #[allow(unused)]
pub use crate::dedicated_worker_ee::*;

#[cfg(not(feature = "private"))]
pub async fn spawn_flow_module_runners(
    _job: &windmill_queue::MiniPulledJob,
    _module: &windmill_common::flows::FlowModule,
    _failure_module: Option<&Box<windmill_common::flows::FlowModule>>,
    _killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    _db: &windmill_common::db::DB,
    _worker_dir: &str,
    _base_internal_url: &str,
    _worker_name: &str,
    _job_completed_tx: &crate::JobCompletedSender,
) -> windmill_common::error::Result<(
    std::collections::HashMap<
        String,
        tokio::sync::mpsc::Sender<windmill_queue::DedicatedWorkerJob>,
    >,
    Vec<tokio::task::JoinHandle<()>>,
)> {
    Err(windmill_common::error::Error::internal_err(
        "Squashing flow loops is not available in OSS".to_string(),
    ))
}
