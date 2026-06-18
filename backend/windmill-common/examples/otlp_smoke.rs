//! Manual smoke test for the LogContext → OTLP bridge pipeline.
//!
//! Run a local OTEL collector with:
//!   docker run -d --rm --name wm-otelcol --network host \
//!     -v /tmp/otel-verify/otelcol-config.yaml:/etc/otelcol-contrib/config.yaml:ro \
//!     -v /tmp/otel-verify:/out \
//!     otel/opentelemetry-collector-contrib:latest
//!
//! Then run this binary:
//!   OTEL_LOGS=1 OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
//!     cargo run --example otlp_smoke --features enterprise,private,otel \
//!     -p windmill-common
//!
//! Inspect the collector's debug output with:
//!   docker logs wm-otelcol
//! Or the file sink:
//!   cat /tmp/otel-verify/logs.json

use windmill_common::log_context::{update_log_context, with_log_context, LogContext};
use windmill_common::utils::Mode;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let (_guard, _meter_provider) =
        windmill_common::tracing_init::initialize_tracing("otlp-smoke-host", &Mode::Server, "dev");

    // Simulate an HTTP request: middleware seeds method/uri, then the auth
    // chain adds email/username/workspace_id.
    let ctx = LogContext {
        method: Some("POST".into()),
        uri: Some("/api/w/acme/jobs/run/f/foo".into()),
        trace_id: Some("smoke-trace-id-abc-123".into()),
        ..Default::default()
    };

    with_log_context(ctx, async {
        // Simulate auth middleware running:
        update_log_context(|c| LogContext {
            email: Some("alice@acme.co".into()),
            username: Some("alice".into()),
            workspace_id: Some("acme".into()),
            ..c.clone()
        });

        // This mimics MyOnResponse::on_response for a 500 error:
        tracing::error!(status = 500u16, latency = 123u64, "response");

        // And an ad-hoc handler error:
        tracing::error!("database connection refused");
    })
    .await;

    // Simulate a worker job context:
    let job_ctx = LogContext {
        workspace_id: Some("acme".into()),
        job_id: Some("00000000-0000-4000-8000-000000000001".into()),
        script_path: Some("f/ingest/run".into()),
        job_kind: Some("script".into()),
        language: Some("python3".into()),
        worker: Some("wk-default-smoke".into()),
        hostname: Some("otlp-smoke-host".into()),
        tag: Some("deno".into()),
        ..Default::default()
    };
    with_log_context(job_ctx, async {
        tracing::error!("job failed: ValueError: bad input");
    })
    .await;

    // And one event outside any scope to prove it still emits (sans fields):
    tracing::error!("background task failure (no context)");

    // Give the batch exporter time to flush before we exit.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    eprintln!("smoke emitted; check `docker logs wm-otelcol` for the debug exporter output");
}
