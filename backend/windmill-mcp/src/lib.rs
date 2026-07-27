//! Windmill MCP (Model Context Protocol) implementation
//!
//! This crate provides:
//! - MCP client for connecting to external MCP servers (used by AI agents)
//! - Common types and utilities for MCP implementations
//! - MCP server types (when `server` feature is enabled)
//! - OAuth support (when `auth` feature is enabled)

// Common types and utilities module
pub mod common;

// Client module
pub mod client;

// Re-export common types at crate root for convenience
pub use common::{
    convert_schema_to_schema_type, is_resource_allowed, parse_mcp_scopes, transform_hub_path,
    transform_path, FlowInfo, HubResponse, HubScriptInfo, ItemSchema, McpScopeConfig, ResourceInfo,
    ResourceType, SchemaType, ScriptInfo, ToolableItem, WorkspaceId,
};

// Re-export client types at crate root for backward compatibility
pub use client::{McpClient, McpResource, McpToolSource};

// Re-export rmcp types for client usage
pub use rmcp::model::Tool as McpTool;

// Server module (when server feature is enabled)
#[cfg(feature = "server")]
pub mod server;

// MCP OAuth client registration (when auth feature is enabled)
#[cfg(feature = "auth")]
pub mod client_registration;

// Re-export rmcp auth types when auth feature is enabled
#[cfg(feature = "auth")]
pub mod oauth {
    //! Re-exports of rmcp auth and oauth2 types for MCP OAuth implementations

    use std::time::Duration;

    pub use rmcp::transport::auth::AuthorizationManager;

    const DEFAULT_OAUTH_HTTP_TIMEOUT: Duration = Duration::from_secs(30);

    pub fn no_redirect_http_client() -> Result<reqwest::Client, reqwest::Error> {
        no_redirect_http_client_with_timeout(DEFAULT_OAUTH_HTTP_TIMEOUT)
    }

    pub(crate) fn no_redirect_http_client_with_timeout(
        timeout: Duration,
    ) -> Result<reqwest::Client, reqwest::Error> {
        reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
    }

    /// Like [`no_redirect_http_client`], but pins DNS to the address the SSRF
    /// guard validated for the request URL so the connect cannot rebind to an
    /// internal IP after the check (TOCTOU). The OAuth DCR/discovery/token
    /// requests target author-controlled URLs and carry secrets, so they must
    /// go through this rather than the unpinned client. `apply_dns_pinning`
    /// lives on windmill-common's reqwest, which this crate resolves at a
    /// different version (via rmcp), so pin directly with the std-typed
    /// host/addrs. Empty `addrs` (IP literal or ALLOW_PRIVATE_MCP_SERVER_URLS)
    /// leaves resolution untouched.
    pub fn no_redirect_http_client_pinned(
        target: &windmill_common::ssrf::ValidatedTarget,
    ) -> Result<reqwest::Client, reqwest::Error> {
        let mut builder = reqwest::Client::builder()
            .timeout(DEFAULT_OAUTH_HTTP_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        if !target.addrs.is_empty() {
            builder = builder.resolve_to_addrs(&target.host, &target.addrs);
        }
        builder.build()
    }

    // Re-export oauth2 types needed for MCP OAuth flow
    pub use oauth2::{
        basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
        RedirectUrl, Scope, TokenUrl,
    };

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            io::Read,
            net::TcpListener,
            thread,
            time::{Duration, Instant},
        };

        #[tokio::test]
        async fn no_redirect_http_client_times_out_stalled_responses() {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();

            let handle = thread::spawn(move || {
                if let Ok((mut stream, _)) = listener.accept() {
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let mut buffer = [0; 1024];
                    let _ = stream.read(&mut buffer);
                    thread::sleep(Duration::from_millis(300));
                }
            });

            let client = no_redirect_http_client_with_timeout(Duration::from_millis(50)).unwrap();
            let started = Instant::now();
            let err = client
                .get(format!("http://{addr}/stall"))
                .send()
                .await
                .expect_err("stalled response should time out");

            assert!(err.is_timeout(), "expected timeout error, got: {err}");
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "stalled request should fail promptly"
            );

            handle.join().unwrap();
        }
    }
}
