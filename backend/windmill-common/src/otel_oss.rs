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
