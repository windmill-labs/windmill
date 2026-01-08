DROP INDEX IF EXISTS idx_account_mcp_server_url;
ALTER TABLE account DROP COLUMN IF EXISTS mcp_server_url;
