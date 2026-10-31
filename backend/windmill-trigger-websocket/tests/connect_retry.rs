//! End-to-end tests for the websocket trigger connection lifecycle against mock
//! TCP servers: which upgrade failures `get_consumer` retries, and when `consume`
//! gives up on an open connection and how it closes it.
//!
//! This lives in an integration-test binary (own process) because it sets
//! ALLOW_PRIVATE_WEBSOCKET_URLS — the mock server listens on 127.0.0.1, which
//! the SSRF check blocks — and that process-global env var must not leak into
//! the crate's unit tests, which assert loopback URLs are rejected.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use futures::StreamExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, oneshot, RwLock},
};
use tokio_tungstenite::tungstenite::Message;
use windmill_trigger::{listener::ListeningTrigger, Listener};
use windmill_trigger_websocket::{
    listener::{CONNECT_RETRY_BASE_DELAY_MS, PING_INTERVAL_MS},
    WebsocketConfig, WebsocketTrigger, ALLOW_PRIVATE_WEBSOCKET_URLS_ENV,
};

/// Mock server: rejects the first `failures` upgrade requests with
/// `status_line` and closes, then completes real websocket handshakes and
/// parks the connection open. Returns the bound address and the
/// connection-attempt counter.
async fn mock_ws_server(
    failures: u32,
    status_line: &'static str,
) -> (std::net::SocketAddr, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicU32::new(0));
    let served = attempts.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = served.fetch_add(1, Ordering::SeqCst);
            if n < failures {
                // Drain the request head, then reject the upgrade.
                let mut reader = BufReader::new(&mut socket);
                let mut line = String::new();
                loop {
                    line.clear();
                    let read = reader.read_line(&mut line).await.unwrap_or(0);
                    if read == 0 || line == "\r\n" {
                        break;
                    }
                }
                socket
                    .write_all(
                        format!("HTTP/1.1 {status_line}\r\nContent-Length: 0\r\n\r\n").as_bytes(),
                    )
                    .await
                    .ok();
            } else if let Ok(ws) = tokio_tungstenite::accept_async(socket).await {
                tokio::spawn(async move {
                    let _open = ws;
                    std::future::pending::<()>().await
                });
            }
        }
    });
    (addr, attempts)
}

fn trigger(url: String, trigger_mode: bool) -> ListeningTrigger<WebsocketConfig> {
    ListeningTrigger {
        path: "f/test/ws".to_string(),
        is_flow: false,
        workspace_id: "test".to_string(),
        edited_by: "test".to_string(),
        permissioned_as: "u/test".to_string(),
        trigger_config: WebsocketConfig {
            url,
            filters: vec![],
            filter_logic: "and".to_string(),
            initial_messages: None,
            url_runnable_args: None,
            can_return_message: false,
            can_return_error_result: false,
            heartbeat: None,
        },
        script_path: "f/test/script".to_string(),
        trigger_mode,
        error_handling: None,
        suspended_mode: false,
    }
}

/// The static-URL path of `get_consumer` never touches the DB, and `consume` only
/// pings the lease, whose failures it logs and ignores: a lazy pool that fails fast
/// satisfies the signatures without a running postgres.
fn unreachable_db() -> windmill_common::DB {
    sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(100))
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap()
}

async fn get_consumer_result(
    lt: &ListeningTrigger<WebsocketConfig>,
    err_message: Arc<RwLock<Option<String>>>,
) -> windmill_common::error::Result<Option<<WebsocketTrigger as Listener>::Consumer>> {
    let db = unreachable_db();
    let (_killpill_tx, killpill_rx) = broadcast::channel::<()>(1);
    WebsocketTrigger
        .get_consumer(&db, lt, err_message, killpill_rx)
        .await
}

#[tokio::test]
async fn transient_errors_are_retried_past_the_fast_attempts_until_the_upgrade_succeeds() {
    std::env::set_var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV, "true");
    CONNECT_RETRY_BASE_DELAY_MS.store(1, Ordering::Relaxed);
    let (addr, attempts) = mock_ws_server(6, "408 Request Timeout").await;
    let lt = trigger(format!("ws://{addr}"), true);
    let err_message = Arc::new(RwLock::new(None));

    let consumer = get_consumer_result(&lt, err_message.clone())
        .await
        .expect("connect should succeed after retries");

    assert!(consumer.is_some(), "expected an established connection");
    assert_eq!(attempts.load(Ordering::SeqCst), 7);
    // Retry progress was reported through the shared status lock.
    let status = err_message.read().await.clone().unwrap();
    assert!(status.contains("attempt 6 failed"), "got status: {status}");
    assert!(status.contains("408"), "got status: {status}");
}

#[tokio::test]
async fn non_transient_http_errors_fail_on_the_first_attempt() {
    std::env::set_var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV, "true");
    let (addr, attempts) = mock_ws_server(u32::MAX, "404 Not Found").await;
    let lt = trigger(format!("ws://{addr}"), true);

    let err = get_consumer_result(&lt, Arc::new(RwLock::new(None)))
        .await
        .expect_err("a 404 upgrade response should not be retried");

    assert!(err.to_string().contains("404"), "got error: {err}");
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn capture_mode_fails_fast_even_on_transient_errors() {
    std::env::set_var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV, "true");
    let (addr, attempts) = mock_ws_server(u32::MAX, "502 Bad Gateway").await;
    let lt = trigger(format!("ws://{addr}"), false);

    let err = get_consumer_result(&lt, Arc::new(RwLock::new(None)))
        .await
        .expect_err("capture mode should surface the first failure immediately");

    assert!(err.to_string().contains("502"), "got error: {err}");
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

/// Accepts one websocket connection. A `responsive` server keeps reading, which is
/// what makes tungstenite answer pings; a silent one never reads, like a peer that
/// vanished. Reports every frame the server reads.
async fn mock_open_server(
    responsive: bool,
) -> (
    std::net::SocketAddr,
    tokio::sync::mpsc::UnboundedReceiver<Message>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (frames_tx, frames_rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        if !responsive {
            return std::future::pending::<()>().await;
        }
        while let Some(Ok(frame)) = ws.next().await {
            let _ = frames_tx.send(frame);
        }
        std::future::pending::<()>().await
    });
    (addr, frames_rx)
}

async fn spawn_consume(
    addr: std::net::SocketAddr,
) -> (tokio::task::JoinHandle<()>, oneshot::Receiver<()>) {
    let lt = trigger(format!("ws://{addr}"), true);
    let consumer = get_consumer_result(&lt, Arc::new(RwLock::new(None)))
        .await
        .unwrap()
        .unwrap();
    let (done_tx, done_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let (_killpill_tx, killpill_rx) = broadcast::channel::<()>(1);
        WebsocketTrigger
            .consume(
                &unreachable_db(),
                consumer,
                &lt,
                Arc::new(RwLock::new(None)),
                killpill_rx,
                None,
            )
            .await;
        let _ = done_tx.send(());
    });
    (handle, done_rx)
}

#[tokio::test]
async fn consume_gives_up_on_a_peer_that_stops_answering() {
    std::env::set_var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV, "true");
    PING_INTERVAL_MS.store(50, Ordering::Relaxed);
    let (addr, _frames) = mock_open_server(false).await;
    let (_handle, done) = spawn_consume(addr).await;

    tokio::time::timeout(std::time::Duration::from_secs(5), done)
        .await
        .expect("consume should return once the peer misses its pongs")
        .unwrap();
}

#[tokio::test]
async fn consume_keeps_a_responsive_peer_and_closes_it_when_stopped() {
    std::env::set_var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV, "true");
    PING_INTERVAL_MS.store(50, Ordering::Relaxed);
    let (addr, mut frames) = mock_open_server(true).await;
    let (handle, mut done) = spawn_consume(addr).await;

    // Well past the 4 missed intervals that drop a silent peer.
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    assert!(
        done.try_recv().is_err(),
        "consume dropped a peer that answers pings"
    );

    // A cancelled consumer (killpill, lost lease) still sends a Close frame.
    handle.abort();
    let closed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(frame) = frames.recv().await {
            if matches!(frame, Message::Close(_)) {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    assert!(closed, "the server never received a Close frame");
}
