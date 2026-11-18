#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::otel_ee::*;

#[cfg(not(feature = "private"))]
use windmill_queue::MiniPulledJob;

#[cfg(not(feature = "private"))]
pub fn add_root_flow_job_to_otlp(_queued_job: &MiniPulledJob, _success: bool) {}

#[cfg(not(feature = "private"))]
pub fn set_job_span_parent(_span: &tracing::Span, _job: &MiniPulledJob, _rj: &uuid::Uuid) {}
