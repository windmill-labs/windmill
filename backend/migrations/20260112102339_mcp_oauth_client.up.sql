CREATE TABLE mcp_oauth_client (
    mcp_server_url TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    client_secret TEXT,
    client_secret_expires_at TIMESTAMP,
    token_endpoint TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX idx_mcp_oauth_client_expires ON mcp_oauth_client(client_secret_expires_at);

ALTER TABLE account ADD COLUMN mcp_server_url TEXT;
CREATE INDEX idx_account_mcp_server_url ON account(mcp_server_url) WHERE mcp_server_url IS NOT NULL;
