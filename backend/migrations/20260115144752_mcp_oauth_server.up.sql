-- OAuth server: clients that have registered with Windmill to access MCP
-- Only public clients are supported (PKCE required, no client secrets)
CREATE TABLE mcp_oauth_server_client (
    client_id VARCHAR(255) PRIMARY KEY,
    client_name VARCHAR(255) NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- OAuth server: authorization codes (short-lived, single-use)
CREATE TABLE mcp_oauth_server_code (
    code VARCHAR(64) PRIMARY KEY,
    client_id VARCHAR(255) NOT NULL REFERENCES mcp_oauth_server_client(client_id) ON DELETE CASCADE,
    user_email VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50),  -- NULL = instance-wide access
    scopes TEXT[] NOT NULL,
    redirect_uri TEXT NOT NULL,
    code_challenge VARCHAR(128),  -- PKCE
    code_challenge_method VARCHAR(10),  -- 'S256' or 'plain'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '10 minutes'
);

CREATE INDEX idx_mcp_oauth_server_code_expires ON mcp_oauth_server_code(expires_at);
