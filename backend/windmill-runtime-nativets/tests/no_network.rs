//! `//no_network` denies every connection, including through the raw ops user
//! code can reach as `Deno.core.ops`, which bypass any JS-level guard. The QUIC
//! endpoint op resolves its hostname before its own permission check, so the
//! deno_net ops must be disabled outright rather than trusted to check first.
//!
//! Hermetic: a loopback listener that counts accepted connections.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use windmill_runtime_nativets::{get_annotation, transpile_ts, PrewarmedIsolate};

async fn run(ts: &str) -> String {
    let js = transpile_ts(ts.to_string()).expect("transpile_ts failed");
    let mut iso = PrewarmedIsolate::spawn(String::new(), js, get_annotation(ts), vec![], None);
    iso.wait_ready().await.expect("isolate failed to pre-warm");
    let res = iso
        .start_execution("{}".to_string())
        .wait()
        .await
        .expect("isolate panicked");
    match res.result {
        Ok(raw) => raw.get().to_string(),
        Err(e) => panic!("script failed: {e}"),
    }
}

async fn spawn_counting_peer(accepted: Arc<AtomicUsize>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            accepted.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf).await;
                let _ = sock
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                    .await;
            });
        }
    });
    port
}

fn script(annotations: &str, port: u16) -> String {
    format!(
        r#"{annotations}
async function attempt(f) {{
  try {{ await f(); return "allowed"; }} catch (e) {{ return String(e); }}
}}
export async function main() {{
  return {{
    fetch: await attempt(() => fetch("http://127.0.0.1:{port}/").then((r) => r.text())),
    data_url: await attempt(() => fetch("data:text/plain,ok").then((r) => r.text())),
    blob_url: await attempt(() => fetch(URL.createObjectURL(new Blob(["ok"]))).then((r) => r.text())),
    raw_fetch: await attempt(() => Deno.core.ops.op_fetch("GET", "http://127.0.0.1:{port}/", [], null, false, null, null)),
    tcp: await attempt(() => Deno.core.ops.op_net_connect_tcp({{ hostname: "127.0.0.1", port: {port} }}, null, null)),
    unix: await attempt(() => Deno.core.ops.op_net_connect_unix("/var/run/docker.sock")),
    quic: await attempt(() => Deno.core.ops.op_quic_endpoint_create({{ hostname: "localhost", port: 0 }}, true)),
  }};
}}
"#
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn no_network_denies_fetch_and_raw_socket_ops() {
    let accepted = Arc::new(AtomicUsize::new(0));
    let port = spawn_counting_peer(accepted.clone()).await;

    let out: serde_json::Value =
        serde_json::from_str(&run(&script("//native\n//no_network", port)).await).unwrap();
    let fetch = out["fetch"].as_str().unwrap();
    assert!(
        fetch.contains("no_network"),
        "fetch was not denied: {fetch}"
    );
    // Neither opens a connection, so neither is network access.
    assert_eq!(out["data_url"], "allowed", "{out}");
    assert_eq!(out["blob_url"], "allowed", "{out}");
    // The global `fetch` only words the refusal; the op under it is what refuses.
    let raw_fetch = out["raw_fetch"].as_str().unwrap();
    assert!(
        raw_fetch.contains("Requires net access"),
        "the fetch op was not denied: {raw_fetch}"
    );
    for key in ["tcp", "unix", "quic"] {
        let msg = out[key].as_str().unwrap();
        assert!(
            msg.contains("op is disabled"),
            "{key} was not disabled: {msg}"
        );
    }
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        0,
        "a connection got through"
    );

    // Without the annotation the same script reaches the peer, so the denial
    // above is the annotation's doing and not a broken harness.
    let out: serde_json::Value =
        serde_json::from_str(&run(&script("//native", port)).await).unwrap();
    assert_eq!(out["fetch"], "allowed", "{out}");
    assert_eq!(out["tcp"], "allowed", "{out}");
    assert!(accepted.load(Ordering::SeqCst) > 0);
}

#[test]
fn only_the_exact_annotation_in_the_leading_block_counts() {
    assert!(get_annotation("//native\n//no_network\n").no_network);
    assert!(get_annotation("// no_network\n").no_network);
    assert!(!get_annotation("//no_networking\n").no_network);
    assert!(!get_annotation("//native\nconst x = 1;\n//no_network\n").no_network);
    // `//native` routing skips blank lines in the header; `//no_network` must
    // too, or a script runs native with the network open.
    assert!(get_annotation("\n//native\n//a comment\n\n//no_network\n").no_network);
}
