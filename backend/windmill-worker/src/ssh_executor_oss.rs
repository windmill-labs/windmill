#[cfg(feature = "private")]
#[allow(unused)]
pub(crate) use crate::ssh_executor_ee::*;

// OSS stub: the `#ssh <resource>` directive is an enterprise feature. The real
// implementation lives in ssh_executor_ee.rs (compiled under the `private`
// feature). See examples/usecase/ssh-execution-wrapper/ for the userland
// (no-license) alternative.
#[cfg(not(feature = "private"))]
pub(crate) async fn handle_ssh_bash_job(
    _ssh_path: &str,
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<windmill_queue::CanceledBy>,
    _job: &windmill_queue::MiniPulledJob,
    _conn: &windmill_common::worker::Connection,
    _client: &windmill_common::client::AuthedClient,
    _content: &str,
    _job_dir: &str,
    _worker_name: &str,
    _occupancy_metrics: &mut crate::common::OccupancyMetrics,
) -> Result<Box<serde_json::value::RawValue>, windmill_common::error::Error> {
    Err(windmill_common::error::Error::ExecutionErr(
        "SSH execution (#ssh) is an enterprise feature. Use the enterprise image, or the userland \
         SSH wrapper in examples/usecase/ssh-execution-wrapper/."
            .to_string(),
    ))
}
