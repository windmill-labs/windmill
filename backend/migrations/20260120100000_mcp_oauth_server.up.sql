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
    workspace_id VARCHAR(50) NOT NULL,
    scopes TEXT[] NOT NULL,
    redirect_uri TEXT NOT NULL,
    code_challenge VARCHAR(128),  -- PKCE
    code_challenge_method VARCHAR(10),  -- 'S256' or 'plain'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '10 minutes'
);

CREATE INDEX idx_mcp_oauth_server_code_expires ON mcp_oauth_server_code(expires_at);

-- MCP OAuth refresh tokens for token rotation
CREATE TABLE mcp_oauth_refresh_token (
    id BIGSERIAL PRIMARY KEY,
    refresh_token VARCHAR(64) NOT NULL UNIQUE,
    access_token VARCHAR(64) NOT NULL,
    client_id VARCHAR(255) NOT NULL REFERENCES mcp_oauth_server_client(client_id) ON DELETE CASCADE,
    user_email VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    scopes TEXT[] NOT NULL,
    token_family UUID NOT NULL,  -- Groups tokens from same auth flow for theft detection
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ DEFAULT NULL,  -- For rotation tracking (single-use)
    revoked BOOLEAN NOT NULL DEFAULT FALSE  -- For theft detection
);

CREATE INDEX idx_mcp_oauth_refresh_token_token ON mcp_oauth_refresh_token(refresh_token);
CREATE INDEX idx_mcp_oauth_refresh_token_expires ON mcp_oauth_refresh_token(expires_at);
CREATE INDEX idx_mcp_oauth_refresh_token_family ON mcp_oauth_refresh_token(token_family);