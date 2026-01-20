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

GRANT ALL ON mcp_oauth_refresh_token TO windmill_user;
GRANT ALL ON mcp_oauth_refresh_token TO windmill_admin;
GRANT ALL ON mcp_oauth_refresh_token_id_seq TO windmill_user;
GRANT ALL ON mcp_oauth_refresh_token_id_seq TO windmill_admin;
