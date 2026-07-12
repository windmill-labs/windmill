//! Opt-in smoke tests for the nativets V8 runtime.
//!
//! Exercise the deno_core / deno_ast / swc surface (TypeScript transpile,
//! fetch, timers, URL, structuredClone, error propagation, concurrent
//! isolates, large payload roundtrip) that the standard worker-level
//! nativets tests in `backend/tests/worker.rs` don't reach — those tests
//! validate value passing through the job queue, but not the JS API
//! surface a deno_core bump would actually move.
//!
//! These tests are `#[ignore]`'d so the regular `cargo test` flow doesn't
//! pay their cost (each spawns a V8 isolate; some hit the network). Run
//! when changing the `deno_core` / `deno_ast` / `deno_runtime` / `swc_*`
//! pins in `backend/Cargo.toml`:
//!
//!     cargo test -p windmill-runtime-nativets smoke -- --ignored
//!
//! Tests prefixed `smoke_net_` hit the public internet (httpbin.org,
//! example.com) and will fail if the runner has no egress. Skip them
//! locally with `cargo test -p windmill-runtime-nativets smoke -- --ignored --skip smoke_net_`.

use crate::{transpile_ts, NativeAnnotation, PrewarmedIsolate, PrewarmedResult};

/// Compile a TS snippet, run it through a fresh isolate with the given
/// positional args, and return the isolate's result + captured logs.
async fn run_ts(ts: &str, arg_names: &[&str], args: serde_json::Value) -> PrewarmedResult {
    let js = transpile_ts(ts.to_string()).expect("transpile_ts failed");
    let ann = NativeAnnotation { useragent: None, proxy: None };
    let arg_names: Vec<String> = arg_names.iter().map(|s| s.to_string()).collect();
    let mut iso = PrewarmedIsolate::spawn(String::new(), js, ann, arg_names, None);
    iso.wait_ready().await.expect("isolate failed to pre-warm");
    iso.start_execution(args.to_string())
        .wait()
        .await
        .expect("isolate execution panicked")
}

fn unwrap_value(r: &PrewarmedResult) -> serde_json::Value {
    let raw = r.result.as_ref().expect("script returned an error");
    serde_json::from_str(raw.get()).expect("result not valid JSON")
}

// -----------------------------------------------------------------------------
// Local (no network) — these still need V8 / deno_core ops to be wired.
// -----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_basic_value_passing() {
    let ts = r#"
export async function main(x: number): Promise<number> {
    return x + 1;
}
"#;
    let r = run_ts(ts, &["x"], serde_json::json!({"x": 41})).await;
    assert_eq!(unwrap_value(&r), serde_json::json!(42));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_transpile_enum_and_union() {
    // Enums + discriminated union + as-cast exercise the swc_ecma_ast +
    // swc_ecma_parser TS-syntax paths the bare value tests don't.
    let ts = r#"
enum Direction { Up = "U", Down = "D" }
type Msg = { kind: "move"; dir: Direction } | { kind: "stop" };
export async function main(): Promise<string> {
    const msgs: Msg[] = [
        { kind: "move", dir: Direction.Up },
        { kind: "stop" },
        { kind: "move", dir: Direction.Down },
    ];
    return msgs.map(m => m.kind === "move" ? m.dir : "_").join(",");
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    assert_eq!(unwrap_value(&r), serde_json::json!("U,_,D"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_set_timeout_and_promise_all() {
    // setTimeout lives in deno_web; Promise.all hits the V8 microtask
    // queue. A bump that breaks timer-op registration or microtask drain
    // would surface here (script would hang or return wrong order).
    let ts = r#"
export async function main(): Promise<number[]> {
    const delays = [40, 10, 20, 30];
    return await Promise.all(delays.map(d =>
        new Promise<number>(resolve => setTimeout(() => resolve(d), d))
    ));
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    // Promise.all preserves input order regardless of resolution order.
    assert_eq!(unwrap_value(&r), serde_json::json!([40, 10, 20, 30]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_url_and_searchparams() {
    // deno_url surface: URL ctor, URLSearchParams parsing + iteration.
    let ts = r#"
export async function main(): Promise<{ host: string; pairs: [string, string][] }> {
    const u = new URL("https://example.com:8443/path?b=2&a=1&a=3");
    const pairs: [string, string][] = [];
    for (const [k, v] of u.searchParams) pairs.push([k, v]);
    return { host: u.host, pairs };
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    assert_eq!(
        unwrap_value(&r),
        serde_json::json!({
            "host": "example.com:8443",
            "pairs": [["b", "2"], ["a", "1"], ["a", "3"]],
        }),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_web_blob_btoa_atob() {
    // deno_web surface: Blob, atob/btoa. `structuredClone` is *not* wired
    // into the nativets global (the deno_web binding doesn't expose it
    // here) — if that's ever changed, extend this test to cover it.
    let ts = r#"
export async function main(): Promise<{ b64: string; round_trip: string; size: number }> {
    const blob = new Blob(["hello"], { type: "text/plain" });
    const b64 = btoa("hello");
    const round_trip = atob(b64);
    return { b64, round_trip, size: blob.size };
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    assert_eq!(
        unwrap_value(&r),
        serde_json::json!({
            "b64": "aGVsbG8=",
            "round_trip": "hello",
            "size": 5,
        }),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_text_encoder_decoder() {
    // deno_web Encoding API: TextEncoder/TextDecoder must be exposed on the
    // global scope. Uses a multi-byte char to exercise real UTF-8 handling.
    let ts = r#"
export async function main(): Promise<{ bytes: number[]; round_trip: string; label: string }> {
    const bytes = new TextEncoder().encode("héllo");
    const dec = new TextDecoder();
    return { bytes: Array.from(bytes), round_trip: dec.decode(bytes), label: dec.encoding };
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    assert_eq!(
        unwrap_value(&r),
        serde_json::json!({
            // "héllo" -> h, é (0xC3 0xA9), l, l, o
            "bytes": [104, 195, 169, 108, 108, 111],
            "round_trip": "héllo",
            "label": "utf-8",
        }),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_web_globals_are_wired() {
    // Standard deno_web / deno_url globals that must be exposed on the global
    // scope. A wrong export name would leave one `undefined` and fail here.
    let ts = r#"
export async function main(): Promise<Record<string, string>> {
    const names = ["TextEncoder", "TextDecoder", "TextEncoderStream", "TextDecoderStream",
        "File", "Event", "EventTarget", "CustomEvent", "ReadableStream", "WritableStream",
        "TransformStream", "ByteLengthQueuingStrategy", "CountQueuingStrategy", "URLPattern",
        "structuredClone", "performance", "CompressionStream", "DecompressionStream",
        "MessageChannel", "MessagePort"];
    const out: Record<string, string> = {};
    for (const n of names) out[n] = typeof (globalThis as any)[n];
    return out;
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    let v = unwrap_value(&r);
    let obj = v.as_object().expect("object result");
    let missing: Vec<&str> = obj
        .iter()
        .filter(|(_, t)| t.as_str() == Some("undefined"))
        .map(|(k, _)| k.as_str())
        .collect();
    assert!(missing.is_empty(), "globals not wired (undefined): {missing:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_structured_clone_is_spec_version() {
    // structuredClone must be the spec global (from the message-port module),
    // which validates its arguments — calling it with none throws
    // "Failed to execute 'structuredClone'". The internal 02_structured_clone.js
    // helper takes a single positional value and would silently clone `undefined`.
    let ts = r#"
export async function main(): Promise<{ threw: boolean; is_spec_error: boolean; cloned: number }> {
    const cloned = (structuredClone({ n: 41 }) as { n: number }).n + 1;
    try {
        (structuredClone as unknown as () => unknown)();
        return { threw: false, is_spec_error: false, cloned };
    } catch (e) {
        return { threw: true, is_spec_error: String((e as Error).message).includes("structuredClone"), cloned };
    }
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    assert_eq!(
        unwrap_value(&r),
        serde_json::json!({ "threw": true, "is_spec_error": true, "cloned": 42 }),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_performance_now() {
    // `performance` is the singleton Performance instance; performance.now()
    // must actually work (it relies on the core op_now op being available).
    let ts = r#"
export async function main(): Promise<{ has_now: boolean; is_number: boolean; nonneg: boolean }> {
    const t = performance.now();
    return { has_now: typeof performance.now === "function", is_number: typeof t === "number", nonneg: t >= 0 };
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    assert_eq!(
        unwrap_value(&r),
        serde_json::json!({ "has_now": true, "is_number": true, "nonneg": true }),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_large_payload_roundtrip() {
    // ~512 KB string in and out — exercises arg encoding + result
    // serialization through the deno_core <-> host op boundary at sizes
    // an op-table change could break.
    let big_in: String = "a".repeat(512 * 1024);
    let ts = r#"
export async function main(s: string): Promise<{ in_len: number; out: string }> {
    if (typeof s !== "string") throw new Error(`expected string, got ${typeof s}`);
    return { in_len: s.length, out: "b".repeat(512 * 1024) };
}
"#;
    let r = run_ts(ts, &["s"], serde_json::json!({"s": big_in})).await;
    let v = unwrap_value(&r);
    assert_eq!(v.get("in_len").and_then(|x| x.as_u64()), Some(512 * 1024));
    let out_len = v
        .get("out")
        .and_then(|x| x.as_str())
        .map(|s| s.len())
        .unwrap_or(0);
    assert_eq!(out_len, 512 * 1024);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_error_propagation_with_message() {
    // Throwing a typed Error must surface as PrewarmedResult::Err with
    // the original message. A deno_core bump that changes the host-side
    // error wrapping would lose this contract.
    let ts = r#"
export async function main(): Promise<void> {
    throw new Error("nativets_smoke_marker_xyz");
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    let err = r.result.expect_err("expected script to fail");
    assert!(
        err.contains("nativets_smoke_marker_xyz"),
        "thrown error message did not reach result: {err}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "deno_core upgrade smoke; run with --ignored"]
async fn smoke_concurrent_isolates() {
    // Spawn N isolates in parallel from the same tokio runtime. Each
    // PrewarmedIsolate uses spawn_blocking + a fresh V8 isolate.
    // Catches isolate-setup races (V8_ISOLATE_CREATE_LOCK ordering) and
    // any per-isolate state that a deno_core bump could break under
    // concurrency.
    let ts = r#"
export async function main(i: number): Promise<number> {
    return i * 10;
}
"#;
    let js = transpile_ts(ts.to_string()).expect("transpile_ts failed");

    const N: i64 = 8;
    let mut handles = Vec::with_capacity(N as usize);
    for i in 0..N {
        let js = js.clone();
        let h = tokio::spawn(async move {
            let ann = NativeAnnotation { useragent: None, proxy: None };
            let mut iso =
                PrewarmedIsolate::spawn(String::new(), js, ann, vec!["i".to_string()], None);
            iso.wait_ready().await.expect("pre-warm failed");
            let res = iso
                .start_execution(serde_json::json!({"i": i}).to_string())
                .wait()
                .await
                .expect("isolate panicked");
            res.result.expect("script errored")
        });
        handles.push(h);
    }

    let mut got: Vec<i64> = Vec::with_capacity(N as usize);
    for h in handles {
        let raw = h.await.expect("join failed");
        let v: serde_json::Value = serde_json::from_str(raw.get()).expect("not JSON");
        got.push(v.as_i64().unwrap_or(-1));
    }
    got.sort();
    let expected: Vec<i64> = (0..N).map(|i| i * 10).collect();
    assert_eq!(got, expected);
}

// -----------------------------------------------------------------------------
// Network — actually exercise deno_fetch end-to-end. Skip in air-gapped CI
// with `--skip smoke_net_`.
// -----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke (network); run with --ignored"]
async fn smoke_net_fetch_example_com() {
    // example.com is one of the most stable hosts on the internet and
    // returns a tiny known-text body, so we can both assert "fetch works"
    // and "the response body parses correctly through deno_fetch".
    let ts = r#"
export async function main(): Promise<{ status: number; has_marker: boolean }> {
    const r = await fetch("https://example.com/");
    const body = await r.text();
    return { status: r.status, has_marker: body.includes("Example Domain") };
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    let v = unwrap_value(&r);
    assert_eq!(v.get("status").and_then(|x| x.as_u64()), Some(200));
    assert_eq!(v.get("has_marker"), Some(&serde_json::json!(true)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "deno_core upgrade smoke (network); run with --ignored"]
async fn smoke_net_fetch_json_and_headers() {
    // httpbin.org/anything echoes request metadata back as JSON, so we
    // can verify: deno_fetch sends custom headers, parses JSON response,
    // and propagates query params end-to-end.
    let ts = r#"
export async function main(): Promise<{ ua: string; arg: string }> {
    const r = await fetch("https://httpbin.org/anything?nativets=ok", {
        headers: { "x-windmill-smoke": "1" },
    });
    if (!r.ok) throw new Error(`status ${r.status}`);
    const j: any = await r.json();
    return {
        ua: j.headers["X-Windmill-Smoke"] ?? "",
        arg: j.args.nativets ?? "",
    };
}
"#;
    let r = run_ts(ts, &[], serde_json::json!({})).await;
    let v = unwrap_value(&r);
    assert_eq!(v.get("ua").and_then(|x| x.as_str()), Some("1"));
    assert_eq!(v.get("arg").and_then(|x| x.as_str()), Some("ok"));
}
