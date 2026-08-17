//! Regression test for the deno_telemetry 0.31 nativets-fetch panic.
//!
//! Setup: when EE's `load_internal_otel_exporter` runs (HTTP tracing
//! enabled), it calls `deno_telemetry::init` and then flips
//! `DENO_OTEL_INITIALIZED=true` so `js_eval` will run
//! `globalThis.__bootstrapOtel()` inside the nativets JsRuntime. That
//! bootstrap unconditionally sets JS-side `TRACING_ENABLED=true`, which
//! makes `deno_fetch`'s `fetch()` call `builtinTracer().startSpan(...)`
//! — and `OtelTracer::builtin()` does `OTEL_GLOBALS.get().unwrap()`.
//!
//! In deno_telemetry 0.31, `init` was given an early-return guard: if
//! `tracing_enabled`, `metrics_enabled`, and `console` are all
//! off/Ignore, it returns `Ok(())` *without populating OTEL_GLOBALS*.
//! v1.700.0 was passing `OtelConfig::default()` (all off) — so the JS
//! bootstrap proceeded but the Rust-side OnceCell was empty, and the
//! first `fetch()` panicked the tokio worker in a context that cannot
//! unwind. Fixed in v1.702.0 by passing `tracing_enabled: true` +
//! `console: Capture` from `load_internal_otel_exporter` (PRs #573 EE
//! / #9163 OSS), matching the JS bootstrap shape `[1, 0, 1, 0]`.
//!
//! This test pins that contract directly against `deno_telemetry::
//! init` — the function the EE call site invokes — so the next time
//! the dep is bumped and someone is tempted to "simplify" the config
//! back to `OtelConfig::default()`, CI catches it.
//!
//! Lives in `tests/` (not `src/`) so it gets its own test binary and
//! its own process — `OTEL_GLOBALS` is a `OnceCell`, so sharing it
//! with the smoke tests in `src/smoke_tests.rs` would race.

use windmill_runtime_nativets::deno_telemetry;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ee_otel_init_config_populates_otel_globals() {
    // deno_telemetry::init builds an HttpExporter (uses rustls) — it
    // needs a process-wide CryptoProvider, which the real binary
    // installs in setup_deno_runtime / main. Mirror that here.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Pre-condition: nothing else in this test binary has touched the
    // OnceCell, so it must be empty.
    assert!(
        deno_telemetry::OTEL_GLOBALS.get().is_none(),
        "OTEL_GLOBALS must start empty in a fresh test process"
    );

    // Step 1: reproduce the footgun. `OtelConfig::default()` has
    // tracing/metrics off and `console = Ignore`, which trips the
    // 0.31 early-return — Ok(()) but OnceCell stays empty.
    deno_telemetry::init(
        deno_telemetry::OtelRuntimeConfig {
            runtime_name: "windmill-nativets-test".into(),
            runtime_version: "0".into(),
        },
        deno_telemetry::OtelConfig::default(),
    )
    .expect("init with default config returns Ok (early-return)");

    assert!(
        deno_telemetry::OTEL_GLOBALS.get().is_none(),
        "deno_telemetry 0.31 contract: init with all-disabled config \
         must NOT populate OTEL_GLOBALS — if this changes on a future \
         bump, load_internal_otel_exporter can drop the explicit \
         tracing_enabled/console fields."
    );

    // Step 2: re-call init with the exact config shape that the EE
    // `load_internal_otel_exporter` ships — this is what production
    // hits, so the test exercises the actual prod call path.
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
    .expect("init with tracing_enabled config must succeed on a fresh OnceCell");

    assert!(
        deno_telemetry::OTEL_GLOBALS.get().is_some(),
        "load_internal_otel_exporter's OtelConfig must populate \
         OTEL_GLOBALS so OtelTracer::builtin() does not panic on \
         .unwrap() once __bootstrapOtel flips JS-side TRACING_ENABLED"
    );
}
