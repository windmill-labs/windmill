use anyhow::Result;
use mcp_rust_sdk::{transport::StdioTransport, Server};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WindmillMCP {
    server: Server,
}

impl WindmillMCP {
    pub fn new() -> Result<Self> {
        let (transport, _) = StdioTransport::new();
        let server = Server::new(transport);
        Ok(Self { server })
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting Windmill MCP server...");
        self.server.start().await?;
        Ok(())
    }

    async fn handle_message(&self, message: Value) -> Result<Value> {
        tracing::info!("Received message: {:?}", message);

        // TODO: Implement proper message handling logic
        // For now, just echo back the message
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let server = WindmillMCP::new();
        assert!(server.is_ok());
    }
}
