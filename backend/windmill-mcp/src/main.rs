use rmcp::transport::sse_server::SseServer;
mod runner;
use crate::runner::Runner;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

const BIND_ADDRESS: &str = "127.0.0.1:8008";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting MCP server");
    let ct = SseServer::serve(BIND_ADDRESS.parse()?)
        .await?
        .with_service(Runner::new);

    tokio::signal::ctrl_c().await?;
    ct.cancel();
    Ok(())
}
