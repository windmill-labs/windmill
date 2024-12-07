/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{jobs::QueuedJob, utils::Mode};
use uuid::Uuid;

pub fn set_span_parent(_span: &tracing::Span, _rj: &Uuid) {}

#[cfg(not(all(feature = "otel", feature = "enterprise")))]
pub(crate) type OtelProvider = ();

pub fn otel_ctx() -> () {}

impl<T: Sized> FutureExt for T {}

pub trait FutureExt: Sized {
    fn with_context(self, _otel_cx: ()) -> Self {
        self
    }
}

use tracing_subscriber::EnvFilter;

pub(crate) fn init_logs_bridge(_mode: &Mode) -> Option<EnvFilter> {
    None
}

#[cfg(all(feature = "otel", feature = "enterprise"))]
pub(crate) fn init_otlp_tracer(_mode: &Mode) {
    todo!()
}

#[cfg(not(all(feature = "otel", feature = "enterprise")))]
pub(crate) fn init_meter_provider(_mode: &Mode) -> OtelProvider {}

pub fn add_root_flow_job_to_otlp(_queued_job: &QueuedJob, _success: bool) {}
