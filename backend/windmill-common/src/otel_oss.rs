#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::otel_ee::*;

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(not(feature = "private"))]
use crate::{jobs::QueuedJob, utils::Mode};
#[cfg(not(feature = "private"))]
use uuid::Uuid;

#[cfg(not(feature = "private"))]
pub fn set_span_parent(_span: &tracing::Span, _rj: &Uuid) {}

#[cfg(all(
    not(all(feature = "otel", feature = "enterprise")),
    not(feature = "private")
))]
pub(crate) type OtelProvider = Option<()>;

#[cfg(all(feature = "otel", feature = "enterprise", not(feature = "private")))]
pub(crate) type OtelProvider = Option<opentelemetry_sdk::metrics::SdkMeterProvider>;

#[cfg(all(not(feature = "otel"), not(feature = "private")))]
pub fn otel_ctx() -> () {}

#[cfg(all(feature = "otel", not(feature = "private")))]
#[inline(always)]
pub fn otel_ctx() -> opentelemetry::Context {
    opentelemetry::Context::current()
}

#[cfg(all(not(feature = "otel"), not(feature = "private")))]
impl<T: Sized> FutureExt for T {}

#[cfg(all(not(feature = "otel"), not(feature = "private")))]
pub trait FutureExt: Sized {
    fn with_context(self, _otel_cx: ()) -> Self {
        self
    }
}

#[cfg(not(feature = "private"))]
use tracing_subscriber::EnvFilter;

#[cfg(not(feature = "private"))]
pub(crate) fn init_logs_bridge(_mode: &Mode, _hostname: &str, _env: &str) -> Option<EnvFilter> {
    None
}

#[cfg(all(feature = "otel", feature = "enterprise", not(feature = "private")))]
pub(crate) fn init_otlp_tracer(
    _mode: &Mode,
    _hostname: &str,
    _env: &str,
) -> Option<opentelemetry_sdk::trace::Tracer> {
    None
}

#[cfg(not(feature = "private"))]
pub(crate) fn init_meter_provider(_mode: &Mode, _hostname: &str, _env: &str) -> OtelProvider {
    None
}

#[cfg(not(feature = "private"))]
pub fn add_root_flow_job_to_otlp(_queued_job: &QueuedJob, _success: bool) {}

// ── OTel metric recording stubs (OSS) ───────────────────────────────────────

#[cfg(not(feature = "private"))]
pub fn otel_incr_queue_push_count() {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_queue_delete_count() {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_queue_pull_count() {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_zombie_restart_count(_count: u64) {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_zombie_delete_count(_count: u64) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_queue_count(_tag: &str, _count: i64) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_queue_running_count(_tag: &str, _count: i64) {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_worker_execution_count(_tag: &str) {}

#[cfg(not(feature = "private"))]
pub fn otel_record_worker_execution_duration(_tag: &str, _secs: f64) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_worker_busy(_worker: &str, _busy: i64) {}

#[cfg(not(feature = "private"))]
pub fn otel_record_worker_pull_duration(_worker: &str, _has_job: bool, _secs: f64) {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_worker_execution_failed(_tag: &str) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_db_pool(_active: i64, _idle: i64, _max: i64) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_health_db_latency(_ms: f64) {}

#[cfg(not(feature = "private"))]
pub fn otel_incr_worker_started() {}

#[cfg(not(feature = "private"))]
pub fn otel_set_worker_uptime(_worker: &str, _secs: f64) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_health_status_phase(_phase: &str) {}

#[cfg(not(feature = "private"))]
pub fn otel_set_health_db_unresponsive(_unresponsive: bool) {}
