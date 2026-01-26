//! OSS stubs for OTEL tracing proxy (EE feature)

#[cfg(all(feature = "private", feature = "enterprise"))]
pub use crate::otel_tracing_proxy_ee::*;
