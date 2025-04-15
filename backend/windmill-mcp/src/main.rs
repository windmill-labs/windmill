use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
use windmill_mcp::{runner::Runner, setup_mcp_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (sse_server, router) = setup_mcp_server()?;

    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    tracing::info!(bind = %sse_server.config.bind, "MCP server listening");

    let shutdown_ct = sse_server.config.ct.child_token();
    let main_ct = sse_server.config.ct.clone();

    let service_ct = sse_server.with_service(Runner::new);

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        shutdown_ct.cancelled().await;
        tracing::info!("MCP server shutdown requested");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "Axum server shutdown with error");
        }
        tracing::info!("Axum server task finished");
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Ctrl-C received, cancelling services");
    main_ct.cancel();
    service_ct.cancelled().await;
    tracing::info!("MCP services cancelled, exiting.");
    Ok(())
}
