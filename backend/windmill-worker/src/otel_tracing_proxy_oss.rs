//! OSS stubs for OTEL tracing proxy (EE feature)

#[cfg(all(feature = "private", feature = "enterprise"))]
pub use crate::otel_tracing_proxy_ee::*;

/// Start the OTEL tracing proxy (no-op in OSS)
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub async fn start_otel_tracing_proxy(
    _db: windmill_common::DB,
    _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
