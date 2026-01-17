//! OTel Auto-Instrumentation Collector - Enterprise Edition
//!
//! This module provides the full implementation for OTel auto-instrumentation
//! collector for Windmill Enterprise Edition.
//!
//! Features:
//! - Config caching from global_settings with TTL
//! - Environment variable generation for Python and TypeScript
//! - OTLP trace parsing (JSON format)
//! - Built-in HTTP collector server
//! - Database storage for spans

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use windmill_common::global_settings::{
    load_value_from_global_settings, OTEL_AUTO_INSTRUMENTATION_SETTING,
};

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

/// Cached configuration with TTL
struct CachedConfig {
    config: OtelAutoInstrumentationConfig,
    fetched_at: Instant,
}

// Global config cache with 60-second TTL
lazy_static::lazy_static! {
    static ref CONFIG_CACHE: Arc<RwLock<Option<CachedConfig>>> = Arc::new(RwLock::new(None));
}

const CONFIG_CACHE_TTL: Duration = Duration::from_secs(60);

/// Check if OTel auto-instrumentation is enabled
pub async fn is_otel_auto_instrumentation_enabled(db: &Pool<Postgres>) -> bool {
    let config = get_otel_auto_instrumentation_config(db).await;
    config.enabled
}

/// Get OTel auto-instrumentation config with caching
pub async fn get_otel_auto_instrumentation_config(
    db: &Pool<Postgres>,
) -> OtelAutoInstrumentationConfig {
    // Try to read from cache first
    {
        let cache = CONFIG_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < CONFIG_CACHE_TTL {
                return cached.config.clone();
            }
        }
    }

    // Cache miss or expired, fetch from database
    let config = match load_value_from_global_settings(db, OTEL_AUTO_INSTRUMENTATION_SETTING).await
    {
        Ok(Some(value)) => {
            serde_json::from_value(value).unwrap_or_else(|_| OtelAutoInstrumentationConfig::default_config())
        }
        _ => OtelAutoInstrumentationConfig::default_config(),
    };

    // Update cache
    {
        let mut cache = CONFIG_CACHE.write().await;
        *cache = Some(CachedConfig {
            config: config.clone(),
            fetched_at: Instant::now(),
        });
    }

    config
}

/// Get OTel environment variables for Python scripts
pub fn get_otel_python_env_vars(
    job_id: &Uuid,
    workspace_id: &str,
    script_path: &str,
    config: &OtelAutoInstrumentationConfig,
) -> Vec<(String, String)> {
    if !config.enabled || !config.python_enabled {
        return vec![];
    }

    let service_name = format!("windmill-python-{}", script_path.replace('/', "-"));
    let endpoint = format!("http://127.0.0.1:{}/v1/traces", config.collector_port);

    vec![
        ("WINDMILL_OTEL_AUTO_INSTRUMENTATION".to_string(), "true".to_string()),
        ("WINDMILL_JOB_ID".to_string(), job_id.to_string()),
        ("WINDMILL_WORKSPACE_ID".to_string(), workspace_id.to_string()),
        ("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT".to_string(), endpoint),
        ("OTEL_SERVICE_NAME".to_string(), service_name),
        (
            "OTEL_RESOURCE_ATTRIBUTES".to_string(),
            format!(
                "service.instance.id={},windmill.job_id={},windmill.workspace_id={},windmill.script_path={}",
                job_id, job_id, workspace_id, script_path
            ),
        ),
        // Python-specific OTel instrumentation settings
        ("OTEL_PYTHON_LOGGING_AUTO_INSTRUMENTATION_ENABLED".to_string(), "false".to_string()),
        ("OTEL_TRACES_EXPORTER".to_string(), "otlp".to_string()),
        ("OTEL_EXPORTER_OTLP_PROTOCOL".to_string(), "http/json".to_string()),
    ]
}

/// Get OTel environment variables for TypeScript scripts (Bun/Deno)
pub fn get_otel_typescript_env_vars(
    job_id: &Uuid,
    workspace_id: &str,
    script_path: &str,
    config: &OtelAutoInstrumentationConfig,
) -> Vec<(String, String)> {
    if !config.enabled || !config.typescript_enabled {
        return vec![];
    }

    let service_name = format!("windmill-typescript-{}", script_path.replace('/', "-"));
    let endpoint = format!("http://127.0.0.1:{}/v1/traces", config.collector_port);

    vec![
        ("WINDMILL_OTEL_AUTO_INSTRUMENTATION".to_string(), "true".to_string()),
        ("WINDMILL_JOB_ID".to_string(), job_id.to_string()),
        ("WINDMILL_WORKSPACE_ID".to_string(), workspace_id.to_string()),
        ("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT".to_string(), endpoint),
        ("OTEL_SERVICE_NAME".to_string(), service_name),
        (
            "OTEL_RESOURCE_ATTRIBUTES".to_string(),
            format!(
                "service.instance.id={},windmill.job_id={},windmill.workspace_id={},windmill.script_path={}",
                job_id, job_id, workspace_id, script_path
            ),
        ),
        ("OTEL_TRACES_EXPORTER".to_string(), "otlp".to_string()),
        ("OTEL_EXPORTER_OTLP_PROTOCOL".to_string(), "http/json".to_string()),
    ]
}

/// Store OTel spans in the database
pub async fn store_otel_spans(
    db: &Pool<Postgres>,
    job_id: &Uuid,
    workspace_id: &str,
    spans: Vec<OtelSpan>,
) -> anyhow::Result<()> {
    if spans.is_empty() {
        return Ok(());
    }

    for span in spans {
        sqlx::query(
            r#"
            INSERT INTO job_otel_traces (
                job_id, workspace_id, trace_id, span_id, parent_span_id,
                operation_name, service_name, start_time_unix_nano, end_time_unix_nano,
                status_code, status_message, attributes, events
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(job_id)
        .bind(workspace_id)
        .bind(&span.trace_id)
        .bind(&span.span_id)
        .bind(&span.parent_span_id)
        .bind(&span.operation_name)
        .bind(&span.service_name)
        .bind(span.start_time_unix_nano)
        .bind(span.end_time_unix_nano)
        .bind(span.status_code)
        .bind(&span.status_message)
        .bind(&span.attributes)
        .bind(&span.events)
        .execute(db)
        .await?;
    }

    Ok(())
}

// ============================================================================
// OTLP JSON Parsing
// ============================================================================

/// OTLP ExportTraceServiceRequest (JSON format)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpExportTraceServiceRequest {
    resource_spans: Option<Vec<OtlpResourceSpans>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpResourceSpans {
    resource: Option<OtlpResource>,
    scope_spans: Option<Vec<OtlpScopeSpans>>,
}

#[derive(Debug, Deserialize)]
struct OtlpResource {
    attributes: Option<Vec<OtlpKeyValue>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpScopeSpans {
    #[allow(dead_code)]
    scope: Option<OtlpInstrumentationScope>,
    spans: Option<Vec<OtlpSpan>>,
}

#[derive(Debug, Deserialize)]
struct OtlpInstrumentationScope {
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpSpan {
    trace_id: Option<String>,
    span_id: Option<String>,
    parent_span_id: Option<String>,
    name: Option<String>,
    #[allow(dead_code)]
    kind: Option<i32>,
    start_time_unix_nano: Option<String>,
    end_time_unix_nano: Option<String>,
    attributes: Option<Vec<OtlpKeyValue>>,
    events: Option<Vec<OtlpEvent>>,
    status: Option<OtlpStatus>,
}

#[derive(Debug, Deserialize)]
struct OtlpKeyValue {
    key: String,
    value: Option<OtlpAnyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpAnyValue {
    string_value: Option<String>,
    int_value: Option<String>,
    double_value: Option<f64>,
    bool_value: Option<bool>,
    #[allow(dead_code)]
    array_value: Option<serde_json::Value>,
    #[allow(dead_code)]
    kvlist_value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpEvent {
    #[allow(dead_code)]
    time_unix_nano: Option<String>,
    name: Option<String>,
    attributes: Option<Vec<OtlpKeyValue>>,
}

#[derive(Debug, Deserialize)]
struct OtlpStatus {
    code: Option<i32>,
    message: Option<String>,
}

/// Convert OTLP attributes to JSON value
fn attributes_to_json(attrs: &Option<Vec<OtlpKeyValue>>) -> serde_json::Value {
    match attrs {
        None => serde_json::json!({}),
        Some(attrs) => {
            let mut map = serde_json::Map::new();
            for attr in attrs {
                if let Some(ref value) = attr.value {
                    let json_value = if let Some(ref s) = value.string_value {
                        serde_json::Value::String(s.clone())
                    } else if let Some(ref i) = value.int_value {
                        // int_value comes as string in JSON format
                        match i.parse::<i64>() {
                            Ok(n) => serde_json::Value::Number(n.into()),
                            Err(_) => serde_json::Value::String(i.clone()),
                        }
                    } else if let Some(d) = value.double_value {
                        serde_json::Number::from_f64(d)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    } else if let Some(b) = value.bool_value {
                        serde_json::Value::Bool(b)
                    } else {
                        serde_json::Value::Null
                    };
                    map.insert(attr.key.clone(), json_value);
                }
            }
            serde_json::Value::Object(map)
        }
    }
}

/// Convert OTLP events to JSON value
fn events_to_json(events: &Option<Vec<OtlpEvent>>) -> serde_json::Value {
    match events {
        None => serde_json::json!([]),
        Some(events) => {
            let arr: Vec<serde_json::Value> = events
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "attributes": attributes_to_json(&e.attributes)
                    })
                })
                .collect();
            serde_json::Value::Array(arr)
        }
    }
}

/// Convert base64-encoded bytes to hex string
fn base64_to_hex(b64: &str) -> String {
    use base64::Engine;
    match base64::engine::general_purpose::STANDARD.decode(b64) {
        Ok(bytes) => bytes_to_hex(&bytes),
        Err(_) => b64.to_string(), // Return as-is if not valid base64
    }
}

/// Convert bytes to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse OTLP JSON request and extract spans
pub fn parse_otlp_traces_json(
    body: &[u8],
    job_id: &Uuid,
    workspace_id: &str,
) -> anyhow::Result<Vec<OtelSpan>> {
    let request: OtlpExportTraceServiceRequest = serde_json::from_slice(body)?;

    let mut spans = Vec::new();

    if let Some(resource_spans) = request.resource_spans {
        for rs in resource_spans {
            // Extract service name from resource attributes
            let service_name = rs
                .resource
                .as_ref()
                .and_then(|r| r.attributes.as_ref())
                .and_then(|attrs| {
                    attrs.iter().find(|kv| kv.key == "service.name").and_then(|kv| {
                        kv.value.as_ref().and_then(|v| v.string_value.clone())
                    })
                });

            if let Some(scope_spans) = rs.scope_spans {
                for ss in scope_spans {
                    if let Some(otlp_spans) = ss.spans {
                        for span in otlp_spans {
                            // Parse trace_id and span_id (base64 encoded in JSON format)
                            let trace_id = span
                                .trace_id
                                .map(|id| base64_to_hex(&id))
                                .unwrap_or_default();
                            let span_id = span
                                .span_id
                                .map(|id| base64_to_hex(&id))
                                .unwrap_or_default();
                            let parent_span_id = span.parent_span_id.map(|id| base64_to_hex(&id));

                            let start_time: i64 = span
                                .start_time_unix_nano
                                .as_ref()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0);
                            let end_time: i64 = span
                                .end_time_unix_nano
                                .as_ref()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0);

                            let status_code = span
                                .status
                                .as_ref()
                                .and_then(|s| s.code)
                                .unwrap_or(0) as i16;
                            let status_message = span.status.as_ref().and_then(|s| s.message.clone());

                            spans.push(OtelSpan {
                                trace_id,
                                span_id,
                                parent_span_id,
                                operation_name: span.name.unwrap_or_else(|| "unknown".to_string()),
                                service_name: service_name.clone(),
                                start_time_unix_nano: start_time,
                                end_time_unix_nano: end_time,
                                status_code,
                                status_message,
                                attributes: attributes_to_json(&span.attributes),
                                events: events_to_json(&span.events),
                            });
                        }
                    }
                }
            }
        }
    }

    // Override with the actual job context
    for span in &mut spans {
        // Add job metadata to attributes if not present
        if let serde_json::Value::Object(ref mut map) = span.attributes {
            if !map.contains_key("windmill.job_id") {
                map.insert(
                    "windmill.job_id".to_string(),
                    serde_json::Value::String(job_id.to_string()),
                );
            }
            if !map.contains_key("windmill.workspace_id") {
                map.insert(
                    "windmill.workspace_id".to_string(),
                    serde_json::Value::String(workspace_id.to_string()),
                );
            }
        }
    }

    Ok(spans)
}

// ============================================================================
// Built-in HTTP Collector Server
// ============================================================================

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};

/// Application state for the collector server
#[derive(Clone)]
struct CollectorState {
    db: Pool<Postgres>,
}

/// Handle OTLP traces endpoint
async fn handle_traces(
    State(state): State<CollectorState>,
    Path((workspace_id, job_id)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    let job_uuid = match Uuid::parse_str(&job_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid job ID");
        }
    };

    match parse_otlp_traces_json(&body, &job_uuid, &workspace_id) {
        Ok(spans) => {
            if let Err(e) = store_otel_spans(&state.db, &job_uuid, &workspace_id, spans).await {
                tracing::error!("Failed to store OTel spans: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to store spans");
            }
            (StatusCode::OK, "")
        }
        Err(e) => {
            tracing::error!("Failed to parse OTLP traces: {}", e);
            (StatusCode::BAD_REQUEST, "Failed to parse traces")
        }
    }
}

/// Handle generic OTLP traces endpoint (extracts job info from body)
async fn handle_traces_generic(State(state): State<CollectorState>, body: Bytes) -> impl IntoResponse {
    // Try to extract job_id and workspace_id from the request body's resource attributes
    let request: Result<OtlpExportTraceServiceRequest, _> = serde_json::from_slice(&body);

    let (job_id, workspace_id) = match &request {
        Ok(req) => {
            let mut job_id = None;
            let mut workspace_id = None;

            if let Some(ref resource_spans) = req.resource_spans {
                for rs in resource_spans {
                    if let Some(ref resource) = rs.resource {
                        if let Some(ref attrs) = resource.attributes {
                            for attr in attrs {
                                if attr.key == "windmill.job_id" {
                                    job_id = attr
                                        .value
                                        .as_ref()
                                        .and_then(|v| v.string_value.clone());
                                } else if attr.key == "windmill.workspace_id" {
                                    workspace_id = attr
                                        .value
                                        .as_ref()
                                        .and_then(|v| v.string_value.clone());
                                }
                            }
                        }
                    }
                }
            }
            (job_id, workspace_id)
        }
        Err(_) => (None, None),
    };

    match (job_id, workspace_id) {
        (Some(job_id_str), Some(ws_id)) => {
            let job_uuid = match Uuid::parse_str(&job_id_str) {
                Ok(id) => id,
                Err(_) => {
                    return (StatusCode::BAD_REQUEST, "Invalid job ID in resource attributes");
                }
            };

            match parse_otlp_traces_json(&body, &job_uuid, &ws_id) {
                Ok(spans) => {
                    if let Err(e) = store_otel_spans(&state.db, &job_uuid, &ws_id, spans).await {
                        tracing::error!("Failed to store OTel spans: {}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to store spans");
                    }
                    (StatusCode::OK, "")
                }
                Err(e) => {
                    tracing::error!("Failed to parse OTLP traces: {}", e);
                    (StatusCode::BAD_REQUEST, "Failed to parse traces")
                }
            }
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "Missing windmill.job_id or windmill.workspace_id in resource attributes",
        ),
    }
}

/// Start the built-in OTel collector HTTP server
pub async fn start_otel_collector_server(
    db: Pool<Postgres>,
    config: OtelAutoInstrumentationConfig,
) -> anyhow::Result<()> {
    if !config.enabled {
        tracing::info!("OTel auto-instrumentation is disabled, not starting collector server");
        return Ok(());
    }

    let state = CollectorState { db };

    let app = Router::new()
        // Path-based endpoint for explicit job context
        .route("/v1/traces/:workspace_id/:job_id", post(handle_traces))
        // Generic endpoint that extracts job info from resource attributes
        .route("/v1/traces", post(handle_traces_generic))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", config.collector_port);
    tracing::info!("Starting OTel collector server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("OTel collector server error: {}", e))?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_hex() {
        assert_eq!(bytes_to_hex(&[0x12, 0x34, 0xab, 0xcd]), "1234abcd");
        assert_eq!(bytes_to_hex(&[]), "");
    }

    #[test]
    fn test_base64_to_hex() {
        // "EjSrzQ==" is base64 for [0x12, 0x34, 0xab, 0xcd]
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &[0x12, 0x34, 0xab, 0xcd],
        );
        assert_eq!(base64_to_hex(&b64), "1234abcd");
    }

    #[test]
    fn test_attributes_to_json() {
        let attrs = vec![
            OtlpKeyValue {
                key: "string_attr".to_string(),
                value: Some(OtlpAnyValue {
                    string_value: Some("hello".to_string()),
                    int_value: None,
                    double_value: None,
                    bool_value: None,
                    array_value: None,
                    kvlist_value: None,
                }),
            },
            OtlpKeyValue {
                key: "int_attr".to_string(),
                value: Some(OtlpAnyValue {
                    string_value: None,
                    int_value: Some("42".to_string()),
                    double_value: None,
                    bool_value: None,
                    array_value: None,
                    kvlist_value: None,
                }),
            },
        ];

        let json = attributes_to_json(&Some(attrs));
        assert_eq!(json["string_attr"], "hello");
        assert_eq!(json["int_attr"], 42);
    }

    #[test]
    fn test_parse_otlp_traces_json() {
        let json_body = r#"{
            "resourceSpans": [{
                "resource": {
                    "attributes": [
                        {"key": "service.name", "value": {"stringValue": "test-service"}}
                    ]
                },
                "scopeSpans": [{
                    "scope": {"name": "test-scope"},
                    "spans": [{
                        "traceId": "EjSrzQ==",
                        "spanId": "VGWN0g==",
                        "name": "test-span",
                        "startTimeUnixNano": "1704067200000000000",
                        "endTimeUnixNano": "1704067201000000000",
                        "attributes": [
                            {"key": "http.method", "value": {"stringValue": "GET"}}
                        ],
                        "status": {"code": 1}
                    }]
                }]
            }]
        }"#;

        let job_id = Uuid::new_v4();
        let workspace_id = "test-workspace";

        let spans = parse_otlp_traces_json(json_body.as_bytes(), &job_id, workspace_id).unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].operation_name, "test-span");
        assert_eq!(spans[0].service_name, Some("test-service".to_string()));
        assert_eq!(spans[0].status_code, 1);
    }

    #[test]
    fn test_get_otel_typescript_env_vars() {
        let job_id = Uuid::new_v4();
        let workspace_id = "test-ws";
        let script_path = "u/user/test_script";
        let config = OtelAutoInstrumentationConfig {
            enabled: true,
            python_enabled: true,
            typescript_enabled: true,
            collector_port: 4318,
        };

        let env_vars = get_otel_typescript_env_vars(&job_id, workspace_id, script_path, &config);
        assert!(!env_vars.is_empty());

        let env_map: std::collections::HashMap<_, _> = env_vars.into_iter().collect();
        assert_eq!(env_map.get("WINDMILL_OTEL_AUTO_INSTRUMENTATION"), Some(&"true".to_string()));
        assert_eq!(env_map.get("WINDMILL_JOB_ID"), Some(&job_id.to_string()));
        assert!(env_map.get("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_some());
    }

    #[test]
    fn test_get_otel_typescript_env_vars_disabled() {
        let job_id = Uuid::new_v4();
        let workspace_id = "test-ws";
        let script_path = "u/user/test_script";
        let config = OtelAutoInstrumentationConfig {
            enabled: true,
            python_enabled: true,
            typescript_enabled: false, // Disabled for TypeScript
            collector_port: 4318,
        };

        let env_vars = get_otel_typescript_env_vars(&job_id, workspace_id, script_path, &config);
        assert!(env_vars.is_empty());
    }

    #[test]
    fn test_json_attributes_to_value() {
        let attrs = Some(vec![
            OtlpKeyValue {
                key: "bool_val".to_string(),
                value: Some(OtlpAnyValue {
                    string_value: None,
                    int_value: None,
                    double_value: None,
                    bool_value: Some(true),
                    array_value: None,
                    kvlist_value: None,
                }),
            },
            OtlpKeyValue {
                key: "double_val".to_string(),
                value: Some(OtlpAnyValue {
                    string_value: None,
                    int_value: None,
                    double_value: Some(3.14),
                    bool_value: None,
                    array_value: None,
                    kvlist_value: None,
                }),
            },
        ]);

        let json = attributes_to_json(&attrs);
        assert_eq!(json["bool_val"], true);
        assert!((json["double_val"].as_f64().unwrap() - 3.14).abs() < 0.001);
    }
}
