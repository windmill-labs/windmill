//! End-to-end coverage for the EE HTTP-tracing path on nativets.
//!
//! Pairs with `otel_init.rs` (which pins only the `OTEL_GLOBALS`
//! population contract). This test exercises the full chain that
//! actually delivers a span to a collector:
//!
//!   `deno_telemetry::init` with EE config (Rust)
//!     → `globalThis.__bootstrapOtel()` (JS, flips TRACING_ENABLED)
//!     → user `fetch()` → deno_fetch's `builtinTracer().startSpan`
//!     → `BatchSpanProcessor` → `HttpExporter` (OTLP/HTTP-binary)
//!     → our mock OTLP listener captures the request bytes
//!
//! Without the v1.702.0 fix (#573 EE / #9163 OSS), the third arrow
//! panics in a tokio worker. With the fix in place, the span is
//! emitted and shows up at the listener — which is what the customer
//! is paying for when they enable HTTP tracing on nativets.
//!
//! `#[ignore]`'d: spins a V8 isolate (~seconds) and binds two TCP
//! listeners. Run with `cargo test -p windmill-runtime-nativets
//! --test otel_e2e -- --ignored`.
//!
//! Owns its own test binary so `OTEL_GLOBALS`'s `OnceCell` doesn't
//! race with `otel_init.rs`.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use windmill_runtime_nativets::{deno_telemetry, transpile_ts, NativeAnnotation, PrewarmedIsolate};

/// Bind 127.0.0.1:0 and spawn an accept loop. Each connection is
/// read until idle/EOF, the body is appended to `captured`, then we
/// respond with HTTP 200. Returns the bound port.
async fn spawn_capturing_http(captured: Arc<Mutex<Vec<Vec<u8>>>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let captured = captured.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 8192];
                let _ = tokio::time::timeout(Duration::from_millis(300), async {
                    loop {
                        match sock.read(&mut tmp).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => buf.extend_from_slice(&tmp[..n]),
                        }
                    }
                })
                .await;
                let _ = sock
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await;
                captured.lock().await.push(buf);
            });
        }
    });
    port
}

/// Initialize `deno_telemetry` with the exact `OtelConfig` shape that
/// the EE `load_internal_otel_exporter` ships in production. Keeps
/// this test in lockstep with the actual call site: if production
/// drifts away from `tracing_enabled + Capture`, this test breaks
/// before the customer-facing panic does.
fn init_with_ee_otel_config() {
    deno_telemetry::init(
        deno_telemetry::OtelRuntimeConfig {
            runtime_name: "windmill-nativets".into(),
            runtime_version: "0".into(),
        },
        deno_telemetry::OtelConfig {
            tracing_enabled: true,
            console: deno_telemetry::OtelConsoleConfig::Capture,
            ..Default::default()
        },
    )
    .expect("deno_telemetry init failed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "spins V8 + tcp listeners; run with --ignored"]
async fn fetch_after_init_otel_emits_span_to_collector() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    // 1. Stand up two listeners: one that pretends to be the user's
    //    fetch target, one that pretends to be the OTLP collector.
    let fetch_hits: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
    let otlp_hits: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
    let fetch_port = spawn_capturing_http(fetch_hits.clone()).await;
    let otlp_port = spawn_capturing_http(otlp_hits.clone()).await;

    // 2. Point the deno_telemetry exporter at the mock collector and
    //    initialize. Mirrors `load_internal_otel_exporter` in EE.
    //
    // SAFETY: test runs in its own process binary; no other thread
    // reads OTEL_EXPORTER_OTLP_ENDPOINT before init returns.
    unsafe {
        std::env::set_var(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            format!("http://127.0.0.1:{otlp_port}"),
        );
    }
    init_with_ee_otel_config();
    assert!(
        deno_telemetry::OTEL_GLOBALS.get().is_some(),
        "OTEL_GLOBALS missing — load_internal_otel_exporter's config regressed?"
    );

    // 3. Run user TS that bootstraps OTel and then issues a fetch
    //    at the mock target. `__bootstrapOtel` is fire-and-forget
    //    (resolves a dynamic import on the microtask queue); the
    //    `setTimeout` loop yields a few times so the import resolves
    //    and the `TRACING_ENABLED` flag is set before fetch runs.
    let ts = format!(
        r#"
declare const globalThis: any;
export async function main(): Promise<number> {{
    globalThis.__bootstrapOtel();
    // Yield multiple microtask + timer turns so the dynamic import
    // in __bootstrapOtel resolves and TRACING_ENABLED flips before
    // fetch runs (otherwise deno_fetch skips the span entirely).
    for (let i = 0; i < 5; i++) {{
        await new Promise<void>(r => setTimeout(r, 10));
    }}
    const resp = await fetch("http://127.0.0.1:{fetch_port}/probe");
    return resp.status;
}}
"#
    );
    let js = transpile_ts(ts).expect("transpile failed");
    let ann = NativeAnnotation { useragent: None, proxy: None };

    let mut iso = PrewarmedIsolate::spawn(String::new(), js, ann, vec![], None);
    iso.wait_ready().await.expect("isolate failed to pre-warm");
    let res = iso
        .start_execution("{}".to_string())
        .wait()
        .await
        .expect("isolate panicked");

    // The exact bug: pre-fix, fetch panics this isolate. Post-fix,
    // we get back the mock target's 200.
    let raw = res.result.expect("user script returned an error");
    assert_eq!(raw.get(), "200", "fetch should return mock target status");

    assert_eq!(
        fetch_hits.lock().await.len(),
        1,
        "fetch target should have been hit exactly once"
    );

    // 4. Force the BatchSpanProcessor to flush so the exporter posts
    //    to our mock collector synchronously (default flush interval
    //    is ~5s; tests can't wait that long).
    deno_telemetry::flush();
    // Exporter is async over the OTel runtime; give it a beat to
    // actually send the HTTP request.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let otlp_captured = otlp_hits.lock().await;
    assert!(
        !otlp_captured.is_empty(),
        "OTLP collector should have received at least one export — \
         spans aren't reaching the collector after init"
    );

    // Verify the export carries our fetch span. OTLP is protobuf, so
    // grep the raw bytes for OTel HTTP semantic-convention markers
    // that deno_fetch's auto-instrumentation attaches:
    //   - the target URL ("url.full" attribute)
    //   - "http.request.method" attribute
    let combined: Vec<u8> = otlp_captured.iter().flatten().copied().collect();
    let bytes_contain = |needle: &[u8]| combined.windows(needle.len()).any(|w| w == needle);

    let url_marker = format!("http://127.0.0.1:{fetch_port}/probe");
    let has_url = bytes_contain(url_marker.as_bytes());
    let has_method_attr = bytes_contain(b"http.request.method");

    if !(has_url && has_method_attr) {
        eprintln!(
            "OTLP bytes ({}): {:?}",
            combined.len(),
            String::from_utf8_lossy(&combined)
        );
    }

    assert!(
        has_url,
        "exported OTLP body should reference the fetched URL ({}); got {} bytes",
        url_marker,
        combined.len()
    );
    assert!(
        has_method_attr,
        "exported OTLP body should carry HTTP semconv attributes (http.request.method); got {} bytes",
        combined.len()
    );
}
