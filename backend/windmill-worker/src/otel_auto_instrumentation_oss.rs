//! OTel Auto-Instrumentation Collector - OSS Stub
//!
//! This module provides stub implementations for the OTel auto-instrumentation
//! collector. The actual implementation is in the EE version.

#[cfg(feature = "enterprise")]
#[allow(unused)]
pub use crate::otel_auto_instrumentation_impl::*;

use serde::{Deserialize, Serialize};

#[cfg(not(feature = "enterprise"))]
use uuid::Uuid;

/// Configuration for OTel auto-instrumentation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OtelAutoInstrumentationConfig {
    pub enabled: bool,
    pub python_enabled: bool,
    pub typescript_enabled: bool,
    pub collector_port: u16,
}

impl OtelAutoInstrumentationConfig {
    pub fn default_config() -> Self {
        Self {
            enabled: false,
            python_enabled: true,
            typescript_enabled: true,
            collector_port: 4318,
        }
    }
}

/// OTel span received from auto-instrumented scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: Option<String>,
    pub start_time_unix_nano: i64,
    pub end_time_unix_nano: i64,
    pub status_code: i16,
    pub status_message: Option<String>,
    pub attributes: serde_json::Value,
    pub events: serde_json::Value,
}

/// OSS stub: Check if OTel auto-instrumentation is enabled
#[cfg(not(feature = "enterprise"))]
pub async fn is_otel_auto_instrumentation_enabled(
    _db: &sqlx::Pool<sqlx::Postgres>,
) -> bool {
    false
}

/// OSS stub: Get OTel auto-instrumentation config
#[cfg(not(feature = "enterprise"))]
pub async fn get_otel_auto_instrumentation_config(
    _db: &sqlx::Pool<sqlx::Postgres>,
) -> OtelAutoInstrumentationConfig {
    OtelAutoInstrumentationConfig::default_config()
}

/// OSS stub: Get OTel environment variables for Python scripts
#[cfg(not(feature = "enterprise"))]
pub fn get_otel_python_env_vars(
    _job_id: &Uuid,
    _workspace_id: &str,
    _script_path: &str,
    _config: &OtelAutoInstrumentationConfig,
) -> Vec<(String, String)> {
    vec![]
}

/// OSS stub: Get OTel environment variables for TypeScript scripts (Bun/Deno)
#[cfg(not(feature = "enterprise"))]
pub fn get_otel_typescript_env_vars(
    _job_id: &Uuid,
    _workspace_id: &str,
    _script_path: &str,
    _config: &OtelAutoInstrumentationConfig,
) -> Vec<(String, String)> {
    vec![]
}

/// OSS stub: Store OTel spans in the database
#[cfg(not(feature = "enterprise"))]
pub async fn store_otel_spans(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _job_id: &Uuid,
    _workspace_id: &str,
    _spans: Vec<OtelSpan>,
) -> anyhow::Result<()> {
    Ok(())
}

/// OSS stub: Start the built-in OTel collector HTTP server
#[cfg(not(feature = "enterprise"))]
pub async fn start_otel_collector_server(
    _db: sqlx::Pool<sqlx::Postgres>,
    _config: OtelAutoInstrumentationConfig,
) -> anyhow::Result<()> {
    // No-op in OSS
    Ok(())
}
