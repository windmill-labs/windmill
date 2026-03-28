//! E2E tests for OpenTelemetry integration.
//!
//! Verify that metrics are recorded with correct names/values/attributes and
//! spans are created with correct trace IDs, attributes, and status codes.
//!
//! Run with: cargo test --features enterprise,private,otel --test otel -- --test-threads=1

#![cfg(all(feature = "otel", feature = "enterprise"))]

use std::sync::{atomic::Ordering, Arc};

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{
    metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider},
    trace::{InMemorySpanExporter, SdkTracerProvider, SimpleSpanProcessor},
};
use windmill_common::otel_ee::*;
use windmill_common::{OTEL_METRICS_ENABLED, OTEL_TRACING_ENABLED};

// ── Global test infrastructure ──────────────────────────────────────────

struct OtelTestState {
    metric_exporter: InMemoryMetricExporter,
    span_exporter: InMemorySpanExporter,
    meter_provider: SdkMeterProvider,
}

static STATE: tokio::sync::OnceCell<Arc<OtelTestState>> = tokio::sync::OnceCell::const_new();

async fn ensure_setup() -> Arc<OtelTestState> {
    STATE
        .get_or_init(|| async {
            // Metrics: InMemoryMetricExporter + PeriodicReader (needs async tokio context)
            let metric_exporter = InMemoryMetricExporter::default();
            let reader = PeriodicReader::builder(metric_exporter.clone()).build();
            let meter_provider = SdkMeterProvider::builder().with_reader(reader).build();
            global::set_meter_provider(meter_provider.clone());
            OTEL_METRICS_ENABLED.store(true, Ordering::SeqCst);

            // Tracing: InMemorySpanExporter + SimpleSpanProcessor
            let span_exporter = InMemorySpanExporter::default();
            let tracer_provider = SdkTracerProvider::builder()
                .with_span_processor(SimpleSpanProcessor::new(span_exporter.clone()))
                .build();
            let tracer = tracer_provider.tracer("windmill");
            *TRACER.write().unwrap() = Some(tracer);
            OTEL_TRACING_ENABLED.store(true, Ordering::SeqCst);

            Arc::new(OtelTestState { metric_exporter, span_exporter, meter_provider })
        })
        .await
        .clone()
}

// ── Metric helper: flush + collect ──────────────────────────────────────

fn flush_and_get_metrics(
    state: &OtelTestState,
) -> Vec<opentelemetry_sdk::metrics::data::ResourceMetrics> {
    state.meter_provider.force_flush().expect("flush failed");
    state
        .metric_exporter
        .get_finished_metrics()
        .expect("get_finished_metrics failed")
}

fn find_metric<'a>(
    all: &'a [opentelemetry_sdk::metrics::data::ResourceMetrics],
    name: &str,
) -> Option<&'a opentelemetry_sdk::metrics::data::Metric> {
    all.iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .find(|m| m.name() == name)
}

fn metric_names(all: &[opentelemetry_sdk::metrics::data::ResourceMetrics]) -> Vec<String> {
    all.iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .map(|m| m.name().to_string())
        .collect()
}

// ── Counter value helpers ───────────────────────────────────────────────

fn sum_u64_value(metric: &opentelemetry_sdk::metrics::data::Metric) -> Option<u64> {
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    match metric.data() {
        AggregatedMetrics::U64(MetricData::Sum(sum)) => {
            Some(sum.data_points().map(|dp| dp.value()).sum())
        }
        _ => None,
    }
}

fn gauge_i64_values(
    metric: &opentelemetry_sdk::metrics::data::Metric,
) -> Vec<(Vec<opentelemetry::KeyValue>, i64)> {
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    match metric.data() {
        AggregatedMetrics::I64(MetricData::Gauge(gauge)) => gauge
            .data_points()
            .map(|dp| (dp.attributes().cloned().collect(), dp.value()))
            .collect(),
        _ => panic!("expected I64 Gauge metric"),
    }
}

fn gauge_f64_value(metric: &opentelemetry_sdk::metrics::data::Metric) -> Option<f64> {
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    match metric.data() {
        AggregatedMetrics::F64(MetricData::Gauge(gauge)) => {
            gauge.data_points().next().map(|dp| dp.value())
        }
        _ => None,
    }
}

fn histogram_f64_count(metric: &opentelemetry_sdk::metrics::data::Metric) -> Option<u64> {
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    match metric.data() {
        AggregatedMetrics::F64(MetricData::Histogram(hist)) => {
            Some(hist.data_points().map(|dp| dp.count()).sum())
        }
        _ => None,
    }
}

fn histogram_f64_sum(metric: &opentelemetry_sdk::metrics::data::Metric) -> Option<f64> {
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    match metric.data() {
        AggregatedMetrics::F64(MetricData::Histogram(hist)) => {
            Some(hist.data_points().map(|dp| dp.sum()).sum())
        }
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// METRICS E2E TEST
//
// All metric assertions live in one test function because the PeriodicReader's
// background task is tied to the tokio runtime that created it. Separate
// #[tokio::test] functions each get their own runtime, and the reader becomes
// disconnected after the first test's runtime is dropped.
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_all_metrics_e2e() {
    let state = ensure_setup().await;

    // ── Counters ────────────────────────────────────────────────────

    otel_incr_queue_push_count();
    otel_incr_queue_push_count();
    otel_incr_queue_push_count();
    otel_incr_queue_delete_count();
    otel_incr_queue_pull_count();
    otel_incr_zombie_restart_count(7);
    otel_incr_zombie_delete_count(3);
    otel_incr_worker_execution_count("bun");
    otel_incr_worker_execution_count("bun");
    otel_incr_worker_execution_failed("go");
    otel_incr_worker_started();

    // ── Gauges ──────────────────────────────────────────────────────

    otel_set_queue_count("python3", 42);
    otel_set_queue_running_count("deno", 5);
    otel_set_worker_busy("worker-test-1", 1);
    otel_set_db_pool(5, 10, 20);
    otel_set_health_db_latency(2.5);
    otel_set_worker_uptime("w-uptime", 3600.0);
    otel_set_health_status_phase("healthy");
    otel_set_health_db_unresponsive(true);

    // ── Histograms ──────────────────────────────────────────────────

    otel_record_worker_execution_duration("python3", 1.5);
    otel_record_worker_execution_duration("python3", 2.5);
    otel_record_worker_pull_duration("w1", true, 0.05);
    otel_record_worker_pull_duration("w1", false, 0.01);

    // ── Flush and collect ───────────────────────────────────────────

    let metrics = flush_and_get_metrics(&state);
    let names = metric_names(&metrics);

    // ── Verify all 20 metric names are present ──────────────────────

    let expected = [
        "windmill.queue.push_count",
        "windmill.queue.delete_count",
        "windmill.queue.pull_count",
        "windmill.queue.zombie_restart_count",
        "windmill.queue.zombie_delete_count",
        "windmill.queue.count",
        "windmill.queue.running_count",
        "windmill.worker.execution_count",
        "windmill.worker.execution_duration",
        "windmill.worker.busy",
        "windmill.worker.pull_duration",
        "windmill.worker.execution_failed",
        "windmill.db.pool.active",
        "windmill.db.pool.idle",
        "windmill.db.pool.max",
        "windmill.health.db_latency",
        "windmill.worker.started",
        "windmill.worker.uptime",
        "windmill.health.status",
        "windmill.health.db_unresponsive",
    ];
    for name in expected {
        assert!(
            names.iter().any(|n| n == name),
            "metric '{}' not found in {:?}",
            name,
            names
        );
    }

    // ── Counter values ──────────────────────────────────────────────

    let m = find_metric(&metrics, "windmill.queue.push_count").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 3, "push_count should be >= 3");

    let m = find_metric(&metrics, "windmill.queue.delete_count").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 1);

    let m = find_metric(&metrics, "windmill.queue.pull_count").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 1);

    let m = find_metric(&metrics, "windmill.queue.zombie_restart_count").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 7);

    let m = find_metric(&metrics, "windmill.queue.zombie_delete_count").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 3);

    let m = find_metric(&metrics, "windmill.worker.execution_count").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 2);

    let m = find_metric(&metrics, "windmill.worker.execution_failed").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 1);

    let m = find_metric(&metrics, "windmill.worker.started").unwrap();
    assert!(sum_u64_value(m).unwrap() >= 1);

    // ── Gauge values ────────────────────────────────────────────────

    let m = find_metric(&metrics, "windmill.queue.count").unwrap();
    let values = gauge_i64_values(m);
    let dp = values
        .iter()
        .find(|(attrs, _)| {
            attrs
                .iter()
                .any(|kv| kv.key.as_str() == "tag" && kv.value.as_str() == "python3")
        })
        .expect("queue.count data point with tag=python3 not found");
    assert_eq!(dp.1, 42);

    let m = find_metric(&metrics, "windmill.queue.running_count").unwrap();
    let values = gauge_i64_values(m);
    let dp = values
        .iter()
        .find(|(attrs, _)| {
            attrs
                .iter()
                .any(|kv| kv.key.as_str() == "tag" && kv.value.as_str() == "deno")
        })
        .expect("running_count data point with tag=deno not found");
    assert_eq!(dp.1, 5);

    let m = find_metric(&metrics, "windmill.worker.busy").unwrap();
    let values = gauge_i64_values(m);
    let dp = values
        .iter()
        .find(|(attrs, _)| {
            attrs
                .iter()
                .any(|kv| kv.key.as_str() == "worker" && kv.value.as_str() == "worker-test-1")
        })
        .expect("worker.busy data point with worker=worker-test-1 not found");
    assert_eq!(dp.1, 1);

    let m = find_metric(&metrics, "windmill.db.pool.active").unwrap();
    assert_eq!(gauge_i64_values(m)[0].1, 5);
    let m = find_metric(&metrics, "windmill.db.pool.idle").unwrap();
    assert_eq!(gauge_i64_values(m)[0].1, 10);
    let m = find_metric(&metrics, "windmill.db.pool.max").unwrap();
    assert_eq!(gauge_i64_values(m)[0].1, 20);

    let m = find_metric(&metrics, "windmill.health.db_latency").unwrap();
    assert!((gauge_f64_value(m).unwrap() - 2.5).abs() < f64::EPSILON);

    let m = find_metric(&metrics, "windmill.worker.uptime").unwrap();
    assert!((gauge_f64_value(m).unwrap() - 3600.0).abs() < f64::EPSILON);

    let m = find_metric(&metrics, "windmill.health.db_unresponsive").unwrap();
    assert_eq!(gauge_i64_values(m)[0].1, 1);

    // ── Health status phase (all 3 phases) ──────────────────────────

    let m = find_metric(&metrics, "windmill.health.status").unwrap();
    let values = gauge_i64_values(m);
    let healthy = values
        .iter()
        .find(|(attrs, _)| {
            attrs
                .iter()
                .any(|kv| kv.key.as_str() == "phase" && kv.value.as_str() == "healthy")
        })
        .expect("phase=healthy");
    let degraded = values
        .iter()
        .find(|(attrs, _)| {
            attrs
                .iter()
                .any(|kv| kv.key.as_str() == "phase" && kv.value.as_str() == "degraded")
        })
        .expect("phase=degraded");
    let unhealthy = values
        .iter()
        .find(|(attrs, _)| {
            attrs
                .iter()
                .any(|kv| kv.key.as_str() == "phase" && kv.value.as_str() == "unhealthy")
        })
        .expect("phase=unhealthy");
    assert_eq!(healthy.1, 1);
    assert_eq!(degraded.1, 0);
    assert_eq!(unhealthy.1, 0);

    // ── Histogram values ────────────────────────────────────────────

    let m = find_metric(&metrics, "windmill.worker.execution_duration").unwrap();
    assert!(histogram_f64_count(m).unwrap() >= 2);
    assert!(histogram_f64_sum(m).unwrap() >= 4.0);

    let m = find_metric(&metrics, "windmill.worker.pull_duration").unwrap();
    assert!(histogram_f64_count(m).unwrap() >= 2);
}

// ═══════════════════════════════════════════════════════════════════════
// SPAN E2E TESTS
// ═══════════════════════════════════════════════════════════════════════

fn make_test_job(id: uuid::Uuid, parent: Option<uuid::Uuid>) -> windmill_queue::MiniPulledJob {
    use windmill_types::jobs::JobKind;
    let mut job = windmill_queue::MiniPulledJob::new_inline(
        "test-workspace".to_string(),
        None,
        "test-user".to_string(),
        "u/test-user".to_string(),
        "test@example.com".to_string(),
        Some("f/test/script".to_string()),
        JobKind::Script,
        None,
        "deno".to_string(),
        None,
    );
    job.id = id;
    job.parent_job = parent;
    job.started_at = Some(chrono::Utc::now());
    job
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_root_job_span_created_on_success() {
    let state = ensure_setup().await;
    state.span_exporter.reset();

    let job_id = uuid::Uuid::new_v4();
    let job = make_test_job(job_id, None);
    windmill_worker::otel_ee::add_root_flow_job_to_otlp(&job, true);

    let spans = state.span_exporter.get_finished_spans().unwrap();
    let span = spans
        .iter()
        .find(|s| s.name == "full_job")
        .expect("full_job span not found");

    assert_eq!(span.status, opentelemetry::trace::Status::Ok,);

    // Verify attributes
    let attrs: Vec<_> = span.attributes.iter().map(|kv| kv.key.as_str()).collect();
    assert!(attrs.contains(&"job_id"), "missing job_id attribute");
    assert!(
        attrs.contains(&"workspace_id"),
        "missing workspace_id attribute"
    );
    assert!(
        attrs.contains(&"script_path"),
        "missing script_path attribute"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_root_job_span_error_on_failure() {
    let state = ensure_setup().await;
    state.span_exporter.reset();

    let job_id = uuid::Uuid::new_v4();
    let job = make_test_job(job_id, None);
    windmill_worker::otel_ee::add_root_flow_job_to_otlp(&job, false);

    let spans = state.span_exporter.get_finished_spans().unwrap();
    let span = spans
        .iter()
        .find(|s| s.name == "full_job")
        .expect("full_job span not found");

    match &span.status {
        opentelemetry::trace::Status::Error { description } => {
            assert_eq!(description.as_ref(), "Job failed");
        }
        other => panic!("expected Error status, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_root_job_trace_id_matches_uuid() {
    let state = ensure_setup().await;
    state.span_exporter.reset();

    let job_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let job = make_test_job(job_id, None);
    windmill_worker::otel_ee::add_root_flow_job_to_otlp(&job, true);

    let spans = state.span_exporter.get_finished_spans().unwrap();
    let span = spans
        .iter()
        .find(|s| s.name == "full_job")
        .expect("full_job span not found");

    let expected_trace_id =
        opentelemetry::trace::TraceId::from_bytes(job_id.as_u128().to_be_bytes());
    assert_eq!(span.span_context.trace_id(), expected_trace_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_root_job_span_id_matches_uuid() {
    let state = ensure_setup().await;
    state.span_exporter.reset();

    let job_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let job = make_test_job(job_id, None);
    windmill_worker::otel_ee::add_root_flow_job_to_otlp(&job, true);

    let spans = state.span_exporter.get_finished_spans().unwrap();
    let span = spans
        .iter()
        .find(|s| s.name == "full_job")
        .expect("full_job span not found");

    let expected_span_id =
        opentelemetry::trace::SpanId::from_bytes(job_id.as_u64_pair().1.to_be_bytes());
    assert_eq!(span.span_context.span_id(), expected_span_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_child_job_produces_no_span() {
    let state = ensure_setup().await;
    state.span_exporter.reset();

    let parent_id = uuid::Uuid::new_v4();
    let job_id = uuid::Uuid::new_v4();
    let job = make_test_job(job_id, Some(parent_id));
    windmill_worker::otel_ee::add_root_flow_job_to_otlp(&job, true);

    let spans = state.span_exporter.get_finished_spans().unwrap();
    let found = spans.iter().any(|s| s.name == "full_job");
    assert!(!found, "child job should not produce a span");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_root_job_span_attributes_values() {
    let state = ensure_setup().await;
    state.span_exporter.reset();

    let job_id = uuid::Uuid::new_v4();
    let job = make_test_job(job_id, None);
    windmill_worker::otel_ee::add_root_flow_job_to_otlp(&job, true);

    let spans = state.span_exporter.get_finished_spans().unwrap();
    let span = spans
        .iter()
        .find(|s| s.name == "full_job")
        .expect("full_job span not found");

    let get_attr = |key: &str| -> String {
        span.attributes
            .iter()
            .find(|kv| kv.key.as_str() == key)
            .map(|kv| kv.value.as_str().to_string())
            .unwrap_or_default()
    };

    assert_eq!(get_attr("job_id"), job_id.to_string());
    assert_eq!(get_attr("workspace_id"), "test-workspace");
    assert_eq!(get_attr("script_path"), "f/test/script");
}
