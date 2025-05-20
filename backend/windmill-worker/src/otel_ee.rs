#[cfg(feature = "private")]
use crate::otel_ee;

use windmill_queue::MiniPulledJob;

pub fn add_root_flow_job_to_otlp(queued_job: &MiniPulledJob, success: bool) {
    #[cfg(feature = "private")]
    {
        otel_ee::add_root_flow_job_to_otlp(queued_job, success);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (queued_job, success);
    }
}
