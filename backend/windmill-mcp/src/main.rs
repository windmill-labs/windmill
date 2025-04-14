use rmcp::transport::sse_server::{SseServer, SseServerConfig};
mod runner;
use crate::runner::Runner;
use axum::{Router, extract::Extension, http::StatusCode, middleware, response::Response};
use hyper::client::HttpConnector;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower::ServiceBuilder;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

const BIND_ADDRESS: &str = "127.0.0.1:8008";

// Middleware function to log query parameters
async fn log_query_params(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // Log the URI and extract query parameters
    let uri = req.uri();
    if let Some(query) = uri.query() {
        tracing::info!("Request query parameters: {}", query);

        // Parse and log individual query parameters
        let params: Vec<(&str, &str)> = query
            .split('&')
            .filter_map(|kv| {
                let mut parts = kv.split('=');
                match (parts.next(), parts.next()) {
                    (Some(k), Some(v)) => Some((k, v)),
                    _ => None,
                }
            })
            .collect();

        for (key, value) in params {
            tracing::info!("Query param: {}={}", key, value);
        }
    } else {
        tracing::info!("No query parameters in request to {}", uri.path());
    }

    // Continue with the request
    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(config);

    // Apply middleware to log query parameters
    let router = router.layer(middleware::from_fn(log_query_params));

    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;

    let ct = sse_server.config.ct.child_token();

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });

    let ct = sse_server.with_service(Runner::new);

    tokio::signal::ctrl_c().await?;
    ct.cancel();
    Ok(())
}
