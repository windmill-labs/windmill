-- Add mcp_server_url column to account table for MCP OAuth token refresh
ALTER TABLE account ADD COLUMN mcp_server_url TEXT;

-- Index for looking up accounts by MCP server URL
CREATE INDEX idx_account_mcp_server_url ON account(mcp_server_url) WHERE mcp_server_url IS NOT NULL;
