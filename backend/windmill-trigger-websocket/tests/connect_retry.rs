//! End-to-end tests for the websocket trigger connect-retry behavior: a mock
//! TCP server rejects the websocket upgrade with a configurable HTTP status a
//! number of times before completing a real handshake, and the tests assert
//! which failures `get_consumer` retries.
//!
//! This lives in an integration-test binary (own process) because it sets
//! ALLOW_PRIVATE_WEBSOCKET_URLS — the mock server listens on 127.0.0.1, which
//! the SSRF check blocks — and that process-global env var must not leak into
//! the crate's unit tests, which assert loopback URLs are rejected.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, RwLock},
};
use windmill_trigger::{listener::ListeningTrigger, Listener};
use windmill_trigger_websocket::{
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

async fn get_consumer_result(
    lt: &ListeningTrigger<WebsocketConfig>,
    err_message: Arc<RwLock<Option<String>>>,
) -> windmill_common::error::Result<Option<<WebsocketTrigger as Listener>::Consumer>> {
    // The static-URL path of `get_consumer` never touches the DB; a lazy pool
    // satisfies the signature without a running postgres.
    let db: windmill_common::DB =
        sqlx::Pool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap();
    let (_killpill_tx, killpill_rx) = broadcast::channel::<()>(1);
    WebsocketTrigger
        .get_consumer(&db, lt, err_message, killpill_rx)
        .await
}

#[tokio::test]
async fn transient_502s_are_retried_until_the_upgrade_succeeds() {
    std::env::set_var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV, "true");
    let (addr, attempts) = mock_ws_server(2, "502 Bad Gateway").await;
    let lt = trigger(format!("ws://{addr}"), true);
    let err_message = Arc::new(RwLock::new(None));

    let consumer = get_consumer_result(&lt, err_message.clone())
        .await
        .expect("connect should succeed after retries");

    assert!(consumer.is_some(), "expected an established connection");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    // Retry progress was reported through the shared status lock.
    let status = err_message.read().await.clone().unwrap();
    assert!(status.contains("attempt 2/5"), "got status: {status}");
    assert!(status.contains("502"), "got status: {status}");
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
