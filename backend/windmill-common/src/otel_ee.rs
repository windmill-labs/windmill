/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(feature = "private")]
use crate::otel_ee; // Assuming this module exists for EE features

use crate::{jobs::QueuedJob, utils::Mode};
use uuid::Uuid;

pub fn set_span_parent(span: &tracing::Span, rj: &Uuid) {
    #[cfg(feature = "private")]
    {
        otel_ee::set_span_parent(span, rj);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (span, rj);
        // Original OSS behavior was empty.
    }
}

#[cfg(not(all(feature = "otel", feature = "enterprise")))]
pub(crate) type OtelProvider = Option<()>; // Stays as is

#[cfg(all(feature = "otel", feature = "enterprise"))]
pub(crate) type OtelProvider = Option<opentelemetry_sdk::metrics::SdkMeterProvider>; // Stays as is

#[cfg(not(feature = "otel"))]
pub fn otel_ctx() -> () { // Stays as is - this is compile-time conditional, not runtime private flag
    // No change based on "private" feature for this one, as it's already conditional on "otel"
}

#[cfg(feature = "otel")]
#[inline(always)]
pub fn otel_ctx() -> opentelemetry::Context { // Stays as is
    opentelemetry::Context::current()
}

#[cfg(not(feature = "otel"))]
impl<T: Sized> FutureExt for T {} // Stays as is

#[cfg(not(feature = "otel"))]
pub trait FutureExt: Sized { // Stays as is
    fn with_context(self, _otel_cx: ()) -> Self {
        self
    }
}

use tracing_subscriber::EnvFilter;

pub(crate) fn init_logs_bridge(mode: &Mode, hostname: &str, env: &str) -> Option<EnvFilter> {
    #[cfg(feature = "private")]
    {
        return otel_ee::init_logs_bridge(mode, hostname, env);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (mode, hostname, env);
        None
    }
}

#[cfg(all(feature = "otel", feature = "enterprise"))]
pub(crate) fn init_otlp_tracer(
    mode: &Mode,
    hostname: &str,
    env: &str,
) -> Option<opentelemetry_sdk::trace::Tracer> {
    #[cfg(feature = "private")]
    {
        // This function is already within outer cfgs
        return otel_ee::init_otlp_tracer(mode, hostname, env);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (mode, hostname, env);
        None
    }
}

pub(crate) fn init_meter_provider(mode: &Mode, hostname: &str, env: &str) -> OtelProvider {
    #[cfg(feature = "private")]
    {
        return otel_ee::init_meter_provider(mode, hostname, env);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (mode, hostname, env);
        None
    }
}

pub fn add_root_flow_job_to_otlp(queued_job: &QueuedJob, success: bool) {
    #[cfg(feature = "private")]
    {
        otel_ee::add_root_flow_job_to_otlp(queued_job, success);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (queued_job, success);
        // Original OSS behavior was empty.
    }
}
