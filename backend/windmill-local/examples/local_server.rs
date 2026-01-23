//! Example: Run the Windmill local server
//!
//! This demonstrates running a local Windmill server with libSQL/SQLite backend.
//!
//! Run with:
//!     cargo run -p windmill-local --example local_server
//!
//! Test with:
//!     # Health check
//!     curl http://localhost:8000/health
//!
//!     # Run a bash preview (async)
//!     curl -X POST http://localhost:8000/api/w/local/jobs/run/preview \
//!       -H "Content-Type: application/json" \
//!       -d '{"content": "echo Hello World", "language": "bash"}'
//!
//!     # Run a bash preview and wait for result
//!     curl -X POST http://localhost:8000/api/w/local/jobs/run_wait_result/preview \
//!       -H "Content-Type: application/json" \
//!       -d '{"content": "echo 42", "language": "bash"}'
//!
//!     # Run a Python preview
//!     curl -X POST http://localhost:8000/api/w/local/jobs/run_wait_result/preview \
//!       -H "Content-Type: application/json" \
//!       -d '{"content": "def main(x=1): return x * 2", "language": "python3", "args": {"x": 21}}'
//!
//!     # Run a flow preview (linear flow with two steps)
//!     curl -X POST http://localhost:8000/api/w/local/jobs/run_wait_result/preview_flow \
//!       -H "Content-Type: application/json" \
//!       -d '{
//!         "value": {
//!           "modules": [
//!             {"id": "step1", "value": {"type": "rawscript", "language": "bash", "content": "echo 10"}},
//!             {"id": "step2", "value": {"type": "identity"}}
//!           ]
//!         },
//!         "args": {}
//!       }'

use std::net::SocketAddr;
use windmill_local::LocalServer;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("windmill_local=info".parse().unwrap()),
        )
        .init();

    let addr: SocketAddr = "0.0.0.0:8000".parse().unwrap();
    println!("Starting Windmill Local Server on {}", addr);
    println!();
    println!("Test endpoints:");
    println!("  Health:    curl http://localhost:8000/health");
    println!("  Preview:   curl -X POST http://localhost:8000/api/w/local/jobs/run_wait_result/preview \\");
    println!("               -H 'Content-Type: application/json' \\");
    println!("               -d '{{\"content\": \"echo Hello\", \"language\": \"bash\"}}'");
    println!();

    let server = LocalServer::new(addr).await.expect("Failed to create server");
    server.run().await.expect("Server error");
}
