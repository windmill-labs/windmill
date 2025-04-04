use rmcp::transport::sse_server::SseServer;
mod runner;
use crate::runner::Runner;
use hyper::client::HttpConnector;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

const BIND_ADDRESS: &str = "127.0.0.1:8008";
const INTERNAL_ADDRESS: &str = "127.0.0.1:8009";

async fn proxy(
    client: Client<HttpConnector>,
    runner: Arc<RwLock<Runner>>,
    req: Request<Body>,
    remote_addr: SocketAddr,
) -> Result<hyper::Response<Body>, hyper::Error> {
    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
    let uri = format!("http://{}{}", INTERNAL_ADDRESS, path);
    let method = req.method().clone();
    let version = req.version();

    tracing::info!(
        "Incoming request from {}: {} {} {:?}",
        remote_addr,
        method,
        req.uri(),
        version
    );
    tracing::info!("Headers: {:?}", req.headers());

    // Log query parameters if present
    if let Some(query) = req.uri().query() {
        tracing::info!("Query parameters: {}", query);
        // parse query parameters
        let query_parts: Vec<&str> = query.split('&').collect();
        for part in query_parts {
            if let Some((key, value)) = part.split_once('=') {
                tracing::info!("Key: {}, Value: {}", key, value);
                if key == "token" {
                    tracing::info!("Key is token");
                    let decoded_value =
                        urlencoding::decode(value).unwrap_or(std::borrow::Cow::Borrowed(value));
                    tracing::info!("Decoded value: {}", decoded_value);
                    let result = runner
                        .write()
                        .unwrap()
                        .add_token(decoded_value.as_ref().to_string(), "123".to_string());
                    match result {
                        Ok(result) => {
                            tracing::info!("Result: {:?}", result);
                        }
                        Err(e) => {
                            tracing::error!("Error: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    let mut proxy_req = Request::builder().method(method.clone()).uri(uri.clone());

    *proxy_req.headers_mut().unwrap() = req.headers().clone();
    let proxy_req = proxy_req.body(req.into_body()).unwrap_or_else(|_| {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    });

    client.request(proxy_req).await
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

    tracing::info!("Starting MCP server");

    let runner = Arc::new(RwLock::new(Runner::new()));
    let runner_clone = runner.clone();

    // Start SSE server on internal port
    let ct = SseServer::serve(INTERNAL_ADDRESS.parse()?)
        .await?
        .with_service(move || runner_clone.read().unwrap().clone());

    // Start proxy server
    let client = Client::new();
    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let client = client.clone();
        let runner = runner.clone();
        let remote_addr = conn.remote_addr();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                proxy(client.clone(), runner.clone(), req, remote_addr)
            }))
        }
    });

    let addr: SocketAddr = BIND_ADDRESS.parse()?;
    let server = Server::bind(&addr).serve(make_svc);
    tracing::info!("Proxy listening on {}", BIND_ADDRESS);

    tokio::select! {
        _ = server => {},
        _ = tokio::signal::ctrl_c() => {
            ct.cancel();
        }
    }

    Ok(())
}
