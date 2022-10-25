use std::net::SocketAddr;

pub mod error;
pub mod external_ip;
pub mod flows;
pub mod more_serde;
pub mod oauth2;
pub mod scripts;
pub mod users;
pub mod utils;
pub mod variables;
pub mod worker_flow;

pub const DEFAULT_NUM_WORKERS: usize = 3;
pub const DEFAULT_TIMEOUT: i32 = 300;
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;
pub const DEFAULT_MAX_CONNECTIONS: u32 = 100;

#[cfg(feature = "tokio")]
pub async fn shutdown_signal(tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    use std::io;
    use tokio::signal::unix::SignalKind;

    async fn terminate() -> io::Result<()> {
        tokio::signal::unix::signal(SignalKind::terminate())?
            .recv()
            .await;
        Ok(())
    }

    tokio::select! {
        _ = terminate() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
    println!("signal received, starting graceful shutdown");
    let _ = tx.send(());
    Ok(())
}

#[cfg(feature = "prometheus")]
pub async fn serve_metrics(
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), anyhow::Error> {
    let server = tiny_http::Server::http(addr).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    for request in server.incoming_requests() {
        if !rx.is_empty() {
            break;
        }
        let response = tiny_http::Response::from_string(metrics().await?);
        let _ = request.respond(response);
    }
    Ok(())
}

#[cfg(feature = "prometheus")]
async fn metrics() -> Result<String, error::Error> {
    let metric_families = prometheus::gather();
    Ok(prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(anyhow::Error::from)?)
}
