-- Replace temporary OAuth state table with persistent client registration cache
DROP TABLE IF EXISTS mcp_oauth_state;

-- Cache dynamic client registrations globally (one per MCP server URL)
-- CSRF and PKCE state will be stored in cookies instead
CREATE TABLE mcp_oauth_client (
    mcp_server_url TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    client_secret TEXT,
    client_secret_expires_at TIMESTAMP,
    token_endpoint TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX idx_mcp_oauth_client_expires ON mcp_oauth_client(client_secret_expires_at);
