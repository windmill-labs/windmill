#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ai_free_tier_ee::*;

// Open-source build: Windmill's free AI tier does not exist. These stubs make the
// callers in `ai.rs` / `workspaces.rs` compile while disabling the feature entirely —
// `resolve_free_tier_credentials` never opts in, so the proxy falls through to its
// normal "AI resource not configured" path and the copilot stays hidden.

#[cfg(not(feature = "private"))]
use crate::ai::AIConfig;
#[cfg(not(feature = "private"))]
use crate::db::DB;
#[cfg(not(feature = "private"))]
use axum::body::Bytes;
#[cfg(not(feature = "private"))]
use windmill_ai::ai_providers::AIProvider;
#[cfg(not(feature = "private"))]
use windmill_ai::credentials::ProviderCredentials;
#[cfg(not(feature = "private"))]
use windmill_common::error::Result;

#[cfg(not(feature = "private"))]
pub struct FreeTierLease;

#[cfg(not(feature = "private"))]
pub async fn resolve_free_tier_credentials(
    _provider: &AIProvider,
    _db: &DB,
    _ai_path: &str,
    _email: &str,
) -> Result<Option<(ProviderCredentials, FreeTierLease)>> {
    Ok(None)
}

#[cfg(not(feature = "private"))]
pub fn enforce_free_tier_body(body: &Bytes) -> Result<Bytes> {
    Ok(body.clone())
}

#[cfg(not(feature = "private"))]
pub async fn free_tier_copilot_config(_db: &DB, _email: &str) -> Result<Option<AIConfig>> {
    Ok(None)
}

#[cfg(not(feature = "private"))]
pub fn record_json_usage(_db: DB, _lease: FreeTierLease, _bytes: &[u8]) {}

#[cfg(not(feature = "private"))]
pub fn meter_usage<S>(
    upstream: S,
    _db: DB,
    _lease: FreeTierLease,
) -> impl futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>>
where
    S: futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream
}
