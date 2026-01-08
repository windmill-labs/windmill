-- MCP OAuth 2.1 temporary state storage for PKCE flow
CREATE TABLE mcp_oauth_state (
    csrf_token VARCHAR(255) PRIMARY KEY,
    pkce_verifier TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret TEXT,
    token_endpoint TEXT NOT NULL,
    email VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    mcp_server_url TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL DEFAULT (NOW() + INTERVAL '10 minutes')
);

CREATE INDEX idx_mcp_oauth_state_expires ON mcp_oauth_state(expires_at);
