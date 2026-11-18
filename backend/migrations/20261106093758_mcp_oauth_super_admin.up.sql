ALTER TABLE mcp_oauth_server_code ADD COLUMN IF NOT EXISTS super_admin BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE mcp_oauth_refresh_token ADD COLUMN IF NOT EXISTS super_admin BOOLEAN NOT NULL DEFAULT false;
