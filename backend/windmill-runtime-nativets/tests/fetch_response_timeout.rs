//! The nativets `fetch()` response timeout.
//!
//! Two halves of one contract, and a fix that satisfies only the first is worse
//! than no fix:
//!
//!   1. a peer that accepts a request and never answers is given up on
//!   2. a response that has begun arriving is never cut off, however long it
//!      takes in total
//!
//! (2) rules out the obvious implementation — `AbortSignal.timeout(N)` around
//! every fetch would satisfy (1) and break every streaming response and long
//! download.
//!
//! Hermetic: loopback listeners, no egress.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use windmill_runtime_nativets::{transpile_ts, NativeAnnotation, PrewarmedIsolate};

/// The annotation is in whole seconds, so the tests scale around this.
const TIMEOUT_SECS: u64 = 2;

async fn run_with_timeout_secs(ts: &str, secs: u64) -> Result<String, String> {
    let js = transpile_ts(ts.to_string()).expect("transpile_ts failed");
    let ann =
        NativeAnnotation { fetch_response_timeout_secs: Some(secs), ..NativeAnnotation::default() };
    let mut iso = PrewarmedIsolate::spawn(String::new(), js, ann, vec![], None);
    iso.wait_ready().await.expect("isolate failed to pre-warm");
    let res = iso
        .start_execution("{}".to_string())
        .wait()
        .await
        .expect("isolate panicked");
    res.result.map(|raw| raw.get().to_string())
}

/// A peer that reads the request and then answers nothing, closing only after
/// `close_after` so no test leaves an isolate wedged on a pending fetch.
///
/// The socket is held rather than dropped on accept: dropping it sends a FIN,
/// which surfaces as a connection error — the easy failure, not this one.
async fn spawn_silent_peer(seen: Arc<Mutex<Vec<u8>>>, close_after: Duration) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let seen = seen.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                if let Ok(n) = sock.read(&mut buf).await {
                    seen.lock().await.extend_from_slice(&buf[..n]);
                }
                tokio::time::sleep(close_after).await;
                drop(sock);
            });
        }
    });
    port
}

/// A peer that responds after `headers_after`, then dribbles a chunked body out
/// over `chunks * chunk_every`.
async fn spawn_streaming_peer(
    headers_after: Duration,
    chunks: usize,
    chunk_every: Duration,
) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf).await;
                tokio::time::sleep(headers_after).await;
                if sock
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\n\r\n",
                    )
                    .await
                    .is_err()
                {
                    return;
                }
                for _ in 0..chunks {
                    tokio::time::sleep(chunk_every).await;
                    if sock.write_all(b"1\r\nx\r\n").await.is_err() {
                        return;
                    }
                }
                let _ = sock.write_all(b"0\r\n\r\n").await;
            });
        }
    });
    port
}

const POST_TO_SILENT_PEER: &str = r#"
export async function main(): Promise<number> {
    const res = await fetch("http://127.0.0.1:{port}/orders", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: 1 }),
    });
    return res.status;
}
"#;

fn post_script(port: u16) -> String {
    POST_TO_SILENT_PEER.replace("{port}", &port.to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_peer_that_never_answers_is_given_up_on() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let port = spawn_silent_peer(seen.clone(), Duration::from_secs(60)).await;

    let started = Instant::now();
    let err = run_with_timeout_secs(&post_script(port), TIMEOUT_SECS)
        .await
        .expect_err("fetch against a peer that never answers must not resolve");
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_secs(TIMEOUT_SECS),
        "gave up after {elapsed:?}, before the configured {TIMEOUT_SECS}s -- \
         the timeout is firing on something other than the wait for a response",
    );
    assert!(
        elapsed < Duration::from_secs(TIMEOUT_SECS + 15),
        "took {elapsed:?} to give up",
    );

    // The message has to stand on its own in a job log: a hung request is
    // otherwise indistinguishable from a slow one.
    assert!(
        err.contains("no response headers arrived within"),
        "error should explain what timed out, got: {err}",
    );
    assert!(
        err.contains("/orders") && err.contains("fetch_response_timeout"),
        "error should name the target and how to change the limit, got: {err}",
    );

    // Without this the test would also pass if the request never went out.
    let body = String::from_utf8_lossy(&seen.lock().await.clone()).to_string();
    assert!(
        body.contains("POST /orders"),
        "peer should have received the request, got: {body}",
    );
}

/// The counterfactual for the test above: with the timeout disabled, the same
/// script against the same peer is still running well past the point the
/// timeout would have fired.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_a_timeout_the_same_request_keeps_running() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    // Closes eventually, so the isolate unwinds instead of pinning a blocking
    // task for the rest of the test binary's life.
    let port = spawn_silent_peer(seen, Duration::from_secs(TIMEOUT_SECS * 3)).await;

    let still_running = tokio::time::timeout(
        Duration::from_secs(TIMEOUT_SECS * 2),
        run_with_timeout_secs(&post_script(port), 0),
    )
    .await
    .is_err();

    assert!(
        still_running,
        "with the timeout disabled the request should still have been pending \
         at {}s -- if it ends on its own, the test above proves nothing",
        TIMEOUT_SECS * 2,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_streaming_body_outliving_the_timeout_is_not_cut_off() {
    // Headers land fast, then the body trickles well past the timeout. A
    // total-duration timeout fails here; that is the point of the test.
    let port =
        spawn_streaming_peer(Duration::from_millis(200), 10, Duration::from_millis(500)).await;

    let ts = format!(
        r#"
export async function main(): Promise<string> {{
    const res = await fetch("http://127.0.0.1:{port}/stream");
    return `${{res.status}}:${{(await res.text()).length}}`;
}}
"#
    );

    let started = Instant::now();
    let out = run_with_timeout_secs(&ts, TIMEOUT_SECS)
        .await
        .expect("a streaming response must not be interrupted");
    let elapsed = started.elapsed();

    assert_eq!(out, "\"200:10\"", "full body should arrive intact");
    assert!(
        elapsed > Duration::from_secs(TIMEOUT_SECS),
        "the transfer ({elapsed:?}) has to outlast the {TIMEOUT_SECS}s timeout \
         for this to be exercising anything",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_slow_but_answering_peer_is_not_cut_off() {
    // A quarter of the window rather than half: CI runs this at
    // --test-threads=10 alongside other V8 isolates, and this margin is what
    // absorbs executor starvation.
    let port = spawn_streaming_peer(
        Duration::from_millis(1_000 * TIMEOUT_SECS / 4),
        1,
        Duration::from_millis(10),
    )
    .await;

    let ts = format!(
        r#"
export async function main(): Promise<number> {{
    const res = await fetch("http://127.0.0.1:{port}/slow");
    await res.text();
    return res.status;
}}
"#
    );

    let out = run_with_timeout_secs(&ts, TIMEOUT_SECS)
        .await
        .expect("a slow but answering peer must not be cut off");
    assert_eq!(out, "200");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_over_large_timeout_does_not_wrap_into_an_instant_one() {
    // deno_web's setTimeout puts its delay through `webidl.converters.long`,
    // which wraps at 32 bits. Unclamped, this ~46-day setting wraps negative and
    // aborts immediately -- asking for a longer leash would kill every fetch.
    let port = spawn_streaming_peer(Duration::from_millis(50), 1, Duration::from_millis(10)).await;

    let ts = format!(
        r#"
export async function main(): Promise<number> {{
    const res = await fetch("http://127.0.0.1:{port}/ok");
    await res.text();
    return res.status;
}}
"#
    );

    let out = run_with_timeout_secs(&ts, 4_000_000)
        .await
        .expect("an over-large timeout must not abort the request");
    assert_eq!(out, "200");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_caller_abort_still_wins_with_its_own_reason() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let port = spawn_silent_peer(seen, Duration::from_secs(60)).await;

    let ts = format!(
        r#"
export async function main(): Promise<string> {{
    const ac = new AbortController();
    setTimeout(() => ac.abort(new Error("caller_abort_marker")), 200);
    try {{
        await fetch("http://127.0.0.1:{port}/probe", {{ signal: ac.signal }});
        return "unexpectedly resolved";
    }} catch (e) {{
        return String((e as Error).message);
    }}
}}
"#
    );

    let out = run_with_timeout_secs(&ts, TIMEOUT_SECS)
        .await
        .expect("script should catch its own abort");
    assert!(
        out.contains("caller_abort_marker"),
        "caller's abort reason should survive being combined with ours, got: {out}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_timeout_is_catchable_as_a_timeout_error() {
    // Scripts that retry on transient failures need to recognise this one;
    // `TimeoutError` matches AbortSignal.timeout()'s reason.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let port = spawn_silent_peer(seen, Duration::from_secs(60)).await;

    let ts = format!(
        r#"
export async function main(): Promise<string> {{
    try {{
        await fetch("http://127.0.0.1:{port}/probe");
        return "unexpectedly resolved";
    }} catch (e) {{
        return (e as Error).name;
    }}
}}
"#
    );

    let out = run_with_timeout_secs(&ts, TIMEOUT_SECS)
        .await
        .expect("script should catch the timeout");
    assert_eq!(out, "\"TimeoutError\"");
}
