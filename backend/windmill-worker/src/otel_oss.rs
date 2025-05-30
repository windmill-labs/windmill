#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::otel_ee::*;

#[cfg(not(feature = "private"))]
use windmill_queue::MiniPulledJob;

#[cfg(not(feature = "private"))]
pub fn add_root_flow_job_to_otlp(_queued_job: &MiniPulledJob, _success: bool) {}
