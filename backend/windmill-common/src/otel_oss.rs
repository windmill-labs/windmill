use crate::{jobs::QueuedJob, server::Mode};
use std::future::Future;
use tracing_subscriber::filter::EnvFilter;
use uuid::Uuid;

pub(crate) type OtelProvider = Option<()>;

pub fn set_span_parent(_span: &tracing::Span, _rj: &Uuid) {
    crate::otel_ee::set_span_parent(_span, _rj)
}

pub fn otel_ctx() -> () {
    crate::otel_ee::otel_ctx()
}

pub(crate) fn init_logs_bridge(_mode: &Mode, _hostname: &str, _env: &str) -> Option<EnvFilter> {
    crate::otel_ee::init_logs_bridge(_mode, _hostname, _env)
}

pub(crate) fn init_meter_provider(_mode: &Mode, _hostname: &str, _env: &str) -> OtelProvider {
    crate::otel_ee::init_meter_provider(_mode, _hostname, _env)
}

pub fn add_root_flow_job_to_otlp(_queued_job: &QueuedJob, _success: bool) {
    crate::otel_ee::add_root_flow_job_to_otlp(_queued_job, _success)
}

pub trait FutureExt: Sized {
    fn with_context(self, _otel_cx: ()) -> Self {
        self
    }
}

impl<T: Future> FutureExt for T {}